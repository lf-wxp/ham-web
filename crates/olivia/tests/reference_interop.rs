//! 与独立参考实现的**双向互操作对拍**（进程内直连，无外部依赖）。
//!
//! 参考实现是 sntfrc/olivia-python（MIT，© Federico Santandrea，与 fldigi 对比
//! 验证过）的 Rust 移植（`tests/reference/olivia_ref.rs`，与 `src/` 不共享代码）。
//! 双向验证：**参考编码 → 本仓解码** 与 **本仓编码 → 参考解码** —— 编解码分属
//! 两套独立实现，双向都能还原同一段文本才是真实现。

#[path = "reference/olivia_ref.rs"]
mod olivia_ref;

#[test]
fn ref_encode_rust_decode_32_1000() {
  let text = "THE QUICK BROWN FOX JUMPS OVER THE LAZY DOG 12345";
  // 参考编码器的 CLI 会在文本后补 \0（块填充），这里同样补上。
  let with_nul = format!("{text}\0");
  let audio = olivia_ref::synthesize(&with_nul, 32, 1000.0, 1500.0);
  let decoded = ham_web_olivia::decode_audio(&audio, olivia_ref::SAMPLE_RATE, 32, 1000.0, 1500.0)
    .expect("应解出文本");
  assert_eq!(decoded, text, "参考编码 → 本仓解码（32/1000）");
}

#[test]
fn rust_encode_ref_decode_8_250() {
  let text = "CQ CQ DE BG1AAA 599";
  let audio = ham_web_olivia::encode_audio(text, 8, 250.0, 750.0, 8000).unwrap();
  let decoded = olivia_ref::decode(&audio, 8, 250.0, 750.0);
  assert_eq!(decoded, text, "本仓编码 → 参考解码（8/250）");
}

#[test]
fn ref_encode_rust_decode_8_250() {
  let text = "CQ CQ DE BG1AAA 599";
  let with_nul = format!("{text}\0");
  let audio = olivia_ref::synthesize(&with_nul, 8, 250.0, 750.0);
  let decoded = ham_web_olivia::decode_audio(&audio, olivia_ref::SAMPLE_RATE, 8, 250.0, 750.0)
    .expect("应解出文本");
  assert_eq!(decoded, text, "参考编码 → 本仓解码（8/250）");
}
