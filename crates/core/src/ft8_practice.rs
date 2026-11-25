//! FT8 通话练习：新手开台最容易错的不是怎么操作软件，而是「对方刚发了这句，我下一句该发什么」。
//!
//! 只练 WSJT-X 的**标准序列**（FT8 协议文档里的 Tx1–Tx6）：
//!
//! ```text
//! 我先呼叫 CQ                         我回应对方的 CQ
//! 我   CQ 我 我的网格                 他   CQ 他 他的网格
//! 他   我 他 他的网格                 我   他 我 我的网格
//! 我   他 我 我给的报告               他   我 他 他给的报告
//! 他   我 他 R他给的报告              我   他 我 R我给的报告
//! 我   他 我 RR73（或 RRR）           他   我 他 RR73（或 RRR）
//! 他   我 他 73                       我   他 我 73
//! ```
//!
//! 报文都是「收件人 发件人 载荷」：**收件人在前、发件人在后**，新手最常把它们写反。
//! 不解调音频，也不涉及远征 / 竞赛类报文；报文格式由 `ham-web-ft8` 的打包逻辑兜底
//! （单测里每一条生成的报文都要能打包成标准消息并原样解回，见 `tests`）。
//!
//! 本模块不含任何界面文字：判错只返回 [`Mistake`]，说明文字由界面按枚举取词条。

/// 我在这场通联里的位置。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
  /// 我先呼叫 CQ，等别人回。
  CallingCq,
  /// 我回应别人的 CQ。
  AnsweringCq,
}

/// 收尾用 RR73 还是 RRR。两种都合法：RR73 把「收到」与「再见」合在一句里，RRR 要再等一句 73。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ending {
  Rr73,
  Rrr,
}

/// 报文的步骤种类（载荷的类型）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
  /// `CQ 呼号 网格`。
  Cq,
  /// `收件人 发件人 网格`。
  Grid,
  /// `收件人 发件人 ±nn`。
  Report,
  /// `收件人 发件人 R±nn`：收到了你的报告，这是我给你的。
  RReport,
  /// `收件人 发件人 RR73`。
  Rr73,
  /// `收件人 发件人 RRR`。
  Rrr,
  /// `收件人 发件人 73`。
  SeventyThree,
}

/// 谁在说话。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Speaker {
  Me,
  Them,
}

/// 信号报告的合法范围（FT8 载荷里的 dB 值）。
pub const REPORT_RANGE: std::ops::RangeInclusive<i8> = -30..=32;

/// 一场练习：双方的呼号、网格与各自给出的报告。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scenario {
  pub role: Role,
  pub me: String,
  pub me_grid: String,
  pub them: String,
  pub them_grid: String,
  /// 我给对方的报告。
  pub my_report: i8,
  /// 对方给我的报告。
  pub their_report: i8,
  /// 对方（或我）收尾用的是哪一种。
  pub ending: Ending,
}

/// 示例呼号与网格：用户没在台站信息里填、或填的不是标准格式时用它们。
pub const DEFAULT_CALL: &str = "BG1AAA";
pub const DEFAULT_GRID: &str = "OM89";

/// 标准呼号：1–2 位前缀（字母或数字，只有 1 位时必须是字母）+ 1 位数字 + 1–3 位字母。
///
/// 这是 FT8 标准消息能打包的呼号的常见子集（`BG1AAA`、`K1ABC`、`3Y0J`、`2E0ABC`）；
/// 带 `/` 的复合呼号与 6 位以上的不在练习范围里。
#[must_use]
pub fn is_standard_call(s: &str) -> bool {
  let b = s.as_bytes();
  if !(3..=6).contains(&b.len())
    || !b
      .iter()
      .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
  {
    return false;
  }
  // 末尾的字母段（1–3 位），其前一位必须是数字，再往前是 1–2 位前缀。
  let letters = b
    .iter()
    .rev()
    .take_while(|c| c.is_ascii_uppercase())
    .count();
  if !(1..=3).contains(&letters) || letters >= b.len() {
    return false;
  }
  let digit_at = b.len() - letters - 1;
  if !b[digit_at].is_ascii_digit() {
    return false;
  }
  let prefix = &b[..digit_at];
  match prefix.len() {
    1 => prefix[0].is_ascii_uppercase(),
    2 => true,
    _ => false,
  }
}

/// 4 位 Maidenhead 网格（`OM89`）。`RR73` 虽然符合格式，却是 FT8 的结束报文，不当网格。
#[must_use]
pub fn is_grid4(s: &str) -> bool {
  let b = s.as_bytes();
  b.len() == 4
    && b[..2].iter().all(|c| (b'A'..=b'R').contains(c))
    && b[2..].iter().all(u8::is_ascii_digit)
    && s != "RR73"
}

/// 报告的文字：`+05`、`-12`（FT8 报文里总是带符号、两位数字）。
#[must_use]
pub fn report_text(n: i8) -> String {
  format!("{n:+03}")
}

impl Kind {
  /// 这个种类的载荷文字（报告取 `report`）。
  fn payload(self, grid: &str, report: i8) -> String {
    match self {
      Self::Cq => grid.to_owned(),
      Self::Grid => grid.to_owned(),
      Self::Report => report_text(report),
      Self::RReport => format!("R{}", report_text(report)),
      Self::Rr73 => "RR73".to_owned(),
      Self::Rrr => "RRR".to_owned(),
      Self::SeventyThree => "73".to_owned(),
    }
  }
}

impl Scenario {
  /// 由种子随机生成一场练习：呼号、网格、报告与收尾方式都由种子决定（同种子同结果，便于单测与复现）。
  ///
  /// `me_call` / `me_grid` 不合格式时退回示例值；对方的呼号与网格保证与我的不同。
  #[must_use]
  pub fn generate(seed: u64, role: Role, me_call: &str, me_grid: &str) -> Self {
    let me = me_call.trim().to_ascii_uppercase();
    let me = if is_standard_call(&me) {
      me
    } else {
      DEFAULT_CALL.to_owned()
    };
    let me_grid: String = me_grid
      .trim()
      .to_ascii_uppercase()
      .chars()
      .take(4)
      .collect();
    let me_grid = if is_grid4(&me_grid) {
      me_grid
    } else {
      DEFAULT_GRID.to_owned()
    };

    let mut rng = Rng(seed);
    let them = loop {
      let c = random_call(&mut rng);
      if c != me {
        break c;
      }
    };
    let them_grid = loop {
      let g = random_grid(&mut rng);
      if g != me_grid {
        break g;
      }
    };
    Self {
      role,
      me,
      me_grid,
      them,
      them_grid,
      my_report: rng.range(-24, 6) as i8,
      their_report: rng.range(-24, 6) as i8,
      ending: if rng.next() & 1 == 0 {
        Ending::Rr73
      } else {
        Ending::Rrr
      },
    }
  }

  /// 某位说话人在某个步骤上发的完整报文。
  #[must_use]
  pub fn message(&self, speaker: Speaker, kind: Kind) -> String {
    let (from, to, grid, report) = match speaker {
      Speaker::Me => (&self.me, &self.them, &self.me_grid, self.my_report),
      Speaker::Them => (&self.them, &self.me, &self.them_grid, self.their_report),
    };
    match kind {
      Kind::Cq => format!("CQ {from} {grid}"),
      _ => format!("{to} {from} {}", kind.payload(grid, report)),
    }
  }

  fn ending_kind(&self) -> Kind {
    match self.ending {
      Ending::Rr73 => Kind::Rr73,
      Ending::Rrr => Kind::Rrr,
    }
  }

  /// 六步剧本：谁说、说哪种。我的回合里，`Kind` 是**标准答案**（收尾那一步另有一个同样合法的替代，见 [`Session::accepted`]）。
  fn script(&self) -> [(Speaker, Kind); 6] {
    use Kind::{Cq, Grid, RReport, Report, SeventyThree};
    use Speaker::{Me, Them};
    match self.role {
      Role::CallingCq => [
        (Me, Cq),
        (Them, Grid),
        (Me, Report),
        (Them, RReport),
        (Me, Kind::Rr73),
        (Them, SeventyThree),
      ],
      Role::AnsweringCq => [
        (Them, Cq),
        (Me, Grid),
        (Them, Report),
        (Me, RReport),
        (Them, self.ending_kind()),
        (Me, SeventyThree),
      ],
    }
  }
}

/// splitmix64：够用的确定性随机数，不引入依赖。
struct Rng(u64);

impl Rng {
  fn next(&mut self) -> u64 {
    self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = self.0;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
  }

  /// `[lo, hi)` 内的整数。
  fn range(&mut self, lo: i64, hi: i64) -> i64 {
    lo + (self.next() % (hi - lo) as u64) as i64
  }

  fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
    &items[(self.next() % items.len() as u64) as usize]
  }
}

fn random_call(rng: &mut Rng) -> String {
  const PREFIXES: &[&str] = &[
    "BA", "BD", "BG", "BH", "BI", "BY", "JA", "JH", "DL", "DK", "G", "M", "K", "W", "N", "VK",
    "ZL", "PY", "LU", "ON", "F", "I", "EA", "SM", "OH", "HB", "OE", "SP", "UA", "VE",
  ];
  let mut s = (*rng.pick(PREFIXES)).to_owned();
  s.push(char::from(b'0' + rng.range(0, 10) as u8));
  for _ in 0..rng.range(2, 4) {
    s.push(char::from(b'A' + rng.range(0, 26) as u8));
  }
  s
}

fn random_grid(rng: &mut Rng) -> String {
  // 纬度 field 取 D–O：避开两极（那里没有业余台站），也就不会撞上 `RR73` 这个「网格」。
  let lon = char::from(b'A' + rng.range(0, 18) as u8);
  let lat = char::from(b'D' + rng.range(0, 12) as u8);
  format!("{lon}{lat}{}{}", rng.range(0, 10), rng.range(0, 10))
}

/// 我这一步发错了什么。界面按枚举取说明文字。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mistake {
  /// 什么都没填。
  Empty,
  /// 不是练习范围内的标准报文（词数不对、呼号 / 网格 / 载荷格式不对）。
  NotStandard,
  /// 收件人与发件人写反了（写成了「我 对方 …」）。
  Swapped,
  /// 收件人不是正在通联的对方。
  WrongAddressee,
  /// 发件人不是我。
  WrongSender,
  /// 报告不在 -30…+32 内，或不是「带符号 + 两位数字」。
  BadReport,
  /// 报文本身没错，但这一步该发的是另一种。
  WrongStep { sent: Kind, expected: Kind },
  /// 网格不是我的。
  GridNotMine,
  /// 练习已经结束。
  Finished,
}

/// 一次回复的评判结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
  /// 对：`text` 是整理过的报文（大写、单空格），`kind` 是它的种类。
  Correct {
    text: String,
    kind: Kind,
  },
  Wrong(Mistake),
}

/// 一条回复解析后的样子。
struct Parsed {
  to: Option<String>,
  from: String,
  kind: Kind,
  /// 载荷里的网格（`Cq` / `Grid`）。
  grid: Option<String>,
}

fn parse_payload(p: &str) -> Result<(Kind, Option<String>), Mistake> {
  match p {
    "RR73" => return Ok((Kind::Rr73, None)),
    "RRR" => return Ok((Kind::Rrr, None)),
    "73" => return Ok((Kind::SeventyThree, None)),
    _ => {}
  }
  if is_grid4(p) {
    return Ok((Kind::Grid, Some(p.to_owned())));
  }
  let (body, kind) = match p.strip_prefix('R') {
    Some(rest) if rest.starts_with(['+', '-']) => (rest, Kind::RReport),
    _ => (p, Kind::Report),
  };
  if body.starts_with(['+', '-']) {
    // 「带符号 + 两位数字」：`-5`、`+5` 在 FT8 里不是合法报告。
    let digits = &body[1..];
    let ok = digits.len() == 2 && digits.bytes().all(|c| c.is_ascii_digit());
    let n: i16 = body.parse().unwrap_or(i16::MAX);
    return if ok && REPORT_RANGE.contains(&(n as i8)) && n.abs() <= 32 {
      Ok((kind, None))
    } else {
      Err(Mistake::BadReport)
    };
  }
  Err(Mistake::NotStandard)
}

fn parse(reply: &str) -> Result<Parsed, Mistake> {
  let tokens: Vec<String> = reply
    .split_whitespace()
    .map(str::to_ascii_uppercase)
    .collect();
  match tokens.as_slice() {
    [] => Err(Mistake::Empty),
    [cq, call, grid] if cq == "CQ" => {
      if is_standard_call(call) && is_grid4(grid) {
        Ok(Parsed {
          to: None,
          from: call.clone(),
          kind: Kind::Cq,
          grid: Some(grid.clone()),
        })
      } else {
        Err(Mistake::NotStandard)
      }
    }
    [to, from, payload] => {
      if !is_standard_call(to) || !is_standard_call(from) {
        return Err(Mistake::NotStandard);
      }
      let (kind, grid) = parse_payload(payload)?;
      Ok(Parsed {
        to: Some(to.clone()),
        from: from.clone(),
        kind,
        grid,
      })
    }
    _ => Err(Mistake::NotStandard),
  }
}

/// 会话记录里的一行。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
  pub speaker: Speaker,
  pub kind: Kind,
  pub text: String,
}

/// 成绩。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Score {
  /// 我要发的报文数（固定 3）。
  pub my_turns: usize,
  /// 已经答对的步数。
  pub done: usize,
  /// 一次就答对的步数。
  pub first_try: usize,
  /// 累计答错的次数。
  pub mistakes: usize,
}

/// 轮到我时的提示。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Prompt {
  /// 第几次轮到我（从 1 起）。
  pub my_turn: usize,
  /// 对方刚发的那句；我先发 CQ 时还没有。
  pub last_heard: Option<String>,
  /// 这一步合法的种类（收尾一步有 RR73 与 RRR 两个）。
  pub accepted: Vec<Kind>,
}

/// 一个可点选的回复。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Choice {
  pub kind: Kind,
  pub text: String,
}

/// 一场进行中的练习。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Session {
  scenario: Scenario,
  cursor: usize,
  log: Vec<Entry>,
  first_try: usize,
  mistakes: usize,
  /// 当前这一步已经答错的次数。
  tries_wrong: usize,
}

impl Session {
  /// 开始一场练习：先把开头属于对方的几句放进记录。
  #[must_use]
  pub fn new(scenario: Scenario) -> Self {
    let mut s = Self {
      scenario,
      cursor: 0,
      log: Vec::new(),
      first_try: 0,
      mistakes: 0,
      tries_wrong: 0,
    };
    s.play_their_turns();
    s
  }

  pub fn scenario(&self) -> &Scenario {
    &self.scenario
  }

  /// 到目前为止的通联记录。
  pub fn log(&self) -> &[Entry] {
    &self.log
  }

  /// 整场是否结束。
  pub fn finished(&self) -> bool {
    self.cursor >= 6
  }

  /// 把轮到对方的句子依次放进记录，直到轮到我或结束。
  fn play_their_turns(&mut self) {
    let script = self.scenario.script();
    while let Some(&(Speaker::Them, kind)) = script.get(self.cursor) {
      self.log.push(Entry {
        speaker: Speaker::Them,
        kind,
        text: self.scenario.message(Speaker::Them, kind),
      });
      self.cursor += 1;
    }
  }

  /// 当前这一步合法的种类；练习结束时为空。
  #[must_use]
  pub fn accepted(&self) -> Vec<Kind> {
    match self.scenario.script().get(self.cursor) {
      Some(&(Speaker::Me, Kind::Rr73)) => vec![Kind::Rr73, Kind::Rrr],
      Some(&(Speaker::Me, kind)) => vec![kind],
      _ => Vec::new(),
    }
  }

  /// 轮到我时的提示；练习结束时为 `None`。
  #[must_use]
  pub fn prompt(&self) -> Option<Prompt> {
    let accepted = self.accepted();
    if accepted.is_empty() {
      return None;
    }
    let my_turn = 1
      + self.scenario.script()[..self.cursor]
        .iter()
        .filter(|(s, _)| *s == Speaker::Me)
        .count();
    let last_heard = self
      .log
      .iter()
      .rev()
      .find(|e| e.speaker == Speaker::Them)
      .map(|e| e.text.clone());
    Some(Prompt {
      my_turn,
      last_heard,
      accepted,
    })
  }

  /// 可点选的回复：七种报文都写成「我发给对方」的完整形式，留给用户判断该发哪一种。
  #[must_use]
  pub fn choices(&self) -> Vec<Choice> {
    [
      Kind::Cq,
      Kind::Grid,
      Kind::Report,
      Kind::RReport,
      Kind::Rr73,
      Kind::Rrr,
      Kind::SeventyThree,
    ]
    .into_iter()
    .map(|kind| Choice {
      kind,
      text: self.scenario.message(Speaker::Me, kind),
    })
    .collect()
  }

  /// 评判一条回复。对：记入记录、推进到下一步（对方的下一句自动出现）；错：计一次错、停在原地。
  pub fn answer(&mut self, reply: &str) -> Verdict {
    let accepted = self.accepted();
    if accepted.is_empty() {
      return Verdict::Wrong(Mistake::Finished);
    }
    match self.judge(reply, &accepted) {
      Ok((text, kind)) => {
        self.log.push(Entry {
          speaker: Speaker::Me,
          kind,
          text: text.clone(),
        });
        if self.tries_wrong == 0 {
          self.first_try += 1;
        }
        self.tries_wrong = 0;
        self.cursor += 1;
        self.play_their_turns();
        Verdict::Correct { text, kind }
      }
      Err(m) => {
        self.mistakes += 1;
        self.tries_wrong += 1;
        Verdict::Wrong(m)
      }
    }
  }

  fn judge(&self, reply: &str, accepted: &[Kind]) -> Result<(String, Kind), Mistake> {
    let sc = &self.scenario;
    let p = parse(reply)?;
    if p.from != sc.me {
      // 收件人 = 我、发件人 = 对方：把「谁发给谁」写反了，是最常见的一种。
      return Err(
        if p.to.as_deref() == Some(sc.me.as_str()) && p.from == sc.them {
          Mistake::Swapped
        } else {
          Mistake::WrongSender
        },
      );
    }
    if p.kind != Kind::Cq && p.to.as_deref() != Some(sc.them.as_str()) {
      return Err(Mistake::WrongAddressee);
    }
    if !accepted.contains(&p.kind) {
      return Err(Mistake::WrongStep {
        sent: p.kind,
        expected: accepted[0],
      });
    }
    if matches!(p.kind, Kind::Cq | Kind::Grid) && p.grid.as_deref() != Some(sc.me_grid.as_str()) {
      return Err(Mistake::GridNotMine);
    }
    let text = match p.to {
      Some(to) => format!(
        "{to} {} {}",
        p.from,
        reply
          .split_whitespace()
          .last()
          .unwrap_or("")
          .to_ascii_uppercase()
      ),
      None => format!("CQ {} {}", p.from, p.grid.unwrap_or_default()),
    };
    Ok((text, p.kind))
  }

  /// 目前的成绩。
  #[must_use]
  pub fn score(&self) -> Score {
    let done = self.scenario.script()[..self.cursor.min(6)]
      .iter()
      .filter(|(s, _)| *s == Speaker::Me)
      .count();
    Score {
      my_turns: 3,
      done,
      first_try: self.first_try,
      mistakes: self.mistakes,
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use ham_web_ft8::{HashTable, MessageType, decode, encode, get_type};

  /// 手写的固定场景：我 BG1AAA（OM89），对方 K1ABC（FN31），我给 -07、对方给 -12。
  fn fixed(role: Role, ending: Ending) -> Scenario {
    Scenario {
      role,
      me: "BG1AAA".into(),
      me_grid: "OM89".into(),
      them: "K1ABC".into(),
      them_grid: "FN31".into(),
      my_report: -7,
      their_report: -12,
      ending,
    }
  }

  fn texts(s: &Session) -> Vec<&str> {
    s.log().iter().map(|e| e.text.as_str()).collect()
  }

  fn wrong(s: &mut Session, reply: &str) -> Mistake {
    match s.answer(reply) {
      Verdict::Wrong(m) => m,
      other => panic!("{reply:?} 应判错，实际 {other:?}"),
    }
  }

  // ---------------- 与 ft8 打包逻辑对拍 ----------------

  /// 这条报文能被 ft8 打包成**标准消息**（不是回退的自由文本），并原样解回。
  fn packs_as_standard(text: &str) -> bool {
    let mut h = HashTable::new();
    let Ok(m) = encode(text, &mut h) else {
      return false;
    };
    get_type(&m) == MessageType::Standard
      && decode(&m, &mut HashTable::new()).as_deref() == Ok(text)
  }

  #[test]
  fn every_message_of_every_generated_scenario_packs_as_a_standard_ft8_message() {
    for seed in 0..1500u64 {
      for role in [Role::CallingCq, Role::AnsweringCq] {
        let sc = Scenario::generate(seed, role, "BG1AAA", "OM89");
        for kind in [
          Kind::Cq,
          Kind::Grid,
          Kind::Report,
          Kind::RReport,
          Kind::Rr73,
          Kind::Rrr,
          Kind::SeventyThree,
        ] {
          for who in [Speaker::Me, Speaker::Them] {
            let text = sc.message(who, kind);
            assert!(
              packs_as_standard(&text),
              "seed {seed} {role:?} {who:?} {kind:?}: {text:?} 打包不成标准消息"
            );
          }
        }
      }
    }
  }

  #[test]
  fn report_boundaries_pack_and_the_judge_agrees_with_ft8() {
    for n in [-30i8, -1, 0, 5, 32] {
      let t = format!("K1ABC BG1AAA {}", report_text(n));
      assert!(packs_as_standard(&t), "{t}");
      let r = format!("K1ABC BG1AAA R{}", report_text(n));
      assert!(packs_as_standard(&r), "{r}");
    }
    // 判分器认的写法，ft8 也得认。「-5」ft8 也能打包，但解回来是规范的「-05」、与原文不同：
    // 练习要求发出去的就是规范写法（WSJT-X 显示与发送的都是两位），所以判分器不认一位数。
    assert!(!packs_as_standard("K1ABC BG1AAA -5"));
    assert!(!packs_as_standard("K1ABC BG1AAA +5"));
    assert!(matches!(parse_payload("-5"), Err(Mistake::BadReport)));
    assert!(matches!(parse_payload("+5"), Err(Mistake::BadReport)));
  }

  #[test]
  fn the_calls_and_grids_we_accept_are_ones_ft8_can_pack() {
    for call in [
      "BG1AAA", "K1ABC", "3Y0J", "2E0ABC", "VK2XX", "W9XYZ", "JA1ABC", "OH2BH", "PY2AA",
    ] {
      assert!(is_standard_call(call), "{call}");
      assert!(
        packs_as_standard(&format!("K1XYZ {call} RR73")) || call == "K1XYZ",
        "{call}"
      );
    }
    for bad in [
      "", "K1", "ABC", "BG1AAAA", "BGAAA1", "1ABC", "K1ABCD", "bg1aaa", "BG1AA-", "BG1/AAA",
      "123456",
    ] {
      assert!(!is_standard_call(bad), "{bad:?}");
    }
  }

  // ---------------- 剧本 ----------------

  #[test]
  fn calling_cq_follows_the_wsjt_x_sequence() {
    let mut s = Session::new(fixed(Role::CallingCq, Ending::Rr73));
    // 我先发 CQ：开头没有对方的话。
    assert!(s.log().is_empty());
    assert_eq!(s.prompt().unwrap().last_heard, None);
    assert!(matches!(
      s.answer("CQ BG1AAA OM89"),
      Verdict::Correct { kind: Kind::Cq, .. }
    ));
    assert_eq!(texts(&s), ["CQ BG1AAA OM89", "BG1AAA K1ABC FN31"]);
    assert!(matches!(
      s.answer("K1ABC BG1AAA -07"),
      Verdict::Correct {
        kind: Kind::Report,
        ..
      }
    ));
    assert_eq!(s.log().last().unwrap().text, "BG1AAA K1ABC R-12");
    assert!(matches!(
      s.answer("K1ABC BG1AAA RR73"),
      Verdict::Correct {
        kind: Kind::Rr73,
        ..
      }
    ));
    assert!(s.finished());
    assert_eq!(
      texts(&s),
      [
        "CQ BG1AAA OM89",
        "BG1AAA K1ABC FN31",
        "K1ABC BG1AAA -07",
        "BG1AAA K1ABC R-12",
        "K1ABC BG1AAA RR73",
        "BG1AAA K1ABC 73",
      ]
    );
  }

  #[test]
  fn answering_cq_follows_the_wsjt_x_sequence_with_rrr() {
    let mut s = Session::new(fixed(Role::AnsweringCq, Ending::Rrr));
    assert_eq!(texts(&s), ["CQ K1ABC FN31"]);
    assert_eq!(
      s.prompt().unwrap().last_heard.as_deref(),
      Some("CQ K1ABC FN31")
    );
    assert!(matches!(
      s.answer("K1ABC BG1AAA OM89"),
      Verdict::Correct { .. }
    ));
    assert_eq!(s.log().last().unwrap().text, "BG1AAA K1ABC -12");
    assert!(matches!(
      s.answer("K1ABC BG1AAA R-07"),
      Verdict::Correct { .. }
    ));
    assert_eq!(s.log().last().unwrap().text, "BG1AAA K1ABC RRR");
    // 对方发的是 RRR：我必须再回 73（不是 RR73，也不是再发一遍 R 报告）。
    assert_eq!(s.prompt().unwrap().accepted, [Kind::SeventyThree]);
    assert!(matches!(
      s.answer("K1ABC BG1AAA 73"),
      Verdict::Correct { .. }
    ));
    assert!(s.finished());
    assert_eq!(s.prompt(), None);
    assert_eq!(wrong(&mut s, "K1ABC BG1AAA 73"), Mistake::Finished);
  }

  #[test]
  fn the_closing_step_when_i_call_accepts_both_rr73_and_rrr() {
    for closing in ["K1ABC BG1AAA RR73", "K1ABC BG1AAA RRR"] {
      let mut s = Session::new(fixed(Role::CallingCq, Ending::Rr73));
      s.answer("CQ BG1AAA OM89");
      s.answer("K1ABC BG1AAA -07");
      assert_eq!(s.prompt().unwrap().accepted, [Kind::Rr73, Kind::Rrr]);
      assert!(
        matches!(s.answer(closing), Verdict::Correct { .. }),
        "{closing}"
      );
      // 记录里写的是用户发的那一句，而不是标准答案。
      assert_eq!(
        s.log()
          .iter()
          .rfind(|e| e.speaker == Speaker::Me)
          .unwrap()
          .text,
        closing
      );
      assert!(s.finished());
    }
  }

  // ---------------- 判错 ----------------

  #[test]
  fn each_wrong_step_is_named_with_what_was_sent_and_what_was_expected() {
    // 我先呼叫 CQ 的第二步（对方发来网格后）：该发报告。
    let mut s = Session::new(fixed(Role::CallingCq, Ending::Rr73));
    s.answer("CQ BG1AAA OM89");
    let cases = [
      ("K1ABC BG1AAA OM89", Kind::Grid),
      ("K1ABC BG1AAA R-07", Kind::RReport),
      ("K1ABC BG1AAA RR73", Kind::Rr73),
      ("K1ABC BG1AAA RRR", Kind::Rrr),
      ("K1ABC BG1AAA 73", Kind::SeventyThree),
      ("CQ BG1AAA OM89", Kind::Cq),
    ];
    for (reply, sent) in cases {
      assert_eq!(
        wrong(&mut s, reply),
        Mistake::WrongStep {
          sent,
          expected: Kind::Report
        },
        "{reply}"
      );
    }
    // 答错不推进：还停在同一步，记录没有多出东西。
    assert_eq!(s.log().len(), 2);
    assert!(matches!(
      s.answer("K1ABC BG1AAA -07"),
      Verdict::Correct { .. }
    ));
  }

  #[test]
  fn who_sends_to_whom_is_checked_and_the_swapped_order_is_named() {
    let mut s = Session::new(fixed(Role::AnsweringCq, Ending::Rr73));
    // 收件人 / 发件人写反：「BG1AAA K1ABC OM89」。
    assert_eq!(wrong(&mut s, "BG1AAA K1ABC OM89"), Mistake::Swapped);
    // 发给了别人。
    assert_eq!(wrong(&mut s, "W9XYZ BG1AAA OM89"), Mistake::WrongAddressee);
    // 发件人不是我。
    assert_eq!(wrong(&mut s, "K1ABC W9XYZ OM89"), Mistake::WrongSender);
    assert_eq!(wrong(&mut s, "CQ W9XYZ OM89"), Mistake::WrongSender);
    // 这一步该发网格，发了别人的 / 另一个网格。
    assert_eq!(wrong(&mut s, "K1ABC BG1AAA FN31"), Mistake::GridNotMine);
    assert!(matches!(
      s.answer("K1ABC BG1AAA OM89"),
      Verdict::Correct { .. }
    ));
  }

  #[test]
  fn input_that_is_not_a_standard_message_is_rejected_without_guessing() {
    let mut s = Session::new(fixed(Role::AnsweringCq, Ending::Rr73));
    assert_eq!(wrong(&mut s, ""), Mistake::Empty);
    assert_eq!(wrong(&mut s, "   "), Mistake::Empty);
    for bad in [
      "K1ABC BG1AAA",
      "K1ABC BG1AAA OM89 EXTRA",
      "K1 BG1AAA OM89",
      "K1ABC BG1AAA HELLO",
      "K1ABC BG1AAA OM8",
      "K1ABC BG1AAA OM89X",
      "CQ DX BG1AAA OM89",
      "CQ BG1AAA",
      "K1ABC BG1AAA R",
      "K1ABC BG1AAA R73",
    ] {
      assert_eq!(wrong(&mut s, bad), Mistake::NotStandard, "{bad:?}");
    }
  }

  #[test]
  fn reports_must_be_signed_two_digits_within_the_ft8_range() {
    let mut s = Session::new(fixed(Role::CallingCq, Ending::Rr73));
    s.answer("CQ BG1AAA OM89");
    for bad in ["-5", "+5", "-31", "+33", "-100", "+99", "R-5", "R+40", "-0"] {
      let reply = format!("K1ABC BG1AAA {bad}");
      assert_eq!(wrong(&mut s, &reply), Mistake::BadReport, "{bad}");
    }
    // 边界值本身合法，任何范围内的数都行（报告是我自己听出来的，没有「标准答案」）。
    let mut edge = Session::new(fixed(Role::CallingCq, Ending::Rr73));
    edge.answer("CQ BG1AAA OM89");
    assert!(matches!(
      edge.answer("K1ABC BG1AAA +32"),
      Verdict::Correct { .. }
    ));
    let mut low = Session::new(fixed(Role::CallingCq, Ending::Rr73));
    low.answer("CQ BG1AAA OM89");
    assert!(matches!(
      low.answer("K1ABC BG1AAA -30"),
      Verdict::Correct { .. }
    ));
  }

  #[test]
  fn replies_are_normalised_before_judging() {
    let mut s = Session::new(fixed(Role::AnsweringCq, Ending::Rr73));
    assert!(matches!(
      s.answer("  k1abc   bg1aaa  om89 "),
      Verdict::Correct { .. }
    ));
    assert_eq!(
      s.log()
        .iter()
        .rfind(|e| e.speaker == Speaker::Me)
        .unwrap()
        .text,
      "K1ABC BG1AAA OM89"
    );
  }

  #[test]
  fn rr73_is_never_mistaken_for_a_grid() {
    assert!(!is_grid4("RR73"));
    assert!(is_grid4("RQ73"));
    let mut s = Session::new(fixed(Role::AnsweringCq, Ending::Rr73));
    // 这一步该发网格：发 RR73 是「步骤错」而不是「网格不是我的」。
    assert_eq!(
      wrong(&mut s, "K1ABC BG1AAA RR73"),
      Mistake::WrongStep {
        sent: Kind::Rr73,
        expected: Kind::Grid
      }
    );
  }

  // ---------------- 成绩与选项 ----------------

  #[test]
  fn score_counts_first_try_steps_and_every_mistake() {
    let mut s = Session::new(fixed(Role::CallingCq, Ending::Rr73));
    assert_eq!(
      s.score(),
      Score {
        my_turns: 3,
        done: 0,
        first_try: 0,
        mistakes: 0
      }
    );
    s.answer("CQ BG1AAA OM89"); // 一次对
    assert_eq!(
      s.score(),
      Score {
        my_turns: 3,
        done: 1,
        first_try: 1,
        mistakes: 0
      }
    );
    wrong(&mut s, "K1ABC BG1AAA OM89");
    wrong(&mut s, "BG1AAA K1ABC -07");
    s.answer("K1ABC BG1AAA -07"); // 错两次后才对：不算一次对
    assert_eq!(
      s.score(),
      Score {
        my_turns: 3,
        done: 2,
        first_try: 1,
        mistakes: 2
      }
    );
    s.answer("K1ABC BG1AAA RR73"); // 一次对
    assert_eq!(
      s.score(),
      Score {
        my_turns: 3,
        done: 3,
        first_try: 2,
        mistakes: 2
      }
    );
    assert!(s.finished());
  }

  #[test]
  fn choices_are_all_seven_kinds_addressed_correctly_and_only_the_right_ones_are_accepted() {
    for role in [Role::CallingCq, Role::AnsweringCq] {
      let mut s = Session::new(fixed(role, Ending::Rr73));
      while !s.finished() {
        let accepted = s.accepted();
        let choices = s.choices();
        assert_eq!(choices.len(), 7);
        // 每个选项在这一步要么被接受，要么只因「步骤不对」被拒——不会因为写错呼号而被拒。
        for c in &choices {
          let mut probe = s.clone();
          match probe.answer(&c.text) {
            Verdict::Correct { kind, .. } => assert!(accepted.contains(&kind), "{role:?} {c:?}"),
            Verdict::Wrong(Mistake::WrongStep { sent, .. }) => {
              assert_eq!(sent, c.kind);
              assert!(!accepted.contains(&c.kind), "{role:?} {c:?}");
            }
            other => panic!("{role:?} {c:?}: {other:?}"),
          }
        }
        // 恰好有 accepted.len() 个选项被接受。
        let right = choices
          .iter()
          .filter(|c| matches!(s.clone().answer(&c.text), Verdict::Correct { .. }))
          .count();
        assert_eq!(right, accepted.len(), "{role:?} {:?}", s.prompt());
        let pick = choices.iter().find(|c| c.kind == accepted[0]).unwrap();
        s.answer(&pick.text);
      }
    }
  }

  #[test]
  fn the_prompt_numbers_my_turns_and_shows_what_was_just_heard() {
    let mut s = Session::new(fixed(Role::AnsweringCq, Ending::Rr73));
    let p = s.prompt().unwrap();
    assert_eq!(
      (p.my_turn, p.last_heard.as_deref()),
      (1, Some("CQ K1ABC FN31"))
    );
    s.answer("K1ABC BG1AAA OM89");
    let p = s.prompt().unwrap();
    assert_eq!(
      (p.my_turn, p.last_heard.as_deref()),
      (2, Some("BG1AAA K1ABC -12"))
    );
    s.answer("K1ABC BG1AAA R-07");
    let p = s.prompt().unwrap();
    assert_eq!(
      (p.my_turn, p.last_heard.as_deref()),
      (3, Some("BG1AAA K1ABC RR73"))
    );
  }

  // ---------------- 场景生成 ----------------

  #[test]
  fn scenarios_are_deterministic_and_well_formed() {
    assert_eq!(
      Scenario::generate(42, Role::CallingCq, "BG1AAA", "OM89"),
      Scenario::generate(42, Role::CallingCq, "BG1AAA", "OM89")
    );
    let mut calls = std::collections::HashSet::new();
    let (mut rr73, mut rrr) = (0, 0);
    for seed in 0..3000u64 {
      let sc = Scenario::generate(seed, Role::AnsweringCq, "BG1AAA", "OM89");
      assert!(is_standard_call(&sc.them), "{}", sc.them);
      assert!(is_grid4(&sc.them_grid), "{}", sc.them_grid);
      assert_ne!(sc.them, sc.me);
      assert_ne!(sc.them_grid, "RR73");
      assert_ne!(sc.them_grid, sc.me_grid);
      assert!(REPORT_RANGE.contains(&sc.my_report) && REPORT_RANGE.contains(&sc.their_report));
      calls.insert(sc.them.clone());
      match sc.ending {
        Ending::Rr73 => rr73 += 1,
        Ending::Rrr => rrr += 1,
      }
    }
    assert!(calls.len() > 1000, "对方呼号应当足够多样：{}", calls.len());
    assert!(
      rr73 > 1000 && rrr > 1000,
      "两种收尾都要出现：{rr73} / {rrr}"
    );
  }

  #[test]
  fn my_own_call_and_grid_fall_back_to_the_example_when_unusable() {
    let sc = Scenario::generate(1, Role::CallingCq, "bg4xyz", "pm01aa");
    assert_eq!(
      (sc.me.as_str(), sc.me_grid.as_str()),
      ("BG4XYZ", "PM01"),
      "小写、6 位网格取前 4 位"
    );
    let sc = Scenario::generate(1, Role::CallingCq, "", "");
    assert_eq!(
      (sc.me.as_str(), sc.me_grid.as_str()),
      (DEFAULT_CALL, DEFAULT_GRID)
    );
    let sc = Scenario::generate(1, Role::CallingCq, "not a call", "ZZ99");
    assert_eq!(
      (sc.me.as_str(), sc.me_grid.as_str()),
      (DEFAULT_CALL, DEFAULT_GRID)
    );
  }
}
