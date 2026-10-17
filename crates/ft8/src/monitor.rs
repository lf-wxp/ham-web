//! FT8 / FT4 的 waterfall 生成（短时傅里叶变换）。
//!
//! 转写自 MIT 许可的 ft8_lib `common/monitor.c`：把采样率 12000 Hz 的音频逐符号
//! 滑窗 + 加 Hann 窗 + 实数 FFT，累积成 `[block][time_osr][freq_osr][bin]` 的幅度谱（0–255，0.5 dB 步进）。

use crate::fft::{Cpx, RealFft};

/// 协议类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Protocol {
  Ft4,
  Ft8,
}

impl Protocol {
  /// 符号周期（秒）。
  pub fn symbol_period(self) -> f32 {
    match self {
      Protocol::Ft4 => 0.048,
      Protocol::Ft8 => 0.160,
    }
  }
  /// 时隙长度（秒）。
  pub fn slot_time(self) -> f32 {
    match self {
      Protocol::Ft4 => 7.5,
      Protocol::Ft8 => 15.0,
    }
  }
}

/// 监控配置。
pub struct MonitorConfig {
  pub f_min: f32,
  pub f_max: f32,
  pub sample_rate: usize,
  pub time_osr: usize,
  pub freq_osr: usize,
  pub protocol: Protocol,
}

/// 幅度谱（0–255，`mag/2 - 120` 还原成 dB）。
pub struct Waterfall {
  pub max_blocks: usize,
  pub num_blocks: usize,
  pub num_bins: usize,
  pub time_osr: usize,
  pub freq_osr: usize,
  pub mag: Vec<u8>,
  pub block_stride: usize,
  pub protocol: Protocol,
}

impl Waterfall {
  /// `(time_offset, time_sub, freq_sub, freq_offset)` 对应的幅度（dB）。
  #[inline]
  pub(crate) fn db_at(
    &self,
    block: isize,
    time_sub: usize,
    freq_sub: usize,
    freq_offset: usize,
  ) -> f32 {
    let idx = (block as usize * self.time_osr + time_sub) * self.freq_osr * self.num_bins
      + freq_sub * self.num_bins
      + freq_offset;
    f32::from(self.mag[idx]) * 0.5 - 120.0
  }
  /// 同上，但直接返回 0–255 的整数值（同步打分用）。
  #[inline]
  pub(crate) fn int_at(
    &self,
    block: isize,
    time_sub: usize,
    freq_sub: usize,
    freq_offset: usize,
  ) -> i32 {
    let idx = (block as usize * self.time_osr + time_sub) * self.freq_osr * self.num_bins
      + freq_sub * self.num_bins
      + freq_offset;
    i32::from(self.mag[idx])
  }
}

/// 监控对象：把音频流累积成 waterfall。
pub struct Monitor {
  pub symbol_period: f32,
  pub min_bin: usize,
  pub max_bin: usize,
  pub block_size: usize,
  pub subblock_size: usize,
  pub nfft: usize,
  window: Vec<f32>,
  last_frame: Vec<f32>,
  /// 加窗后的时域帧与 FFT 输出，逐 subblock 复用（解码一个时隙要跑上百次 FFT，
  /// 每次新分配两个 `Vec` 在 wasm 上是实打实的开销）。
  timedata: Vec<f32>,
  freqdata: Vec<Cpx>,
  fft: RealFft,
  pub wf: Waterfall,
  pub max_mag: f32,
}

impl Monitor {
  /// 按配置初始化。
  ///
  /// `sample_rate`、`time_osr`、`freq_osr` 都会参与除法（`subblock_size`、`nfft`、bin 上限），
  /// 传 0 是调用方的编程错误：这里显式拒绝，免得后面算出 `block_size = 0` 再越界。
  pub fn new(cfg: &MonitorConfig) -> Self {
    assert!(cfg.sample_rate > 0, "采样率必须为正");
    assert!(
      cfg.time_osr > 0 && cfg.freq_osr > 0,
      "time_osr / freq_osr 必须为正"
    );
    let symbol_period = cfg.protocol.symbol_period();
    let slot_time = cfg.protocol.slot_time();
    let block_size = (cfg.sample_rate as f32 * symbol_period) as usize;
    let subblock_size = block_size / cfg.time_osr;
    let nfft = block_size * cfg.freq_osr;
    let fft_norm = 2.0 / nfft as f32;

    // Hann 窗（sin(π·i/N)²）。
    let window: Vec<f32> = (0..nfft)
      .map(|i| {
        let x = (std::f32::consts::PI * i as f32 / nfft as f32).sin();
        fft_norm * x * x
      })
      .collect();

    // 实数 FFT 只给出 nfft/2 + 1 个 bin，且每个源 bin 还要按 freq_osr 再细分：
    // 超过这个上限的 f_max 会让 `src_bin` 越界，所以按窗口本身的能力裁掉。
    let max_src_bin = nfft / (2 * cfg.freq_osr);
    let mut min_bin = (cfg.f_min * symbol_period) as usize;
    let mut max_bin = (cfg.f_max * symbol_period) as usize + 1;
    max_bin = max_bin.min(max_src_bin);
    if min_bin >= max_bin {
      min_bin = max_bin.saturating_sub(1);
    }
    let num_bins = max_bin - min_bin;
    let max_blocks = (slot_time / symbol_period) as usize;
    let block_stride = cfg.time_osr * cfg.freq_osr * num_bins;
    assert!(
      num_bins > 0,
      "f_min / f_max 与采样率不匹配，waterfall 没有任何 bin"
    );

    let wf = Waterfall {
      max_blocks,
      num_blocks: 0,
      num_bins,
      time_osr: cfg.time_osr,
      freq_osr: cfg.freq_osr,
      mag: vec![0u8; max_blocks * block_stride],
      block_stride,
      protocol: cfg.protocol,
    };

    Self {
      symbol_period,
      min_bin,
      max_bin,
      block_size,
      subblock_size,
      nfft,
      window,
      last_frame: vec![0.0f32; nfft],
      timedata: vec![0.0f32; nfft],
      freqdata: vec![Cpx { r: 0.0, i: 0.0 }; nfft / 2 + 1],
      fft: RealFft::new(nfft),
      wf,
      max_mag: -120.0,
    }
  }

  /// 清空累积（准备下一个时隙）。
  pub fn reset(&mut self) {
    self.wf.num_blocks = 0;
    self.max_mag = -120.0;
  }

  /// 处理一个 block（`block_size` 个采样），滑窗 FFT 后写入 waterfall。
  ///
  /// 返回 `false` 表示这一帧被丢弃（长度不是 `block_size`，或时隙已经收满）——调用方
  /// 决定是补齐尾部还是换下一个时隙，而不是在库里静默越界。
  pub fn process(&mut self, frame: &[f32]) -> bool {
    if frame.len() != self.block_size || self.wf.num_blocks >= self.wf.max_blocks {
      return false;
    }
    let mut offset = self.wf.num_blocks * self.wf.block_stride;
    let mut frame_pos = 0;

    for _time_sub in 0..self.wf.time_osr {
      // 移入 subblock_size 个新采样。
      let shift = self.subblock_size;
      self.last_frame.copy_within(shift..self.nfft, 0);
      for pos in (self.nfft - shift)..self.nfft {
        self.last_frame[pos] = frame[frame_pos];
        frame_pos += 1;
      }

      // 加窗 + 实数 FFT（buffer 复用，见 `Monitor` 的字段注释）。
      for i in 0..self.nfft {
        self.timedata[i] = self.window[i] * self.last_frame[i];
      }
      self.fft.fft(&self.timedata, &mut self.freqdata);

      // 提取幅度（dB），缩放到 0–255。
      for freq_sub in 0..self.wf.freq_osr {
        for bin in self.min_bin..self.max_bin {
          let src_bin = bin * self.wf.freq_osr + freq_sub;
          let mag2 = self.freqdata[src_bin].i * self.freqdata[src_bin].i
            + self.freqdata[src_bin].r * self.freqdata[src_bin].r;
          let db = 10.0 * (1e-12 + mag2).log10();
          let scaled = (2.0 * db + 240.0) as i32;
          self.wf.mag[offset] = scaled.clamp(0, 255) as u8;
          offset += 1;
          if db > self.max_mag {
            self.max_mag = db;
          }
        }
      }
    }

    self.wf.num_blocks += 1;
    true
  }
}
