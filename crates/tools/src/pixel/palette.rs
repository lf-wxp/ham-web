//! 按「实际会出现的表面」校正像素调色板里的文字色阶，使 WCAG AA（>= 4.5）有余量。
//!
//! 为什么需要它：`palette.css` 把 Tailwind 的默认色阶整体换成了像素调色板，存量页面里大量的
//! `text-green-600` / `dark:text-red-400` / `text-gray-500` 因此直接换了色值。但文字不只落在
//! 纯卡片底上，还会落在徽章的色调底（`bg-indigo-100`、`dark:bg-indigo-900/40`、`bg-emerald-500/10`）
//! 上，那里的对比度比卡片底低一截 —— 只对着卡片底调色阶会在 axe 里翻车。
//!
//! 做法：只动「会被当文字用」的色阶，别的不碰。
//! - 亮色：600 / 700 / 800（以及中性色族的 500）要在纸色、卡片、柔和底、以及各自色调底上 >= 目标；
//! - 暗色：300 / 400 要在深色卡片、弹层、以及各自色调底（含 `900/40`）上 >= 目标。
//!
//! 不达标就向墨色（亮）/ 向白（暗）一格一格混合，直到达标；700+ 由校正后的 600 按原规则重算。
//!
//! 「表面」不止经典方案一套：每个配色方案（[`ham_web_core::color_scheme`]）的亮 / 暗表面都要过，
//! 否则切到「森林」后 `text-green-600` 落在浅绿底上就可能不够。

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use anyhow::{Result, bail};
use ham_web_core::color::Rgb;
use regex::{Captures, Regex};

/// 校正目标：比 AA 的 4.5 留一点余量，抗字号抗锯齿与色调底的小偏差。
const TARGET: f64 = 4.6;
/// WCAG AA 正文下限：审计的及格线。
const AA: f64 = 4.5;
/// 墨色：亮色下文字色阶向它混合。
const INK: Rgb = Rgb(0x1a, 0x1c, 0x2c);
/// 中性色族：500 档也被大量当次要文字用（Tailwind 默认值就能过 4.5）。
const NEUTRALS: [&str; 5] = ["gray", "slate", "zinc", "neutral", "stone"];

/// 一份调色板：色族 → 档位 → 色值。
type Palette = BTreeMap<String, BTreeMap<u32, Rgb>>;
/// 一个文字档在某组表面上的审计项：`(明暗, 档位, 色值, 表面)`。
type Tier<'a> = (&'a str, u32, Rgb, Vec<(String, Rgb)>);

/// 一个方案在某个明暗下的「文字可能落到的纯色表面」。
#[derive(Clone, Debug)]
pub struct SurfaceSet {
  /// 方案名，用于报告里指明是哪一套（经典方案留空，保持报告简洁）。
  pub name: String,
  /// 卡片底：色调底（徽章）是半透明色叠在它上面，所以单独拿出来。
  pub card: Rgb,
  /// 全部纯色表面（含卡片）。
  pub plain: Vec<(String, Rgb)>,
}

/// 亮 / 暗两侧各方案的表面。
#[derive(Clone, Debug, Default)]
pub struct Surfaces {
  pub light: Vec<SurfaceSet>,
  pub dark: Vec<SurfaceSet>,
}

impl SurfaceSet {
  /// 由一组 CSS 变量（`--card` → `#rrggbb`）造出表面集；亮色另外追加纯白（`bg-white`）。
  ///
  /// 亮 / 暗两侧取的表面不同，沿用当年审计里验证过的口径：亮色不含 `accent`（金色底上
  /// 本来就只放墨色字），暗色含 `accent`（深蓝底上会放次要文字）。
  pub fn from_tokens(name: &str, vars: &BTreeMap<String, Rgb>, dark: bool) -> Result<Self> {
    let get = |var: &str| {
      vars
        .get(var)
        .copied()
        .ok_or_else(|| anyhow::anyhow!("{name}：缺少 {var}"))
    };
    let mut plain: Vec<(String, Rgb)> = if dark {
      vec![
        ("card".into(), get("--card")?),
        ("bg".into(), get("--background")?),
        ("popover".into(), get("--popover")?),
        ("secondary".into(), get("--secondary")?),
        ("muted".into(), get("--muted")?),
        ("accent".into(), get("--accent")?),
      ]
    } else {
      vec![
        ("card".into(), get("--card")?),
        ("bg".into(), get("--background")?),
        ("muted".into(), get("--muted")?),
        ("secondary".into(), get("--secondary")?),
        ("popover".into(), get("--popover")?),
        ("white".into(), Rgb::WHITE),
      ]
    };
    let card = plain[0].1;
    if !name.is_empty() {
      for (n, _) in &mut plain {
        *n = format!("{name}:{n}");
      }
    }
    Ok(Self {
      name: name.to_owned(),
      card,
      plain,
    })
  }

  fn tag(&self, s: &str) -> String {
    if self.name.is_empty() {
      s.to_owned()
    } else {
      format!("{}:{s}", self.name)
    }
  }
}

/// 从 `tokens.css` 里取出 `:root`（亮）与 `.dark`（暗）两块里的十六进制变量。
pub fn classic_tokens(tokens_css: &str) -> Result<(BTreeMap<String, Rgb>, BTreeMap<String, Rgb>)> {
  let (Some(root_at), Some(dark_at)) = (tokens_css.find(":root {"), tokens_css.find("\n.dark {"))
  else {
    bail!("tokens.css 里找不到 `:root {{` 或 `.dark {{` 块");
  };
  let dark_end = tokens_css[dark_at..]
    .find("\n}")
    .map_or(tokens_css.len(), |i| dark_at + i);
  let re = Regex::new(r"(--[a-z0-9-]+):\s*(#[0-9a-fA-F]{6});")?;
  let read = |block: &str| -> BTreeMap<String, Rgb> {
    re.captures_iter(block)
      .filter_map(|c| Rgb::parse(&c[2]).map(|rgb| (c[1].to_owned(), rgb)))
      .collect()
  };
  Ok((
    read(&tokens_css[root_at..dark_at]),
    read(&tokens_css[dark_at..dark_end]),
  ))
}

/// 解析 `palette.css` 里的 `--color-<族>-<档>: #rrggbb;`。
fn load(css: &str) -> Result<Palette> {
  let re = Regex::new(r"--color-([a-z]+)-(\d+):\s*(#[0-9a-fA-F]{6});")?;
  let mut pal = Palette::new();
  for c in re.captures_iter(css) {
    let (Some(rgb), Ok(step)) = (Rgb::parse(&c[3]), c[2].parse::<u32>()) else {
      continue;
    };
    pal.entry(c[1].to_owned()).or_default().insert(step, rgb);
  }
  Ok(pal)
}

/// 取 `fg` 在一组表面上的最差对比度与那个表面的名字。
fn worst(fg: Rgb, surfaces: &[(String, Rgb)]) -> (f64, &str) {
  surfaces
    .iter()
    .map(|(n, bg)| (fg.contrast(*bg), n.as_str()))
    .fold(
      (f64::INFINITY, ""),
      |acc, cur| {
        if cur.0 < acc.0 { cur } else { acc }
      },
    )
}

/// 亮色下文字可能落到的表面。
///
/// 口径取自存量代码的真实用法：`bg-X-50` / `bg-X-100` 与 `bg-X-500/10..20` 最常见，
/// `bg-X-200` 只有 3 处且几乎不配同族的深字，所以不纳入 —— 否则会把每个色阶都压得过暗。
/// `tinted = false` 用于中性色的 500：它只当「次要文字」放在纯表面上。
fn light_surfaces(
  sets: &[SurfaceSet],
  fam: &str,
  steps: &BTreeMap<u32, Rgb>,
  tinted: bool,
) -> Vec<(String, Rgb)> {
  let mut out: Vec<(String, Rgb)> = sets.iter().flat_map(|s| s.plain.clone()).collect();
  if !tinted {
    return out;
  }
  let base = steps.get(&500).copied().unwrap_or(Rgb(0x80, 0x80, 0x80));
  for step in [50, 100] {
    if let Some(&c) = steps.get(&step) {
      out.push((format!("{fam}-{step}"), c));
    }
  }
  for set in sets {
    for t in [0.1, 0.2] {
      out.push((
        set.tag(&format!("card+{fam}{}%", pct(t))),
        set.card.mix(base, t),
      ));
    }
  }
  out
}

/// 暗色下文字可能落到的表面：纯表面，以及 `dark:bg-X-900/40`、`dark:bg-X-950/30..40`
/// 这类「深色色阶半透明叠在卡片上」的色调底（`/browse` 的 indigo 徽章就是这么写的）。
fn dark_surfaces(sets: &[SurfaceSet], fam: &str, steps: &BTreeMap<u32, Rgb>) -> Vec<(String, Rgb)> {
  let mut out: Vec<(String, Rgb)> = sets.iter().flat_map(|s| s.plain.clone()).collect();
  let base = steps.get(&500).copied().unwrap_or(Rgb(0x80, 0x80, 0x80));
  for set in sets {
    for t in [0.1, 0.2] {
      out.push((
        set.tag(&format!("card+{fam}{}%", pct(t))),
        set.card.mix(base, t),
      ));
    }
    for (step, a) in [(900, 0.4), (950, 0.4), (950, 0.3)] {
      if let Some(&c) = steps.get(&step) {
        out.push((
          set.tag(&format!("card+{fam}-{step}@{}", pct(a))),
          set.card.mix(c, a),
        ));
      }
    }
  }
  out
}

/// 比例 → 报告里用的整数百分比。
fn pct(t: f64) -> u32 {
  (t * 100.0).round() as u32
}

/// 把 `fg` 向 `toward` 一格（1%）一格混合，直到在所有表面上都 >= 目标。
fn push(fg: Rgb, toward: Rgb, surfaces: &[(String, Rgb)]) -> Rgb {
  let (mut t, mut cur) = (0.0_f64, fg);
  while worst(cur, surfaces).0 < TARGET && t < 0.95 {
    t += 0.01;
    cur = fg.mix(toward, t);
  }
  cur
}

/// 一遍校正的结果：本遍改动了的色阶。
type Changed = BTreeMap<(String, u32), Rgb>;

/// 对整份调色板做**一遍**校正（算法与早年的 Python 版逐字节一致，见模块测试）。
fn tune_once(pal: &Palette, surfaces: &Surfaces) -> Changed {
  let mut changed = Changed::new();
  for (fam, original) in pal {
    let mut steps = original.clone();
    let ls = light_surfaces(&surfaces.light, fam, &steps, true);
    let mut ds = dark_surfaces(&surfaces.dark, fam, &steps);

    // --- 亮色：600（文字档）---
    if let Some(&c600) = steps.get(&600) {
      let new600 = push(c600, INK, &ls);
      if new600 != c600 {
        changed.insert((fam.clone(), 600), new600);
        steps.insert(600, new600);
        // 700+ 按固定规则由 600 重算：600 与墨色按 30 / 55 / 75 / 88% 混合。
        for (step, t) in [(700, 0.30), (800, 0.55), (900, 0.75), (950, 0.88)] {
          let c = new600.mix(INK, t);
          changed.insert((fam.clone(), step), c);
          steps.insert(step, c);
        }
        // 900 / 950 变了，暗色的 `bg-X-900/40` 色调底也跟着变 —— 重算暗色表面。
        ds = dark_surfaces(&surfaces.dark, fam, &steps);
      }
    }
    // 中性色族的 500：被大量用作次要文字。
    if NEUTRALS.contains(&fam.as_str())
      && let Some(&c500) = steps.get(&500)
    {
      let new500 = push(
        c500,
        INK,
        &light_surfaces(&surfaces.light, fam, &steps, false),
      );
      if new500 != c500 {
        changed.insert((fam.clone(), 500), new500);
        steps.insert(500, new500);
      }
    }

    // --- 暗色：400 / 300（文字档）---
    if let Some(&c400) = steps.get(&400) {
      let new400 = push(c400, Rgb::WHITE, &ds);
      if new400 != c400 {
        changed.insert((fam.clone(), 400), new400);
        steps.insert(400, new400);
      }
    }
    if let Some(&c300) = steps.get(&300) {
      let mut new300 = push(c300, Rgb::WHITE, &ds);
      // 300 必须不暗于 400（色阶单调），否则「更浅的档」反而更难读。
      if let Some(&c400) = steps.get(&400)
        && new300.luminance() < c400.luminance()
      {
        new300 = c400.mix(Rgb::WHITE, 0.35);
      }
      if new300 != c300 {
        changed.insert((fam.clone(), 300), new300);
      }
    }
  }
  changed
}

/// 反复校正直到不再有改动（不动点）。
///
/// 为什么不能只跑一遍：中性色族的 500 是在 600 之后才校正的，而 500 又决定了「色调底」
/// （`bg-slate-500/20` 叠在卡片上）的颜色 —— 500 一变，刚算好的 600 就又差了一格。
/// 只跑一遍的话，对一份已校正的文件再跑一次会继续改动，提交入库的色值就不稳定。
fn converge(pal: &Palette, surfaces: &Surfaces) -> Result<Palette> {
  const MAX_PASSES: usize = 16;
  let mut cur = pal.clone();
  for _ in 0..MAX_PASSES {
    let changed = tune_once(&cur, surfaces);
    if changed.is_empty() {
      return Ok(cur);
    }
    for ((fam, step), rgb) in changed {
      cur.entry(fam).or_default().insert(step, rgb);
    }
  }
  bail!("调色板校正 {MAX_PASSES} 遍仍未收敛，请检查表面是否互相矛盾")
}

/// 两份调色板之间变了的色阶（`原始 → 最终`），供写回与报告。
fn diff(before: &Palette, after: &Palette) -> Changed {
  let mut out = Changed::new();
  for (fam, steps) in after {
    for (&step, &rgb) in steps {
      if before.get(fam).and_then(|s| s.get(&step)) != Some(&rgb) {
        out.insert((fam.clone(), step), rgb);
      }
    }
  }
  out
}

/// 审计：每个文字档在对应表面上都要 >= AA。返回不达标的描述。
fn audit(pal: &Palette, surfaces: &Surfaces) -> Vec<String> {
  let mut bad = Vec::new();
  for (fam, steps) in pal {
    let ls = light_surfaces(&surfaces.light, fam, steps, true);
    let ds = dark_surfaces(&surfaces.dark, fam, steps);
    let mut tiers: Vec<Tier> = Vec::new();
    for s in [600, 700, 800] {
      if let Some(&c) = steps.get(&s) {
        tiers.push(("亮", s, c, ls.clone()));
      }
    }
    if NEUTRALS.contains(&fam.as_str())
      && let Some(&c) = steps.get(&500)
    {
      tiers.push((
        "亮",
        500,
        c,
        light_surfaces(&surfaces.light, fam, steps, false),
      ));
    }
    for s in [300, 400] {
      if let Some(&c) = steps.get(&s) {
        tiers.push(("暗", s, c, ds.clone()));
      }
    }
    for (scheme, step, fg, surf) in tiers {
      let (ratio, on) = worst(fg, &surf);
      if ratio < AA {
        bad.push(format!(
          "[{scheme}] {fam}-{step} {} 最差 {ratio:.2} 在 {on}",
          fg.hex()
        ));
      }
    }
  }
  bad
}

/// 把校正后的色值写回 CSS 文本，其余字节原样保留。
fn apply(css: &str, changed: &BTreeMap<(String, u32), Rgb>) -> Result<String> {
  let re = Regex::new(r"(--color-([a-z]+)-(\d+):\s*)#[0-9a-fA-F]{6};")?;
  Ok(
    re.replace_all(css, |c: &Captures| {
      let key = (c[2].to_owned(), c[3].parse::<u32>().unwrap_or(0));
      match changed.get(&key) {
        Some(rgb) => format!("{}{};", &c[1], rgb.hex()),
        None => c[0].to_owned(),
      }
    })
    .into_owned(),
  )
}

/// 读取表面（经典方案来自 `tokens.css`；其余来自配色方案表）。
pub fn load_surfaces(root: &Path) -> Result<Surfaces> {
  let tokens = fs::read_to_string(root.join(super::TOKENS_CSS))?;
  let (light, dark) = classic_tokens(&tokens)?;
  let mut surfaces = Surfaces {
    light: vec![SurfaceSet::from_tokens("", &light, false)?],
    dark: vec![SurfaceSet::from_tokens("", &dark, true)?],
  };
  super::schemes::extend_surfaces(&mut surfaces, root)?;
  Ok(surfaces)
}

/// `pixel-palette`：校正文字色阶；`check` 为真时只审计、不写文件。
pub fn run(root: &Path, check: bool) -> Result<()> {
  let path = root.join(super::PALETTE_CSS);
  let css = fs::read_to_string(&path)?;
  let pal = load(&css)?;
  let surfaces = load_surfaces(root)?;

  if check {
    let bad = audit(&pal, &surfaces);
    if bad.is_empty() {
      println!("调色板：全部达标（{} 个色族）", pal.len());
      return Ok(());
    }
    bail!(
      "调色板有 {} 项文字色阶对比度不达标：\n{}",
      bad.len(),
      bad.join("\n")
    );
  }

  let tuned = converge(&pal, &surfaces)?;
  let changed = diff(&pal, &tuned);
  fs::write(&path, apply(&css, &changed)?)?;
  println!("校正 {} 个色阶", changed.len());
  for ((fam, step), rgb) in &changed {
    let before = pal[fam]
      .get(step)
      .map_or_else(|| "-".to_owned(), |c| c.hex());
    println!("{fam}-{step} {before} -> {}", rgb.hex());
  }
  let bad = audit(&load(&fs::read_to_string(&path)?)?, &surfaces);
  if bad.is_empty() {
    println!("复检：全部达标");
    Ok(())
  } else {
    bail!("复检仍有 {} 项不达标：\n{}", bad.len(), bad.join("\n"));
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn set(name: &str, card: &str, extra: &[(&str, &str)]) -> SurfaceSet {
    let card = Rgb::parse(card).unwrap();
    let mut plain = vec![("card".to_owned(), card)];
    plain.extend(
      extra
        .iter()
        .map(|(n, c)| ((*n).to_owned(), Rgb::parse(c).unwrap())),
    );
    SurfaceSet {
      name: name.to_owned(),
      card,
      plain,
    }
  }

  fn classic() -> Surfaces {
    Surfaces {
      light: vec![set(
        "",
        "#fff8e1",
        &[("bg", "#f2e8cf"), ("secondary", "#e8dbb5")],
      )],
      dark: vec![set(
        "",
        "#262b45",
        &[("bg", "#1a1c2c"), ("accent", "#29366f")],
      )],
    }
  }

  fn one_family(steps: &[(u32, &str)]) -> Palette {
    let mut pal = Palette::new();
    pal.insert(
      "green".to_owned(),
      steps
        .iter()
        .map(|&(s, c)| (s, Rgb::parse(c).unwrap()))
        .collect(),
    );
    pal
  }

  #[test]
  fn push_stops_as_soon_as_every_surface_passes() {
    let surfaces = vec![("card".to_owned(), Rgb::parse("#fff8e1").unwrap())];
    let bright = Rgb::parse("#38b764").unwrap();
    let out = push(bright, INK, &surfaces);
    let ratio = out.contrast(surfaces[0].1);
    // 刚好越过目标：一格（1%）的混合只会让对比度变化零点几，所以不会比目标高出多少。
    assert!((TARGET..TARGET + 0.4).contains(&ratio), "{ratio}");
  }

  #[test]
  fn already_readable_colors_are_left_alone() {
    let surfaces = vec![("card".to_owned(), Rgb::parse("#fff8e1").unwrap())];
    assert_eq!(push(INK, INK, &surfaces), INK);
  }

  #[test]
  fn tuning_fixes_a_failing_palette_and_is_idempotent() {
    let pal = one_family(&[
      (50, "#e8f6ec"),
      (100, "#d6efdf"),
      (300, "#7ece90"),
      (400, "#38b764"),
      (500, "#38b764"),
      (600, "#38b764"),
    ]);
    let surfaces = classic();
    assert!(!audit(&pal, &surfaces).is_empty(), "起点应当不达标");

    let fixed = converge(&pal, &surfaces).unwrap();
    assert_eq!(audit(&fixed, &surfaces), Vec::<String>::new());
    assert!(
      tune_once(&fixed, &surfaces).is_empty(),
      "已收敛的结果再校正不应有改动"
    );
    assert_eq!(converge(&fixed, &surfaces).unwrap(), fixed);
  }

  #[test]
  fn neutral_500_changing_after_600_still_converges() {
    // 中性色族：500 在 600 之后才被校正、并改变色调底 —— 只跑一遍会留下不稳定的残差。
    let mut pal = Palette::new();
    pal.insert(
      "slate".to_owned(),
      [
        (50, "#e9eaed"),
        (100, "#dcdfe5"),
        (300, "#919da6"),
        (400, "#8796a8"),
        (500, "#7c8196"),
        (600, "#7c8196"),
      ]
      .iter()
      .map(|&(s, c)| (s, Rgb::parse(c).unwrap()))
      .collect(),
    );
    let surfaces = classic();
    let fixed = converge(&pal, &surfaces).unwrap();
    assert!(tune_once(&fixed, &surfaces).is_empty());
    assert!(audit(&fixed, &surfaces).is_empty());
  }

  #[test]
  fn darker_600_rederives_the_700_to_950_ramp() {
    let pal = one_family(&[(500, "#38b764"), (600, "#38b764")]);
    let changed = tune_once(&pal, &classic());
    let c600 = changed[&("green".to_owned(), 600)];
    for (step, t) in [(700, 0.30), (800, 0.55), (900, 0.75), (950, 0.88)] {
      assert_eq!(changed[&("green".to_owned(), step)], c600.mix(INK, t));
    }
  }

  #[test]
  fn tone_300_is_never_darker_than_400() {
    let pal = one_family(&[(300, "#404040"), (400, "#a0a0a0"), (500, "#808080")]);
    let fixed = converge(&pal, &classic()).unwrap();
    let steps = &fixed["green"];
    assert!(steps[&300].luminance() >= steps[&400].luminance());
  }

  #[test]
  fn apply_rewrites_only_the_changed_variables() {
    let css = "@theme {\n  --color-green-600: #38b764;\n  --color-green-500: #38b764;\n  --other: #123456;\n}\n";
    let mut changed = BTreeMap::new();
    changed.insert(("green".to_owned(), 600), Rgb(1, 2, 3));
    assert_eq!(
      apply(css, &changed).unwrap(),
      "@theme {\n  --color-green-600: #010203;\n  --color-green-500: #38b764;\n  --other: #123456;\n}\n"
    );
  }

  #[test]
  fn a_second_scheme_with_harder_surfaces_darkens_the_text_tiers() {
    let pal = one_family(&[
      (500, "#38b764"),
      (600, "#2d7f50"),
      (400, "#7ece90"),
      (300, "#a7f070"),
    ]);
    let easy = classic();
    let mut hard = classic();
    hard.light.push(set("mid", "#c8d0a8", &[]));
    let (a, b) = (
      converge(&pal, &easy).unwrap(),
      converge(&pal, &hard).unwrap(),
    );
    assert!(b["green"][&600].luminance() < a["green"][&600].luminance());
  }

  #[test]
  fn classic_tokens_are_read_from_the_two_blocks() {
    let css = "@theme inline { --color-card: var(--card); }\n:root {\n  --card: #fff8e1;\n  --border: rgb(1 2 3 / .2);\n}\n\n.dark {\n  --card: #262b45;\n}\n.other { --card: #000000; }\n";
    let (light, dark) = classic_tokens(css).unwrap();
    assert_eq!(light["--card"], Rgb::parse("#fff8e1").unwrap());
    assert_eq!(dark["--card"], Rgb::parse("#262b45").unwrap());
    assert!(!light.contains_key("--border"), "非十六进制的值不读");
  }
}
