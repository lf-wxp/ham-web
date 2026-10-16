//! Super Check Partial（SCP）式呼号补全：输入前几位 / 中间几位，列出可能的呼号；
//! 输入了库里没有的呼号时，列出只差一位的相近呼号（常是打错了）。
//!
//! **本模块不带任何呼号数据。** SCP 数据库（`MASTER.SCP`）的站点页脚写明 "All rights reserved"，
//! 也没有再分发许可，所以本站不分发它。候选来自两处，都是用户自己的：
//!
//! 1. 用户自己通联日志里出现过的呼号；
//! 2. 用户**自己从官方站下载并选择**的 `.scp` 文件（只存在本机，见 [`parse_scp`]）。
//!
//! `.scp` 是 N1MM+ / Win-Test 用的纯文本格式：一行一个呼号，字母序，CRLF 换行，
//! 以 `!` 或 `#` 开头的行是头 / 注释（`!!Order,1,1` 声明排序，`# Release 2026.10.14` 是版本）。
//!
//! 与登录软件里的 SCP 有两处不同，界面也要写明：候选按字母序排，不按「这个呼号一年里出现多少次」
//! 排（`.scp` 文件里没有这个信息）；也不区分竞赛类别（`master` / `masterdx` / `masterss`
//! 是不同的文件，请自己选对应的那份）。

use std::collections::HashSet;

/// 解析时最多接受的数据行数：真实的 `master.scp` 约 5 万行，留出十几倍余量，再大多半不是 SCP 文件。
pub const MAX_LINES: usize = 300_000;

/// 载入文件的大小上限（字节）：真实的 `master.scp` 约 360 KB，超过这个多半不是 SCP 文件。
pub const MAX_FILE_BYTES: u64 = 8 * 1024 * 1024;

/// 候选里最短要输入几位才开始找「包含」匹配。
pub const MIN_PARTIAL: usize = 2;
/// 相近呼号（只差一位）要输入几位才开始找：太短的呼号随便改一位都撞一大堆。
pub const MIN_SIMILAR: usize = 3;

/// 看起来像呼号：3–14 个字符，只有字母数字与 `/`，至少一个字母和一个数字，`/` 不在两端。
///
/// 与 `cabrillo_import` 里判断呼号的规则一致（导入的 Cabrillo 通联与 SCP 文件互相印证）。
#[must_use]
pub fn is_callsign(s: &str) -> bool {
  (3..=14).contains(&s.len())
    && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'/')
    && (s.bytes().any(|b| b.is_ascii_digit())
      || s
        .split('/')
        .next()
        .is_some_and(|base| base.eq_ignore_ascii_case("RAEM")))
    && s.bytes().any(|b| b.is_ascii_alphabetic())
    && !s.starts_with('/')
    && !s.ends_with('/')
}

/// 解析好的 `.scp` 文件。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ScpFile {
  /// 大写、去重、字母序。
  pub calls: Vec<String>,
  /// `# Release …` 里的版本（没有则为 `None`）。
  pub release: Option<String>,
  /// 数据行里不像呼号而被丢掉的条数。
  pub skipped: usize,
}

impl ScpFile {
  /// 存进浏览器用的规范文本：可选的 `# Release …` 头 + 一行一个呼号（LF）。
  ///
  /// 不原样存用户选的文件：那里面可能有别人加的注释，也没必要占那个空间。
  #[must_use]
  pub fn to_text(&self) -> String {
    let mut out = String::with_capacity(self.calls.len() * 7 + 32);
    if let Some(r) = &self.release {
      out.push_str("# Release ");
      out.push_str(r);
      out.push('\n');
    }
    for c in &self.calls {
      out.push_str(c);
      out.push('\n');
    }
    out
  }
}

/// 解析失败的原因。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScpError {
  /// 一个能用的呼号也没有。
  NoCallsigns,
  /// 不像呼号的行比像呼号的还多：多半选错了文件。
  MostlyNotCallsigns,
  /// 数据行超过 [`MAX_LINES`]。
  TooLarge,
}

/// 解析 `.scp` 文本。
///
/// 容忍：CRLF / LF、行首尾空白、小写、重复行、UTF-8 BOM、空行、没有头信息。
///
/// # Errors
///
/// 见 [`ScpError`]。
pub fn parse_scp(text: &str) -> Result<ScpFile, ScpError> {
  let text = text.strip_prefix('\u{feff}').unwrap_or(text);
  let mut release = None;
  let mut seen: HashSet<String> = HashSet::new();
  let mut bad = 0usize;
  let mut data_lines = 0usize;

  for raw in text.lines() {
    let line = raw.trim();
    if line.is_empty() {
      continue;
    }
    if line.starts_with(['!', '#']) {
      if release.is_none()
        && let Some((_, rest)) = line.split_once("Release")
      {
        let rest = rest.trim();
        if !rest.is_empty() {
          release = Some(rest.to_owned());
        }
      }
      continue;
    }
    data_lines += 1;
    if data_lines > MAX_LINES {
      return Err(ScpError::TooLarge);
    }
    let call = line.to_ascii_uppercase();
    if is_callsign(&call) {
      seen.insert(call);
    } else {
      bad += 1;
    }
  }

  if seen.is_empty() {
    return Err(ScpError::NoCallsigns);
  }
  if bad > data_lines - bad {
    return Err(ScpError::MostlyNotCallsigns);
  }
  let mut calls: Vec<String> = seen.into_iter().collect();
  calls.sort_unstable();
  Ok(ScpFile {
    calls,
    release,
    skipped: bad,
  })
}

/// 候选索引：大写、去重、字母序的呼号集合。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Index {
  calls: Vec<String>,
}

/// 一次查询的结果。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Lookup {
  /// 输入的呼号本身就在库里。
  pub exact: bool,
  /// 包含输入的呼号（不含它自己）：以输入开头的在前，其余在后，各自字母序；最多 `limit` 个。
  pub partial: Vec<String>,
  /// 包含输入的呼号总数（`partial` 可能被 `limit` 截断）。
  pub partial_total: usize,
  /// 只差一位（改一位 / 多一位 / 少一位）、且不包含输入的呼号；最多 `limit` 个。
  pub similar: Vec<String>,
  pub similar_total: usize,
}

/// 两个字符串是否恰好差一次编辑（替换、插入或删除一个字符）。相同返回 `false`。
fn one_edit_apart(a: &[u8], b: &[u8]) -> bool {
  let (short, long) = if a.len() <= b.len() { (a, b) } else { (b, a) };
  if long.len() - short.len() > 1 || a == b {
    return false;
  }
  if short.len() == long.len() {
    return short.iter().zip(long).filter(|(x, y)| x != y).count() == 1;
  }
  // 长的删掉一个字符后要与短的相同：找到第一个不同的位置，跳过长的那一个再比后半段。
  let i = short.iter().zip(long).take_while(|(x, y)| x == y).count();
  short[i..] == long[i + 1..]
}

impl Index {
  /// 由若干来源合并建立：大写、丢掉不像呼号的、去重、排序。
  pub fn build<I, S>(sources: I) -> Self
  where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
  {
    let mut set: HashSet<String> = HashSet::new();
    for s in sources {
      let c = s.as_ref().trim().to_ascii_uppercase();
      if is_callsign(&c) {
        set.insert(c);
      }
    }
    let mut calls: Vec<String> = set.into_iter().collect();
    calls.sort_unstable();
    Self { calls }
  }

  #[must_use]
  pub fn len(&self) -> usize {
    self.calls.len()
  }

  #[must_use]
  pub fn is_empty(&self) -> bool {
    self.calls.is_empty()
  }

  #[must_use]
  pub fn contains(&self, call: &str) -> bool {
    self
      .calls
      .binary_search_by(|c| c.as_str().cmp(call))
      .is_ok()
  }

  /// 查询。输入不足 [`MIN_PARTIAL`] 位时返回 `None`（一两个字符会命中一大半的库，没有意义）。
  #[must_use]
  pub fn lookup(&self, typed: &str, limit: usize) -> Option<Lookup> {
    let t = typed.trim().to_ascii_uppercase();
    if t.chars().count() < MIN_PARTIAL {
      return None;
    }
    let exact = self.contains(&t);

    // 「包含」与「差一位」合并成单遍扫描（此前是两遍全表遍历）：每次按键 O(N)、
    // 常数减半。命中「包含」的行不会再进「差一位」的字节比较，行为与原先一致。
    // 以输入开头的（字母序）与其余「包含」的分开收，最后先放前者。
    let mut prefix: Vec<&String> = Vec::new();
    let mut inside: Vec<&String> = Vec::new();
    let mut similar: Vec<String> = Vec::new();
    let mut similar_total = 0usize;
    let want_similar = t.chars().count() >= MIN_SIMILAR;
    for c in &self.calls {
      if c == &t {
        continue;
      }
      if c.contains(t.as_str()) {
        if c.starts_with(t.as_str()) {
          prefix.push(c);
        } else {
          inside.push(c);
        }
        continue;
      }
      if want_similar && one_edit_apart(c.as_bytes(), t.as_bytes()) {
        similar_total += 1;
        if similar.len() < limit {
          similar.push(c.clone());
        }
      }
    }
    let partial_total = prefix.len() + inside.len();
    let partial: Vec<String> = prefix
      .into_iter()
      .chain(inside)
      .take(limit)
      .cloned()
      .collect();

    Some(Lookup {
      exact,
      partial,
      partial_total,
      similar,
      similar_total,
    })
  }
}

#[cfg(test)]
mod tests {
  #[test]
  fn special_calls_and_duplicate_valid_lines_are_accepted() {
    assert!(super::is_callsign("RAEM"));
    assert!(super::is_callsign("RAEM/P"));
    assert!(!super::is_callsign("hello"));
    assert_eq!(
      super::parse_scp("W1AW\nW1AW\nbad\nbad\n").unwrap().skipped,
      2
    );
    assert_eq!(super::parse_scp("RAEM\n").unwrap().calls, ["RAEM"]);
  }

  use super::*;

  /// 呼号取自真实的 `MASTER.SCP`（2026-10-14 版）里的一小撮：它们是事实，不是库本身。
  const CALLS: &[&str] = &[
    "2D0OMN",
    "2E0AOZ",
    "G5Y/P",
    "GM0FRT/P",
    "I/DL3NC",
    "N6QEK/KL7",
    "VO2DX/9",
    "W1ABK",
    "W1ABY",
    "W1AEC",
    "W1AM",
    "W1AN",
    "W1AO",
    "W1AQ",
    "W1AW",
    "W1CW",
    "W1MAW",
    "W1TW",
    "W1YW",
    "W2AW",
    "W3AW",
    "W6AW",
    "WX1AW",
  ];

  fn index() -> Index {
    Index::build(CALLS)
  }

  /// 真实文件的头 + 上面的数据行，CRLF。
  fn scp_text() -> String {
    let mut s = String::from(
      "!!Order,1,1\r\n#\r\n# Super Check Partial\r\n# Release 2026.10.14\r\n# Generated by bb-scp\r\n#\r\n",
    );
    for c in CALLS {
      s.push_str(c);
      s.push_str("\r\n");
    }
    s
  }

  fn strs(v: &[String]) -> Vec<&str> {
    v.iter().map(String::as_str).collect()
  }

  #[test]
  fn parses_the_real_file_layout() {
    let f = parse_scp(&scp_text()).unwrap();
    assert_eq!(f.release.as_deref(), Some("2026.10.14"));
    assert_eq!(f.skipped, 0);
    assert_eq!(f.calls.len(), CALLS.len());
    // 输出是字母序、大写、去重的。
    assert!(f.calls.windows(2).all(|w| w[0] < w[1]));
    assert!(f.calls.contains(&"N6QEK/KL7".to_owned()));
  }

  #[test]
  fn tolerates_lf_bom_blank_lines_case_duplicates_and_missing_header() {
    let f = parse_scp("\u{feff}w1aw\n\n  BG4ABC  \nW1AW\nja1abc\n").unwrap();
    assert_eq!(strs(&f.calls), ["BG4ABC", "JA1ABC", "W1AW"]);
    assert_eq!(f.release, None);
    // 头里没有 Release 但有别的注释：照样当注释跳过，不当成呼号也不计入 skipped。
    let g = parse_scp("!!Order,1,1\n# something else\nW1AW\n").unwrap();
    assert_eq!((g.calls.len(), g.skipped, g.release), (1, 0, None));
  }

  #[test]
  fn the_canonical_text_round_trips_through_the_parser() {
    let f = parse_scp(&scp_text()).unwrap();
    let text = f.to_text();
    assert!(
      text.starts_with("# Release 2026.10.14\nW1ABK")
        || text.starts_with("# Release 2026.10.14\n2D0OMN")
    );
    assert!(!text.contains('\r'));
    let back = parse_scp(&text).unwrap();
    assert_eq!(
      (back.calls, back.release, back.skipped),
      (f.calls, f.release, 0)
    );
    // 没有版本时不写头。
    let bare = parse_scp("W1AW\nJA1ABC\n").unwrap().to_text();
    assert_eq!(bare, "JA1ABC\nW1AW\n");
  }

  #[test]
  fn drops_lines_that_are_not_callsigns_and_counts_them() {
    let f = parse_scp("W1AW\nJA1ABC\nDL1ABC\nhello world\n???\nAB\nBG4ABC\n").unwrap();
    assert_eq!(f.calls.len(), 4);
    assert_eq!(f.skipped, 3);
  }

  #[test]
  fn refuses_files_that_are_clearly_not_scp() {
    assert_eq!(parse_scp(""), Err(ScpError::NoCallsigns));
    assert_eq!(
      parse_scp("!!Order,1,1\n# only a header\n"),
      Err(ScpError::NoCallsigns)
    );
    assert_eq!(
      parse_scp("this is not\na callsign list\nat all\n"),
      Err(ScpError::NoCallsigns)
    );
    // 好行比坏行少：多半选错了文件（比如把 Cabrillo 或 ADIF 选来了）。
    assert_eq!(
      parse_scp("W1AW\nSTART-OF-LOG: 3.0\nCONTEST: CQ-WW-CW\nQSO: 14021 CW 2026-10-31 0010\n"),
      Err(ScpError::MostlyNotCallsigns)
    );
    // 恰好一样多不算「更多」：放行（宁可多收一个残缺文件，用户看得到被丢的条数）。
    assert!(parse_scp("W1AW\nnope\n").is_ok());
  }

  #[test]
  fn caps_the_number_of_data_lines() {
    let big: String = (0..=MAX_LINES).map(|i| format!("W{i}AAA\n")).collect();
    assert_eq!(parse_scp(&big), Err(ScpError::TooLarge));
    let ok: String = (0..1000).map(|i| format!("W{i}AAA\n")).collect();
    assert!(parse_scp(&ok).is_ok());
  }

  #[test]
  fn partial_matches_list_prefix_hits_before_inside_hits() {
    let l = index().lookup("W1A", 20).unwrap();
    assert!(!l.exact);
    // 以 W1A 开头的（字母序）在前；W1MAW 等不含 W1A 的不会出现。
    assert_eq!(
      strs(&l.partial),
      [
        "W1ABK", "W1ABY", "W1AEC", "W1AM", "W1AN", "W1AO", "W1AQ", "W1AW"
      ]
    );
    assert_eq!(l.partial_total, 8);
    // 「1AW」：W1AW 以 W 开头，不是以 1AW 开头，所以两个都在「包含」组，字母序。
    let l = index().lookup("1aw", 20).unwrap();
    assert_eq!(strs(&l.partial), ["W1AW", "WX1AW"]);
    // 中间位置也能命中：DL3NC 在 I/DL3NC 里。
    assert_eq!(
      strs(&index().lookup("DL3NC", 5).unwrap().partial),
      ["I/DL3NC"]
    );
  }

  #[test]
  fn an_exact_hit_is_flagged_and_not_listed_among_the_partials() {
    let l = index().lookup("w1aw", 20).unwrap();
    assert!(l.exact);
    assert!(!l.partial.contains(&"W1AW".to_owned()));
    // 带斜杠的呼号能精确命中。
    assert!(index().lookup("N6QEK/KL7", 5).unwrap().exact);
    // 输入的是别人的一部分、自己不在库里：不是 exact。
    assert!(!index().lookup("N6QEK", 5).unwrap().exact);
  }

  #[test]
  fn similar_calls_differ_by_one_substitution_insertion_or_deletion() {
    // 真实库里与 W1AW 差一位的（独立脚本算出）：替换 W1AM/W1AN/W1AO/W1AQ/W1CW/W1TW/W1YW/W2AW/W3AW/W6AW，
    // 插入 W1MAW/WX1AW。
    let l = index().lookup("W1AW", 50).unwrap();
    assert_eq!(
      strs(&l.similar),
      [
        "W1AM", "W1AN", "W1AO", "W1AQ", "W1CW", "W1MAW", "W1TW", "W1YW", "W2AW", "W3AW", "W6AW",
        "WX1AW"
      ]
    );
    assert_eq!(l.similar_total, 12);
    // 打错一位的呼号：库里没有，列出相近的。
    let l = index().lookup("W1AX", 20).unwrap();
    assert!(!l.exact);
    assert_eq!(strs(&l.similar), ["W1AM", "W1AN", "W1AO", "W1AQ", "W1AW"]);
    // 多打了一位：库里的 W1AW 是它删掉一位的结果。
    assert_eq!(
      strs(&index().lookup("W1AWW", 20).unwrap().similar),
      ["W1AW"]
    );
  }

  #[test]
  fn a_call_is_never_both_partial_and_similar() {
    for t in ["W1AW", "W1A", "W1AX", "W1AM", "DL3NC", "1AW", "VO2DX/"] {
      let l = index().lookup(t, 50).unwrap();
      for s in &l.similar {
        assert!(!l.partial.contains(s), "{t}: {s} 同时出现在两组");
        assert!(
          !s.contains(&t.to_ascii_uppercase()),
          "{t}: {s} 包含输入却在 similar 里"
        );
      }
    }
  }

  #[test]
  fn needs_enough_characters_before_it_says_anything() {
    let ix = index();
    assert_eq!(ix.lookup("", 5), None);
    assert_eq!(ix.lookup("W", 5), None);
    assert_eq!(ix.lookup("  w ", 5), None);
    // 两位：给「包含」候选，但不给相近呼号（改一位的组合太多）。
    let l = ix.lookup("W1", 5).unwrap();
    assert_eq!(l.partial.len(), 5);
    assert!(l.partial_total > 5);
    assert!(l.similar.is_empty() && l.similar_total == 0);
  }

  #[test]
  fn limit_truncates_the_list_but_the_total_stays_honest() {
    let l = index().lookup("W1", 3).unwrap();
    assert_eq!(l.partial.len(), 3);
    assert_eq!(l.partial_total, 12); // W1ABK W1ABY W1AEC W1AM W1AN W1AO W1AQ W1AW W1CW W1MAW W1TW W1YW
    assert_eq!(strs(&l.partial), ["W1ABK", "W1ABY", "W1AEC"]);
    let z = index().lookup("W1", 0).unwrap();
    assert!(z.partial.is_empty() && z.partial_total == 12);
  }

  #[test]
  fn nothing_matches_nothing() {
    let l = index().lookup("ZZ9ZZZ", 5).unwrap();
    assert_eq!(l, Lookup::default());
    assert!(
      Index::default()
        .lookup("W1AW", 5)
        .unwrap()
        .partial
        .is_empty()
    );
    assert!(Index::default().is_empty());
  }

  #[test]
  fn one_edit_apart_is_exactly_one_edit() {
    let e = |a: &str, b: &str| one_edit_apart(a.as_bytes(), b.as_bytes());
    assert!(e("W1AW", "W1AX")); // 替换
    assert!(e("W1AW", "W1AWW")); // 插入（反方向也成立）
    assert!(e("W1AWW", "W1AW"));
    assert!(e("W1AW", "1AW")); // 删除开头
    assert!(e("W1AW", "W1A")); // 删除结尾
    assert!(e("W1AW", "WAW")); // 删除中间
    assert!(!e("W1AW", "W1AW")); // 相同不算
    assert!(!e("W1AW", "W1XY")); // 两处替换
    assert!(!e("W1AW", "W1AWWW")); // 差两位
    assert!(!e("W1AW", "A1WW")); // 换位是两次替换，不算
    assert!(!e("", "")); // 空串
    assert!(e("", "A")); // 空串到一个字符是一次插入
  }

  #[test]
  fn build_merges_sources_and_ignores_junk() {
    let ix = Index::build(["w1aw", "W1AW", " ja1abc ", "???", "ab", "BG4ABC"]);
    assert_eq!(ix.len(), 3);
    assert!(ix.contains("W1AW") && ix.contains("JA1ABC") && ix.contains("BG4ABC"));
    assert!(!ix.contains("AB"));
    // 日志呼号 + SCP 文件呼号合并后去重。
    let from_file = parse_scp("W1AW\nDL1ABC\n").unwrap().calls;
    let merged = Index::build(
      ["W1AW", "VK2XX"]
        .into_iter()
        .map(str::to_owned)
        .chain(from_file),
    );
    assert_eq!(merged.len(), 3);
  }

  #[test]
  fn callsign_shape_rules() {
    for ok in [
      "W1AW",
      "BG4ABC",
      "N6QEK/KL7",
      "I/DL3NC",
      "9A1AA",
      "VO2DX/9",
      "2E0AOZ",
    ] {
      assert!(is_callsign(ok), "{ok}");
    }
    for bad in [
      "AB",
      "ABCDEF",
      "123456",
      "/W1AW",
      "W1AW/",
      "W1 AW",
      "W1AW-1",
      "",
      "W1AWABCDEFGHIJK",
    ] {
      assert!(!is_callsign(bad), "{bad}");
    }
  }
}
