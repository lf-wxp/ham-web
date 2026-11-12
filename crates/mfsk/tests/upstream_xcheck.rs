//! 与 fldigi 上游代码逐位对拍（需要 `FLDIGI_SRC`，见 `support/upstream.rs`；用例标了 `#[ignore]`，不设环境变量时在默认测试里显示为 ignored）。

#[path = "support/upstream.rs"]
mod upstream;

use ham_web_mfsk::fec::{Code, Viterbi};
use ham_web_mfsk::{MODES, encode_symbols};

const TEXTS: &[&str] = &[
  "the quick brown fox jumps over the lazy dog 1234567890",
  "CQ CQ DE BG1AAA BG1AAA PSE K",
  "A",
  "Hello, 世界! ~ \u{e9}",
];

#[test]
#[ignore = "需要 FLDIGI_SRC：用 cargo make mfsk-xcheck 运行（见 crates/mfsk/reference/README.md）"]
fn tone_sequences_match_upstream_for_every_mode() {
  let reference = upstream::reference();
  for mode in MODES {
    for text in TEXTS {
      let want = reference.tones(&[
        "mfsk",
        &mode.symbits.to_string(),
        &mode.depth.to_string(),
        &mode.preamble.to_string(),
        text,
      ]);
      let got = encode_symbols(text, mode);
      assert_eq!(got, want, "MFSK{} {text:?}", mode.name);
    }
  }
}

#[test]
#[ignore = "需要 FLDIGI_SRC：用 cargo make mfsk-xcheck 运行（见 crates/mfsk/reference/README.md）"]
fn viterbi_matches_upstream_bit_for_bit() {
  let reference = upstream::reference();
  for (code, traceback) in [(Code::NASA_K7, 45usize), (Code::IEEE_K15, 180)] {
    for seed in [1u32, 42, 0xDEAD_BEEF] {
      let n = 3000usize;
      let want = reference.run(&[
        "viterbi",
        &code.k.to_string(),
        &code.poly1.to_string(),
        &code.poly2.to_string(),
        &traceback.to_string(),
        &seed.to_string(),
        &n.to_string(),
      ]);
      let mut dec = Viterbi::new(code, traceback);
      let mut s = seed;
      let mut next = || {
        s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        (s >> 8) as u8
      };
      for (i, line) in want.lines().enumerate() {
        let (a, b) = (next(), next());
        let (bit, met) = dec.decode([a, b]);
        let mut it = line.split_whitespace();
        let want_bit: i32 = it.next().unwrap().parse().unwrap();
        let want_met: i32 = it.next().unwrap().parse().unwrap();
        assert_eq!(
          i32::from(bit),
          want_bit,
          "K={} seed={seed} 第 {i} 步的位",
          code.k
        );
        assert_eq!(met, want_met, "K={} seed={seed} 第 {i} 步的度量", code.k);
      }
    }
  }
}
