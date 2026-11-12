//! 读一份 `.nec` 文本（参数给路径，或从标准输入读），打印阻抗 / 驻波比 / 最大增益。
//!
//! 与 `reference/tl_necpp.py` 配合：那边把同一份卡片组分别交给 necpp 与本程序，逐点对拍
//! 带传输线（含交叉线）的结构。成功时输出固定为一行
//! `Z=<R> <X> SWR=<s> G=<dBi> AZ=<°> FB=<dB>`，便于脚本解析；失败（读不到输入、卡片组解析不了、
//! 求解器无解）写一行说明到标准错误并以非零状态退出，脚本据此区分「算不出」与「输出格式变了」。
use std::io::Read;
use std::process::ExitCode;

use ham_web_nec::nec::{parse_nec, solve_with_lines};

fn read_deck() -> Result<String, String> {
  match std::env::args().nth(1) {
    Some(path) => std::fs::read_to_string(&path).map_err(|e| format!("读取 {path} 失败：{e}")),
    None => {
      let mut text = String::new();
      std::io::stdin()
        .read_to_string(&mut text)
        .map_err(|e| format!("读取标准输入失败：{e}"))?;
      Ok(text)
    }
  }
}

fn run() -> Result<String, String> {
  let text = read_deck()?;
  let f = parse_nec(&text).ok_or("卡片组解析失败（缺导线 / 馈电 / 频率）")?;
  for note in &f.ignored {
    eprintln!("忽略：{note}");
  }
  let r = solve_with_lines(&f.input, &f.lines).ok_or("求解器无解")?;
  Ok(format!(
    "Z={:.6} {:.6} SWR={:.4} G={:.4} AZ={:.1} FB={:.3}",
    r.impedance.0,
    r.impedance.1,
    r.swr_50,
    r.gain_max_dbi,
    r.gain_max_azimuth_deg,
    r.front_to_back_db
  ))
}

fn main() -> ExitCode {
  match run() {
    Ok(line) => {
      println!("{line}");
      ExitCode::SUCCESS
    }
    Err(e) => {
      eprintln!("{e}");
      ExitCode::FAILURE
    }
  }
}
