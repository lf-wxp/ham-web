//! 链路预算纯计算内核：自由空间损耗与接收功率。
//!
//! 公式是业余无线电的通用口径（Friis 自由空间公式的 dB 形式）：
//!
//! ```text
//! FSPL(dB) = 20·lg(f_MHz) + 20·lg(d_km) + 32.45
//! P_rx(dBm) = P_tx(dBm) + G_tx(dBi) + G_rx(dBi) − FSPL − L_extra(dB)
//! ```
//!
//! 简化模型：不考虑极化的失配损耗、地面反射与多径（这些属于传播模型，
//! 见 `propagation`）；常数 32.45 对应「频率 MHz、距离 km」的组合。
//! 开放 API（`/api/v1/link-budget`）与前端共用的单一实现。

/// FSPL 常数：`20·lg(4π/c)` 折算到「MHz + km」的量纲。
pub const FSPL_CONSTANT_DB: f64 = 32.45;

/// 自由空间路径损耗（dB）：`freq_mhz` 与 `dist_km` 都必须 > 0。
#[must_use]
pub fn free_space_path_loss_db(freq_mhz: f64, dist_km: f64) -> Option<f64> {
  (freq_mhz > 0.0 && freq_mhz.is_finite() && dist_km > 0.0 && dist_km.is_finite())
    .then(|| FSPL_CONSTANT_DB + 20.0 * freq_mhz.log10() + 20.0 * dist_km.log10())
}

/// 接收功率（dBm）。任一距离 / 频率非法时返回 `None`。
#[must_use]
pub fn received_dbm(
  tx_dbm: f64,
  tx_gain_dbi: f64,
  rx_gain_dbi: f64,
  freq_mhz: f64,
  dist_km: f64,
  extra_loss_db: f64,
) -> Option<f64> {
  // 功率 / 增益 / 附加损耗也要有限：开放 API 把 `inf` 序列化成 `null`，
  // 与其让客户端拿到 `null`，不如在这里按非法输入拒绝。
  if ![tx_dbm, tx_gain_dbi, rx_gain_dbi, extra_loss_db]
    .iter()
    .all(|v| v.is_finite())
  {
    return None;
  }
  // 各项都有限，求和仍可能溢出（如 1e308 + 1e308）：结果同样要有限。
  let rx = tx_dbm + tx_gain_dbi + rx_gain_dbi
    - free_space_path_loss_db(freq_mhz, dist_km)?
    - extra_loss_db;
  rx.is_finite().then_some(rx)
}

#[cfg(test)]
mod tests {
  use super::*;

  fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-3
  }

  #[test]
  fn fspl_matches_the_classic_2m_example() {
    // 146 MHz、100 km：20·lg146 = 43.287，20·lg100 = 40 → 32.45 + 83.287 = 115.737 dB。
    assert!(close(
      free_space_path_loss_db(146.0, 100.0).unwrap(),
      115.737
    ));
    // 距离翻倍 → 损耗 +6.02 dB（20·lg2）。
    assert!(close(
      free_space_path_loss_db(146.0, 200.0).unwrap(),
      121.758
    ));
    // 频率翻倍 → 同样 +6.02 dB。
    assert!(close(
      free_space_path_loss_db(292.0, 100.0).unwrap(),
      121.758
    ));
  }

  #[test]
  fn received_power_adds_gains_and_losses() {
    // 20 dBm + 2×2 dBi − 115.737 − 0 ≈ −91.737 dBm。
    assert!(close(
      received_dbm(20.0, 2.0, 2.0, 146.0, 100.0, 0.0).unwrap(),
      -91.737
    ));
    // 馈线损耗 3 dB 再减 3。
    assert!(close(
      received_dbm(20.0, 2.0, 2.0, 146.0, 100.0, 3.0).unwrap(),
      -94.737
    ));
  }

  #[test]
  fn invalid_inputs_are_rejected() {
    assert!(free_space_path_loss_db(0.0, 1.0).is_none());
    assert!(free_space_path_loss_db(146.0, 0.0).is_none());
    assert!(free_space_path_loss_db(146.0, f64::NAN).is_none());
    assert!(received_dbm(20.0, 2.0, 2.0, 146.0, -1.0, 0.0).is_none());
    assert!(received_dbm(f64::INFINITY, 2.0, 2.0, 146.0, 100.0, 0.0).is_none());
    // 各项有限但求和溢出：不能把 `inf` 当结果交出去。
    assert!(received_dbm(1e308, 1e308, 1e308, 146.0, 100.0, 0.0).is_none());
  }
}
