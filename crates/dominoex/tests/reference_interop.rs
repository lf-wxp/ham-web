//! 与独立参考实现的**双向互操作对拍**（进程内直连，无外部依赖）。
//!
//! 参考实现是 fldigi（GPL-3.0，DominoEX 的事实参考实现）的 Rust 移植
//! （`tests/reference/dominoex_ref.rs`，与 `src/` 不共享代码，表数据转写自
//! 同源的 `dominovar.cxx`）。双向验证：**参考编码 → 本仓解码** 与
//! **本仓编码 → 参考解码**。

#[path = "reference/dominoex_ref.rs"]
mod dominoex_ref;

use ham_web_dominoex::parse_mode;

#[test]
fn ref_encode_rust_decode_ex8() {
  let text = "THE QUICK BROWN FOX JUMPS OVER THE LAZY DOG 12345";
  let (baud, spacing, rate) = dominoex_ref::mode("8").expect("EX8 参数");
  let audio = dominoex_ref::synth(text, baud, spacing, rate, 0.5);
  let (primary, _) =
    ham_web_dominoex::decode_audio(&audio, rate, baud, spacing).expect("应解出文本");
  assert!(
    primary.contains(text),
    "参考编码 → 本仓解码（EX8）：{primary}"
  );
}

#[test]
fn rust_encode_ref_decode_ex8() {
  let text = "CQ CQ DE BG1AAA 599";
  let (baud, spacing) = parse_mode("8").unwrap();
  let audio = ham_web_dominoex::encode_audio(text, baud, spacing, 8000, 0.5).unwrap();
  let decoded = dominoex_ref::decode(&audio, 8000, baud, spacing);
  assert!(
    decoded.contains(text),
    "本仓编码 → 参考解码（EX8）：{decoded}"
  );
}

#[test]
fn ref_encode_rust_decode_ex16() {
  let text = "DOMINOEX 16 TEST";
  let (baud, spacing, rate) = dominoex_ref::mode("16").expect("EX16 参数");
  let audio = dominoex_ref::synth(text, baud, spacing, rate, 0.5);
  let (primary, _) =
    ham_web_dominoex::decode_audio(&audio, rate, baud, spacing).expect("应解出文本");
  assert!(
    primary.contains(text),
    "参考编码 → 本仓解码（EX16）：{primary}"
  );
}
