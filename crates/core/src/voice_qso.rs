//! 话音通联脚本：HF 话音里「对方叫过来，你限时回一整段」的练习。
//!
//! 站内已经有字母解释法、RST 试听和呼号抄收，缺的是把它们连起来的一整段：
//! 对方（语音合成）报上呼号、给你信号报告、说他的名字、QTH、设备与天线，最后 `over`；
//! 你要在限定时间里回：**双方呼号、给对方的报告、你的名字、QTH、设备、天线，再说 73**。
//!
//! 不做语音识别：本机录音只用来回放自己的声音，「说到了哪几项」由用户自评打勾，
//! 本模块只管剧本、计时与漏项 / 超时的判定。
//!
//! 脚本有中文与英文两套（HF 话音在国内两种都有）；界面语言是另一回事（西语界面走译文）。
//! 本模块不含界面文字，各项的说明由界面按 [`Item`] 取词条。

use crate::phonetic::word_of;

/// 通话语言（脚本语言，不是界面语言）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
  Zh,
  En,
}

impl Lang {
  /// 语音合成的 BCP-47 标签。
  #[must_use]
  pub const fn speech_tag(self) -> &'static str {
    match self {
      Self::Zh => "zh-CN",
      Self::En => "en-US",
    }
  }
}

/// 回话里必须说到的项（顺序即建议的说话顺序）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Item {
  /// 双方呼号（用字母解释法）。
  Callsigns,
  /// 给对方的信号报告（RS，如 5 9）。
  Report,
  /// 你的名字。
  Name,
  /// 你的 QTH（所在地）。
  Qth,
  /// 你的设备。
  Rig,
  /// 你的天线。
  Antenna,
  /// 以 73 与双方呼号收尾。
  SignOff,
}

/// 全部必说项，按建议顺序。
pub const ITEMS: [Item; 7] = [
  Item::Callsigns,
  Item::Report,
  Item::Name,
  Item::Qth,
  Item::Rig,
  Item::Antenna,
  Item::SignOff,
];

/// 回话的时间限制（秒）。
pub const LIMIT_SECS: u32 = 60;

/// 一场练习的对方与示例的「我」。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scenario {
  /// 我的呼号。
  pub me: String,
  /// 对方呼号。
  pub partner: String,
  persona: usize,
  rig: usize,
  antenna: usize,
  /// 对方给我的 RS 报告（R, S）。
  pub partner_rs: (u8, u8),
  /// 参考答案里「我」用的示例资料（用户的真实资料站内没有）。
  me_persona: usize,
  me_rig: usize,
  me_antenna: usize,
  /// 参考答案里我给对方的 RS 报告。
  pub my_rs: (u8, u8),
}

/// 一个人物：呼号前缀与所在地要对得上。
struct Persona {
  prefix: &'static str,
  name_en: &'static str,
  name_zh: &'static str,
  qth_en: &'static str,
  qth_zh: &'static str,
}

const fn p(
  prefix: &'static str,
  name_en: &'static str,
  name_zh: &'static str,
  qth_en: &'static str,
  qth_zh: &'static str,
) -> Persona {
  Persona {
    prefix,
    name_en,
    name_zh,
    qth_en,
    qth_zh,
  }
}

const PERSONAS: &[Persona] = &[
  p("K", "Bob", "鲍勃", "Boston", "波士顿"),
  p("W", "Sue", "苏", "Denver", "丹佛"),
  p("JA", "Hiro", "弘", "Osaka", "大阪"),
  p("DL", "Anna", "安娜", "Hamburg", "汉堡"),
  p("G", "Tom", "汤姆", "Leeds", "利兹"),
  p("VK", "Jack", "杰克", "Perth", "珀斯"),
  p("ZL", "Kate", "凯特", "Wellington", "惠灵顿"),
  p("PY", "Carlos", "卡洛斯", "Sao Paulo", "圣保罗"),
  p("VE", "Eve", "伊芙", "Calgary", "卡尔加里"),
  p("ON", "Luc", "吕克", "Ghent", "根特"),
  p("BG", "Wang", "老王", "Wuhan", "武汉"),
  p("BH", "Chen", "小陈", "Chengdu", "成都"),
  p("BA", "Li", "老李", "Xian", "西安"),
  p("BD", "Zhao", "小赵", "Harbin", "哈尔滨"),
];

/// 设备型号在两种语言里写法相同。
const RIGS: &[&str] = &["FT-991A", "IC-7300", "TS-590SG", "FT-710", "IC-705", "KX3"];

/// 天线：（英文，中文）。
const ANTENNAS: &[(&str, &str)] = &[
  ("dipole", "偶极子天线"),
  ("vertical", "垂直天线"),
  ("three-element Yagi", "三单元八木"),
  ("end-fed half-wave", "端馈半波天线"),
  ("magnetic loop", "磁环天线"),
  ("inverted V", "倒 V 天线"),
];

/// splitmix64。
struct Rng(u64);

impl Rng {
  fn next(&mut self) -> u64 {
    self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = self.0;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
  }

  fn below(&mut self, n: usize) -> usize {
    (self.next() % n as u64) as usize
  }
}

/// 示例呼号：用户没填、或填的不像呼号时用它。
pub const DEFAULT_CALL: &str = "BG1AAA";

/// 呼号是否「像样」：3–7 位字母数字、至少一位数字与一位字母（话音里不挑 FT8 那样严的格式，复合呼号也行）。
#[must_use]
pub fn looks_like_call(s: &str) -> bool {
  (3..=7).contains(&s.len())
    && s
      .bytes()
      .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
    && s.bytes().any(|c| c.is_ascii_digit())
    && s.bytes().any(|c| c.is_ascii_uppercase())
}

impl Scenario {
  /// 由种子随机生成一场练习：对方的呼号、名字、QTH、设备、天线与报告都由种子决定。
  #[must_use]
  pub fn generate(seed: u64, me_call: &str) -> Self {
    let me = me_call.trim().to_ascii_uppercase();
    let me = if looks_like_call(&me) {
      me
    } else {
      DEFAULT_CALL.to_owned()
    };
    let mut rng = Rng(seed);
    let persona = rng.below(PERSONAS.len());
    let partner = loop {
      let mut c = PERSONAS[persona].prefix.to_owned();
      c.push(char::from(b'0' + rng.below(10) as u8));
      for _ in 0..2 + rng.below(2) {
        c.push(char::from(b'A' + rng.below(26) as u8));
      }
      if c != me {
        break c;
      }
    };
    let rig = rng.below(RIGS.len());
    let antenna = rng.below(ANTENNAS.len());
    // 报告：R 恒为 5（HF 话音里清晰可懂就是 5），S 取 5–9。
    let partner_rs = (5, 5 + rng.below(5) as u8);
    let my_rs = (5, 5 + rng.below(5) as u8);
    // 参考答案里的「我」：随便取一个与对方不同的人物（只用名字与所在地做示例）。
    let me_persona = (persona + 1 + rng.below(PERSONAS.len() - 1)) % PERSONAS.len();
    Self {
      me,
      partner,
      persona,
      rig,
      antenna,
      partner_rs,
      me_persona,
      me_rig: rng.below(RIGS.len()),
      me_antenna: rng.below(ANTENNAS.len()),
      my_rs,
    }
  }

  /// 对方的名字。
  #[must_use]
  pub fn partner_name(&self, lang: Lang) -> &'static str {
    let p = &PERSONAS[self.persona];
    match lang {
      Lang::En => p.name_en,
      Lang::Zh => p.name_zh,
    }
  }

  /// 对方的 QTH。
  #[must_use]
  pub fn partner_qth(&self, lang: Lang) -> &'static str {
    let p = &PERSONAS[self.persona];
    match lang {
      Lang::En => p.qth_en,
      Lang::Zh => p.qth_zh,
    }
  }

  /// 对方的设备。
  #[must_use]
  pub fn partner_rig(&self) -> &'static str {
    RIGS[self.rig]
  }

  /// 对方的天线。
  #[must_use]
  pub fn partner_antenna(&self, lang: Lang) -> &'static str {
    let a = ANTENNAS[self.antenna];
    match lang {
      Lang::En => a.0,
      Lang::Zh => a.1,
    }
  }

  /// 对方来电的整段话（交给语音合成，也可以显示成文字）。
  #[must_use]
  pub fn partner_speech(&self, lang: Lang) -> String {
    let me = spell(&self.me, lang);
    let them = spell(&self.partner, lang);
    let rs = rs_words(self.partner_rs, lang);
    let (name, qth, rig, ant) = (
      self.partner_name(lang),
      self.partner_qth(lang),
      self.partner_rig(),
      self.partner_antenna(lang),
    );
    match lang {
      Lang::En => format!(
        "{me}, this is {them}. You are {rs}. My name is {name}, {name}. My QTH is {qth}. \
         I am running a {rig} and a {ant}. {me}, this is {them}, over."
      ),
      Lang::Zh => format!(
        "{me}，这里是{them}。你的信号报告是{rs}。我叫{name}，{name}。我的 QTH 在{qth}。\
         我用的设备是 {rig}，天线是{ant}。{me}，这里是{them}，请回答。"
      ),
    }
  }

  /// 参考答案：把七项都说到的一段示范（名字、QTH、设备、天线是示例，不是用户的真实资料）。
  #[must_use]
  pub fn model_reply(&self, lang: Lang) -> String {
    let me = spell(&self.me, lang);
    let them = spell(&self.partner, lang);
    let rs = rs_words(self.my_rs, lang);
    let mp = &PERSONAS[self.me_persona];
    let rig = RIGS[self.me_rig];
    let ant = ANTENNAS[self.me_antenna];
    match lang {
      Lang::En => format!(
        "{them}, this is {me}. Thank you for the call. Your report is {rs}. \
         My name is {}, my QTH is {}. I am running a {rig} and a {}. \
         Seventy-three, {them}, this is {me}.",
        mp.name_en, mp.qth_en, ant.0
      ),
      Lang::Zh => format!(
        "{them}，这里是{me}。谢谢你的呼叫。你的信号报告是{rs}。\
         我叫{}，我的 QTH 在{}。我用的设备是 {rig}，天线是{}。\
         七三，{them}，这里是{me}。",
        mp.name_zh, mp.qth_zh, ant.1
      ),
    }
  }
}

/// 数字的英文读法（9 读 Niner，0 读 Zero）。
fn digit_en(c: char) -> &'static str {
  match c {
    '0' => "Zero",
    '1' => "One",
    '2' => "Two",
    '3' => "Three",
    '4' => "Four",
    '5' => "Five",
    '6' => "Six",
    '7' => "Seven",
    '8' => "Eight",
    '9' => "Niner",
    _ => "",
  }
}

/// 数字的中文读法。
fn digit_zh(c: char) -> &'static str {
  match c {
    '0' => "零",
    '1' => "一",
    '2' => "二",
    '3' => "三",
    '4' => "四",
    '5' => "五",
    '6' => "六",
    '7' => "七",
    '8' => "八",
    '9' => "九",
    _ => "",
  }
}

/// 用字母解释法拼读呼号：字母用 ITU 单词，数字按语言读（英文 Niner，中文一二三）。
#[must_use]
pub fn spell(call: &str, lang: Lang) -> String {
  call
    .chars()
    .filter_map(|c| {
      let c = c.to_ascii_uppercase();
      if c.is_ascii_digit() {
        Some(match lang {
          Lang::En => digit_en(c),
          Lang::Zh => digit_zh(c),
        })
      } else {
        word_of(&c.to_string())
      }
    })
    .collect::<Vec<_>>()
    .join(" ")
}

/// RS 报告的读法：英文「five nine」，中文「五九」。
fn rs_words((r, s): (u8, u8), lang: Lang) -> String {
  let (r, s) = (char::from(b'0' + r), char::from(b'0' + s));
  match lang {
    Lang::En => format!(
      "{} {}",
      digit_en(r).to_lowercase(),
      digit_en(s).to_lowercase()
    ),
    Lang::Zh => format!("{}{}", digit_zh(r), digit_zh(s)),
  }
}

/// 倒计时显示。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Clock {
  /// 还剩几秒（向上取整：剩 0.4 秒仍显示 1，恰好用完显示 0）。
  Left(u32),
  /// 已超时几秒（向上取整）。
  Over(u32),
}

const LIMIT_MS: u64 = LIMIT_SECS as u64 * 1000;

/// 已用 `elapsed_ms` 毫秒时的倒计时。
#[must_use]
pub fn clock(elapsed_ms: u64) -> Clock {
  if elapsed_ms <= LIMIT_MS {
    Clock::Left((LIMIT_MS - elapsed_ms).div_ceil(1000) as u32)
  } else {
    Clock::Over((elapsed_ms - LIMIT_MS).div_ceil(1000) as u32)
  }
}

/// 一次回话的评判。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Review {
  /// 用时（秒，向上取整）。
  pub elapsed_secs: u32,
  /// 超时几秒；没超时为 0。
  pub overtime_secs: u32,
  /// 没说到的项，按建议顺序。
  pub missing: Vec<Item>,
}

impl Review {
  /// 七项都说到、也没超时。
  #[must_use]
  pub fn complete(&self) -> bool {
    self.missing.is_empty() && self.overtime_secs == 0
  }
}

/// 评判：`done` 是用户自评说到的项（重复的只算一次），`elapsed_ms` 是回话用时。
#[must_use]
pub fn review(done: &[Item], elapsed_ms: u64) -> Review {
  let elapsed_secs = elapsed_ms.div_ceil(1000) as u32;
  Review {
    elapsed_secs,
    overtime_secs: match clock(elapsed_ms) {
      Clock::Over(n) => n,
      Clock::Left(_) => 0,
    },
    missing: ITEMS
      .iter()
      .copied()
      .filter(|i| !done.contains(i))
      .collect(),
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  /// ITU 语音字母表（测试里独立写一份，不借被测代码的表）。
  const NATO: [&str; 26] = [
    "Alfa", "Bravo", "Charlie", "Delta", "Echo", "Foxtrot", "Golf", "Hotel", "India", "Juliett",
    "Kilo", "Lima", "Mike", "November", "Oscar", "Papa", "Quebec", "Romeo", "Sierra", "Tango",
    "Uniform", "Victor", "Whiskey", "X-ray", "Yankee", "Zulu",
  ];
  const DIGITS_EN: [&str; 10] = [
    "Zero", "One", "Two", "Three", "Four", "Five", "Six", "Seven", "Eight", "Niner",
  ];
  const DIGITS_ZH: [&str; 10] = ["零", "一", "二", "三", "四", "五", "六", "七", "八", "九"];

  fn independent_spell(call: &str, digits: &[&str; 10]) -> String {
    call
      .bytes()
      .map(|b| {
        if b.is_ascii_digit() {
          digits[usize::from(b - b'0')].to_owned()
        } else {
          NATO[usize::from(b - b'A')].to_owned()
        }
      })
      .collect::<Vec<_>>()
      .join(" ")
  }

  fn scenario() -> Scenario {
    Scenario::generate(7, "BG4XYZ")
  }

  // ---------------- 漏项与超时 ----------------

  #[test]
  fn all_seven_items_in_time_is_complete() {
    let r = review(&ITEMS, 45_000);
    assert!(r.complete());
    assert_eq!((r.elapsed_secs, r.overtime_secs), (45, 0));
    assert!(r.missing.is_empty());
  }

  #[test]
  fn every_single_missing_item_is_reported_in_suggested_order() {
    for skipped in ITEMS {
      let done: Vec<Item> = ITEMS.iter().copied().filter(|i| *i != skipped).collect();
      let r = review(&done, 30_000);
      assert_eq!(r.missing, [skipped]);
      assert!(!r.complete(), "{skipped:?}");
    }
    // 什么都没说：七项全在，且顺序就是建议的说话顺序。
    assert_eq!(review(&[], 1000).missing, ITEMS);
    // 说了几项：缺的按顺序列出。
    let r = review(&[Item::Name, Item::Callsigns, Item::SignOff], 10_000);
    assert_eq!(
      r.missing,
      [Item::Report, Item::Qth, Item::Rig, Item::Antenna]
    );
  }

  #[test]
  fn duplicates_in_the_ticks_count_once() {
    let r = review(&[Item::Name, Item::Name, Item::Name], 0);
    assert_eq!(r.missing.len(), 6);
  }

  #[test]
  fn the_limit_is_inclusive_and_overtime_rounds_up() {
    // 60 s 整不算超时；多出 1 ms 就是 1 秒；61 s 整仍是 1 秒；61.001 s 是 2 秒。
    for (ms, over, used) in [
      (0u64, 0u32, 0u32),
      (59_999, 0, 60),
      (60_000, 0, 60),
      (60_001, 1, 61),
      (61_000, 1, 61),
      (61_001, 2, 62),
      (90_000, 30, 90),
    ] {
      let r = review(&ITEMS, ms);
      assert_eq!((r.overtime_secs, r.elapsed_secs), (over, used), "{ms} ms");
      assert_eq!(r.complete(), over == 0, "{ms} ms");
    }
  }

  #[test]
  fn timing_out_fails_even_with_all_items_and_missing_items_fail_even_in_time() {
    assert!(!review(&ITEMS, 60_001).complete());
    assert!(!review(&ITEMS[..6], 10_000).complete());
  }

  #[test]
  fn the_countdown_agrees_with_the_review() {
    assert_eq!(clock(0), Clock::Left(60));
    assert_eq!(clock(1), Clock::Left(60), "剩 59.999 s 仍显示 60");
    assert_eq!(clock(59_600), Clock::Left(1), "剩 0.4 s 显示 1");
    assert_eq!(clock(60_000), Clock::Left(0));
    assert_eq!(clock(60_001), Clock::Over(1));
    assert_eq!(clock(61_001), Clock::Over(2));
    for ms in [0u64, 1, 59_999, 60_000, 60_001, 61_000, 61_001, 125_000] {
      let over = match clock(ms) {
        Clock::Over(n) => n,
        Clock::Left(_) => 0,
      };
      assert_eq!(over, review(&ITEMS, ms).overtime_secs, "{ms} ms");
    }
  }

  // ---------------- 剧本 ----------------

  #[test]
  fn the_partner_speech_spells_both_calls_with_the_itu_alphabet_in_both_languages() {
    for seed in 0..200u64 {
      let sc = Scenario::generate(seed, "BG4XYZ");
      let en = sc.partner_speech(Lang::En);
      let zh = sc.partner_speech(Lang::Zh);
      for call in [&sc.me, &sc.partner] {
        assert!(
          en.contains(&independent_spell(call, &DIGITS_EN)),
          "seed {seed} en: {call}\n{en}"
        );
        assert!(
          zh.contains(&independent_spell(call, &DIGITS_ZH)),
          "seed {seed} zh: {call}\n{zh}"
        );
      }
    }
  }

  #[test]
  fn the_partner_speech_carries_every_fact_the_listener_must_copy() {
    let sc = scenario();
    let en = sc.partner_speech(Lang::En);
    assert!(
      en.contains(&format!("My name is {}", sc.partner_name(Lang::En))),
      "{en}"
    );
    assert!(
      en.contains(&format!("My QTH is {}", sc.partner_qth(Lang::En))),
      "{en}"
    );
    assert!(
      en.contains(sc.partner_rig()) && en.contains(sc.partner_antenna(Lang::En)),
      "{en}"
    );
    assert!(en.trim_end().ends_with("over."), "最后要交还话权：{en}");
    // 给我的报告：R 恒为 five，S 在 5–9。
    let words = ["five", "six", "seven", "eight", "nine"];
    assert!(
      words
        .iter()
        .any(|w| en.contains(&format!("You are five {w}."))),
      "{en}"
    );
    let zh = sc.partner_speech(Lang::Zh);
    assert!(
      zh.contains(sc.partner_name(Lang::Zh)) && zh.contains(sc.partner_qth(Lang::Zh)),
      "{zh}"
    );
    assert!(
      zh.contains(sc.partner_rig()) && zh.contains(sc.partner_antenna(Lang::Zh)),
      "{zh}"
    );
    assert!(zh.trim_end().ends_with("请回答。"), "{zh}");
    assert!(!zh.contains("over"));
  }

  #[test]
  fn the_model_reply_covers_all_seven_items() {
    for seed in 0..200u64 {
      let sc = Scenario::generate(seed, "BG4XYZ");
      for lang in [Lang::En, Lang::Zh] {
        let reply = sc.model_reply(lang);
        let digits = if lang == Lang::En {
          &DIGITS_EN
        } else {
          &DIGITS_ZH
        };
        // 呼号：双方都拼读。
        assert!(
          reply.contains(&independent_spell(&sc.me, digits)),
          "{lang:?} {seed}"
        );
        assert!(
          reply.contains(&independent_spell(&sc.partner, digits)),
          "{lang:?} {seed}"
        );
        // 报告、名字、QTH、设备、天线、73。
        let markers: [&str; 6] = match lang {
          Lang::En => [
            "Your report is five",
            "My name is",
            "my QTH is",
            "I am running a",
            "Seventy-three",
            "this is",
          ],
          Lang::Zh => [
            "你的信号报告是五",
            "我叫",
            "我的 QTH 在",
            "我用的设备是",
            "七三",
            "这里是",
          ],
        };
        for marker in markers {
          assert!(
            reply.contains(marker),
            "{lang:?} {seed}: 缺「{marker}」\n{reply}"
          );
        }
        // 天线：示例资料里的天线写法一定在。
        let ant_ok = ANTENNAS
          .iter()
          .any(|a| reply.contains(if lang == Lang::En { a.0 } else { a.1 }));
        assert!(ant_ok, "{lang:?} {seed}: 缺天线\n{reply}");
        assert!(
          RIGS.iter().any(|r| reply.contains(r)),
          "{lang:?} {seed}: 缺设备\n{reply}"
        );
        // 收尾要再报双方呼号：呼号拼读至少出现两次。
        assert!(
          reply.matches(&independent_spell(&sc.me, digits)).count() >= 2,
          "{reply}"
        );
        assert!(reply.trim_end().ends_with(['.', '。']), "{reply}");
      }
    }
  }

  // ---------------- 场景生成 ----------------

  #[test]
  fn scenarios_are_deterministic_and_the_prefix_matches_the_place() {
    assert_eq!(
      Scenario::generate(5, "BG4XYZ"),
      Scenario::generate(5, "BG4XYZ")
    );
    let mut partners = std::collections::HashSet::new();
    let mut personas = std::collections::HashSet::new();
    for seed in 0..3000u64 {
      let sc = Scenario::generate(seed, "BG4XYZ");
      assert!(looks_like_call(&sc.partner), "{}", sc.partner);
      assert_ne!(sc.partner, sc.me);
      // 呼号前缀与所在地对得上：K/W 在美国、JA 在日本……（人物表按前缀建，这里按所在地反查前缀）。
      let place = sc.partner_qth(Lang::En);
      let expect: &[&str] = match place {
        "Boston" => &["K"],
        "Denver" => &["W"],
        "Osaka" => &["JA"],
        "Hamburg" => &["DL"],
        "Leeds" => &["G"],
        "Perth" => &["VK"],
        "Wellington" => &["ZL"],
        "Sao Paulo" => &["PY"],
        "Calgary" => &["VE"],
        "Ghent" => &["ON"],
        "Wuhan" => &["BG"],
        "Chengdu" => &["BH"],
        "Xian" => &["BA"],
        "Harbin" => &["BD"],
        other => panic!("未登记的所在地 {other}"),
      };
      assert!(
        expect.iter().any(|p| sc.partner.starts_with(p)),
        "{} 不在 {place}",
        sc.partner
      );
      assert_eq!(sc.partner_rs.0, 5);
      assert!((5..=9).contains(&sc.partner_rs.1) && (5..=9).contains(&sc.my_rs.1));
      partners.insert(sc.partner.clone());
      personas.insert(place);
    }
    assert!(
      partners.len() > 1000,
      "对方呼号应当足够多样：{}",
      partners.len()
    );
    assert_eq!(personas.len(), 14, "每个人物都要出现");
  }

  #[test]
  fn my_own_call_falls_back_to_the_example_when_unusable() {
    assert_eq!(Scenario::generate(1, "bg4xyz").me, "BG4XYZ");
    for bad in ["", "AB", "ABCDEFGH", "1234", "ABCD", "BG 4XY", "bg-1"] {
      assert_eq!(Scenario::generate(1, bad).me, DEFAULT_CALL, "{bad:?}");
    }
    // 复合呼号话音里常见，也放行。
    assert_eq!(Scenario::generate(1, "BG4X").me, "BG4X");
  }

  #[test]
  fn spell_uses_niner_and_skips_symbols() {
    assert_eq!(spell("K9", Lang::En), "Kilo Niner");
    assert_eq!(spell("k9", Lang::En), "Kilo Niner");
    assert_eq!(spell("BG1/P", Lang::En), "Bravo Golf One Papa", "斜杠不读");
    assert_eq!(spell("K9", Lang::Zh), "Kilo 九");
  }

  #[test]
  fn speech_tags() {
    assert_eq!(Lang::En.speech_tag(), "en-US");
    assert_eq!(Lang::Zh.speech_tag(), "zh-CN");
  }
}
