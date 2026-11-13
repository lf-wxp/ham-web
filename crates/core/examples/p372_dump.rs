//! 打印 P.372 大气噪声，供 `crates/core/reference/p372_check.py` 与官方程序对拍。
//!
//! 每行输入 `月 小时 纬度 经度 频率 环境`（月份 1 起；环境 0 城市 / 1 住宅 / 2 农村 / 3 安静乡村，同官方），
//! 每行输出 `Fa Du Dl 合成Fa 合成Du 合成Dl`；合成不适用（频率超出人为噪声曲线）时后三项写 `NA`。
use std::io::{BufRead, Write};

use ham_web_core::atmospheric_noise::{AtmNoiseInput, atmospheric, combined};
use ham_web_core::noise_model::Environment;

fn main() {
  let stdin = std::io::stdin();
  let mut out = std::io::BufWriter::new(std::io::stdout());
  for line in stdin.lock().lines() {
    let line = line.expect("读取标准输入");
    let f: Vec<f64> = line
      .split_whitespace()
      .map(|v| v.parse().expect("数字"))
      .collect();
    if f.len() != 6 {
      continue;
    }
    let input = AtmNoiseInput {
      month: f[0] as u32,
      hour_utc: f[1],
      lat_deg: f[2],
      lon_deg: f[3],
      freq_mhz: f[4],
    };
    let env = [
      Environment::City,
      Environment::Residential,
      Environment::Rural,
      Environment::QuietRural,
    ][f[5] as usize];
    match atmospheric(&input) {
      Some(n) => {
        let c = combined(
          env,
          input.month,
          input.hour_utc,
          input.lat_deg,
          input.lon_deg,
          input.freq_mhz,
        );
        let tail = c.map_or_else(
          || "NA NA NA".to_owned(),
          |c| format!("{:.9} {:.9} {:.9}", c.fa_db, c.du_db, c.dl_db),
        );
        writeln!(out, "{:.9} {:.9} {:.9} {tail}", n.fa_db, n.du_db, n.dl_db).expect("写标准输出");
      }
      None => writeln!(out, "ERR").expect("写标准输出"),
    }
  }
}
