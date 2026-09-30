//! ITU 语音字母表（字母解释法）：用固定单词代表字母，避免话音通联中听错。
//!
//! 读音提示采用 ITU 官方拼读（大写为重读音节），如 `A` → `Alfa`（AL FAH）。

/// 一个语音字母。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhoneticEntry {
  /// 字母。
  pub letter: &'static str,
  /// 代表单词。
  pub word: &'static str,
  /// 读音提示（大写为重读）。
  pub pronunciation: &'static str,
}

const fn ph(
  letter: &'static str,
  word: &'static str,
  pronunciation: &'static str,
) -> PhoneticEntry {
  PhoneticEntry {
    letter,
    word,
    pronunciation,
  }
}

/// A–Z 的语音字母表。
pub const PHONETIC: &[PhoneticEntry] = &[
  ph("A", "Alfa", "AL FAH"),
  ph("B", "Bravo", "BRAH VOH"),
  ph("C", "Charlie", "CHAR LEE"),
  ph("D", "Delta", "DELL TAH"),
  ph("E", "Echo", "ECK OH"),
  ph("F", "Foxtrot", "FOKS TROT"),
  ph("G", "Golf", "GOLF"),
  ph("H", "Hotel", "HOH TELL"),
  ph("I", "India", "IN DEE AH"),
  ph("J", "Juliett", "JEW LEE ETT"),
  ph("K", "Kilo", "KEY LOH"),
  ph("L", "Lima", "LEE MAH"),
  ph("M", "Mike", "MIKE"),
  ph("N", "November", "NO VEM BER"),
  ph("O", "Oscar", "OSS CAH"),
  ph("P", "Papa", "PAH PAH"),
  ph("Q", "Quebec", "KEH BECK"),
  ph("R", "Romeo", "ROW ME OH"),
  ph("S", "Sierra", "SEE AIR RAH"),
  ph("T", "Tango", "TANG GO"),
  ph("U", "Uniform", "YOU NEE FORM"),
  ph("V", "Victor", "VIK TAH"),
  ph("W", "Whiskey", "WISS KEY"),
  ph("X", "X-ray", "ECKS RAY"),
  ph("Y", "Yankee", "YANG KEY"),
  ph("Z", "Zulu", "ZOO LOO"),
];

/// 按字母查找语音单词（如 `A` → `Alfa`）。
#[must_use]
pub fn word_of(letter: &str) -> Option<&'static str> {
  PHONETIC.iter().find(|e| e.letter == letter).map(|e| e.word)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn alphabet_is_complete() {
    assert_eq!(PHONETIC.len(), 26);
    let letters: Vec<&str> = PHONETIC.iter().map(|e| e.letter).collect();
    for (i, expect) in ('A'..='Z').enumerate() {
      assert_eq!(letters[i], expect.to_string(), "第 {i} 个字母应为 {expect}");
    }
  }

  #[test]
  fn entries_are_non_empty() {
    for e in PHONETIC {
      assert!(!e.word.is_empty());
      assert!(!e.pronunciation.is_empty());
    }
  }

  #[test]
  fn spot_checks() {
    assert_eq!(PHONETIC[0].word, "Alfa");
    assert_eq!(PHONETIC[25].word, "Zulu");
    assert_eq!(PHONETIC[9].word, "Juliett");
  }
}
