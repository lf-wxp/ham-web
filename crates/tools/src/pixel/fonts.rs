//! 像素字体子集化：把整套字体切成「按需加载」的 woff2 分片。
//!
//! 为什么需要它：缝合像素字体（Fusion Pixel）单个 TTF 有 7MB（约 2.8 万字形），不可能整包下发。
//! 本命令做三件事：
//!
//! 1. 统计仓库里**实际出现**的汉字（`data/`、`public/`、`crates/` 下的 json / rs / html / md），
//!    按频次排序；
//! 2. 把「全角标点 + 高频字」「中频字」「GB2312 一级字里语料没出现的字」依次切成若干分片，
//!    每片一个 woff2，并生成带 `unicode-range` 的 `@font-face`。浏览器只会下载页面里真正
//!    用到字符所在的分片，首屏通常只拉 1–2 片；
//! 3. 拉丁 / HUD 字体（Press Start 2P、Silkscreen）与缝合像素的拉丁部分各出一个小文件。
//!
//! 产物（均提交入库，日常开发 / CI **不需要**运行本命令）：
//! - `public/fonts/pixel-*.woff2`
//! - `crates/app/style/pixel/fonts-pixel.css`（生成物，勿手改）
//!
//! 什么时候要重跑：新增了大量中文文案、换了字体版本。语料之外的生僻字不会丢 ——
//! 字体栈末尾保留了系统中文字体兜底，只是那几个字会用系统字体显示。
//!
//! 全程纯 Rust（fontcull-klippa 子集化 + ttf2woff2 编码），不需要 Python / fonttools / C++ 工具链。
//! `--src` 目录里需要放：
//! `fusion-pixel-12px-proportional-zh_hans.ttf`、`fusion-pixel-12px-proportional-latin.ttf`、
//! `fusion-pixel-12px-monospaced-latin.ttf`、`PressStart2P-Regular.ttf`、`Silkscreen-Regular.ttf`。
//! 下载地址与许可证见 `public/fonts/LICENSES`。

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result, anyhow, ensure};
use fontcull_klippa::{Plan, SubsetFlags, subset_font};
use fontcull_skrifa::raw::collections::IntSet;
use fontcull_skrifa::raw::types::NameId;
use fontcull_skrifa::{FontRef, GlyphId, MetadataProvider, Tag};
use ttf2woff2::BrotliQuality;
use walkdir::WalkDir;

/// 语料扫描范围：与前端实际渲染的文本来源一致。
const CORPUS_ROOTS: &[&str] = &[
  "data",
  "public/data",
  "public/questions",
  "crates",
  "README.md",
];
const CORPUS_EXT: &[&str] = &["json", "rs", "html", "md", "txt"];
/// 体积巨大且与界面文字无关的目录。
const SKIP_DIRS: &[&str] = &["target", "node_modules", "dist", ".git"];

/// 分片大小：太碎请求数多，太大则首屏白下载。500 字约 60–90KB（woff2）。
/// 第 0 片：全角标点 + 最高频字。
const CORE_COUNT: usize = 600;
const SHARD_COUNT: usize = 500;
/// GB2312 一级字里语料没出现的字。
const EXTRA_SHARD: usize = 640;

/// 拉丁部分：基本拉丁 + 拉丁补充 + 拉丁扩展 A（西班牙语 ñ á é í ó ú ¿ ¡ 等）、
/// 希腊字母（μ Ω 等电子学单位）、常用标点 / 箭头 / 数学符号 / 制表符 / 几何形状。
const LATIN_RANGES: &[(u32, u32)] = &[
  (0x0020, 0x007E),
  (0x00A0, 0x017F),
  (0x0370, 0x03FF),
  (0x2000, 0x206F),
  (0x20A0, 0x20BF),
  (0x2100, 0x214F),
  (0x2190, 0x21FF),
  (0x2200, 0x22FF),
  (0x2500, 0x259F),
  (0x25A0, 0x25FF),
  (0x2600, 0x26FF),
];

/// 保留 OFL 要求的版权与许可证信息（name 表 0 / 13 / 14），其余常规条目一并带上。
const NAME_IDS: [u16; 9] = [0, 1, 2, 3, 4, 5, 6, 13, 14];
/// name 表只留英文（美国）一种语言，与 fonttools 的默认一致。
const NAME_LANG_EN_US: u16 = 0x409;
/// 排版特性：只保留这几个，其余（`calt` / `dlig` / `vert` ……）丢掉省体积。
const LAYOUT_FEATURES: [&[u8; 4]; 6] = [b"kern", b"liga", b"ccmp", b"locl", b"mark", b"mkmk"];
/// 要丢掉的表：
/// - `DSIG`：数字签名，子集化后必然失效；
/// - `vhea` / `vmtx`：竖排度量。站点没有竖排文字，且 klippa 子集化时不会重算
///   `vhea.numberOfVMetrics`，留着会得到一个 `numberOfVMetrics > numGlyphs` 的无效表，
///   浏览器的字体消毒器（OTS）可能因此整字体拒收。
const DROP_TABLES: [&[u8; 4]; 3] = [b"DSIG", b"vhea", b"vmtx"];

/// 中文全角标点与符号：必须放进第 0 片（出现频率极高，一片都不能缺）。
fn cjk_punct() -> Vec<u32> {
  (0x3000..0x303F)
    .chain(0xFF00..0xFFEF)
    .chain([
      0x00B7, 0x2014, 0x2018, 0x2019, 0x201C, 0x201D, 0x2026, 0x2022,
    ])
    .collect()
}

/// 扫描语料里每个汉字出现的次数。
fn collect_corpus(root: &Path) -> BTreeMap<char, u64> {
  let mut counter: BTreeMap<char, u64> = BTreeMap::new();
  for entry in CORPUS_ROOTS {
    let base = root.join(entry);
    let walker = WalkDir::new(&base).into_iter().filter_entry(|e| {
      // 只看相对仓库根的路径分量：仓库本身放在叫 `dist` 之类的目录下也不能误伤。
      let rel = e.path().strip_prefix(root).unwrap_or(e.path());
      !rel
        .components()
        .any(|c| SKIP_DIRS.contains(&c.as_os_str().to_string_lossy().as_ref()))
    });
    for e in walker.filter_map(Result::ok) {
      let path = e.path();
      let ext_ok = path
        .extension()
        .and_then(|x| x.to_str())
        .is_some_and(|x| CORPUS_EXT.contains(&x));
      if !e.file_type().is_file() || !ext_ok {
        continue;
      }
      // 读不出来（非 UTF-8、权限）就跳过：语料只是用来排序的。
      let Ok(text) = fs::read_to_string(path) else {
        continue;
      };
      for ch in text
        .chars()
        .filter(|c| ('\u{4E00}'..='\u{9FFF}').contains(c))
      {
        *counter.entry(ch).or_default() += 1;
      }
    }
  }
  counter
}

/// 按频次从高到低；同频次按码点升序。
///
/// 同频次必须有确定的顺序：目录遍历的先后因文件系统而异，不加这条的话，
/// 同一份仓库在两台机器上会切出不同的分片。
fn rank_by_frequency(counter: &BTreeMap<char, u64>) -> Vec<char> {
  let mut ranked: Vec<(char, u64)> = counter.iter().map(|(&c, &n)| (c, n)).collect();
  ranked.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
  ranked.into_iter().map(|(c, _)| c).collect()
}

/// GB2312 一级汉字（3755 个，覆盖现代汉语 99.7% 的用字）。
///
/// 区位码 16–55 区（高字节 0xB0–0xD7）；55 区只有前 89 个是 GB2312 字符，
/// 0xD7FA–0xD7FE 是 GBK 才有的扩展，必须排除。
fn gb2312_level1() -> Vec<char> {
  let mut out = Vec::with_capacity(3755);
  for hi in 0xB0u8..=0xD7 {
    for lo in 0xA1u8..=0xFE {
      if hi == 0xD7 && lo > 0xF9 {
        continue;
      }
      let bytes = [hi, lo];
      let (text, _, bad) = encoding_rs::GBK.decode(&bytes);
      let mut it = text.chars();
      if let (false, Some(c), None) = (bad, it.next(), it.next()) {
        out.push(c);
      }
    }
  }
  out
}

/// 码点列表压成 `U+4E00-4E05, U+4E10` 形式的 unicode-range。
fn to_ranges(codepoints: &[u32]) -> String {
  let cps: BTreeSet<u32> = codepoints.iter().copied().collect();
  let cps: Vec<u32> = cps.into_iter().collect();
  let mut parts = Vec::new();
  let mut i = 0;
  while i < cps.len() {
    let mut j = i;
    while j + 1 < cps.len() && cps[j + 1] == cps[j] + 1 {
      j += 1;
    }
    parts.push(if i == j {
      format!("U+{:X}", cps[i])
    } else {
      format!("U+{:X}-{:X}", cps[i], cps[j])
    });
    i = j + 1;
  }
  parts.join(", ")
}

/// 一条 `@font-face`。
fn face(family: &str, url: &str, ranges: Option<&str>) -> String {
  let rng = ranges.map_or_else(String::new, |r| format!(" unicode-range: {r};"));
  format!(
    "@font-face {{ font-family: \"{family}\"; font-style: normal; font-weight: 400; font-display: swap; src: url(\"/fonts/{url}\") format(\"woff2\");{rng} }}"
  )
}

/// 字体里有字形的码点。
fn cmap_of(font: &FontRef) -> BTreeSet<u32> {
  font.charmap().mappings().map(|(cp, _)| cp).collect()
}

/// 某字体在拉丁范围内实际有字形的码点。
fn latin_cps(have: &BTreeSet<u32>) -> Vec<u32> {
  LATIN_RANGES
    .iter()
    .flat_map(|&(lo, hi)| lo..=hi)
    .filter(|c| have.contains(c))
    .collect()
}

/// 切分片：第 0 片 = 全角标点 + 最高频字；其后每 [`SHARD_COUNT`] 个字一片；
/// 最后是 GB2312 一级字里语料没出现的字，每 [`EXTRA_SHARD`] 个一片。
fn plan_shards(punct: &[u32], ranked: &[u32], extra: &[u32]) -> Vec<Vec<u32>> {
  let core_end = CORE_COUNT.min(ranked.len());
  let mut shards = vec![[punct, &ranked[..core_end]].concat()];
  shards.extend(ranked[core_end..].chunks(SHARD_COUNT).map(<[u32]>::to_vec));
  shards.extend(extra.chunks(EXTRA_SHARD).map(<[u32]>::to_vec));
  shards
}

/// 把 TTF 子集化到指定码点，并编码成 woff2。
fn subset_to_woff2(ttf: &[u8], codepoints: &[u32]) -> Result<Vec<u8>> {
  let font = FontRef::new(ttf).map_err(|e| anyhow!("解析字体失败：{e:?}"))?;
  let unicodes: IntSet<u32> = codepoints.iter().copied().collect();
  let gids: IntSet<GlyphId> = IntSet::empty();
  let drop: IntSet<Tag> = DROP_TABLES.iter().map(|t| Tag::new(t)).collect();
  let scripts: IntSet<Tag> = IntSet::all();
  let features: IntSet<Tag> = LAYOUT_FEATURES.iter().map(|t| Tag::new(t)).collect();
  let names: IntSet<NameId> = NAME_IDS.iter().map(|&n| NameId::new(n)).collect();
  let langs: IntSet<u16> = [NAME_LANG_EN_US].into_iter().collect();
  // 丢提示指令（体积大头，点阵字体用不上）；保留 .notdef 的轮廓。
  let flags = SubsetFlags::SUBSET_FLAGS_NO_HINTING | SubsetFlags::SUBSET_FLAGS_NOTDEF_OUTLINE;
  let plan = Plan::new(
    &gids, &unicodes, &font, flags, &drop, &scripts, &features, &names, &langs,
  );
  let subset = subset_font(&font, &plan).map_err(|e| anyhow!("子集化失败：{e:?}"))?;
  ttf2woff2::encode(&subset, BrotliQuality::default()).map_err(|e| anyhow!("WOFF2 编码失败：{e}"))
}

/// 子集化并写出一个 woff2，返回体积。
fn write_subset(ttf: &[u8], codepoints: &[u32], dest: &Path) -> Result<usize> {
  let woff2 = subset_to_woff2(ttf, codepoints)?;
  fs::write(dest, &woff2).with_context(|| dest.display().to_string())?;
  Ok(woff2.len())
}

/// `pixel-fonts`：生成全部分片与 `fonts-pixel.css`。
pub fn generate(root: &Path, src: &Path) -> Result<()> {
  let out_fonts = root.join("public/fonts");
  let out_css = root.join("crates/app/style/pixel/fonts-pixel.css");
  fs::create_dir_all(&out_fonts)?;

  let read = |name: &str| -> Result<Vec<u8>> {
    let p = src.join(name);
    ensure!(p.exists(), "缺少字体文件：{}", p.display());
    fs::read(&p).with_context(|| p.display().to_string())
  };
  let hans = read("fusion-pixel-12px-proportional-zh_hans.ttf")?;
  let latin_prop = read("fusion-pixel-12px-proportional-latin.ttf")?;
  let latin_mono = read("fusion-pixel-12px-monospaced-latin.ttf")?;
  let ps2p = read("PressStart2P-Regular.ttf")?;
  let silk = read("Silkscreen-Regular.ttf")?;

  let mut css = vec![
    "/* 由 `cargo make fonts-pixel`（crates/tools/src/pixel/fonts.rs）生成，请勿手改。".to_owned(),
    "   像素字体分片：每片带 unicode-range，浏览器只下载页面用到字符所在的分片。".to_owned(),
    "   许可证与来源见 public/fonts/LICENSES。 */".to_owned(),
    String::new(),
  ];
  let mut total = 0usize;
  let mut emit = |data: &[u8], cps: &[u32], name: &str, family: &str| -> Result<()> {
    let size = write_subset(data, cps, &out_fonts.join(format!("{name}.woff2")))?;
    total += size;
    css.push(face(
      family,
      &format!("{name}.woff2"),
      Some(&to_ranges(cps)),
    ));
    println!(
      "{name}.woff2  {:6.1} KB  ({} 字形)",
      size as f64 / 1024.0,
      cps.len()
    );
    Ok(())
  };

  let parse = |data: &[u8]| -> Result<BTreeSet<u32>> {
    let font = FontRef::new(data).map_err(|e| anyhow!("解析字体失败：{e:?}"))?;
    Ok(cmap_of(&font))
  };

  // ---- HUD / 标题字体（只含拉丁）----
  for (data, name, family) in [
    (&ps2p, "pixel-press-start-2p", "Press Start 2P"),
    (&silk, "pixel-silkscreen", "Silkscreen"),
  ] {
    emit(data, &latin_cps(&parse(data)?), name, family)?;
  }
  // ---- 缝合像素：拉丁（比例 / 等宽）----
  for (data, name, family) in [
    (&latin_prop, "pixel-fusion-latin", "Fusion Pixel"),
    (&latin_mono, "pixel-fusion-mono-latin", "Fusion Pixel Mono"),
  ] {
    emit(data, &latin_cps(&parse(data)?), name, family)?;
  }

  // ---- 缝合像素：中文分片 ----
  let have = parse(&hans)?;
  let corpus = collect_corpus(root);
  // 语料一个汉字都没扫到 = `CORPUS_ROOTS` 写错或目录被删。这时分片会退化成「只按 GB2312
  // 顺序排」，产物照样能生成，但高频字挤不进第 0 片、首屏要等后面几片才显示完整。
  ensure!(
    !corpus.is_empty(),
    "语料里一个汉字都没扫到：检查 CORPUS_ROOTS 指向的目录是否还在"
  );
  let ranked_chars: Vec<char> = rank_by_frequency(&corpus)
    .into_iter()
    .filter(|&c| have.contains(&u32::from(c)))
    .collect();
  let ranked: Vec<u32> = ranked_chars.iter().map(|&c| u32::from(c)).collect();
  // 拉丁文件已经覆盖的码点（如 · — “ ”）不必在中文片里重复，否则 unicode-range 重叠时
  // 浏览器会按声明顺序择一加载，徒增体积。
  let latin_have: BTreeSet<u32> = LATIN_RANGES.iter().flat_map(|&(lo, hi)| lo..=hi).collect();
  let punct: Vec<u32> = cjk_punct()
    .into_iter()
    .filter(|c| have.contains(c) && !latin_have.contains(c))
    .collect();
  let seen: BTreeSet<u32> = ranked.iter().copied().collect();
  let extra: Vec<u32> = gb2312_level1()
    .into_iter()
    .map(u32::from)
    .filter(|c| !seen.contains(c) && have.contains(c))
    .collect();

  let shards = plan_shards(&punct, &ranked, &extra);
  for (i, cps) in shards.iter().enumerate() {
    emit(&hans, cps, &format!("pixel-fusion-cjk-{i}"), "Fusion Pixel")?;
  }

  let mut text = css.join("\n");
  text.push('\n');
  fs::write(&out_css, text)?;
  let mut summary = String::new();
  let _ = write!(
    summary,
    "\n合计 {:.0} KB，{} 个中文分片；样式写入 {}\n语料汉字 {} 个，其中字体内含 {} 个",
    total as f64 / 1024.0,
    shards.len(),
    out_css.strip_prefix(root).unwrap_or(&out_css).display(),
    corpus.len(),
    ranked.len()
  );
  println!("{summary}");
  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn ranges_merge_consecutive_codepoints() {
    assert_eq!(
      to_ranges(&[0x4E10, 0x4E00, 0x4E01, 0x4E02, 0x4E00]),
      "U+4E00-4E02, U+4E10"
    );
    assert_eq!(to_ranges(&[0x41]), "U+41");
    assert_eq!(to_ranges(&[]), "");
  }

  #[test]
  fn font_face_declares_swap_and_range() {
    let f = face("Fusion Pixel", "a.woff2", Some("U+41"));
    assert!(f.contains("font-display: swap"));
    assert!(f.contains("src: url(\"/fonts/a.woff2\") format(\"woff2\");"));
    assert!(f.ends_with("unicode-range: U+41; }"));
    assert!(!face("X", "b.woff2", None).contains("unicode-range"));
  }

  #[test]
  fn gb2312_level1_has_exactly_3755_characters() {
    let all = gb2312_level1();
    assert_eq!(all.len(), 3755);
    assert_eq!((all[0], all[all.len() - 1]), ('啊', '座'));
    let unique: BTreeSet<char> = all.iter().copied().collect();
    assert_eq!(unique.len(), 3755, "不应有重复");
  }

  #[test]
  fn ties_in_frequency_break_by_codepoint() {
    let counter: BTreeMap<char, u64> = [('乙', 3), ('甲', 3), ('丙', 9), ('丁', 1)]
      .into_iter()
      .collect();
    // 丙 最高；甲(U+7532) < 乙(U+4E59)? 按码点升序：乙 在前。
    assert_eq!(rank_by_frequency(&counter), vec!['丙', '乙', '甲', '丁']);
  }

  #[test]
  fn shards_cover_everything_exactly_once() {
    let punct: Vec<u32> = (1..=10).collect();
    let ranked: Vec<u32> = (1000..1000 + 1700).collect();
    let extra: Vec<u32> = (9000..9000 + 1500).collect();
    let shards = plan_shards(&punct, &ranked, &extra);
    // 第 0 片：标点 + 前 600；其后 (1700-600)/500 → 3 片；extra 1500/640 → 3 片。
    assert_eq!(shards.len(), 1 + 3 + 3);
    assert_eq!(shards[0].len(), 10 + CORE_COUNT);
    assert_eq!(shards[1].len(), SHARD_COUNT);
    let flat: Vec<u32> = shards.concat();
    let uniq: BTreeSet<u32> = flat.iter().copied().collect();
    assert_eq!(flat.len(), uniq.len(), "分片之间不能重叠");
    assert_eq!(flat.len(), punct.len() + ranked.len() + extra.len());
  }

  #[test]
  fn few_characters_still_yield_a_core_shard() {
    let shards = plan_shards(&[1, 2], &[100, 101], &[]);
    assert_eq!(shards, vec![vec![1, 2, 100, 101]]);
  }

  #[test]
  fn corpus_counts_han_characters_and_skips_build_dirs() {
    let dir = std::env::temp_dir().join(format!("ham-fonts-corpus-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("data/target")).unwrap();
    fs::create_dir_all(dir.join("crates/a")).unwrap();
    fs::write(dir.join("data/x.json"), "天天 abc 地").unwrap();
    fs::write(dir.join("data/target/y.json"), "玄").unwrap();
    fs::write(dir.join("crates/a/z.rs"), "// 天").unwrap();
    fs::write(dir.join("crates/a/z.bin"), "黄").unwrap();
    fs::write(dir.join("README.md"), "宇").unwrap();
    let c = collect_corpus(&dir);
    let _ = fs::remove_dir_all(&dir);
    assert_eq!(c.get(&'天'), Some(&3));
    assert_eq!(c.get(&'地'), Some(&1));
    assert_eq!(c.get(&'宇'), Some(&1));
    assert!(!c.contains_key(&'玄'), "target 目录要跳过");
    assert!(!c.contains_key(&'黄'), "未知扩展名要跳过");
  }

  #[test]
  fn cjk_punctuation_includes_the_fullwidth_block() {
    let p = cjk_punct();
    for cp in [0x3002, 0xFF0C, 0x2014, 0x2026] {
      assert!(p.contains(&cp), "{cp:X}");
    }
  }
}
