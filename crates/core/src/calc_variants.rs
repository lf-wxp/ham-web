//! 计算题参数化变体：波长 / 增益 dB / 馈线衰减 / LC 谐振四类同型题。
//!
//! # 为什么这么做
//!
//! 计算题的选项是死的，答案可以直接背 —— 换数不换型是标准的「防背答案」做法。
//! 变体**只在练习时本地生成**：id 一律 [`VARIANT_PREFIX`] 开头，官方题库碰不到它们；
//! 统计（`record_answer`）与收藏对变体 id 直接跳过，不污染官方内容指纹。
//!
//! # 确定性
//!
//! 生成只依赖 `(种子, 模板, 序号)`：种子取**当日序号**，同一天内同一套（断点续做能原样
//! 重建题目），次日自动换一套。参数池是人工挑的干净值；正确答案由**同一个公式现场算出**，
//! 单测对池里每个参数组逐条复核。

use crate::question::{Codes, QuestionItem, QuestionOption, QuestionType};

/// 变体 id 前缀（统计与收藏据此跳过）。
pub const VARIANT_PREFIX: &str = "variant-";

/// 由毫秒时间戳得到**本地**当天使用的种子（同一天内不变）。
///
/// `utc_offset_minutes` 是本机时区相对 UTC 的偏移（东八区 = 480）：按本地日历天换题，
/// 而不是按 UTC —— 否则东八区要到早上 8 点才「次日换题」。
#[must_use]
pub fn day_seed(now_ms: i64, utc_offset_minutes: i32) -> u32 {
  let local_ms = now_ms + i64::from(utc_offset_minutes) * 60_000;
  local_ms.div_euclid(86_400_000) as u32
}

/// 每类模板每次生成的数量。
const VARIANTS_PER_TEMPLATE: u32 = 2;

/// 全部变体（4 类 × 2 道）。
#[must_use]
pub fn variant_questions(seed: u32) -> Vec<QuestionItem> {
  let mut out = Vec::with_capacity(8);
  for template in [
    Template::Wavelength,
    Template::Gain,
    Template::Loss,
    Template::Lc,
  ] {
    for index in 0..VARIANTS_PER_TEMPLATE {
      out.push(build_variant(seed, template, index));
    }
  }
  out
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Template {
  Wavelength,
  Gain,
  Loss,
  Lc,
}

impl Template {
  fn id(self) -> &'static str {
    match self {
      Self::Wavelength => "wavelength",
      Self::Gain => "gain",
      Self::Loss => "loss",
      Self::Lc => "lc",
    }
  }
}

/// 极简 xorshift：确定性、够用（不需要密码学性质）。
fn next(rng: &mut u64) -> u64 {
  *rng ^= *rng << 13;
  *rng ^= *rng >> 7;
  *rng ^= *rng << 17;
  *rng
}

/// 每个（模板, 序号）各自的随机流：同一种子永远同一道题。
fn rng_for(seed: u32, template: Template, index: u32) -> u64 {
  let tag = match template {
    Template::Wavelength => 0x01u64,
    Template::Gain => 0x02,
    Template::Loss => 0x03,
    Template::Lc => 0x04,
  };
  let mut rng = (u64::from(seed) << 32) | (tag << 16) | u64::from(index);
  for _ in 0..4 {
    next(&mut rng);
  }
  rng
}

/// 3 位有效数字（1 位小数 → 2 位 → 3 位，随数值大小切换）。
fn fmt3(v: f64) -> String {
  // 先圆整到千分位，去掉浮点噪声（10×lg(2) 是 3.0102999…，不能打成 3.01 的尾巴）。
  let v = (v * 1000.0).round() / 1000.0;
  // 按绝对值分档：负数（如 -3 dB）与正数用同一套有效位规则。
  let a = v.abs();
  if a >= 100.0 {
    format!("{v:.0}")
  } else if a >= 10.0 {
    format!("{v:.1}")
  } else if a >= 1.0 {
    format!("{v:.2}")
  } else {
    format!("{v:.3}")
  }
}

/// 2 位小数（LC 谐振的读数习惯）。
fn fmt2(v: f64) -> String {
  let v = (v * 100.0).round() / 100.0;
  format!("{v:.2}")
}

// ── 参数池（人工挑的干净值）────────────────────────────────────────

/// 波长：频率 MHz → 300/f 米。
const POOL_WAVELENGTH: &[f64] = &[3.5, 7.0, 14.0, 21.0, 28.0, 50.0, 144.0, 430.0];

/// 增益：(P1, P2) W → 10·lg(P2/P1) dB。
const POOL_GAIN: &[(f64, f64)] = &[
  (10.0, 20.0),
  (10.0, 40.0),
  (5.0, 50.0),
  (100.0, 50.0),
  (10.0, 100.0),
  (2.0, 20.0),
  (50.0, 100.0),
  (100.0, 25.0),
];

/// 衰减：(P, L) W/dB → P·10^(-L/10) W。
const POOL_LOSS: &[(f64, f64)] = &[
  (100.0, 3.0),
  (50.0, 6.0),
  (10.0, 10.0),
  (100.0, 10.0),
  (20.0, 3.0),
  (50.0, 3.0),
  (100.0, 6.0),
  (200.0, 10.0),
];

/// LC 谐振：(L μH, C pF) → 1/(2π√(L·C)) MHz。
const POOL_LC: &[(f64, f64)] = &[
  (10.0, 100.0),
  (1.0, 100.0),
  (10.0, 1000.0),
  (1.0, 50.0),
  (2.0, 100.0),
  (20.0, 20.0),
];

/// 每个模板的公式算出的「答案数字」。
enum Answer {
  Wavelength(f64),
  Gain(f64),
  Loss(f64),
  Lc(f64),
}

fn compute(template: Template, p: &(f64, f64)) -> Answer {
  match template {
    Template::Wavelength => Answer::Wavelength(300.0 / p.0),
    Template::Gain => Answer::Gain(10.0 * (p.1 / p.0).log10()),
    Template::Loss => Answer::Loss(p.0 * 10f64.powf(-p.1 / 10.0)),
    // L 用 μH、C 用 pF：L×C 以 H×F 计是 ×1e-18，结果除以 1e6 换成 MHz。
    Template::Lc => {
      Answer::Lc(1.0 / (2.0 * std::f64::consts::PI * (p.0 * p.1 * 1e-18).sqrt()) / 1e6)
    }
  }
}

fn answer_text(answer: &Answer) -> String {
  match *answer {
    Answer::Wavelength(v) => format!("{} 米", fmt3(v)),
    Answer::Gain(v) => format!("{} dB", fmt3(v)),
    Answer::Loss(v) => format!("{} W", fmt3(v)),
    Answer::Lc(v) => format!("{} MHz", fmt2(v)),
  }
}

/// 干扰项：与正确答案同量级、按模板特性的错法（倍半、±3dB 等）。
fn distractors(answer: &Answer) -> Vec<String> {
  match *answer {
    Answer::Wavelength(v) => [fmt3(v * 0.5), fmt3(v * 2.0), fmt3(v * 1.1)]
      .into_iter()
      .map(|s| format!("{s} 米"))
      .collect(),
    Answer::Gain(v) => {
      // ±3 dB 是「翻倍/减半混淆」、+10 dB 是「×10 混淆」。答案落在 ±3 dB 附近时，
      // ±3 的干扰项会跨到 0（如 -3.01 + 3 = -0.01），格式化成 -0.010 这类尾巴 ——
      // 遇到这种情况改用「符号反了」的 -v 顶替，仍是自然的错法。
      let plus3 = if (v + 3.0).abs() < 0.05 { -v } else { v + 3.0 };
      let minus3 = if (v - 3.0).abs() < 0.05 { -v } else { v - 3.0 };
      [fmt3(plus3), fmt3(minus3), fmt3(v + 10.0)]
        .into_iter()
        .map(|s| format!("{s} dB"))
        .collect()
    }
    Answer::Loss(v) => [fmt3(v * 0.5), fmt3(v * 2.0), fmt3(v * 0.8)]
      .into_iter()
      .map(|s| format!("{s} W"))
      .collect(),
    Answer::Lc(v) => [fmt2(v * 0.5), fmt2(v * 2.0), fmt2(v * 1.2)]
      .into_iter()
      .map(|s| format!("{s} MHz"))
      .collect(),
  }
}

/// 题干（带变体标签，练习界面一眼可辨）。
fn stem(template: Template, p: &(f64, f64)) -> String {
  match template {
    Template::Wavelength => format!(
      "〔变体 · 波长换算〕频率 {} MHz 的无线电波，其波长约为：",
      fmt3(p.0)
    ),
    Template::Gain => format!(
      "〔变体 · 功率增益〕发射功率由 {} W 变为 {} W，功率增益约为：",
      fmt3(p.0),
      fmt3(p.1)
    ),
    Template::Loss => format!(
      "〔变体 · 馈线衰减〕发射功率 {} W，馈线损耗 {} dB，天线端功率约为：",
      fmt3(p.0),
      fmt3(p.1)
    ),
    Template::Lc => format!(
      "〔变体 · LC 谐振〕电感 {} μH 与电容 {} pF 的并联谐振回路，谐振频率约为：",
      fmt3(p.0),
      fmt3(p.1)
    ),
  }
}

/// 解析：同一公式现场代值，讲清每一步。
fn explanation(template: Template, p: &(f64, f64), answer_text: &str) -> String {
  match template {
    Template::Wavelength => format!(
      "波长 λ（米）= 300 ÷ 频率 f（MHz）。本题：300 ÷ {} ≈ {}。",
      fmt3(p.0),
      answer_text
    ),
    Template::Gain => format!(
      "增益 G（dB）= 10 × lg(P₂ ÷ P₁)。本题：10 × lg({} ÷ {}) ≈ {}。",
      fmt3(p.1),
      fmt3(p.0),
      answer_text
    ),
    Template::Loss => format!(
      "衰减后功率 = P × 10^(-L/10)。本题：{} × 10^(-{}/10) ≈ {}。馈线损耗每 3 dB，功率减半。",
      fmt3(p.0),
      fmt3(p.1),
      answer_text
    ),
    Template::Lc => format!(
      "谐振频率 f（MHz）= 1 ÷ (2π × √(L×C))，L 用 μH、C 用 pF。本题 ≈ {}。",
      answer_text
    ),
  }
}

/// 参数池下标：同一模板的各题互不相同。
///
/// 序号 0 的随机流给出起点与步长（步长在 `1..len` 内），第 `index` 题取 `起点 + index × 步长`：
/// 步长不为 0 且小于池长，所以序号 0 与 1 一定不同；[`VARIANTS_PER_TEMPLATE`] 若调到 2 以上，
/// 第 3 题起可能与前面重复，要换选法并扩充单测。取模用 `u64`，wasm32 与本机的结果一致
/// （先 `as usize` 会在 wasm32 上截断高位）。
fn pool_index(seed: u32, template: Template, index: u32, len: usize) -> usize {
  let len = len as u64;
  let mut rng = rng_for(seed, template, 0);
  let first = next(&mut rng) % len;
  let step = 1 + next(&mut rng) % (len - 1);
  ((first + u64::from(index) * step) % len) as usize
}

fn build_variant(seed: u32, template: Template, index: u32) -> QuestionItem {
  let mut rng = rng_for(seed, template, index);
  let p: (f64, f64) = match template {
    Template::Wavelength => (
      POOL_WAVELENGTH[pool_index(seed, template, index, POOL_WAVELENGTH.len())],
      0.0,
    ),
    Template::Gain => POOL_GAIN[pool_index(seed, template, index, POOL_GAIN.len())],
    Template::Loss => POOL_LOSS[pool_index(seed, template, index, POOL_LOSS.len())],
    Template::Lc => POOL_LC[pool_index(seed, template, index, POOL_LC.len())],
  };
  let answer = compute(template, &p);
  let correct = answer_text(&answer);
  let mut texts = distractors(&answer);
  let pos = (next(&mut rng) % 4) as usize;
  texts.insert(pos, correct.clone());

  let options: Vec<QuestionOption> = ["A", "B", "C", "D"]
    .iter()
    .zip(texts)
    .map(|(key, text)| QuestionOption {
      key: key.to_string(),
      text,
    })
    .collect();
  let answer_keys = vec![["A", "B", "C", "D"][pos].to_owned()];

  QuestionItem {
    id: Some(format!("{VARIANT_PREFIX}{}-{index}", template.id())),
    codes: Codes::default(),
    question: stem(template, &p),
    options,
    answer_keys,
    kind: QuestionType::Single,
    pages: None,
    image_url: None,
    explanation: Some(explanation(template, &p, &correct)),
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn generation_is_deterministic_and_varies_by_seed() {
    let a = variant_questions(42);
    let b = variant_questions(42);
    assert_eq!(a, b, "同一种子同一套题（断点续做的前提）");
    let c = variant_questions(43);
    assert_ne!(
      a.iter().map(|q| &q.question).collect::<Vec<_>>(),
      c.iter().map(|q| &q.question).collect::<Vec<_>>(),
      "换一天要换题，否则防背答案无从谈起"
    );
    assert_eq!(a.len(), 8, "4 类 × 2 道");
  }

  #[test]
  fn every_pool_entry_solves_to_the_pinned_answer() {
    // 池里每个参数组的答案都要按公式逐条钉住 —— 改池、改舍入都会在这里红。
    let pinned_wavelength = [
      "85.7 米",
      "42.9 米",
      "21.4 米",
      "14.3 米",
      "10.7 米",
      "6.00 米",
      "2.08 米",
      "0.698 米",
    ];
    for (i, &f) in POOL_WAVELENGTH.iter().enumerate() {
      let a = Answer::Wavelength(300.0 / f);
      assert_eq!(answer_text(&a), pinned_wavelength[i], "波长池第 {i} 组");
    }
    // 10×lg(2) = 3.0102999…，3 位有效数字是 3.01（「3 dB」只是口头约数）。
    let pinned_gain = [
      "3.01 dB", "6.02 dB", "10.0 dB", "-3.01 dB", "10.0 dB", "10.0 dB", "3.01 dB", "-6.02 dB",
    ];
    for (i, &p) in POOL_GAIN.iter().enumerate() {
      assert_eq!(
        answer_text(&compute(Template::Gain, &p)),
        pinned_gain[i],
        "增益池第 {i} 组"
      );
    }
    let pinned_loss = [
      "50.1 W", "12.6 W", "1.00 W", "10.0 W", "10.0 W", "25.1 W", "25.1 W", "20.0 W",
    ];
    for (i, &p) in POOL_LOSS.iter().enumerate() {
      assert_eq!(
        answer_text(&compute(Template::Loss, &p)),
        pinned_loss[i],
        "衰减池第 {i} 组"
      );
    }
    let pinned_lc = [
      "5.03 MHz",
      "15.92 MHz",
      "1.59 MHz",
      "22.51 MHz",
      "11.25 MHz",
      "7.96 MHz",
    ];
    for (i, &p) in POOL_LC.iter().enumerate() {
      assert_eq!(
        answer_text(&compute(Template::Lc, &p)),
        pinned_lc[i],
        "LC 池第 {i} 组"
      );
    }
  }

  #[test]
  fn variants_are_well_formed_and_marked() {
    for q in variant_questions(7) {
      let id = q.id_str().expect("变体必须有 id");
      assert!(id.starts_with(VARIANT_PREFIX), "{id}");
      assert!(q.question.contains("〔变体"), "题干要带变体标签");
      assert_eq!(q.options.len(), 4);
      assert_eq!(q.answer_keys.len(), 1);
      let pos = ["A", "B", "C", "D"]
        .iter()
        .position(|k| k == &q.answer_keys[0])
        .expect("答案键合法");
      // 正确答案 = 按公式重算出的文本（不是随手写的选项）。
      let correct = q.options[pos].text.clone();
      assert!(
        q.explanation
          .as_deref()
          .is_some_and(|e| e.contains(&correct)),
        "解析里要出现答案"
      );
      // 四个选项互不相同。
      let mut texts: Vec<&str> = q.options.iter().map(|o| o.text.as_str()).collect();
      let n = texts.len();
      texts.sort_unstable();
      texts.dedup();
      assert_eq!(texts.len(), n, "选项不能重复");
    }
  }

  #[test]
  fn day_seed_rolls_over_once_per_day() {
    assert_eq!(day_seed(86_399_999, 0), day_seed(0, 0));
    assert_eq!(day_seed(86_400_000, 0), 1);
    assert_eq!(day_seed(2 * 86_400_000 - 1, 0), 1);
  }

  #[test]
  fn day_seed_follows_the_local_calendar_day() {
    // 东八区：UTC 16:00 就是本地 0 点 —— 在这一刻换题，而不是等到本地早上 8 点。
    let h = 3_600_000;
    assert_eq!(day_seed(16 * h - 1, 480), 0);
    assert_eq!(day_seed(16 * h, 480), 1);
    // 西五区：UTC 05:00 才是本地 0 点（取一个真实量级的日期，避开 1970 年之前的负数天）。
    let day = 20_000 * 86_400_000;
    assert_eq!(day_seed(day + 5 * h - 1, -300), 19_999);
    assert_eq!(day_seed(day + 5 * h, -300), 20_000);
  }

  #[test]
  fn the_two_variants_of_a_template_never_share_parameters() {
    // 同一模板的两道题抽到同一参数组 = 用户看到两道一模一样的题：每个种子都要错开。
    for seed in 0..2_000u32 {
      let qs = variant_questions(seed);
      for pair in qs.chunks(VARIANTS_PER_TEMPLATE as usize) {
        assert_ne!(
          pair[0].question, pair[1].question,
          "种子 {seed} 抽到重复参数"
        );
      }
    }
  }

  #[test]
  fn gain_distractors_avoid_the_near_zero_tail() {
    // (100W→50W) 的增益是 -3.01 dB：原本的「+3」干扰项会得到 -0.010 dB 这种三位小数
    // 尾巴，改用「符号反了」的 +3.01 dB 顶替。
    let d = distractors(&Answer::Gain(10.0 * (50.0f64 / 100.0).log10()));
    for expect in ["-6.01 dB", "3.01 dB", "6.99 dB"] {
      assert!(d.contains(&expect.to_owned()), "缺 {expect}：{d:?}");
    }
    // 对称的 (50W→100W) = +3.01 dB：「-3」干扰项同样不该变成 +0.010 dB。
    let d = distractors(&Answer::Gain(10.0 * (100.0f64 / 50.0).log10()));
    for expect in ["-3.01 dB", "6.01 dB", "13.0 dB"] {
      assert!(d.contains(&expect.to_owned()), "缺 {expect}：{d:?}");
    }
  }
}
