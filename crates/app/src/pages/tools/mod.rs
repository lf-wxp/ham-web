//! 业余无线电小工具：频率↔波长、dBm↔功率、分贝增益、欧姆定律、CW 必要带宽、LC 谐振、容抗/感抗。

mod antenna_length;
mod antenna_matcher;
mod backup_tool;
mod battery_runtime;
mod callsign_lookup;
mod cascade_gain;
mod coil_yagi_calculator;
mod contest_scorer;
mod cw_bandwidth;
mod dbm_converter;
mod dbm_dbuv;
mod decibel_gain;
mod distance_bearing;
mod doppler_calculator;
mod eirp_calculator;
mod feedline_loss;
mod freq_wavelength;
mod frequency_units;
mod fspl_calculator;
mod gain_conversion;
mod lc_resonance;
mod link_budget_calculator;
mod noise_cascade;
mod ohms_law;
mod propagation_estimator;
mod reactance;
mod receiver_sensitivity;
mod resistor_color_code;
mod resistor_parallel;
mod swr_converter;
mod tools_page;

pub use tools_page::ToolsPage;

fn fmt_num(v: f64) -> String {
  if v.abs() >= 1000.0 {
    format!("{v:.1}")
  } else if v.abs() >= 1.0 {
    format!("{v:.2}")
  } else if v.abs() >= 0.001 {
    format!("{v:.4}")
  } else {
    format!("{v:.3e}")
  }
}

/// 阻值格式化：Ω / kΩ / MΩ。
fn fmt_resistance(ohm: f64) -> String {
  if ohm >= 1_000_000.0 {
    format!("{:.2} MΩ", ohm / 1_000_000.0)
  } else if ohm >= 1000.0 {
    format!("{:.2} kΩ", ohm / 1000.0)
  } else {
    format!("{:.2} Ω", ohm)
  }
}

/// 2π。
const TAU: f64 = std::f64::consts::TAU;

const INPUT: &str = "h-10 rounded-lg border bg-background px-3 text-sm tabular-nums outline-none focus:ring-2 focus:ring-ring/50";
const RESULT: &str = "sm:col-span-2 rounded-lg bg-muted/40 px-3 py-2 text-sm text-muted-foreground";
