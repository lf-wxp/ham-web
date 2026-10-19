//! 业余无线电小工具集合：换算、电路与天线估算、代码解码、数据备份。
//!
//! 不在这里维护工具清单（列一次就会过期）：导航项与区块见 `tools_page.rs` 的 `TOOL_NAV`
//! 与 `<ToolSection>`，两者一一对应。

mod antenna_length;
mod antenna_matcher;
mod aprs_codec;
mod attenuator_calculator;
mod backup_tool;
mod battery_runtime;
mod callsign_lookup;
mod capacitor_code;
mod cascade_gain;
mod coil_yagi_calculator;
mod contest_scorer;
mod cw_bandwidth;
mod dbm_converter;
mod dbm_dbuv;
mod decibel_gain;
mod distance_bearing;
mod doppler_calculator;
mod duty_power;
mod eirp_calculator;
mod feedline_loss;
mod filter_design;
mod freq_wavelength;
mod frequency_units;
mod fresnel_zone;
mod fspl_calculator;
mod gain_conversion;
mod intermod_calculator;
mod ip3_cascade;
mod lc_resonance;
mod link_budget_calculator;
mod loading_coil;
mod lora_calculator;
mod magnetic_loop;
mod mode_encoder;
mod noise_cascade;
mod off_grid;
mod ohms_law;
mod oscillator;
mod parabolic_dish;
mod path_profile;
mod path_profile_chart;
mod propagation_estimator;
mod radiogram_generator;
mod rain_fade;
mod reactance;
mod receiver_sensitivity;
mod resistor_color_code;
mod resistor_parallel;
mod rf_exposure;
mod smd_resistor_code;
mod smith_chart;
mod soft_tnc;
mod stub_match;
mod swr_converter;
mod tone_squelch;
mod tool_section;
mod tools_page;
mod toroid_winding;
mod transformer_calculator;
mod trap;
mod tx_line;
mod utc_clock;
mod waveguide_calculator;
mod waveguide_report;
mod wire_gauge;

pub use tools_page::ToolsPage;

fn fmt_num(v: f64) -> String {
  // 0 不能落进最后的科学计数分支（会显示成 `0.000e0`）。
  if v == 0.0 {
    "0".to_string()
  } else if v.abs() >= 1000.0 {
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

/// 结果区（只读，不是表单控件）。
const RESULT: &str = "sm:col-span-2 bg-muted/40 px-3 py-2 text-sm text-muted-foreground";

/// 结果区（单列布局用）：与 [`RESULT`] 同一套底色 / 内距 / 字号，但不跨列。
///
/// 单列网格里写 `sm:col-span-2` 会凭空长出第二列，所以单列工具（代码解码这类）用这一份。
const RESULT_INLINE: &str = "bg-muted/40 px-3 py-2 text-sm tabular-nums text-muted-foreground";
