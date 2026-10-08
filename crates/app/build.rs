//! 构建期把 `data/i18n/{lang}/{domain}.json` 编译成静态词典表。
//!
//! 界面文案的唯一事实源是仓库里的 JSON（按域拆分）：作者改的是 JSON，不必碰 Rust
//! 字符串字面量的转义，也不必为了对齐 rustfmt 的换行布局而折腾格式。生成物只写进
//! `OUT_DIR`、不入库，运行时仍是同步查表。
//!
//! 只有 [`BUNDLED_DOMAINS`] 里的域会编进 wasm：其余域的译文由运行时语言包提供
//! （`crates/app/src/i18n/pack.rs`），中文因是默认语言而全量内嵌 —— 中文用户不该为
//! 「界面已经是中文」付一次网络请求。
//!
//! 值为对象的词条是**复数词条**（`{ "one": …, "other": … }`），单独编成
//! `NAME_PLURAL` 表；中文不允许复数对象。详见 `docs/i18n-refactor.md` 的 P3-C。

use std::path::{Path, PathBuf};

/// 资源目录名 → 生成的静态表名。
///
/// `zh` 也在其中：语义化 key 之后源码里不再有中文原文，中文变成一种「译文」，
/// 与其他语言同等对待（新增语言只需加一个 `data/i18n/<lang>/` 目录）。
const LANGS: [(&str, &str); 3] = [("zh", "ZH"), ("en", "EN"), ("es", "ES")];

/// 编进 wasm 的域：首屏渲染与离线兜底所需的最小集合。
///
/// 其余域的译文走运行时语言包（`i18n/pack.rs`），切换语言时才拉取。清单只此一处，
/// 且只对非中文生效（中文全量内嵌）。
const BUNDLED_DOMAINS: [&str; 2] = ["common", "shell"];

fn main() {
  let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
  let assets = root.join("data").join("i18n");
  let out_dir = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR not set"));

  let mut buf = String::from("// 由 crates/app/build.rs 从 data/i18n/ 生成，请勿手改。\n");
  for (lang, name) in LANGS {
    // 复数变体只对非中文语言生成：中文没有复数变化，保持 zh 全量扁平，
    // 前端在中文下就不必查复数表。
    let plural_allowed = lang != "zh";
    let (flat, plural) = load(
      &assets.join(lang),
      (lang != "zh").then_some(&BUNDLED_DOMAINS),
      plural_allowed,
    );
    buf.push_str(&render(name, &flat));
    if plural_allowed {
      buf.push_str(&render_plural(name, &plural));
    }
  }
  buf.push_str(&render_bundled());
  let out = out_dir.join("i18n_catalog.rs");
  std::fs::write(&out, buf).unwrap_or_else(|e| panic!("write {}: {e}", out.display()));
}

/// 扁平词条：`key → 文本`。
type FlatEntries = Vec<(String, String)>;
/// 复数词条：`key → [(类别, 文本)]`。
type PluralEntries = Vec<(String, Vec<(String, String)>)>;

/// 读入一种语言的域文件，按「文件名 → 词条」的确定顺序展开。
///
/// 值为字符串的是普通词条；值为对象（`{ "one": …, "other": … }`）的是复数词条，
/// 单独成表 —— 复数词条只占全部词条的百分之几，拆开后扁平表的结构与体积都不受影响。
///
/// `only` 为 `Some` 时只收这些域（非中文语言只内嵌内嵌域）；`rerun-if-changed` 仍然
/// 对每个文件都声明 —— 新增或删除域文件也要能触发重建。
///
/// 用 `BTreeMap` 解析，因此同一文件内的词条按 key 排序，输出稳定可复现：
/// 否则每次构建生成的文件字节都可能不同，白白触发下游重编译。
fn load(dir: &Path, only: Option<&[&str]>, plural_allowed: bool) -> (FlatEntries, PluralEntries) {
  // 目录本身也要声明：Cargo 只盯「声明过的路径」，逐文件声明感知不到**新增**的域文件
  // （或新增的语言目录）—— 那样生成的静态表里没有新 key，界面直接漏出 key，而
  // `check-i18n` 读的是 JSON、全绿误导。Cargo 对目录会整目录扫描。
  println!("cargo:rerun-if-changed={}", dir.display());
  let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
    .unwrap_or_else(|e| panic!("read {}: {e}", dir.display()))
    .map(|e| e.expect("dir entry").path())
    .filter(|p| p.extension().is_some_and(|x| x == "json"))
    .collect();
  files.sort();

  let mut flat = Vec::new();
  let mut plural = Vec::new();
  // key → 首次出现的文件，用于跨域去重。
  //
  // 同一语言的两个域文件里出现同一个 key 时，生成表会有两条同名条目，运行时
  // `HashMap` 收集时**静默取后一条**，前一条永远不可达；而 `trunk build`（以及 Docker
  // 镜像构建）都不跑测试，`catalog.rs` 的重复 key 断言根本不会执行。这里 fail-fast，
  // 让部署构建也当场红，并指出冲突的两个文件。
  let mut origin: std::collections::HashMap<String, String> = std::collections::HashMap::new();
  for f in files {
    println!("cargo:rerun-if-changed={}", f.display());
    let domain = f.file_stem().and_then(|s| s.to_str()).unwrap_or_default();
    if only.is_some_and(|d| !d.contains(&domain)) {
      continue;
    }
    let raw = std::fs::read_to_string(&f).unwrap_or_else(|e| panic!("read {}: {e}", f.display()));
    let map: std::collections::BTreeMap<String, serde_json::Value> =
      serde_json::from_str(&raw).unwrap_or_else(|e| panic!("parse {}: {e}", f.display()));
    for (k, v) in map {
      if let Some(first) = origin.insert(k.clone(), f.display().to_string()) {
        panic!(
          "{k} 在 {first} 与 {} 里重复定义；同一语言的 key 必须唯一（否则其中一条不可达）",
          f.display()
        );
      }
      match v {
        serde_json::Value::String(s) => flat.push((k, s)),
        serde_json::Value::Object(o) => {
          if !plural_allowed {
            panic!(
              "中文词典不使用复数变体（{} 的 {k}）：请写成字符串",
              f.display()
            );
          }
          let mut variants: Vec<(String, String)> = o
            .into_iter()
            .map(|(c, text)| {
              let text = match text {
                serde_json::Value::String(s) => s,
                other => panic!("{k} 的复数变体 {c} 不是字符串：{other}"),
              };
              (c, text)
            })
            .collect();
          variants.sort_by(|a, b| a.0.cmp(&b.0));
          if !variants.iter().any(|(c, _)| c == "other") {
            panic!("{k} 的复数变体缺少 other（兜底类别，必填）");
          }
          plural.push((k, variants));
        }
        other => panic!("{k} 的值必须是字符串或复数对象，实际是 {other}"),
      }
    }
  }
  (flat, plural)
}

/// 渲染内嵌域清单，供 `catalog.rs` 的测试确认「内嵌域在每种语言里都齐全」。
fn render_bundled() -> String {
  // 目前只有 `catalog.rs` 的测试用它确认「内嵌域在每种语言里都齐全」，`cfg(test)` 免得
  // 发布构建里多一个没人引用的静态量（clippy 按 -D warnings 跑，dead_code 会红）。
  let mut s = String::from(
    "/// 编进 wasm 的域；其余域的译文由运行时语言包提供（见 `i18n/pack.rs`）。\n#[cfg(test)]\npub static BUNDLED_DOMAINS: &[&str] = &[\n",
  );
  for d in BUNDLED_DOMAINS {
    s.push_str("  \"");
    s.push_str(d);
    s.push_str("\",\n");
  }
  s.push_str("];\n");
  s
}

/// 渲染一个 `pub static NAME: &[(&str, &str)] = &[ … ];`。
fn render(name: &str, entries: &[(String, String)]) -> String {
  let mut s = format!("pub static {name}: &[(&str, &str)] = &[\n");
  for (k, v) in entries {
    s.push_str("  (\"");
    s.push_str(&escape_rust(k));
    s.push_str("\", \"");
    s.push_str(&escape_rust(v));
    s.push_str("\"),\n");
  }
  s.push_str("];\n");
  s
}

/// 渲染复数表 `pub static NAME_PLURAL: &[(&str, &[(&str, &str)])] = &[ … ];`。
///
/// 表里只有复数词条（预计一百余条），扁平表因此不必为它们付出嵌套切片的体积代价。
fn render_plural(name: &str, entries: &PluralEntries) -> String {
  let mut s = format!("pub static {name}_PLURAL: &[(&str, &[(&str, &str)])] = &[\n");
  for (k, variants) in entries {
    s.push_str("  (\"");
    s.push_str(&escape_rust(k));
    s.push_str("\", &[");
    for (i, (category, text)) in variants.iter().enumerate() {
      if i > 0 {
        s.push_str(", ");
      }
      s.push_str("(\"");
      s.push_str(&escape_rust(category));
      s.push_str("\", \"");
      s.push_str(&escape_rust(text));
      s.push_str("\")");
    }
    s.push_str("]),\n");
  }
  s.push_str("];\n");
  s
}

/// 转成 Rust 字符串字面量的内容。
///
/// 译文里若含裸换行或引号，直接写进源码会截断字面量、把生成文件写坏。除这几个常见
/// 转义外，**其余控制字符**（如 JSON `\u0000` 解出来的裸 NUL、`\u{7f}`）也必须转义 ——
/// 原样写进源码会产出带控制字符的 `include!` 文件，编辑器与 diff 都会出问题。
fn escape_rust(s: &str) -> String {
  let mut out = String::with_capacity(s.len());
  for c in s.chars() {
    match c {
      '\\' => out.push_str("\\\\"),
      '"' => out.push_str("\\\""),
      '\n' => out.push_str("\\n"),
      '\r' => out.push_str("\\r"),
      '\t' => out.push_str("\\t"),
      c if c.is_control() || is_invisible_format(c) => {
        out.push_str(&format!("\\u{{{:x}}}", c as u32))
      }
      _ => out.push(c),
    }
  }
  out
}

/// 需要显式转义的「隐形」格式字符（Unicode 类别 Cf）。
///
/// `char::is_control()` 只覆盖 Cc（NUL、DEL 这些），而 Cf 里的零宽空格、bidi 覆盖
/// （U+202E）、BOM 不在其中。它们**不会**破坏编译，但原样写进生成文件会让译文里出现
/// 看不见的字符 —— trojan-source 式的阅读困惑，diff 与编辑器也会显示异常。
/// 这里按常见区段白名单列出，覆盖实际译文里可能出现的范围。
fn is_invisible_format(c: char) -> bool {
  matches!(
    c as u32,
    0x00AD | 0x061C | 0x180E | 0xFEFF
      | 0x200B..=0x200F
      | 0x202A..=0x202E
      | 0x2060..=0x2064
      | 0x2066..=0x206F
      | 0xE0001
      | 0xE0020..=0xE007F
  )
}
