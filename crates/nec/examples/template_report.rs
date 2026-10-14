//! 打印方案库每份模板的求解结果（交叉验证时与 necpp 对照用）。
use ham_web_nec::nec::{parse_nec, solve};
use ham_web_nec::nec_templates::NEC_TEMPLATES;

fn main() {
  for t in NEC_TEMPLATES {
    let f = parse_nec(t.text).expect("解析");
    match solve(&f.input) {
      Some(r) => println!(
        "{:<18} Z = {:9.3} {:+10.3}j Ω  G = {:6.3} dBi",
        t.id, r.impedance.0, r.impedance.1, r.gain_max_dbi
      ),
      None => println!("{:<18} 无法求解", t.id),
    }
  }
}
