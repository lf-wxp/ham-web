//! 占空比与平均功率：发射峰值包络功率（PEP）折算成平均功率，用于射频暴露的时间平均估算。
//!
//! 算法与数据来自 FCC OET Bulletin 65 **Supplement B**（业余无线电）：
//!
//! - 表 2：常见模式的占空比因子（话音 SSB 20% / 50%、CW 40%、FM 与数据 100%）；
//! - 平均功率 = PEP × 模式占空比 × 平均时间窗内的最坏发射时间占比
//!   （受控暴露的平均时间 6 分钟，不受控 30 分钟，窗口是**滑动**的）；
//! - 表 32：距离折算系数 = 占空比的平方根（功率密度 ∝ P / R²）。
//!
//! 它只算「功率怎么平均」，**不判断是否合规**：合规还要看天线增益、距离、频段限值与
//! 地面反射，那要查 OET-65 的附录表格或做专门的暴露评估。

/// 一个模式的占空比因子。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ModeDuty {
  pub id: &'static str,
  /// 界面显示名（中文，走 `t()` 反查词条）。
  pub label: &'static str,
  /// 占空比因子（0–1）。
  pub duty: f64,
}

/// OET-65B 表 2 的模式（外加正文里的「按下不放」100%）。
pub const MODES: &[ModeDuty] = &[
  ModeDuty {
    id: "ssb",
    label: "SSB 通话（无语音压缩）",
    duty: 0.2,
  },
  ModeDuty {
    id: "ssb-proc",
    label: "SSB 通话（重度语音压缩）",
    duty: 0.5,
  },
  ModeDuty {
    id: "cw",
    label: "CW 通联（电键通断）",
    duty: 0.4,
  },
  ModeDuty {
    id: "fm",
    label: "调频话音 FM",
    duty: 1.0,
  },
  ModeDuty {
    id: "fsk",
    label: "FSK / RTTY",
    duty: 1.0,
  },
  ModeDuty {
    id: "afsk",
    label: "AFSK 数据（SSB 声卡数据模式）",
    duty: 1.0,
  },
  ModeDuty {
    id: "keydown",
    label: "连续载波 / 按住不放",
    duty: 1.0,
  },
];

/// 表 32 的行：（占空比，距离折算系数）。表里的系数是四舍五入到两位的。
///
/// 公告里标「66%」和「33%」的两行实际是 2/3 与 1/3：√0.66 = 0.81，而印出来的是 0.82，
/// 只有 √(2/3) = 0.816 才对得上；「33%」同理（√0.33 = 0.57，印的是 0.58 = √(1/3)）。
pub const DISTANCE_TABLE: &[(f64, f64)] = &[
  (0.75, 0.87),
  (2.0 / 3.0, 0.82),
  (0.50, 0.71),
  (1.0 / 3.0, 0.58),
  (0.25, 0.50),
  (0.10, 0.32),
];

/// 受控 / 不受控暴露的平均时间（分钟）。
pub const CONTROLLED_MIN: f64 = 6.0;
pub const UNCONTROLLED_MIN: f64 = 30.0;

/// 发射时间怎么给。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Pattern {
  /// 直接给「平均窗内发射了多少时间」（OET-65B 算例的写法）。
  InWindow { on_s: f64, window_s: f64 },
  /// 周期性「发 x 秒、停 y 秒」，按最坏的滑动窗口算。
  Periodic {
    on_s: f64,
    off_s: f64,
    window_s: f64,
  },
  /// 脉冲：脉宽 × 重复频率（包络恒定的脉冲载波）。
  Pulsed { width_s: f64, rate_hz: f64 },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Input {
  /// 峰值包络功率（W）。
  pub pep_w: f64,
  /// 模式占空比因子（0 < d ≤ 1）。
  pub mode_duty: f64,
  pub pattern: Pattern,
}

/// 输入不合法的原因（界面把它们翻成各语言的提示）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DutyError {
  /// 功率不是正的有限数。
  Power,
  /// 模式占空比不在 (0, 1]。
  ModeDuty,
  /// 时间必须是正的有限数。
  Time,
  /// 窗内发射时间比窗口还长。
  OnLongerThanWindow,
  /// 脉冲占空比（脉宽 × 重复频率）超过 100%：脉冲重叠了。
  PulseOverlap,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Output {
  /// 窗内最坏发射时间占比（0–1）。
  pub on_fraction: f64,
  /// 总占空比 = 模式因子 × 发射时间占比。
  pub total_duty: f64,
  pub average_w: f64,
  pub average_dbm: f64,
  /// 距离折算系数 = √总占空比：保持同一功率密度时，允许的距离可缩到原来的这个倍数。
  pub distance_factor: f64,
}

fn pos(x: f64) -> bool {
  x.is_finite() && x > 0.0
}

/// 周期性「发 `on`、停 `off`」在**任意**长 `window` 的滑动窗口里最多发射多久（同单位）。
///
/// 窗口 = q 个整周期 + 余量 r：最坏情况是窗口恰好从一段发射的起点开始，
/// 发射时间 = q·on + min(on, r)。
#[must_use]
pub fn worst_on_time(on: f64, off: f64, window: f64) -> f64 {
  if window <= on {
    return window;
  }
  let period = on + off;
  if !period.is_finite() {
    return on.min(window);
  }
  let r = window % period;
  let fraction = on / period;
  (window * fraction + on.min(r) - r * fraction).clamp(0.0, window)
}

/// 计算平均功率。
///
/// # Errors
///
/// 输入非有限 / 非正、窗内发射时间超过窗口、脉冲占空比超过 100% 时返回对应的 [`DutyError`]。
pub fn average(input: &Input) -> Result<Output, DutyError> {
  if !pos(input.pep_w) {
    return Err(DutyError::Power);
  }
  if !(input.mode_duty.is_finite() && input.mode_duty > 0.0 && input.mode_duty <= 1.0) {
    return Err(DutyError::ModeDuty);
  }
  let on_fraction = match input.pattern {
    Pattern::InWindow { on_s, window_s } => {
      if !(on_s.is_finite() && on_s >= 0.0) || !pos(window_s) {
        return Err(DutyError::Time);
      }
      if on_s > window_s {
        return Err(DutyError::OnLongerThanWindow);
      }
      on_s / window_s
    }
    Pattern::Periodic {
      on_s,
      off_s,
      window_s,
    } => {
      if !pos(on_s) || !(off_s.is_finite() && off_s >= 0.0) || !pos(window_s) {
        return Err(DutyError::Time);
      }
      worst_on_time(on_s, off_s, window_s) / window_s
    }
    Pattern::Pulsed { width_s, rate_hz } => {
      if !pos(width_s) || !pos(rate_hz) {
        return Err(DutyError::Time);
      }
      let d = width_s * rate_hz;
      if d > 1.0 + 1e-12 {
        return Err(DutyError::PulseOverlap);
      }
      d.min(1.0)
    }
  };
  let total_duty = input.mode_duty * on_fraction;
  let average_w = input.pep_w * total_duty;
  Ok(Output {
    on_fraction,
    total_duty,
    average_w,
    average_dbm: if average_w > 0.0 {
      10.0 * average_w.log10() + 30.0
    } else {
      f64::NEG_INFINITY
    },
    distance_factor: total_duty.sqrt(),
  })
}

#[cfg(test)]
mod tests {
  use super::*;

  fn run(pep: f64, mode: f64, pattern: Pattern) -> Output {
    average(&Input {
      pep_w: pep,
      mode_duty: mode,
      pattern,
    })
    .unwrap()
  }

  fn near(a: f64, b: f64, tol: f64) {
    assert!((a - b).abs() <= tol, "{a} vs {b}");
  }

  /// OET-65B 里的算例，一字不改（单位换成秒）。
  #[test]
  fn the_bulletins_own_worked_examples() {
    // 1500 W PEP SSB（无压缩，20%），6 分钟里发 4 分钟、30 分钟里发 20 分钟 → 200 W。
    let w = |on, win| Pattern::InWindow {
      on_s: on * 60.0,
      window_s: win * 60.0,
    };
    near(run(1500.0, 0.2, w(4.0, 6.0)).average_w, 200.0, 1e-9);
    near(run(1500.0, 0.2, w(20.0, 30.0)).average_w, 200.0, 1e-9);
    // 500 W CW pileup：每两分钟发 15 秒，6 分钟里 45 秒、30 分钟里 225 秒 → 25 W。
    let s = |on, win| Pattern::InWindow {
      on_s: on,
      window_s: win,
    };
    near(run(500.0, 0.4, s(45.0, 360.0)).average_w, 25.0, 1e-9);
    near(run(500.0, 0.4, s(225.0, 1800.0)).average_w, 25.0, 1e-9);
    // 250 W FM：6 分钟里最坏发 5 分钟 → 208.3 W；30 分钟里 15 分钟 → 125 W。
    near(run(250.0, 1.0, w(5.0, 6.0)).average_w, 208.333_333, 1e-5);
    near(run(250.0, 1.0, w(15.0, 30.0)).average_w, 125.0, 1e-9);
  }

  #[test]
  fn periodic_worst_case_reproduces_the_bulletins_sliding_window_examples() {
    let p = |on_min: f64, off_min: f64, win_min: f64| Pattern::Periodic {
      on_s: on_min * 60.0,
      off_s: off_min * 60.0,
      window_s: win_min * 60.0,
    };
    // 7 分钟发 / 7 分钟停：6 分钟窗口里最坏是连续 6 分钟 → 300 W；30 分钟窗口里 16 分钟（0.53）→ 159 W。
    let a = run(1500.0, 0.2, p(7.0, 7.0, 6.0));
    near(a.on_fraction, 1.0, 1e-12);
    near(a.average_w, 300.0, 1e-9);
    let b = run(1500.0, 0.2, p(7.0, 7.0, 30.0));
    near(b.on_fraction, 16.0 / 30.0, 1e-12);
    near(b.average_w, 160.0, 1e-9); // 公告取两位小数 0.53 → 159 W
    near(1500.0 * 0.2 * 0.53, 159.0, 1e-9);
    // CW pileup：每 120 秒发 15 秒。
    let c = Pattern::Periodic {
      on_s: 15.0,
      off_s: 105.0,
      window_s: 360.0,
    };
    near(run(500.0, 0.4, c).average_w, 25.0, 1e-9);
    // FM 5 分钟发 / 5 分钟停：6 分钟窗口 5/6、30 分钟窗口 15/30。
    near(
      run(250.0, 1.0, p(5.0, 5.0, 6.0)).average_w,
      208.333_333,
      1e-5,
    );
    near(run(250.0, 1.0, p(5.0, 5.0, 30.0)).average_w, 125.0, 1e-9);
    // 2 分钟发 / 2 分钟停，6 分钟窗口：4/6 → 200 W。
    near(run(1500.0, 0.2, p(2.0, 2.0, 6.0)).average_w, 200.0, 1e-9);
  }

  #[test]
  fn the_bulletin_scales_its_six_minute_pattern_to_thirty_minutes_but_a_strict_cycle_does_not() {
    // 公告对「发 2 分 / 停 2 分 / 再发 2 分」在 30 分钟窗口里写的是 20 分钟（把 6 分钟里的 4/6 原样放大）。
    // 严格按 2 发 2 停循环，任意 30 分钟里最多只有 16 分钟在发射 → 160 W，而不是 200 W。
    // 两者都是对的，前提不同：这里按字面的周期循环算，要照公告的 200 W 就用「窗内发射时间」模式。
    let strict = run(
      1500.0,
      0.2,
      Pattern::Periodic {
        on_s: 120.0,
        off_s: 120.0,
        window_s: 1800.0,
      },
    );
    near(strict.average_w, 160.0, 1e-9);
    let bulletin = run(
      1500.0,
      0.2,
      Pattern::InWindow {
        on_s: 1200.0,
        window_s: 1800.0,
      },
    );
    near(bulletin.average_w, 200.0, 1e-9);
  }

  #[test]
  fn worst_on_time_matches_a_brute_force_scan_of_every_window_position() {
    // 独立实现：以 1/20 单位为步长把窗口起点扫一遍，取最大发射时间。
    fn brute(on: f64, off: f64, window: f64) -> f64 {
      let period = on + off;
      let step = 0.05;
      let mut best: f64 = 0.0;
      let mut s = 0.0;
      while s < period {
        // 窗口 [s, s+window] 与「发射」区间 [k·period, k·period+on] 的交长。
        let mut total = 0.0;
        let mut k = (s / period).floor();
        loop {
          let a = k * period;
          if a > s + window {
            break;
          }
          let lo = a.max(s);
          let hi = (a + on).min(s + window);
          if hi > lo {
            total += hi - lo;
          }
          k += 1.0;
        }
        best = best.max(total);
        s += step;
      }
      best
    }
    for &(on, off, window) in &[
      (7.0, 7.0, 6.0),
      (7.0, 7.0, 30.0),
      (2.0, 2.0, 6.0),
      (2.0, 2.0, 30.0),
      (1.0, 9.0, 6.0),
      (3.0, 1.0, 6.0),
      (3.0, 1.0, 30.0),
      (5.5, 2.5, 13.0),
      (0.25, 1.75, 6.0),
      (4.0, 0.0, 6.0),
    ] {
      near(worst_on_time(on, off, window), brute(on, off, window), 1e-6);
    }
  }

  #[test]
  fn never_more_than_the_window_or_the_on_time_available() {
    for (on, off, win) in [(10.0, 0.0, 6.0), (100.0, 1.0, 6.0), (1.0, 100.0, 0.5)] {
      let t = worst_on_time(on, off, win);
      assert!(t <= win + 1e-12 && t >= 0.0, "{on} {off} {win} -> {t}");
    }
    // 一直发（off = 0）：整个窗口都在发射。
    near(worst_on_time(5.0, 0.0, 6.0), 6.0, 1e-12);
    // 窗口比一次发射还短：最多就是窗口本身。
    near(worst_on_time(100.0, 100.0, 6.0), 6.0, 1e-12);
    // 窗口比一个周期里的发射段短、又恰好落在停歇里也不会更多：上界是窗口。
    near(worst_on_time(1.0, 100.0, 0.5), 0.5, 1e-12);
  }

  #[test]
  fn table_two_and_table_thirty_two_are_what_the_bulletin_prints() {
    let d = |id: &str| MODES.iter().find(|m| m.id == id).unwrap().duty;
    assert_eq!(
      [
        d("ssb"),
        d("ssb-proc"),
        d("cw"),
        d("fm"),
        d("fsk"),
        d("afsk"),
        d("keydown")
      ],
      [0.2, 0.5, 0.4, 1.0, 1.0, 1.0, 1.0]
    );
    // 表 32：系数就是占空比的平方根，公告四舍五入到两位。
    for &(duty, factor) in DISTANCE_TABLE {
      // 印出的系数是四舍五入到两位的：与精确平方根相差不超过半个末位。
      near(f64::sqrt(duty), factor, 0.005);
      let out = run(
        100.0,
        1.0,
        Pattern::InWindow {
          on_s: duty * 100.0,
          window_s: 100.0,
        },
      );
      near(out.distance_factor, f64::sqrt(duty), 1e-12);
    }
    // 公告的算例：4.3 m × 0.71 = 3 m。
    near(4.3 * 0.71, 3.053, 1e-3);
  }

  #[test]
  fn pulsed_and_dbm() {
    // 1 µs 脉宽、1 kHz → 0.1%；10 kW 峰值 → 10 W 平均 → 40 dBm。
    let o = run(
      10_000.0,
      1.0,
      Pattern::Pulsed {
        width_s: 1e-6,
        rate_hz: 1000.0,
      },
    );
    near(o.total_duty, 1e-3, 1e-15);
    near(o.average_w, 10.0, 1e-9);
    near(o.average_dbm, 40.0, 1e-9);
    // 占空比正好 100%（脉冲首尾相接）可以；超过则脉冲重叠。
    let full = run(
      5.0,
      1.0,
      Pattern::Pulsed {
        width_s: 1e-3,
        rate_hz: 1000.0,
      },
    );
    near(full.on_fraction, 1.0, 1e-12);
    assert_eq!(
      average(&Input {
        pep_w: 5.0,
        mode_duty: 1.0,
        pattern: Pattern::Pulsed {
          width_s: 2e-3,
          rate_hz: 1000.0
        }
      }),
      Err(DutyError::PulseOverlap)
    );
    // 0 发射 → 平均功率 0，dBm 为 −∞，距离系数 0，不 panic。
    let none = run(
      100.0,
      0.4,
      Pattern::InWindow {
        on_s: 0.0,
        window_s: 360.0,
      },
    );
    assert_eq!((none.average_w, none.distance_factor), (0.0, 0.0));
    assert!(none.average_dbm.is_infinite() && none.average_dbm < 0.0);
  }

  #[test]
  fn rejects_nonsense_with_a_specific_reason() {
    let base = Input {
      pep_w: 100.0,
      mode_duty: 0.4,
      pattern: Pattern::InWindow {
        on_s: 60.0,
        window_s: 360.0,
      },
    };
    let with = |f: fn(&mut Input)| {
      let mut i = base;
      f(&mut i);
      average(&i)
    };
    assert!(average(&base).is_ok());
    assert_eq!(with(|i| i.pep_w = 0.0), Err(DutyError::Power));
    assert_eq!(with(|i| i.pep_w = -5.0), Err(DutyError::Power));
    assert_eq!(with(|i| i.pep_w = f64::NAN), Err(DutyError::Power));
    assert_eq!(with(|i| i.pep_w = f64::INFINITY), Err(DutyError::Power));
    assert_eq!(with(|i| i.mode_duty = 0.0), Err(DutyError::ModeDuty));
    assert_eq!(with(|i| i.mode_duty = 1.5), Err(DutyError::ModeDuty));
    assert_eq!(with(|i| i.mode_duty = f64::NAN), Err(DutyError::ModeDuty));
    assert_eq!(
      with(|i| i.pattern = Pattern::InWindow {
        on_s: 400.0,
        window_s: 360.0
      }),
      Err(DutyError::OnLongerThanWindow)
    );
    assert_eq!(
      with(|i| i.pattern = Pattern::InWindow {
        on_s: -1.0,
        window_s: 360.0
      }),
      Err(DutyError::Time)
    );
    assert_eq!(
      with(|i| i.pattern = Pattern::InWindow {
        on_s: 1.0,
        window_s: 0.0
      }),
      Err(DutyError::Time)
    );
    assert_eq!(
      with(|i| i.pattern = Pattern::Periodic {
        on_s: 0.0,
        off_s: 1.0,
        window_s: 360.0
      }),
      Err(DutyError::Time)
    );
    assert_eq!(
      with(|i| i.pattern = Pattern::Periodic {
        on_s: 1.0,
        off_s: -1.0,
        window_s: 360.0
      }),
      Err(DutyError::Time)
    );
    assert_eq!(
      with(|i| i.pattern = Pattern::Pulsed {
        width_s: 0.0,
        rate_hz: 1.0
      }),
      Err(DutyError::Time)
    );
  }
}
