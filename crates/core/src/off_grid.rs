//! 离网供电逐日能量平衡：太阳能板 + 电池 + 负载，在最坏月能否自持。
//!
//! POTA / 应急 / 中继站的「能不能撑过冬天」问题。这里只做**逐日能量平衡**的纯算术：
//!
//! - 每日发电 `= 板功率 × 峰值日照时数 × 控制器效率 × 充电效率`；
//! - 电池可用 `= 容量 × 电压 × DOD`；
//! - 净平衡 `= 每日发电 − 每日负载`；
//! - 净平衡 ≥ 0 时长期自持；< 0 时电池以 `|净平衡|` 每天的速度放电，
//!   满电电池可撑 `可用能量 / |净平衡|` 天。
//!
//! 峰值日照时数（最坏月）由用户按所在地给（北半球中纬度冬季常见 2–4 h），
//! 不在站内嵌日照表——那是随纬度 / 季节变化的外部数据。算术本身可独立对拍。

/// 离网供电参数。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OffGridInput {
  /// 太阳能板标称功率（W）。
  pub panel_watts: f64,
  /// 最坏月峰值日照时数（h/day）。
  pub peak_sun_hours: f64,
  /// 每日负载（Wh）。
  pub daily_load_wh: f64,
  /// 电池容量（Ah）。
  pub battery_ah: f64,
  /// 电池标称电压（V）。
  pub battery_voltage_v: f64,
  /// 放电深度（0..1）。
  pub dod: f64,
  /// 电池充电效率（0..1）。
  pub charge_eff: f64,
  /// 控制器 / MPPT 效率（0..1）。
  pub controller_eff: f64,
}

/// 能量平衡结果。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OffGrid {
  /// 每日发电（Wh）。
  pub daily_solar_wh: f64,
  /// 电池可用能量（Wh）。
  pub battery_usable_wh: f64,
  /// 每日净平衡（Wh，正 = 盈余，负 = 亏空）。
  pub net_wh: f64,
  /// 亏空时满电电池能撑几天；盈余时为 `None`（长期自持）。
  pub autonomy_days: Option<f64>,
}

/// 计算离网逐日能量平衡；输入不合法返回 `None`。
#[must_use]
pub fn off_grid(input: OffGridInput) -> Option<OffGrid> {
  let OffGridInput {
    panel_watts,
    peak_sun_hours,
    daily_load_wh,
    battery_ah,
    battery_voltage_v,
    dod,
    charge_eff,
    controller_eff,
  } = input;
  // 负载或电池容量为零的「纯充电 / 无储能」系统没有逐日平衡可谈（亏空时的续航天数没有定义），
  // 因此连同功率 / 日照 / 电压一并按非正拒绝，别把 0 当成合法输入放进来算出「0 天」这类无意义值。
  //
  // 日照时数还要卡 24 h 的物理上界：填成 100 h/天会直接把每日发电量放大十几倍，结论从
  // 「撑不过冬天」翻成「可长期自持」，而用户只会以为自己填对了。
  if panel_watts <= 0.0
    || peak_sun_hours <= 0.0
    || peak_sun_hours > 24.0
    || daily_load_wh <= 0.0
    || battery_ah <= 0.0
    || battery_voltage_v <= 0.0
    || dod <= 0.0
    || dod > 1.0
    || charge_eff <= 0.0
    || charge_eff > 1.0
    || controller_eff <= 0.0
    || controller_eff > 1.0
    || !panel_watts.is_finite()
    || !peak_sun_hours.is_finite()
    || !daily_load_wh.is_finite()
    || !battery_ah.is_finite()
    || !battery_voltage_v.is_finite()
    // 这三个只被范围比较挡着，而 NaN 与任何值比较都是 false —— 必须显式查有限性。
    || !dod.is_finite()
    || !charge_eff.is_finite()
    || !controller_eff.is_finite()
  {
    return None;
  }
  let daily_solar_wh = panel_watts * peak_sun_hours * controller_eff * charge_eff;
  let battery_usable_wh = battery_ah * battery_voltage_v * dod;
  let net_wh = daily_solar_wh - daily_load_wh;
  let autonomy_days = if net_wh >= 0.0 {
    None
  } else {
    Some(battery_usable_wh / (-net_wh))
  };
  Some(OffGrid {
    daily_solar_wh,
    battery_usable_wh,
    net_wh,
    autonomy_days,
  })
}

#[cfg(test)]
mod tests {
  use super::*;

  fn input() -> OffGridInput {
    OffGridInput {
      panel_watts: 100.0,
      peak_sun_hours: 4.0,
      daily_load_wh: 300.0,
      battery_ah: 100.0,
      battery_voltage_v: 12.8,
      dod: 0.5,
      charge_eff: 0.9,
      controller_eff: 0.95,
    }
  }

  #[test]
  fn daily_solar_applies_both_efficiencies() {
    let r = off_grid(input()).unwrap();
    // 100 W × 4 h × 0.95 × 0.9 = 342 Wh。
    assert!(
      (r.daily_solar_wh - 342.0).abs() < 1e-9,
      "{}",
      r.daily_solar_wh
    );
    // 100 Ah × 12.8 V × 0.5 = 640 Wh。
    assert!((r.battery_usable_wh - 640.0).abs() < 1e-9);
  }

  #[test]
  fn surplus_is_self_sustaining() {
    let r = off_grid(input()).unwrap();
    assert!((r.net_wh - 42.0).abs() < 1e-9); // 342 − 300
    assert_eq!(r.autonomy_days, None);
  }

  #[test]
  fn deficit_drains_battery_over_known_days() {
    let r = off_grid(OffGridInput {
      daily_load_wh: 500.0,
      ..input()
    })
    .unwrap();
    assert!((r.net_wh - (-158.0)).abs() < 1e-9); // 342 − 500
    // 640 Wh / 158 Wh/day ≈ 4.05 天。
    let days = r.autonomy_days.expect("亏空应有续航天数");
    assert!((days - 4.05).abs() < 0.01, "{days}");
  }

  #[test]
  fn invalid_inputs_return_none() {
    let p = input();
    assert_eq!(
      off_grid(OffGridInput {
        panel_watts: 0.0,
        ..p
      }),
      None
    );
    assert_eq!(
      off_grid(OffGridInput {
        peak_sun_hours: 0.0,
        ..p
      }),
      None
    );
    // 日照时数的物理上界是 24 h/天：超了会把每日发电放大成十几倍、结论反过来（「可长期自持」）。
    assert_eq!(
      off_grid(OffGridInput {
        peak_sun_hours: 24.5,
        ..p
      }),
      None
    );
    assert!(
      off_grid(OffGridInput {
        peak_sun_hours: 24.0,
        ..p
      })
      .is_some(),
      "正好 24 h/天仍是合法输入"
    );
    assert_eq!(
      off_grid(OffGridInput {
        daily_load_wh: 0.0,
        ..p
      }),
      None
    );
    assert_eq!(
      off_grid(OffGridInput {
        battery_ah: 0.0,
        ..p
      }),
      None
    );
    assert_eq!(off_grid(OffGridInput { dod: 1.5, ..p }), None);
    assert_eq!(off_grid(OffGridInput { dod: 0.0, ..p }), None);
    assert_eq!(
      off_grid(OffGridInput {
        charge_eff: -0.1,
        ..p
      }),
      None
    );
    assert_eq!(
      off_grid(OffGridInput {
        controller_eff: 1.2,
        ..p
      }),
      None
    );
    // NaN 必须走「参数非法」，不能算出一个全 NaN 的结果。
    assert_eq!(off_grid(OffGridInput { dod: f64::NAN, ..p }), None);
    assert_eq!(
      off_grid(OffGridInput {
        charge_eff: f64::NAN,
        ..p
      }),
      None
    );
    assert_eq!(
      off_grid(OffGridInput {
        controller_eff: f64::NAN,
        ..p
      }),
      None
    );
  }
}
