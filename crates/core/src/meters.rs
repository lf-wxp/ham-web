//! 测量仪表：万用表、驻波表、功率计、天线分析仪等用途速查，
//! 以及驻波 / 反射系数换算与**读数误差的定量模型**（交互演示的数学放在核心，可以单测）。

/// (仪表, 测量对象, 用途)
pub const METERS: &[(&str, &str, &str)] = &[
  (
    "万用表",
    "电压 / 电流 / 电阻 / 通断",
    "最基础仪表，检修与调试必备，可测交直流电压、电阻、电流。",
  ),
  (
    "驻波表（SWR 表）",
    "驻波比 / 正向与反向功率",
    "串接在电台与天线之间，检测天线系统匹配情况。",
  ),
  (
    "功率计",
    "发射功率",
    "测量发射机实际输出功率，常与驻波表合一。",
  ),
  (
    "天线分析仪",
    "阻抗 / 驻波 / 谐振频率",
    "扫频测量天线谐振点与阻抗，调天线利器。",
  ),
  (
    "场强仪",
    "电场强度",
    "测量某处信号场强，用于评估辐射与天线方向图。",
  ),
  ("频率计", "频率", "精确测量信号频率，校准发射机。"),
  (
    "示波器",
    "波形 / 幅度 / 周期",
    "观察信号波形，分析放大、调制与失真。",
  ),
  (
    "假负载",
    "吸收功率",
    "替代天线的纯阻性负载，调机不发信号时使用，避免干扰。",
  ),
  ("频谱分析仪", "频谱", "观察信号的频谱分布，分析杂散与谐波。"),
];

/// 驻波比 → 反射系数（幅度）。
///
/// SWR 1:1 → 0；无穷大 → 1。所有「读数误差」模型都从这里出发：
/// 表头实际读的是**反射系数**，驻波只是它的一种表示。
#[must_use]
pub fn rho_of_swr(swr: f64) -> f64 {
  if swr <= 1.0 {
    return 0.0;
  }
  (swr - 1.0) / (swr + 1.0)
}

/// 反射系数（幅度，< 1）→ 驻波比。
#[must_use]
pub fn swr_of_rho(rho: f64) -> f64 {
  let rho = rho.clamp(0.0, 0.999_999);
  (1.0 + rho) / (1.0 - rho)
}

/// 回波损耗（dB，正值越大匹配越好）。
#[must_use]
pub fn return_loss_db(swr: f64) -> f64 {
  let rho = rho_of_swr(swr);
  if rho <= 0.0 {
    return 60.0; // 完美匹配时回波损耗趋于无穷，这里给个展示用上限。
  }
  -20.0 * rho.log10()
}

/// 馈线掩盖：台站端表头读到的驻波。
///
/// 反射波在馈线里走了两趟，被衰减了 `2 × 单程损耗` 分贝：
/// 台站端看到的反射系数 = 天线端反射系数 × 10^(-L/10)（L 为**单程**损耗，dB）。
/// 因此**表头读数永远不高于天线端的真实驻波**，馈线越差掩盖越多 ——
/// 「表头 1.5」不等于「天线 1.5」。
#[must_use]
pub fn swr_at_meter(swr_antenna: f64, line_loss_db: f64) -> f64 {
  let rho = rho_of_swr(swr_antenna) * 10f64.powf(-line_loss_db / 10.0);
  swr_of_rho(rho)
}

/// 由台站端表头读数反推天线端真实驻波（[`swr_at_meter`] 的逆运算）。
///
/// 反射波在馈线里被衰减了 `2 × 单程损耗` 分贝，所以**真实反射 = 表头反射 × 10^(L/10)**：
/// 表头读到的 3:1，在天线端可能是 3.6（馈线好）到 20+（馈线差）—— 演示页画的就是这条。
#[must_use]
pub fn swr_at_antenna(meter_swr: f64, line_loss_db: f64) -> f64 {
  let rho = rho_of_swr(meter_swr) * 10f64.powf(line_loss_db / 10.0);
  swr_of_rho(rho)
}

/// 方向性 D（dB）的驻波表，读数的可信区间 `(下限, 上限)`。
///
/// 方向性耦合器测反射时，正向功率会以 `10^(-D/20)` 的比例漏进来：
/// 读到的反射系数 = 真实值 ± 漏泄项。真实反射小于漏泄项时，读数被顶高到下限之上 ——
/// 也就是说**低于方向性下限的驻波读不出来**（普通驻波表约 20 dB → 漏泄 0.1 → 完美匹配也读 1.22:1，约 1.2:1 以下不可信）。
#[must_use]
pub fn swr_reading_bounds(swr_true: f64, directivity_db: f64) -> (f64, f64) {
  let rho = rho_of_swr(swr_true);
  let leak = 10f64.powf(-directivity_db / 20.0);
  let lo = (rho - leak).max(0.0);
  let hi = rho + leak;
  (swr_of_rho(lo), swr_of_rho(hi))
}

/// 驻波与功率读数的常见陷阱：`(标题, 说明)`。
pub const SWR_PITFALLS: &[(&str, &str)] = &[
  (
    "驻波 1:1 ≠ 好天线",
    "假负载的驻波也是 1:1，但它几乎不辐射：驻波衡量的是**匹配**，不是效率与方向性。",
  ),
  (
    "长馈线让表头读数「更好看」",
    "馈线损耗同时衰减反射波，台站端读到的驻波比天线端真实值低；馈线越差，掩盖越多。",
  ),
  (
    "方向性不足，小驻波读不出",
    "普通驻波表方向性约 20 dB，约 1.2:1 以下的读数落在方向性下限里，不可信。",
  ),
  (
    "SSB 的功率读数随话音跳",
    "表针显示的是平均功率；SSB 的平均功率只有峰值包络（PEP）的几分之一，且随话音内容起伏。",
  ),
  (
    "天线分析仪怕本地强信号",
    "广播等强场信号进入仪表会叠加到测量信号上，使读数跳动；远离强发射台，必要时用仪表的抗干扰模式。",
  ),
];

/// SOLT 校准的四个步骤：`(校准件, 作用)`。
///
/// SOLT = Short / Open / Load / Thru。前三个反射校准件一起定出**三项误差**
/// （方向性、源匹配、反射跟踪）：负载直接给出方向性，短路 + 开路解出另两项；
/// 直通件修正**传输跟踪**（与负载匹配）。
pub const SOLT_STEPS: &[(&str, &str)] = &[
  (
    "短路 Short",
    "标准短路件：定标反射系数 -1 的参考点，与开路件一起解出源匹配与反射跟踪误差。",
  ),
  (
    "开路 Open",
    "标准开路件：定标 +1 的参考点，配合短路件解出源匹配与反射跟踪误差。",
  ),
  (
    "负载 Load",
    "标准匹配负载：定标 0 反射的参考点，直接测出方向性误差（泄漏进反射通道的那部分正向信号）。",
  ),
  (
    "直通 Thru",
    "两个端口直连：修正传输跟踪与负载匹配，是 S21 增益 / 插损读数的基准。",
  ),
];

#[cfg(test)]
mod tests {
  use super::*;

  fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-3
  }

  #[test]
  fn meters_data_populated() {
    assert!(METERS.len() >= 7);
    for (name, target, usage) in METERS {
      assert!(!name.is_empty());
      assert!(!target.is_empty());
      assert!(!usage.is_empty());
    }
  }

  #[test]
  fn conversions_round_trip_and_match_known_values() {
    // 2:1 → 反射 1/3，回波损耗 9.54 dB。
    let rho = rho_of_swr(2.0);
    assert!(close(rho, 1.0 / 3.0), "{rho}");
    assert!(close(swr_of_rho(rho), 2.0));
    assert!(close(return_loss_db(2.0), 9.542));
    // 1:1 → 反射 0；3:1 → 反射 0.5。
    assert_eq!(rho_of_swr(1.0), 0.0);
    assert!(close(rho_of_swr(3.0), 0.5));
    // 反射 → 驻波：0.5 → 3:1。
    assert!(close(swr_of_rho(0.5), 3.0));
  }

  #[test]
  fn feedline_loss_masks_the_true_swr() {
    // 天线端 3:1（反射 0.5），1 dB 单程损耗：反射 ×10^(-0.1) ≈ 0.397 → 驻波约 2.32。
    let at_meter = swr_at_meter(3.0, 1.0);
    assert!(close(at_meter, 2.318), "{at_meter}");
    // 表头读数永远不高于真实值，且损耗越大掩盖越多。
    for loss in [0.2, 0.5, 1.0, 2.0, 3.0] {
      let masked = swr_at_meter(5.0, loss);
      assert!(masked <= 5.0, "损耗 {loss} dB 时读数 {masked} 高于真实值");
      assert!(masked < swr_at_meter(5.0, loss - 0.1));
    }
    // 完美匹配不受馈线影响。
    assert!(close(swr_at_meter(1.0, 3.0), 1.0));
  }

  #[test]
  fn meter_and_antenna_are_inverses() {
    // 表头读数 ≤ 真实值；两个函数互为逆运算（往返）。
    assert!(
      close(swr_at_antenna(3.0, 0.5), 3.556),
      "表头 3:1、0.5 dB 损耗 → 天线端 3.56"
    );
    assert!(close(swr_at_antenna(3.0, 1.0), 4.398));
    for loss in [0.2, 0.5, 1.0, 2.0, 3.0] {
      for meter in [1.2, 2.0, 3.0, 5.0] {
        let true_swr = swr_at_antenna(meter, loss);
        assert!(true_swr >= meter, "表头 {meter} 不该高于天线端 {true_swr}");
        // 反推出的反射超过 1（表头读数在数学上对应一个「比全反射还大」的天线端）时，
        // 真实情况是驻波趋于无穷 —— 此时往返会被截断，跳过（见 `swr_of_rho` 的 clamp）。
        if rho_of_swr(meter) * 10f64.powf(loss / 10.0) < 1.0 {
          assert!(close(swr_at_meter(true_swr, loss), meter), "往返应一致");
        }
      }
    }
  }

  #[test]
  fn directivity_sets_a_reading_floor() {
    // 20 dB 方向性 → 漏泄 0.1。真实 1.5:1（反射 0.2）→ 读数落在 0.1..0.3 → 驻波 1.22..1.86。
    let (lo, hi) = swr_reading_bounds(1.5, 20.0);
    assert!(close(lo, 1.222), "{lo}");
    assert!(close(hi, 1.857), "{hi}");
    // 真实驻波低于方向性下限时，下限顶到 1:1（读不出）。
    let (lo, _) = swr_reading_bounds(1.05, 20.0);
    assert!(close(lo, 1.0), "{lo}");
    // 真实值落在区间内；方向性越好区间越窄。
    // 注意：驻波是反射系数的非线性函数，反射误差 ±0.01（40 dB 方向性）在高驻波处被放大 ——
    // 3:1（反射 0.5）时读数区间约 2.92..3.08，这正是「高驻波的读数误差更大」的由来。
    let (lo, hi) = swr_reading_bounds(3.0, 40.0);
    assert!(lo <= 3.0 && 3.0 <= hi);
    assert!(close(lo, 2.9216), "{lo}");
    assert!(close(hi, 3.0816), "{hi}");
  }

  #[test]
  fn pitfalls_and_solt_steps_are_populated() {
    assert_eq!(SWR_PITFALLS.len(), 5);
    assert_eq!(SOLT_STEPS.len(), 4);
    for (title, text) in SWR_PITFALLS.iter().chain(SOLT_STEPS.iter()) {
      assert!(!title.is_empty() && !text.is_empty());
    }
    // 顺序即演示顺序：S-O-L-T。
    let initials: Vec<&str> = SOLT_STEPS.iter().map(|(t, _)| *t).collect();
    for (title, word) in initials.iter().zip(["Short", "Open", "Load", "Thru"]) {
      assert!(title.contains(word), "{title} 与 SOLT 顺序不符");
    }
  }
}
