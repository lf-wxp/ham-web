//! 混合基复数 FFT 与实数 FFT。
//!
//! 转写自 BSD-3-Clause 的 [KISS FFT](https://github.com/mborgerding/kissfft)（Copyright (c) 2003-2010 Mark Borgerding）
//! 的 `kiss_fft.c` / `kiss_fftr.c`（浮点版：`C_FIXDIV` 为 NOOP、`HALF_OF` 为 `*0.5`，输出未归一化）。
//!
//! FT8 / FT4 的 `nfft` 是 3840 = 4⁴×3×5 与 1152 = 4³×2×3²，都只用基 2/3/4/5 蝴蝶；
//! 基 >5 的质因子走 `bfly_generic`（p 点 DFT），保持任意长度可用。

/// 复数（f32）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cpx {
  pub r: f32,
  pub i: f32,
}

impl Cpx {
  #[inline]
  fn add(self, o: Cpx) -> Cpx {
    Cpx {
      r: self.r + o.r,
      i: self.i + o.i,
    }
  }
  #[inline]
  fn sub(self, o: Cpx) -> Cpx {
    Cpx {
      r: self.r - o.r,
      i: self.i - o.i,
    }
  }
  #[inline]
  fn mul(self, o: Cpx) -> Cpx {
    Cpx {
      r: self.r * o.r - self.i * o.i,
      i: self.r * o.i + self.i * o.r,
    }
  }
  #[inline]
  fn scale(self, s: f32) -> Cpx {
    Cpx {
      r: self.r * s,
      i: self.i * s,
    }
  }
}

/// 复数 FFT 计划（混合基 Cooley-Tukey）。
pub struct Fft {
  nfft: usize,
  twiddles: Vec<Cpx>,
  factors: Vec<usize>, // p1, m1, p2, m2, ...
}

impl Fft {
  /// 分配一个 `nfft` 点 FFT 计划。`inverse` 为真时是逆变换。
  ///
  /// `nfft` 必须为正：因子分解与递归蝴蝶都按「至少一个点」假设，传 0 是调用方的编程错误。
  pub fn new(nfft: usize, inverse: bool) -> Self {
    assert!(nfft > 0, "FFT 长度必须为正");
    let mut twiddles = Vec::with_capacity(nfft);
    for i in 0..nfft {
      let mut phase = -2.0 * std::f64::consts::PI * i as f64 / nfft as f64;
      if inverse {
        phase = -phase;
      }
      twiddles.push(Cpx {
        r: phase.cos() as f32,
        i: phase.sin() as f32,
      });
    }
    let factors = factor(nfft);
    Self {
      nfft,
      twiddles,
      factors,
    }
  }

  /// 复数 FFT（`fin.len() == nfft`）。
  ///
  /// 长度不符时直接返回、不写输出（调用方给错长度不该变成越界 panic）。
  pub fn fft(&self, fin: &[Cpx], fout: &mut [Cpx]) {
    if fin.len() != self.nfft || fout.len() != self.nfft {
      return;
    }
    self.kf_work(fout, fin, 0, 1, 1, &self.factors, 0);
  }

  /// 递归混合基 FFT 主体。`f_off` 是输入 `fin` 的基偏移，`fstride` 是抽取步长。
  #[allow(clippy::too_many_arguments)]
  fn kf_work(
    &self,
    fout: &mut [Cpx],
    fin: &[Cpx],
    f_off: usize,
    fstride: usize,
    in_stride: usize,
    factors: &[usize],
    idx: usize,
  ) {
    let p = factors[idx];
    let m = factors[idx + 1];

    if m == 1 {
      for (i, o) in fout.iter_mut().enumerate().take(p) {
        *o = fin[f_off + i * fstride * in_stride];
      }
    } else {
      for q in 0..p {
        let lo = q * m;
        self.kf_work(
          &mut fout[lo..lo + m],
          fin,
          f_off + q * fstride * in_stride,
          fstride * p,
          in_stride,
          factors,
          idx + 2,
        );
      }
    }

    // 重组 p 个较小 DFT。
    match p {
      2 => bfly2(fout, fstride, m, self),
      3 => bfly3(fout, fstride, m, self),
      4 => bfly4(fout, fstride, m, self),
      5 => bfly5(fout, fstride, m, self),
      _ => bfly_generic(fout, fstride, m, p, self),
    }
  }
}

/// 因子分解：产出 `p1, m1, p2, m2, ...`（`p[i] * m[i] = m[i-1]`，`m0 = n`）。
fn factor(n: usize) -> Vec<usize> {
  let mut n = n;
  let mut out = Vec::new();
  let floor_sqrt = (n as f64).sqrt().floor() as usize;
  let mut p = 4usize;
  while n > 1 {
    while !n.is_multiple_of(p) {
      p = match p {
        4 => 2,
        2 => 3,
        _ => p + 2,
      };
      if p > floor_sqrt {
        p = n; // 没有更多因子
      }
    }
    n /= p;
    out.push(p);
    out.push(n);
  }
  out
}

fn bfly2(fout: &mut [Cpx], fstride: usize, m: usize, st: &Fft) {
  for i in 0..m {
    let a = fout[i];
    let b = fout[i + m].mul(st.twiddles[i * fstride]);
    fout[i] = a.add(b);
    fout[i + m] = a.sub(b);
  }
}

fn bfly3(fout: &mut [Cpx], fstride: usize, m: usize, st: &Fft) {
  let m2 = 2 * m;
  let epi3 = st.twiddles[fstride * m];
  for i in 0..m {
    let a = fout[i];
    let b = fout[i + m].mul(st.twiddles[i * fstride]);
    let c = fout[i + m2].mul(st.twiddles[2 * i * fstride]);
    let s3 = b.add(c);
    let s0 = b.sub(c);
    fout[i + m] = Cpx {
      r: a.r - s3.r * 0.5,
      i: a.i - s3.i * 0.5,
    };
    let s0 = s0.scale(epi3.i);
    fout[i] = a.add(s3);
    fout[i + m2] = Cpx {
      r: fout[i + m].r + s0.i,
      i: fout[i + m].i - s0.r,
    };
    fout[i + m].r -= s0.i;
    fout[i + m].i += s0.r;
  }
}

fn bfly4(fout: &mut [Cpx], fstride: usize, m: usize, st: &Fft) {
  let m2 = 2 * m;
  let m3 = 3 * m;
  for i in 0..m {
    let a = fout[i];
    let b = fout[i + m].mul(st.twiddles[i * fstride]);
    let c = fout[i + m2].mul(st.twiddles[2 * i * fstride]);
    let d = fout[i + m3].mul(st.twiddles[3 * i * fstride]);
    let s5 = a.sub(c);
    let s3 = b.add(d);
    let s4 = b.sub(d);
    let ac = a.add(c); // C 里 Fout[0] 先 += scratch[1]（即 c）
    fout[i + m2] = ac.sub(s3);
    fout[i] = ac.add(s3);
    // 前向：Fout[m] = s5 + i*s4；Fout[m3] = s5 - i*s4。
    fout[i + m] = Cpx {
      r: s5.r + s4.i,
      i: s5.i - s4.r,
    };
    fout[i + m3] = Cpx {
      r: s5.r - s4.i,
      i: s5.i + s4.r,
    };
  }
}

fn bfly5(fout: &mut [Cpx], fstride: usize, m: usize, st: &Fft) {
  let ya = st.twiddles[fstride * m];
  let yb = st.twiddles[fstride * 2 * m];
  for u in 0..m {
    let a = fout[u];
    let b = fout[u + m].mul(st.twiddles[u * fstride]);
    let c = fout[u + 2 * m].mul(st.twiddles[2 * u * fstride]);
    let d = fout[u + 3 * m].mul(st.twiddles[3 * u * fstride]);
    let e = fout[u + 4 * m].mul(st.twiddles[4 * u * fstride]);

    let s7 = b.add(e);
    let s10 = b.sub(e);
    let s8 = c.add(d);
    let s9 = c.sub(d);

    fout[u] = a.add(s7).add(s8);

    let s5 = Cpx {
      r: a.r + s7.r * ya.r + s8.r * yb.r,
      i: a.i + s7.i * ya.r + s8.i * yb.r,
    };
    let s6 = Cpx {
      r: s10.i * ya.i + s9.i * yb.i,
      i: -s10.r * ya.i - s9.r * yb.i,
    };
    fout[u + m] = s5.sub(s6);
    fout[u + 4 * m] = s5.add(s6);

    let s11 = Cpx {
      r: a.r + s7.r * yb.r + s8.r * ya.r,
      i: a.i + s7.i * yb.r + s8.i * ya.r,
    };
    let s12 = Cpx {
      r: -s10.i * yb.i + s9.i * ya.i,
      i: s10.r * yb.i - s9.r * ya.i,
    };
    fout[u + 2 * m] = s11.add(s12);
    fout[u + 3 * m] = s11.sub(s12);
  }
}

/// 基 >5 的通用蝴蝶：p 点 DFT（不追求最优，只保证正确）。
fn bfly_generic(fout: &mut [Cpx], fstride: usize, m: usize, p: usize, st: &Fft) {
  for u in 0..m {
    let mut scratch = Vec::with_capacity(p);
    for q1 in 0..p {
      scratch.push(fout[u + q1 * m]);
    }
    for q1 in 0..p {
      let k = u + q1 * m;
      let mut acc = scratch[0];
      for (q, &s) in scratch.iter().enumerate().skip(1) {
        let twidx = (q * fstride * k) % st.nfft;
        acc = acc.add(s.mul(st.twiddles[twidx]));
      }
      fout[k] = acc;
    }
  }
}

/// 实数 FFT（输出 `nfft/2 + 1` 个 bin，未归一化）。
pub struct RealFft {
  substate: Fft,
  super_twiddles: Vec<Cpx>,
  /// 打包缓冲与子变换输出，逐帧复用（每帧新分配两个 `Vec` 是 wasm 上的主要多余开销）。
  packed: Vec<Cpx>,
  tmp: Vec<Cpx>,
}

impl RealFft {
  /// 分配一个 `nfft` 点实数 FFT（`nfft` 必须为偶数）。
  pub fn new(nfft: usize) -> Self {
    assert!(nfft.is_multiple_of(2), "实数 FFT 的长度必须为偶数");
    let ncfft = nfft / 2;
    let substate = Fft::new(ncfft, false);
    let mut super_twiddles = Vec::with_capacity(ncfft / 2);
    for i in 0..ncfft / 2 {
      let phase = -std::f64::consts::PI * ((i + 1) as f64 / ncfft as f64 + 0.5);
      super_twiddles.push(Cpx {
        r: phase.cos() as f32,
        i: phase.sin() as f32,
      });
    }
    Self {
      substate,
      super_twiddles,
      packed: vec![Cpx { r: 0.0, i: 0.0 }; ncfft],
      tmp: vec![Cpx { r: 0.0, i: 0.0 }; ncfft],
    }
  }

  /// 实数 FFT：`timedata.len() == nfft`，`freqdata` 输出 `nfft/2 + 1` 个 bin。
  ///
  /// 长度不符时直接返回（不写输出）。
  pub fn fft(&mut self, timedata: &[f32], freqdata: &mut [Cpx]) {
    let n = self.substate.nfft; // = nfft / 2
    if timedata.len() != 2 * n || freqdata.len() != n + 1 {
      return;
    }

    // 把两个实信号打包进复数的实/虚部。
    for i in 0..n {
      self.packed[i] = Cpx {
        r: timedata[2 * i],
        i: timedata[2 * i + 1],
      };
    }
    let RealFft {
      substate,
      super_twiddles,
      packed,
      tmp,
    } = self;
    substate.fft(packed, tmp);

    // DC 与 Nyquist（浮点版 C_FIXDIV 为 NOOP，不除 2）。
    freqdata[0] = Cpx {
      r: tmp[0].r + tmp[0].i,
      i: 0.0,
    };
    freqdata[n] = Cpx {
      r: tmp[0].r - tmp[0].i,
      i: 0.0,
    };

    for k in 1..=n / 2 {
      let fpk = tmp[k];
      let fpnk = Cpx {
        r: tmp[n - k].r,
        i: -tmp[n - k].i,
      };
      let f1k = fpk.add(fpnk);
      let f2k = fpk.sub(fpnk);
      let tw = f2k.mul(super_twiddles[k - 1]);
      freqdata[k] = Cpx {
        r: (f1k.r + tw.r) * 0.5,
        i: (f1k.i + tw.i) * 0.5,
      };
      freqdata[n - k] = Cpx {
        r: (f1k.r - tw.r) * 0.5,
        i: (tw.i - f1k.i) * 0.5,
      };
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  /// 独立 naive DFT（无 FFT 加速），用于对拍。
  fn naive_dft(x: &[Cpx]) -> Vec<Cpx> {
    let n = x.len();
    (0..n)
      .map(|k| {
        let mut acc = Cpx { r: 0.0, i: 0.0 };
        for (t, &v) in x.iter().enumerate() {
          let phase = -2.0 * std::f64::consts::PI * k as f64 * t as f64 / n as f64;
          let w = Cpx {
            r: phase.cos() as f32,
            i: phase.sin() as f32,
          };
          acc = acc.add(v.mul(w));
        }
        acc
      })
      .collect()
  }

  #[test]
  fn complex_fft_matches_naive_dft_for_ft8_sizes() {
    for &n in &[3840usize, 1152, 256, 512] {
      let input: Vec<Cpx> = (0..n)
        .map(|i| {
          let x = (i as f32 * 0.7).sin() + (i as f32 * 0.13).cos();
          Cpx { r: x, i: -x * 0.3 }
        })
        .collect();
      let fft = Fft::new(n, false);
      let mut got = vec![Cpx { r: 0.0, i: 0.0 }; n];
      fft.fft(&input, &mut got);
      let expect = naive_dft(&input);
      for k in 0..n {
        let (g, e) = (got[k], expect[k]);
        // f32 的 FFT 在大 N 下有浮点累积误差，阈值放 1e-2。
        assert!(
          (g.r - e.r).abs() < 1e-2,
          "n={n} k={k} re got={} want={}",
          g.r,
          e.r
        );
        assert!(
          (g.i - e.i).abs() < 1e-2,
          "n={n} k={k} im got={} want={}",
          g.i,
          e.i
        );
      }
    }
  }

  #[test]
  fn real_fft_matches_naive_dft_of_the_real_signal() {
    let n = 3840;
    let signal: Vec<f32> = (0..n)
      .map(|i| (i as f32 * 0.31).sin() + (i as f32 * 0.07).cos())
      .collect();
    let mut rfft = RealFft::new(n);
    let mut freq = vec![Cpx { r: 0.0, i: 0.0 }; n / 2 + 1];
    rfft.fft(&signal, &mut freq);

    let cinput: Vec<Cpx> = signal.iter().map(|&s| Cpx { r: s, i: 0.0 }).collect();
    let expect = naive_dft(&cinput);
    for k in 0..=n / 2 {
      let (g, e) = (freq[k], expect[k]);
      assert!(
        (g.r - e.r).abs() < 1e-2,
        "k={k} re got={} want={}",
        g.r,
        e.r
      );
      assert!(
        (g.i - e.i).abs() < 1e-2,
        "k={k} im got={} want={}",
        g.i,
        e.i
      );
    }
  }
}
