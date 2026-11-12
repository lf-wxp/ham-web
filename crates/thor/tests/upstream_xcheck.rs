//! 与 fldigi 上游代码逐位对拍（需要 `FLDIGI_SRC`，见 `ham-web-mfsk` 的 `tests/support/upstream.rs`；
//! 用例标了 `#[ignore]`，不设环境变量时在默认测试里显示为 ignored）。

#[path = "../../mfsk/tests/support/upstream.rs"]
mod upstream;

use ham_web_thor::{MODES, encode_symbols};

const TEXTS: &[&str] = &[
  "the quick brown fox jumps over the lazy dog 1234567890",
  "CQ CQ DE BG1AAA BG1AAA PSE K",
  "A",
  // `\x01` 之后的字节走二次信道（ID 信标）。
  "CQ \u{1}f\u{1}l\u{1}d\u{1}i\u{1}g\u{1}i DE BG1AAA",
  "Hello, 世界! ~ \u{e9}",
];

#[test]
#[ignore = "需要 FLDIGI_SRC：用 cargo make thor-xcheck 运行（见 crates/mfsk/reference/README.md）"]
fn tone_sequences_match_upstream_for_every_mode() {
  let reference = upstream::reference();
  for mode in MODES {
    for text in TEXTS {
      let want = reference.tones(&[
        "thor",
        if mode.k15 { "1" } else { "0" },
        &mode.depth.to_string(),
        &mode.flush.to_string(),
        if mode.micro { "1" } else { "0" },
        text,
      ]);
      let got = encode_symbols(text, mode);
      assert_eq!(got, want, "THOR {} {text:?}", mode.name);
    }
  }
}
