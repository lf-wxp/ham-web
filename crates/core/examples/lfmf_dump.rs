//! 与 NTIA 原件（`LFMF Source` 的 C++）逐点对拍：打印本站求解结果，供
//! `crates/core/reference/lfmf_check.py` 与原件比较。
//!
//! 用法（在仓库根目录）：
//! ```text
//! cargo run -q -p ham-web-core --release --example lfmf_dump < cases.txt
//! ```
//! 每行输入 `h_tx h_rx f P Ns d eps sigma pol`，每行输出 `E_dBuVm A_btl method`。
use std::io::{BufRead, Write};

use ham_web_core::ground_wave::{GroundWaveInput, Polarization, solve};

fn main() {
  let stdin = std::io::stdin();
  let mut out = std::io::BufWriter::new(std::io::stdout());
  for line in stdin.lock().lines() {
    let line = line.expect("读取标准输入");
    let f: Vec<f64> = line
      .split_whitespace()
      .map(|v| v.parse().expect("数字"))
      .collect();
    if f.len() != 9 {
      continue;
    }
    let input = GroundWaveInput {
      h_tx_m: f[0],
      h_rx_m: f[1],
      freq_mhz: f[2],
      power_w: f[3],
      surface_refractivity: f[4],
      distance_km: f[5],
      epsilon_r: f[6],
      sigma_s_m: f[7],
      polarization: if f[8] == 1.0 {
        Polarization::Vertical
      } else {
        Polarization::Horizontal
      },
    };
    match solve(&input) {
      Some(r) => writeln!(
        out,
        "{:.10} {:.10} {}",
        r.field_dbu_vm,
        r.basic_loss_db,
        match r.method {
          ham_web_core::ground_wave::Method::FlatEarthCurve => 0,
          ham_web_core::ground_wave::Method::ResidueSeries => 1,
        }
      )
      .expect("写标准输出"),
      None => writeln!(out, "ERR").expect("写标准输出"),
    }
  }
}
