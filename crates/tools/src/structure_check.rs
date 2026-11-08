//! 结构与架构的机械守卫：把 `docs/engineering/code-quality.md` 里「应为 0 / 单一来源」的几条搬进
//! `cargo make check`。
//!
//! 为什么落成工具：分层穿透（core 碰浏览器 API、crate 反向依赖 app/server）、一文件多组件、
//! 页面手写吸顶偏移、散落的 `alert()` 这些「结构回潮」靠人记得手跑 grep 不可靠 ——
//! 和 `ui_check` 把界面规范收口是同一个道理。
//!
//! 覆盖这几条（带 ID 的沿用 `docs/engineering/code-quality.md` 的检查项编号；后加的现代风守卫在该文档的
//! 「机械守卫」小节里按名字登记，这里不再另起编号 —— 之前在注释里自编的 C5 / C5.1 / C7.7
//! 与文档里同名编号的含义冲突，反而误导）：
//! - C3.1 一文件一组件：一个文件里出现两个以上「返回视图的函数」（`-> impl IntoView / AnyView`）；
//! - C2.3 页内手写吸顶偏移：`sticky top-[…]` 出现在页面里（第二根吸顶栏唯一来源是 StickyPanel）；
//! - C7.6 组件体里订阅风格信号：函数体顶层（缩进 2 空格）出现 `kit::is_modern()`，
//!   那里该用 `kit::is_modern_at_mount()`；另有按**调用点**判的一条 —— 读风格信号的闭包被
//!   立即调用（`{f()}`）时它跑在组件体构造期，同样没有跟踪上下文；
//! - C6.1 散落的 `alert()`：反馈应走 Toast / MessageDialog；
//! - A1.1 core 依赖浏览器 API：`web_sys` / `js_sys` / `wasm_bindgen` 出现在 core 源码；
//! - A1.3 crate 反向依赖 app/server：任何 crate 的依赖里出现 ham-web-app / ham-web-server；
//! - A2.1 数据显示层带色值：`crates/core` 里出现 `"#rrggbb"` 字面量（分类色这类「标识色」
//!   必须只给档位，色值归界面层的 `--cat-*`，否则 11 套配色方案会失效）；
//! - 运行时 panic 宏：`todo!(…)` / `unimplemented!(…)` 不应留到提交；
//! - 知识库译文词典挂载：页面用了 `kt_tracked` / `core_text_tracked` 渲染 `crates/core` 的中文原文，
//!   就必须在该页面（或其目录）里调 `use_knowledge_i18n()` 或走 `KnowledgePage` 外壳，
//!   否则以英文 / 西语直接打开该页时词典永远不来、文案静默停在中文；
//! - 双设计系统守卫：现代风目录不得出现像素风标识、像素专用文件不得出现 `md-*`、
//!   现代风样式的风格门禁 / 层外门禁 / `!important` 标注 / 视口断点白名单 / 类名索引；
//! - 阅读正文：`--md-text-prose` 必须是 `clamp(…vw…)`、`--md-measure` 必须是 `em`、
//!   `.md-article-page` 必须落这两个 token，且与 `docs/ui/modern.md` §2.3 写出的值一致
//!   （规范见该节「正文分两档」）；
//! - 现代风 `:has()` 标注 / 状态色字面值：`:has()` 必须同行写明缺失时的降级（`has-ok`），
//!   红 / 琥珀色相的十六进制只能写在 `tokens.css`（其余地方走 `--md-danger*` / `--md-warn*` / `--md-gold*`）；
//! - 退场时长 / 延迟卸载：退场动画（`*-out`）的时长必须走 token（两套皮肤读同一个值，Rust 侧的卸载延迟读它），
//!   `EXIT_MS` 常量只能出现在 `presence.rs`；
//! - 弹层三件套：带 `data-modal-root` 的面板必须装齐焦点 / 滚动锁 / Esc；
//! - 贴底固定层标记：`fixed` 贴底的元素必须带 `data-fixed-bottom`（窗口滚动条靠它让位）；
//! - 文档内链：相对链接指向的文件与 `§N` 小节都必须真实存在。
//!
//! 判定口径：每行先剥掉 `//` 之后的内容，`/*` 与 `*` 开头的行整行跳过（历史说明不算违规），
//! 与 `ui_check::code_part` 同语义 —— 这比手工 grep 更严格地「只看代码」，`qsl_image.rs` 里
//! 注释提到的 `web_sys::File::size()` 因此不会误报。

use std::path::{Component, Path, PathBuf};
use std::sync::LazyLock;

use anyhow::{Result, bail};
use regex::Regex;
use walkdir::WalkDir;

/// 每条规则最多打印多少条命中，避免一次刷屏。
const MAX_REPORT: usize = 10;

/// 一条逐行守卫规则。
struct LineRule {
  /// 规则名（输出用）
  name: &'static str,
  /// 扫描根（相对项目根目录）
  roots: &'static [&'static str],
  /// 相对路径里含这些片段就跳过
  skip: &'static [&'static str],
  /// 命中即违规
  hits: fn(&str) -> bool,
  /// 整改提示
  advice: &'static str,
}

const RULES: &[LineRule] = &[
  LineRule {
    name: "页面手写吸顶偏移（`sticky top-[…]`）",
    roots: &["crates/app/src/pages"],
    skip: &[],
    hits: has_sticky_top,
    advice: "页内第二根吸顶栏走 common::StickyPanel；页头吸顶骨架是 PAGE_HEADER_BAR 的唯一来源，页面里不要手写固定偏移",
  },
  LineRule {
    name: "风格开关被绕过（直接读 `ui:uiStyle` / `data-ui-style`）",
    roots: &["crates"],
    skip: &[
      "core/src/saved_state.rs",
      "core/src/ui_style.rs",
      "core/src/remote_sync.rs",
      "app/src/theme.rs",
      "tools/src/structure_check.rs",
    ],
    hits: has_style_switch_literal,
    advice: "风格开关只在 `theme.rs` 读写（`UiStyle` + `DisplayPrefs`）；页面与组件一律用 `crate::kit::is_modern()`，不要自己读存储键或 `<html>` 属性",
  },
  LineRule {
    name: "组件体里订阅风格信号（`kit::is_modern()`）",
    roots: &["crates/app/src"],
    skip: &[],
    hits: has_body_level_style_read,
    advice: "组件体里的派发用 `kit::is_modern_at_mount()`（构造期求值、不订阅信号）；`kit::is_modern()` 只留给 `view!` 属性闭包 / `move ||`，否则 dev 构建刷「在跟踪上下文之外读信号」告警",
  },
  LineRule {
    name: "散落的 `alert()`",
    roots: &["crates/app/src"],
    skip: &[],
    hits: has_alert,
    advice: "轻量反馈走 Toast，需确认的走 MessageDialog，不要用浏览器原生 alert",
  },
  LineRule {
    name: "core 依赖浏览器 API（`web_sys` / `js_sys` / `wasm_bindgen`）",
    roots: &["crates/core/src"],
    skip: &[],
    hits: has_browser_api,
    advice: "不依赖浏览器的逻辑放 ham-web-core；这三样只能在 crates/app 与各 worker 里出现",
  },
  // 色值白名单：`color.rs` / `color_scheme.rs` / 像素调色板工具本来就是「定义颜色」的地方。
  LineRule {
    name: "数据显示层带色值（`crates/core` 里的 `\"#rrggbb\"`）",
    roots: &["crates/core/src"],
    skip: &["color.rs", "color_scheme.rs", "/pixel/"],
    hits: has_hex_color,
    advice: "分类标识色只给档位（`CategoryTone`），色值在 `crates/app/src/tone.rs` 映射到 `--cat-*`；数据层写死色值会让 11 套配色方案失效",
  },
  // 本文件自身需要写 `todo!()` / `unimplemented!()` 这些词来识别它们，跳过自身避免误报。
  LineRule {
    name: "运行时 panic 宏（`todo!(…)` / `unimplemented!(…)`）",
    roots: &["crates"],
    skip: &["structure_check.rs"],
    hits: has_panic_macro,
    advice: "未完成 / 不可达用 Result / Option 表达，别把 todo!() / unimplemented!() 留到提交",
  },
];

fn has_sticky_top(line: &str) -> bool {
  line.contains("sticky top-[")
}

/// 浏览器原生 `alert(`:前面不能紧挨着标识符字符,否则 `show_alert(` / `raise_alert(` 这类
/// 自定义函数会被误判;`window.alert(` 与行首的 `alert(` 仍要命中。
fn has_alert(line: &str) -> bool {
  line.match_indices("alert(").any(|(i, _)| {
    line[..i]
      .chars()
      .next_back()
      .is_none_or(|c| !(c.is_alphanumeric() || c == '_'))
  })
}

fn has_browser_api(line: &str) -> bool {
  ["web_sys", "js_sys", "wasm_bindgen"]
    .iter()
    .any(|n| line.contains(n))
}

/// `"#rrggbb"` / `"#rgb"` 字面量（色值）：大小写都算，`"#` 后面至少 3 位十六进制。
fn has_hex_color(line: &str) -> bool {
  let bytes = line.as_bytes();
  for i in 0..bytes.len().saturating_sub(3) {
    if bytes[i] == b'"' && bytes[i + 1] == b'#' {
      let hex = &line[i + 2..];
      let n = hex.chars().take_while(|c| c.is_ascii_hexdigit()).count();
      if matches!(n, 3 | 4 | 6 | 8) && hex[n..].starts_with('"') {
        return true;
      }
    }
  }
  false
}

/// 现代风目录里出现像素风标识即违规（类名 `pxl-*` / 变量 `--pxl-*`）。
fn has_pixel_marker(line: &str) -> bool {
  line.contains("pxl-")
}

/// 现代风样式里「会指向像素风 DOM」的标识：选择器提到它们就必须带风格门禁。
///
/// 只列真正跨风格的：`pxl-*` 类名、`data-sig`（两套风格都会写到 `#route-content`）、
/// 桥接页卡片的调色板工具类（`bg-card` / `bg-muted` 这些不出现在现代风 DOM 上），
/// 以及两套皮肤共用的站点页脚 `body > footer`（像素风在 `pixel/surfaces.css` 对它另有底部让位，
/// 现代风的规则层序更靠后，漏掉门禁就会把像素风的 4rem 盖成现代风的 84px）。
const PIXEL_DOM_MARKERS: &[&str] = &[
  "pxl-",
  "bg-card",
  "bg-muted",
  "bg-background",
  "border-ink",
  "body > footer",
];

/// 按**顶层**逗号切开选择器列表（括号 / 方括号里的逗号不切，如 `:is(a, b)` / `[x="a,b"]`）。
///
/// 为什么要逐项判：`a, b {}` 里任何一项命中像素风 DOM 都会生效，而门禁标识只写在其中一项上时，
/// 按整条字符串判定会把其余几项一并放过。
fn split_selector_list(selector: &str) -> Vec<&str> {
  let mut items = Vec::new();
  let (mut depth, mut start) = (0i32, 0usize);
  let mut quote: Option<char> = None;
  for (i, c) in selector.char_indices() {
    match (quote, c) {
      (Some(q), c) if c == q => quote = None,
      (Some(_), _) => {}
      (None, '"' | '\'') => quote = Some(c),
      (None, '(' | '[') => depth += 1,
      (None, ')' | ']') => depth -= 1,
      (None, ',') if depth == 0 => {
        items.push(selector[start..i].trim());
        start = i + 1;
      }
      _ => {}
    }
  }
  items.push(selector[start..].trim());
  items.retain(|item| !item.is_empty());
  items
}

/// 单项选择器是否需要 `html[data-ui-style="modern"]` 门禁。
///
/// 成立条件有二：提到像素风 DOM 的标识；或者按 `data-sig` 分支却又没提到任何 `md-` 类
/// （`md-` 类只存在于现代风 DOM 里，本身就是门禁）。
fn item_needs_style_gate(item: &str) -> bool {
  if item.contains("data-ui-style") {
    return false;
  }
  if PIXEL_DOM_MARKERS.iter().any(|m| item.contains(m)) {
    return true;
  }
  item.contains("data-sig=") && !item.contains("md-")
}

/// 这条选择器（列表）是否有**任何一项**需要门禁却没带。
fn needs_style_gate(selector: &str) -> bool {
  split_selector_list(selector)
    .into_iter()
    .any(item_needs_style_gate)
}

fn has_style_switch_literal(line: &str) -> bool {
  line.contains("ui:uiStyle") || line.contains("data-ui-style")
}

/// 组件体里的顶层语句缩进固定是 2 空格（rustfmt `tab_spaces = 2`），响应式闭包里的调用必然
/// 更深。所以「缩进 ≤ 2 且调 `kit::is_modern()`」= 在组件体里订阅信号：那里只跑一次，
/// 订阅不会带来重渲染，只会让 dev 构建刷「在跟踪上下文之外读信号」的告警。
///
/// 只看行首缩进是有意为之：真要判「是否处于 `move ||` / `Signal::derive` 内」得做括号配对，
/// 而 rustfmt 已经把「函数体顶层 = 2 空格」变成结构事实，够用且不会误报。
///
/// 它抓不到「读信号的闭包被**立即调用**」（`{f()}`）：那种写法读信号那行确实在闭包里，
/// 由 [`check_eager_style_call`] 按调用点判。
fn has_body_level_style_read(line: &str) -> bool {
  let indent = line.len() - line.trim_start().len();
  indent <= 2 && line.contains("kit::is_modern()")
}

/// 订阅版风格读取的完整写法。`is_modern_at_mount()` 不含这个子串，不会被误认。
const STYLE_READ: &str = "kit::is_modern()";

/// 偏移量在剥注释后的代码里的行号（从 1 起）。
fn line_of(code: &str, offset: usize) -> usize {
  code[..offset.min(code.len())].matches('\n').count() + 1
}

/// `let NAME = <expr>;` 里 `<expr>` 的字节范围（`=` 之后到同一层的 `;`）。
///
/// 只按 `{}` `()` `[]` 计深度，不认字符串字面量：字符串里出现 `;` / 括号会让范围算歪，
/// 代价是这条闭包不再登记（漏报），不会凭空报别人的错。
fn binding_body(code: &str, from: usize) -> Option<(usize, usize)> {
  let bytes = code.as_bytes();
  let (mut depth, mut i, mut eq) = (0i32, from, None);
  while i < bytes.len() {
    match bytes[i] {
      b'{' | b'(' | b'[' => depth += 1,
      b'}' | b')' | b']' => depth -= 1,
      b';' if depth <= 0 => return eq.map(|e: usize| (e + 1, i)),
      // 跳过 `==` / `=>` / `!=` / `<=` / `>=`，只认赋值号。
      b'=' if depth <= 0 && eq.is_none() => {
        let next = bytes.get(i + 1).copied().unwrap_or(b' ');
        let prev = i.checked_sub(1).map(|p| bytes[p]).unwrap_or(b' ');
        if !matches!(next, b'=' | b'>') && !matches!(prev, b'!' | b'<' | b'>' | b'=') {
          eq = Some(i);
        }
      }
      _ => {}
    }
    i += 1;
  }
  None
}

/// 绑定到「读风格信号的闭包」的名字：`let NAME = … || … kit::is_modern() …;`。
///
/// 返回 `(名字, 语句体起, 语句体止)`；起止用来把定义处自身排除在调用点扫描之外。
fn style_read_closure_names(code: &str) -> Vec<(String, usize, usize)> {
  let mut out = Vec::new();
  for (at, _) in code.match_indices("let ") {
    // 紧挨着标识符字符的不是 `let` 关键字（`outlet ` 之类）。
    if code[..at]
      .chars()
      .next_back()
      .is_some_and(|c| c.is_alphanumeric() || c == '_')
    {
      continue;
    }
    let after = &code[at + 4..];
    let name: String = after
      .chars()
      .take_while(|c| c.is_alphanumeric() || *c == '_')
      .collect();
    if name.is_empty() || name.starts_with(|c: char| c.is_ascii_digit()) {
      continue;
    }
    // `let Some(x) = …` / `let (a, b) = …` 是解构，别把 `Some` 当成闭包名。
    let tail = after[name.len()..].trim_start();
    if !(tail.starts_with('=') || tail.starts_with(':')) {
      continue;
    }
    let Some((start, end)) = binding_body(code, at + 4) else {
      continue;
    };
    let body = &code[start..end];
    if body.contains("||") && body.contains(STYLE_READ) {
      out.push((name, start, end));
    }
  }
  out
}

/// 这一行是不是一个函数项的开头（自由函数或 `impl` 里的方法）。
///
/// 只认缩进 ≤ 2 的：rustfmt 已经把「函数体里的语句 = 2 空格以上」变成结构事实，更深的嵌套 `fn`
/// 不切（切了会把外层函数里「先定义闭包、后调用」拆到两段，漏报）。宁可漏报，不误报。
fn is_fn_item_line(line: &str) -> bool {
  let trimmed = line.trim_start();
  if line.len() - trimmed.len() > 2 {
    return false;
  }
  let mut rest = trimmed;
  if let Some(after_pub) = rest.strip_prefix("pub") {
    rest = after_pub.trim_start();
    // `pub(crate)` / `pub(super)`。
    if rest.starts_with('(') {
      rest = rest.split_once(')').map_or(rest, |(_, r)| r).trim_start();
    }
  }
  for keyword in ["const ", "async ", "unsafe "] {
    rest = rest.strip_prefix(keyword).unwrap_or(rest);
  }
  rest.starts_with("fn ")
}

/// 把剥注释后的代码按函数项切开：`(此段之前的行数, 片段)`。
///
/// 闭包名只在定义它的那个函数里有意义；另一个函数里的同名标识符（`chips` / `grid` 这类常见名字）
/// 不是它的调用点。
fn fn_segments(code: &str) -> Vec<(usize, &str)> {
  let mut cuts = vec![0usize];
  let mut offset = 0usize;
  for line in code.split('\n') {
    if offset > 0 && is_fn_item_line(line) {
      cuts.push(offset);
    }
    offset += line.len() + 1;
  }
  cuts.push(code.len());
  cuts
    .windows(2)
    .map(|w| (code[..w[0]].matches('\n').count(), &code[w[0]..w[1]]))
    .collect()
}

/// 被**立即调用**的「读风格信号的闭包」：`let f = … || { … kit::is_modern() … };` 之后又出现 `f()`。
///
/// 为什么单列一条：读信号那行确实在闭包里，[`has_body_level_style_read`] 按缩进判不出来。
/// 而这样的闭包只有在交给 `view!` 渲染（或 `Signal::derive` 这类构造器）时才跑在跟踪上下文里；
/// `{f()}` 是组件体构造期就把它跑完了 —— dev 构建会刷「在跟踪上下文之外读信号」，那块内容
/// 也不会随风格切换重渲染。
///
/// 按函数项分段匹配（[`fn_segments`]）：别的函数里同名的标识符不算。返回的行号是整个文件里的行号。
fn eager_style_calls(code: &str) -> Vec<(usize, String)> {
  let mut out: Vec<(usize, String)> = Vec::new();
  for (lines_before, segment) in fn_segments(code) {
    for (line, name) in eager_style_calls_in(segment) {
      out.push((lines_before + line, name));
    }
  }
  out.sort();
  out.dedup();
  out
}

/// [`eager_style_calls`] 在一个函数项范围内的匹配；行号相对这一段（从 1 起）。
fn eager_style_calls_in(code: &str) -> Vec<(usize, String)> {
  let mut out: Vec<(usize, String)> = Vec::new();
  for (name, start, end) in style_read_closure_names(code) {
    let mut from = 0usize;
    while let Some(rel) = code[from..].find(name.as_str()) {
      let pos = from + rel;
      from = pos + name.len();
      if pos >= start && pos < end {
        continue;
      }
      // 定义处 / 字段 / 路径上的同名标识符不算调用。
      if code[..pos]
        .chars()
        .next_back()
        .is_some_and(|c| c.is_alphanumeric() || c == '_' || c == '.' || c == ':')
      {
        continue;
      }
      if code[from..].trim_start().starts_with('(') {
        out.push((line_of(code, pos), name.clone()));
      }
    }
  }
  out.sort();
  out.dedup();
  out
}

/// `todo!()` / `todo!("…")` / `unimplemented!(…)` 都算：只认空括号的话，带说明文字的写法会漏过。
fn has_panic_macro(line: &str) -> bool {
  line.contains("todo!(") || line.contains("unimplemented!(")
}

/// 剥掉行注释，得到「参与判定的代码部分」。与 `ui_check::code_part` 同语义。
///
/// `/*` 与 `*` 开头的整行按块注释续行处理（`*slot = …` 这种解引用语句因此不参与扫描，
/// 代价是漏报 —— 它本来也不该出现被守卫的字符串）。字符串里含 `//`（URL）的行会被截断，
/// 只可能漏报、不会误报。
fn code_part(line: &str) -> &str {
  let trimmed = line.trim_start();
  if trimmed.starts_with("/*") || trimmed.starts_with('*') {
    return "";
  }
  match line.find("//") {
    Some(i) => &line[..i],
    None => line,
  }
}

/// 扫描若干目录下所有 `.rs`，对每行（剥注释后）跑 `hits`，返回 (命中总数, 前 MAX_REPORT 条证据)。
fn scan(
  roots: &[&str],
  root: &Path,
  skip: &[&str],
  hits: fn(&str) -> bool,
) -> (usize, Vec<String>) {
  let mut total = 0usize;
  let mut report: Vec<String> = Vec::new();
  for dir in roots {
    for entry in WalkDir::new(root.join(dir))
      .sort_by_file_name()
      .into_iter()
      .filter_map(Result::ok)
    {
      let path = entry.path();
      if !entry.file_type().is_file() || path.extension().is_none_or(|e| e != "rs") {
        continue;
      }
      let rel = path
        .strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/");
      if skip.iter().any(|s| rel.contains(s)) {
        continue;
      }
      let Ok(text) = std::fs::read_to_string(path) else {
        continue;
      };
      for (i, line) in text.lines().enumerate() {
        let code = code_part(line);
        if code.is_empty() || !hits(code) {
          continue;
        }
        total += 1;
        if report.len() < MAX_REPORT {
          report.push(format!("    {rel}:{}  {}", i + 1, code.trim()));
        }
      }
    }
  }
  (total, report)
}

/// 扫描前的存在性断言：目录不在 = 这条规则没生效，不能当成「没有违规」。
///
/// 规则全靠路径定位（`style/modern/`、`kit_modern/`、`docs/`…）。路径改名 / 挪位置之后
/// 旧代码会 `continue` 掉、照样打印 ✓ —— 比规则报错危险得多，因为没人会去看那行。
fn require_dir(root: &Path, rel: &str, label: &str) -> bool {
  if root.join(rel).is_dir() {
    return true;
  }
  println!("✗ {label}：目录 `{rel}` 不存在（改名 / 挪位置了？同步改这条规则的路径）");
  false
}

/// 扫描结果的统一收尾：**一个文件都没扫到也算失败**（目录在、但里面没有目标后缀的文件）。
fn scanned_ok(files: usize, label: &str) -> bool {
  if files > 0 {
    return true;
  }
  println!("✗ {label}：一个文件都没扫到（后缀或目录结构改了？）");
  false
}

/// 跑全部逐行规则，返回未通过规则的名称列表。
fn run_line_rules(root: &Path) -> Vec<String> {
  let mut failed = Vec::new();
  for rule in RULES {
    // 扫描根不存在 = 这条规则没生效，不能当成「零违规」打印 ✓（目录改名后最常见）。
    if let Some(missing) = rule.roots.iter().find(|dir| !root.join(dir).is_dir()) {
      println!(
        "✗ {}：扫描根 `{missing}` 不存在（改名 / 挪位置了？同步改这条规则的路径）",
        rule.name
      );
      failed.push(rule.name.to_owned());
      continue;
    }
    let (total, report) = scan(rule.roots, root, rule.skip, rule.hits);
    if total == 0 {
      println!("✓ {}", rule.name);
    } else {
      println!("✗ {}：{total} 处", rule.name);
      for line in &report {
        println!("{line}");
      }
      if total > report.len() {
        println!("    …（另有 {} 处，先修上面这些）", total - report.len());
      }
      println!("    → {}", rule.advice);
      failed.push(rule.name.to_owned());
    }
  }
  failed
}

/// 「一个文件只放一个组件」的正则：`-> impl IntoView` / `-> AnyView` / `-> IntoView`。
///
/// 与 `AGENTS.md` 的自检命令同口径（`impl` 后的可选路径 + 两个返回类型），逐行应用
/// （`\s` 会匹配换行，逐行跑才不会跨行误匹配）。
static VIEW_FN_RE: LazyLock<Regex> = LazyLock::new(|| {
  Regex::new(r"->\s*(impl\s+[A-Za-z_:]*\s*)?(IntoView|AnyView)").expect("一文件一组件正则应能编译")
});

/// C3.1：每个 `.rs` 文件里「返回视图的函数」最多一个，多一个就该拆文件。
fn check_one_component_per_file(root: &Path) -> bool {
  if !require_dir(root, "crates/app/src", "一文件一组件") {
    return false;
  }
  let mut files = 0usize;
  let mut report: Vec<String> = Vec::new();
  for entry in WalkDir::new(root.join("crates/app/src"))
    .sort_by_file_name()
    .into_iter()
    .filter_map(Result::ok)
  {
    let path = entry.path();
    if !entry.file_type().is_file() || path.extension().is_none_or(|e| e != "rs") {
      continue;
    }
    let rel = path
      .strip_prefix(root)
      .unwrap_or(path)
      .to_string_lossy()
      .replace('\\', "/");
    let Ok(text) = std::fs::read_to_string(path) else {
      continue;
    };
    files += 1;
    let count: usize = text
      .lines()
      .map(|l| VIEW_FN_RE.find_iter(code_part(l)).count())
      .sum();
    if count > 1 {
      report.push(format!("    {rel}: {count} 个返回视图的函数"));
    }
  }

  if !scanned_ok(files, "一文件一组件") {
    return false;
  }
  if report.is_empty() {
    println!("✓ 一文件一组件：每个文件最多一个返回视图的函数");
    return true;
  }
  println!("✗ 一文件一组件：{} 个文件超标", report.len());
  for line in &report {
    println!("{line}");
  }
  println!(
    "    → 一个文件只放一个组件（含任何 `-> impl IntoView / AnyView` 的视图函数）；主从组件用文件夹模块（mod.rs + 子文件）"
  );
  false
}

/// 组件体里「读风格信号的闭包被立即调用」（`{f()}`）：`is_modern()` 因此在没有跟踪上下文的
/// 构造期读一次信号 —— dev 构建刷「在跟踪上下文之外读信号」，那块内容也不会随风格切换重渲染。
///
/// 与 C7.6 的逐行规则互补：那条只看「组件体顶层有没有读订阅版信号」，这条看**调用点**。
fn check_eager_style_call(root: &Path) -> bool {
  if !require_dir(root, "crates/app/src", "风格读取闭包被立即调用") {
    return false;
  }
  let mut files = 0usize;
  let mut report: Vec<String> = Vec::new();
  for entry in WalkDir::new(root.join("crates/app/src"))
    .sort_by_file_name()
    .into_iter()
    .filter_map(Result::ok)
  {
    let path = entry.path();
    if !entry.file_type().is_file() || path.extension().is_none_or(|e| e != "rs") {
      continue;
    }
    let rel = path
      .strip_prefix(root)
      .unwrap_or(path)
      .to_string_lossy()
      .replace('\\', "/");
    let Ok(text) = std::fs::read_to_string(path) else {
      continue;
    };
    files += 1;
    // 拼接时保持行数不变，命中行号才和源码对得上。
    let code = text.lines().map(code_part).collect::<Vec<_>>().join("\n");
    for (line, name) in eager_style_calls(&code) {
      report.push(format!("    {rel}:{line}  {name}()"));
    }
  }

  if !scanned_ok(files, "风格读取闭包被立即调用") {
    return false;
  }
  if report.is_empty() {
    println!("✓ 风格读取闭包被立即调用：无");
    return true;
  }
  println!("✗ 风格读取闭包被立即调用：{} 处", report.len());
  for line in report.iter().take(MAX_REPORT) {
    println!("{line}");
  }
  if report.len() > MAX_REPORT {
    println!(
      "    …（另有 {} 处，先修上面这些）",
      report.len() - MAX_REPORT
    );
  }
  println!(
    "    → 闭包交给 `view!` 渲染（写 `{{f}}`，别写 `{{f()}}`）才跑在跟踪上下文里；确实要在构造期跑，就把里面的 `kit::is_modern()` 换成 `kit::is_modern_at_mount()`"
  );
  false
}

/// A1.3：任何 crate 的依赖都不得反向指向 `ham-web-app` / `ham-web-server`。
///
/// 只看「依赖 key」：行首（剥注释、去空白后）以 `ham-web-app` / `ham-web-server` 开头的才算，
/// 所以 `crates/app/Cargo.toml` 里 `name = "ham-web-app"` 这类 `[package]` 声明不会被误判。
fn check_no_reverse_deps(root: &Path) -> bool {
  let mut report: Vec<String> = Vec::new();
  let Ok(entries) = std::fs::read_dir(root.join("crates")) else {
    println!("✗ crate 依赖方向：目录 `crates` 不存在（改名了？）");
    return false;
  };
  for entry in entries.flatten() {
    let dir = entry.path();
    if !dir.is_dir() {
      continue;
    }
    let manifest = dir.join("Cargo.toml");
    if !manifest.is_file() {
      continue;
    }
    let crate_name = dir
      .file_name()
      .unwrap_or_default()
      .to_string_lossy()
      .to_string();
    let Ok(text) = std::fs::read_to_string(&manifest) else {
      continue;
    };
    for (i, line) in text.lines().enumerate() {
      let key = code_part(line).trim_start();
      if key.starts_with("ham-web-app") || key.starts_with("ham-web-server") {
        report.push(format!(
          "    crates/{crate_name}/Cargo.toml:{}  {key}",
          i + 1
        ));
      }
    }
  }

  if report.is_empty() {
    println!("✓ crate 依赖方向：无任何 crate 反向依赖 ham-web-app / ham-web-server");
    return true;
  }
  println!("✗ crate 依赖方向：{} 处反向依赖", report.len());
  for line in &report {
    println!("{line}");
  }
  println!("    → 分层依赖只能从上往下：core / 解码 crate / worker 不得依赖 app / server");
  false
}

/// 这一行是不是在用知识库译文（`crates/core` 中文原文 → 当前语言）。
fn uses_knowledge_text(code: &str) -> bool {
  code.contains("kt_tracked(") || code.contains("core_text_tracked(")
}

/// 这一行是不是在保证译文词典会被拉取：直接调 `use_knowledge_i18n`，或走自带它的 `KnowledgePage` 外壳。
fn loads_knowledge_dict(code: &str) -> bool {
  code.contains("use_knowledge_i18n") || code.contains("KnowledgePage")
}

/// A3.1：用了知识库译文的页面必须挂词典拉取。
///
/// 以 `pages/` 下的一级条目为单位（单文件页面看这个文件；目录页面看整个目录——子组件的词典由
/// 同目录的页面主体负责拉取）。`components/` 里的共享组件挂在各页面之下，没有「页面」这个边界可判，
/// 不在这条规则里。
fn check_knowledge_dict_is_loaded(root: &Path) -> bool {
  let pages = root.join("crates/app/src/pages");
  let Ok(entries) = std::fs::read_dir(&pages) else {
    println!("✗ 知识库译文词典：目录 `crates/app/src/pages` 不存在（改名了？）");
    return false;
  };
  let mut units: Vec<_> = entries.flatten().map(|e| e.path()).collect();
  units.sort();

  let mut report: Vec<String> = Vec::new();
  for unit in units {
    let (mut uses, mut loads) = (false, false);
    for entry in WalkDir::new(&unit).into_iter().filter_map(Result::ok) {
      let path = entry.path();
      if !entry.file_type().is_file() || path.extension().is_none_or(|e| e != "rs") {
        continue;
      }
      let Ok(text) = std::fs::read_to_string(path) else {
        continue;
      };
      for line in text.lines() {
        let code = code_part(line);
        uses |= uses_knowledge_text(code);
        loads |= loads_knowledge_dict(code);
      }
    }
    if uses && !loads {
      let rel = unit
        .strip_prefix(root)
        .unwrap_or(&unit)
        .to_string_lossy()
        .replace('\\', "/");
      report.push(format!("    {rel}"));
    }
  }

  if report.is_empty() {
    println!("✓ 知识库译文词典：用了 `kt_tracked` / `core_text_tracked` 的页面都挂了词典拉取");
    return true;
  }
  println!(
    "✗ 知识库译文词典：{} 个页面用了知识库译文却没挂词典拉取",
    report.len()
  );
  for line in &report {
    println!("{line}");
  }
  println!(
    "    → 在页面组件体里调 `data::use_knowledge_i18n()`（或改用 KnowledgePage 外壳）；否则英文 / 西语界面直接打开该页时文案停在中文"
  );
  false
}

/// 去掉 CSS 的 `/* … */` 注释（保留行结构，方便报行号）。
pub(crate) fn strip_css_comments(text: &str) -> String {
  let mut out = String::with_capacity(text.len());
  let mut in_comment = false;
  let mut chars = text.chars().peekable();
  while let Some(c) = chars.next() {
    if in_comment {
      if c == '*' && chars.peek() == Some(&'/') {
        chars.next();
        in_comment = false;
      } else if c == '\n' {
        out.push('\n');
      }
      continue;
    }
    if c == '/' && chars.peek() == Some(&'*') {
      chars.next();
      in_comment = true;
      continue;
    }
    out.push(c);
  }
  out
}

/// C5：双设计系统隔离 —— 现代风目录（`kit_modern/` 与 `style/modern/`）不得出现
/// 像素风的类名或变量。出现就说明两个 Kit 开始互相引用，切换风格时会串味。
fn check_modern_isolation(root: &Path) -> bool {
  let mut report: Vec<String> = Vec::new();
  let mut files = 0usize;
  for dir in ["crates/app/src/kit_modern", "crates/app/style/modern"] {
    if !require_dir(root, dir, "双设计系统隔离") {
      return false;
    }
  }
  let dirs = [
    (root.join("crates/app/src/kit_modern"), "rs"),
    (root.join("crates/app/style/modern"), "css"),
  ];
  for (dir, ext) in dirs {
    for entry in WalkDir::new(&dir)
      .sort_by_file_name()
      .into_iter()
      .filter_map(Result::ok)
    {
      let path = entry.path();
      if !entry.file_type().is_file() || path.extension().is_none_or(|e| e != ext) {
        continue;
      }
      let rel = path
        .strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/");
      let Ok(text) = std::fs::read_to_string(path) else {
        continue;
      };
      files += 1;
      let body = if ext == "rs" {
        text.lines().map(code_part).collect::<Vec<_>>().join("\n")
      } else {
        strip_css_comments(&text)
      };
      for (i, line) in body.lines().enumerate() {
        if has_pixel_marker(line) {
          report.push(format!("    {rel}:{}  {}", i + 1, line.trim()));
        }
      }
    }
  }

  if !scanned_ok(files, "双设计系统隔离") {
    return false;
  }
  if report.is_empty() {
    println!("✓ 双设计系统隔离：现代风的组件与样式里没有像素风标识");
    return true;
  }
  println!(
    "✗ 双设计系统隔离：{} 处现代风目录引用了像素风标识",
    report.len()
  );
  for line in report.iter().take(MAX_REPORT) {
    println!("{line}");
  }
  println!(
    "    → 现代风只能用 `md-*` / `--md-*`；需要复用的逻辑提到 `kit/`，不要借像素风的类名与变量"
  );
  false
}

/// 把一份 CSS 切成「非 at 规则」的选择器（多行选择器合并成一行），带起始行号。
///
/// 逐字符切「选择器块」：`{` 之前的文本是选择器（`@` 开头的是 at 规则，跳过），
/// `;` 与 `}` 清空缓冲。这样多行选择器、`@layer` / `@media` 嵌套里的规则都能被切出来。
///
/// `keep` 决定哪些收进结果：`all_selectors` 全收，门禁判定只收「需要门禁的」。
/// 这份切分器以前各写一遍（两份逐行相同），改一处必然漏一处。
fn selectors(css: &str, keep: impl Fn(&str) -> bool) -> Vec<(usize, String)> {
  let mut out = Vec::new();
  let mut buf = String::new();
  let (mut line, mut sel_line, mut line_started) = (1usize, 1usize, false);
  for ch in css.chars() {
    match ch {
      '\n' => {
        line += 1;
        buf.push(ch);
      }
      '{' => {
        let selector = buf.trim();
        if !selector.is_empty() && !selector.starts_with('@') && keep(selector) {
          let flat = selector.split_whitespace().collect::<Vec<_>>().join(" ");
          out.push((sel_line, flat));
        }
        buf.clear();
        line_started = false;
        sel_line = line;
      }
      '}' | ';' => {
        buf.clear();
        line_started = false;
        sel_line = line;
      }
      c => {
        if !c.is_whitespace() && !line_started {
          line_started = true;
          sel_line = line;
        }
        buf.push(c);
      }
    }
  }
  out
}

/// 文件是否「会进现代风」（共享文件）。
///
/// 判定是文本启发式，**宁可多扫**：出现 `is_modern` / `kit::` / `crate::ui::` /
/// `components::common::` 任一个就算共享。后两个不能省 —— 只用共享控件（`ui::Button`、
/// `common::EmptyState`）而不显式引用 `kit::` 的页面同样会进现代风，漏判会让无条件的
/// `border bg-inset` 从这里溜过去；误判的代价只是多扫几行（那些文件里本来也没有这种写法，
/// 有的话正是这条规则要抓的）。
fn is_shared_file(joined: &str) -> bool {
  joined.contains("is_modern")
    || joined.contains("kit::")
    || joined.contains("crate::ui::")
    || joined.contains("components::common::")
}

/// 无条件的 `class="…"` 字面量：同时含 `border` 与 `bg-inset` 两个类名（顺序不限、可跨行）。
/// 返回 `(起始行号, 字面量压成一行的样子)`；非共享文件返回空。
fn unconditional_inset_blocks(code: &[&str]) -> Vec<(usize, String)> {
  static CLASS_LITERAL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"class="([^"]*)""#).expect("class 字面量正则应能编译"));
  let joined = code.join("\n");
  if !is_shared_file(&joined) {
    return Vec::new();
  }
  CLASS_LITERAL
    .captures_iter(&joined)
    .filter_map(|c| {
      let body = c.get(1)?;
      let has = |name: &str| body.as_str().split_whitespace().any(|t| t == name);
      if !(has("border") && has("bg-inset")) {
        return None;
      }
      let line = joined[..body.start()].matches('\n').count() + 1;
      let one_line = body
        .as_str()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
      Some((line, format!("class=\"{one_line}\"")))
    })
    .collect()
}

/// 共享页面里不许再出现「像素式内嵌块」：`class="… border bg-inset …"`。
///
/// 为什么需要（台账 C1）：`bg-inset` 是像素风的凹陷块（1px 墨框 + 灰底 + 直角），
/// 在现代风里只能靠 `legacy-utilities.css` 把它映射成「灰底 + 6px 圆角」，看起来仍像
/// **像素页面换了配色**。现代风的内嵌块是 `.md-tile`（12px 内距、`--md-r-md` 圆角、悬停时描边转橙）。
///
/// 判据是「**无条件下**写在这一个 `class="…"` 字面量里」：字面量里同时有 `border` 与 `bg-inset`
/// 两个类名（顺序不限、可跨行）。按皮肤派发的写法
/// （`class=if crate::kit::is_modern_at_mount() { "md-tile …" } else { "border bg-inset …" }`）
/// 里右侧那串在 else 分支、不带 `class="` 前缀，不会被这条命中。像素专用文件（没有 `is_modern`
/// 引用）同样豁免优先级 —— 它们根本不进现代风。
fn check_pixel_inset_blocks(root: &Path) -> bool {
  if !require_dir(root, "crates/app/src", "像素式内嵌块") {
    return false;
  }
  let mut files = 0usize;
  let mut report: Vec<String> = Vec::new();
  for entry in WalkDir::new(root.join("crates/app/src"))
    .sort_by_file_name()
    .into_iter()
    .filter_map(Result::ok)
  {
    let path = entry.path();
    if !entry.file_type().is_file() || path.extension().is_none_or(|e| e != "rs") {
      continue;
    }
    let rel = path
      .strip_prefix(root)
      .unwrap_or(path)
      .to_string_lossy()
      .replace('\\', "/");
    let Ok(text) = std::fs::read_to_string(path) else {
      continue;
    };
    files += 1;
    let code: Vec<&str> = text.lines().map(code_part).collect();
    for (line, snippet) in unconditional_inset_blocks(&code) {
      report.push(format!("    {rel}:{line}  {snippet}"));
    }
  }

  if !scanned_ok(files, "像素式内嵌块") {
    return false;
  }
  if report.is_empty() {
    println!("✓ 像素式内嵌块：共享页面都用 `.md-tile` 或按皮肤派发");
    return true;
  }
  println!(
    "✗ 像素式内嵌块：{} 处无条件的 `border bg-inset`",
    report.len()
  );
  for line in report.iter().take(MAX_REPORT) {
    println!("{line}");
  }
  println!(
    "    → 改成 `class=if crate::kit::is_modern_at_mount() {{ \"md-tile …\" }} else {{ \"border bg-inset …\" }}`；见 docs/ui/modern-review.md 的 C1"
  );
  false
}

/// 反向隔离：**像素专用文件**里不能出现 `md-*` 类名。
///
/// 判定「像素专用」的口径很保守：文件里出现 `pxl-` 类名，且完全没有 `is_modern` / `kit::`
/// 的引用 —— 说明它只有像素分支，现代风根本不会渲染到它。这类文件里写 `md-*` 一定无效
/// （`settings_dialog.rs` 就被误改成 `md-setting-title`，而现代风的设置面板在
/// `kit_modern/settings.rs`），要么把现代类名漏进像素模式。
///
/// 注意**不能**再加「文件里没有 `md-`」这个前提：这条规则要找的正是 `md-`，把它写进
/// 进入条件，含 `md-` 的文件就全被跳过、规则永远空转（曾经就是这样，始终打印 ✓）。
fn check_pixel_isolation(root: &Path) -> bool {
  if !require_dir(root, "crates/app/src", "反向隔离") {
    return false;
  }
  let mut files = 0usize;
  let mut report: Vec<String> = Vec::new();
  for entry in WalkDir::new(root.join("crates/app/src"))
    .sort_by_file_name()
    .into_iter()
    .filter_map(Result::ok)
  {
    let path = entry.path();
    if !entry.file_type().is_file() || path.extension().is_none_or(|e| e != "rs") {
      continue;
    }
    let rel = path
      .strip_prefix(root)
      .unwrap_or(path)
      .to_string_lossy()
      .replace('\\', "/");
    if rel.contains("kit_modern") {
      continue;
    }
    let Ok(text) = std::fs::read_to_string(path) else {
      continue;
    };
    files += 1;
    let code: Vec<&str> = text.lines().map(code_part).collect();
    for (i, line) in md_class_lines_in_pixel_only(&code) {
      report.push(format!("    {rel}:{i}  {line}"));
    }
  }

  if !scanned_ok(files, "反向隔离") {
    return false;
  }
  if report.is_empty() {
    println!("✓ 反向隔离：像素专用文件里没有现代类名");
    return true;
  }
  println!("✗ 反向隔离：{} 处像素专用文件引用了 `md-*`", report.len());
  for line in report.iter().take(MAX_REPORT) {
    println!("{line}");
  }
  println!("    → 这些文件只在像素风下渲染；要改现代风请改对应分支或 `kit_modern/` 里的实现");
  false
}

/// 现代风的类名（`md-xxx`）。
static MD_CLASS: LazyLock<Regex> =
  LazyLock::new(|| Regex::new(r"\bmd-[a-z][a-z0-9-]*").expect("现代类名正则应能编译"));

/// 文件是否「只在像素风渲染」：有 `pxl-` 标识，且没有任何风格派发（`is_modern` / `kit::`）。
fn is_pixel_only(code: &[&str]) -> bool {
  let joined = code.join("\n");
  joined.contains("pxl-") && !joined.contains("is_modern") && !joined.contains("kit::")
}

/// 像素专用文件里所有出现 `md-*` 的行（行号从 1 起），其它文件返回空。
fn md_class_lines_in_pixel_only(code: &[&str]) -> Vec<(usize, String)> {
  if !is_pixel_only(code) {
    return Vec::new();
  }
  code
    .iter()
    .enumerate()
    .filter(|(_, line)| MD_CLASS.is_match(line))
    .map(|(i, line)| (i + 1, line.trim().to_owned()))
    .collect()
}

/// 去掉所有 `@layer … { … }` 块，只留层外内容。
fn strip_layer_blocks(css: &str) -> String {
  let chars: Vec<char> = css.chars().collect();
  let mut out = String::with_capacity(css.len());
  let mut i = 0usize;
  while i < chars.len() {
    let mut start = None;
    let mut k = i;
    while k < chars.len() {
      if chars[k] == '@' {
        let head: String = chars[k..(k + 6).min(chars.len())].iter().collect();
        if head.starts_with("@layer") {
          start = Some(k);
          break;
        }
      }
      k += 1;
    }
    let Some(start) = start else {
      out.extend(&chars[i..]);
      break;
    };
    out.extend(&chars[i..start]);
    let mut j = start;
    while j < chars.len() && chars[j] != '{' && chars[j] != ';' {
      j += 1;
    }
    if j >= chars.len() {
      break;
    }
    // `@layer a, b;`（只声明层顺序、没有块）：只丢掉这一句。按「找下一个 `{`」会把紧跟着的
    // 下一条规则连同它的选择器一起吞掉，等于把它排出检查范围。
    if chars[j] == ';' {
      i = j + 1;
      continue;
    }
    let mut depth = 0i32;
    while j < chars.len() {
      match chars[j] {
        '{' => depth += 1,
        '}' => {
          depth -= 1;
          if depth == 0 {
            j += 1;
            break;
          }
        }
        _ => {}
      }
      j += 1;
    }
    i = j;
  }
  out
}

/// 收集一份 CSS 里所有「非 at 规则」的选择器（含多行选择器），带行号。
fn all_selectors(css: &str) -> Vec<(usize, String)> {
  selectors(css, |_| true)
}

/// 层外样式必须自带现代风标识。
///
/// 层外（不在 `@layer` 里）的规则优先级高于 Tailwind 工具类 —— 这是刻意用来压住
/// `px-3 py-2` 那类旧单元格工具类的。代价是：一旦这样的规则命中像素风 DOM，就会
/// 直接改掉像素表格的外观（`TableDensity` 的 `md-table-compact` 漏进像素模式那次就是）。
/// 所以层外规则必须二选一：选择器带 `md-`，或带 `data-ui-style` 门禁。
fn check_unlayered_modern_css(root: &Path) -> bool {
  if !require_dir(root, "crates/app/style", "层外样式门禁") {
    return false;
  }
  let dir = root.join("crates/app/style");
  let mut files = 0usize;
  let mut report: Vec<String> = Vec::new();
  for entry in WalkDir::new(&dir)
    .sort_by_file_name()
    .into_iter()
    .filter_map(Result::ok)
  {
    let path = entry.path();
    if !entry.file_type().is_file() || path.extension().is_none_or(|e| e != "css") {
      continue;
    }
    let rel = path
      .strip_prefix(root)
      .unwrap_or(path)
      .to_string_lossy()
      .replace('\\', "/");
    if !rel.contains("style/modern") {
      continue;
    }
    let Ok(text) = std::fs::read_to_string(path) else {
      continue;
    };
    files += 1;
    let stripped = strip_layer_blocks(&strip_css_comments(&text));
    for (line, selector) in all_selectors(&stripped) {
      // 逐项判：`a, .md-b {}` 里的 `a` 照样命中像素风 DOM，不能因为另一项带了 `md-` 就整条放过。
      let ungated = split_selector_list(&selector).into_iter().any(|item| {
        let keyframe_step = item == "from" || item == "to" || item.ends_with('%');
        // View Transition 伪元素：只有 `startViewTransition` 期间才存在，而它只由现代风的
        // `with_view_transition` 发起（像素风从不调用），不需要再挂风格门禁。
        let view_transition = item.starts_with("::view-transition");
        !(keyframe_step
          || view_transition
          || item.contains("md-")
          || item.contains("data-ui-style"))
      });
      if ungated {
        report.push(format!(
          "    {rel}:{line}  {}",
          selector.chars().take(90).collect::<String>()
        ));
      }
    }
  }

  if !scanned_ok(files, "层外样式门禁") {
    return false;
  }
  if report.is_empty() {
    println!("✓ 层外样式门禁：层外规则都带 `md-` 或 `data-ui-style` 门禁");
    return true;
  }
  println!("✗ 层外样式门禁：{} 处层外规则没有现代风标识", report.len());
  for line in report.iter().take(MAX_REPORT) {
    println!("{line}");
  }
  println!(
    "    → 层外规则优先级高于工具类，必须只在现代风生效：选择器带 `md-` 或 `html[data-ui-style=\"modern\"]`"
  );
  false
}

/// 一行（已剥注释）CSS 里是否真的在**使用** `:has()` 选择器。
///
/// `@supports not selector(:has(*))` 是特性检测，不是在用 —— 回退规则正是靠它写的。
fn uses_has_selector(line: &str) -> bool {
  line.replace("selector(:has(", "").contains(":has(")
}

/// 用了 `:has()` 却没在**同一行**写 `has-ok` 的行号（从 1 起）。
///
/// `:has()` 在 Firefox 120 及以下不支持，整条规则会被丢掉。丢掉可以，但必须是有意的：
/// 版式 / 焦点环这类「丢了就坏」的用途要换成属性选择器或写 `@supports` 回退，
/// 纯装饰的用途在行尾写明缺失时的后果。`has-ok` 就是这条声明。
fn unmarked_has_lines(text: &str) -> Vec<usize> {
  let stripped = strip_css_comments(text);
  let orig: Vec<&str> = text.lines().collect();
  stripped
    .lines()
    .enumerate()
    .filter(|&(i, line)| {
      uses_has_selector(line) && !orig.get(i).copied().unwrap_or("").contains("has-ok")
    })
    .map(|(i, _)| i + 1)
    .collect()
}

/// 一行里的红 / 琥珀（含橙）色相十六进制字面值。
///
/// 判定：色相 ≤ 50° 或 ≥ 340°，饱和度 ≥ 0.35，明度 ≥ 0.3。明度下限把「深棕墨水字」（`#21120a`）
/// 排除在外 —— 它们是实心橙面上的字，浅深两套明暗都成立；要拦的是「按深色底调的浅粉红 / 亮金」。
fn warm_hex_literals(line: &str) -> Vec<String> {
  let bytes = line.as_bytes();
  let mut out = Vec::new();
  let mut i = 0;
  while i < bytes.len() {
    if bytes[i] != b'#' {
      i += 1;
      continue;
    }
    let start = i + 1;
    let mut end = start;
    while end < bytes.len() && bytes[end].is_ascii_hexdigit() {
      end += 1;
    }
    // 后面紧跟标识符字符的是 id / 片段（`#md-moon`），不是色值
    let glued = bytes
      .get(end)
      .is_some_and(|b| b.is_ascii_alphanumeric() || *b == b'-' || *b == b'_');
    let digits = &line[start..end];
    if !glued && matches!(digits.len(), 3 | 6 | 8) {
      // 8 位带透明度的只看前 6 位的色相
      let take = if digits.len() == 3 { 3 } else { 6 };
      let hex = format!("#{}", &digits[..take]);
      if let Some(rgb) = ham_web_core::color::Rgb::parse_css(&hex)
        && is_warm(rgb)
      {
        out.push(hex);
      }
    }
    i = end.max(i + 1);
  }
  out
}

fn is_warm(rgb: ham_web_core::color::Rgb) -> bool {
  let (r, g, b) = (
    f64::from(rgb.0) / 255.0,
    f64::from(rgb.1) / 255.0,
    f64::from(rgb.2) / 255.0,
  );
  let max = r.max(g).max(b);
  let min = r.min(g).min(b);
  let delta = max - min;
  if max < 0.3 || delta == 0.0 || delta / max < 0.35 {
    return false;
  }
  let hue = if (max - r).abs() < f64::EPSILON {
    ((g - b) / delta).rem_euclid(6.0) * 60.0
  } else if (max - g).abs() < f64::EPSILON {
    ((b - r) / delta + 2.0) * 60.0
  } else {
    ((r - g) / delta + 4.0) * 60.0
  };
  !(50.0..340.0).contains(&hue)
}

/// 写了红 / 琥珀色字面值却没在同一行写 `color-ok` 的 `(行号, 色值)`。
fn unmarked_warm_color_lines(text: &str) -> Vec<(usize, String)> {
  let stripped = strip_css_comments(text);
  let orig: Vec<&str> = text.lines().collect();
  let mut out = Vec::new();
  for (i, line) in stripped.lines().enumerate() {
    if orig.get(i).copied().unwrap_or("").contains("color-ok") {
      continue;
    }
    for hex in warm_hex_literals(line) {
      out.push((i + 1, hex));
    }
  }
  out
}

/// 遍历 `style/modern/*.css`，对每个文件跑 `scan`；`skip` 里的文件名不扫。
fn scan_modern_css<T>(
  root: &Path,
  label: &str,
  skip: &[&str],
  scan: impl Fn(&str) -> Vec<(usize, T)>,
  show: impl Fn(&T) -> String,
) -> Option<Vec<String>> {
  scan_css_dir(root, "crates/app/style/modern", label, skip, scan, show)
}

/// 遍历 `rel_dir` 下的 `*.css`（两套皮肤各一个目录），对每个文件跑 `scan`；`skip` 里的文件名不扫。
fn scan_css_dir<T>(
  root: &Path,
  rel_dir: &str,
  label: &str,
  skip: &[&str],
  scan: impl Fn(&str) -> Vec<(usize, T)>,
  show: impl Fn(&T) -> String,
) -> Option<Vec<String>> {
  if !require_dir(root, rel_dir, label) {
    return None;
  }
  let mut files = 0usize;
  let mut report = Vec::new();
  for entry in WalkDir::new(root.join(rel_dir))
    .sort_by_file_name()
    .into_iter()
    .filter_map(Result::ok)
  {
    let path = entry.path();
    if !entry.file_type().is_file() || path.extension().is_none_or(|e| e != "css") {
      continue;
    }
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    if skip.contains(&name) {
      continue;
    }
    let Ok(text) = std::fs::read_to_string(path) else {
      continue;
    };
    files += 1;
    let orig: Vec<&str> = text.lines().collect();
    for (line_no, item) in scan(&text) {
      report.push(format!(
        "    {name}:{line_no}  {}  {}",
        show(&item),
        orig
          .get(line_no - 1)
          .map(|l| l.trim().chars().take(60).collect::<String>())
          .unwrap_or_default()
      ));
    }
  }
  scanned_ok(files, label).then_some(report)
}

/// 现代风 `:has()` 必须声明缺失时的降级（同一行 `has-ok`）。
fn check_modern_has_marker(root: &Path) -> bool {
  let label = "`:has()` 降级声明";
  let Some(report) = scan_modern_css(
    root,
    label,
    &[],
    |text| {
      unmarked_has_lines(text)
        .into_iter()
        .map(|n| (n, ()))
        .collect()
    },
    |()| String::new(),
  ) else {
    return false;
  };
  if report.is_empty() {
    println!("✓ {label}：每处 `:has()` 都写明了缺失时的降级");
    return true;
  }
  println!("✗ {label}：{} 处没写 `has-ok`", report.len());
  for line in report.iter().take(MAX_REPORT) {
    println!("{line}");
  }
  println!(
    "    → 版式 / 焦点环别靠 `:has()`（改属性选择器，或加 `@supports not selector(:has(*))` 回退）；\n      纯装饰的在同一行补 `/* has-ok: 缺失时的后果 */`"
  );
  false
}

/// 红 / 琥珀色相的十六进制只能写在 `tokens.css`（品牌渐变等例外在同行写 `color-ok`）。
fn check_modern_state_colors(root: &Path) -> bool {
  let label = "状态色字面值";
  let Some(report) = scan_modern_css(
    root,
    label,
    &["tokens.css"],
    unmarked_warm_color_lines,
    String::clone,
  ) else {
    return false;
  };
  if report.is_empty() {
    println!("✓ {label}：组件里没有写死的红 / 琥珀色（都走 token）");
    return true;
  }
  println!("✗ {label}：{} 处写死了红 / 琥珀色", report.len());
  for line in report.iter().take(MAX_REPORT) {
    println!("{line}");
  }
  println!(
    "    → 按承载面取 `--md-danger*` / `--md-warn*` / `--md-gold*`（见 tokens.css 的说明）；\n      品牌渐变这类确需字面值的，在同一行补 `/* color-ok: 理由 */`"
  );
  false
}

/// 退场动画（关键帧名以 `-out` / `-out-only` 结尾、且不是无限循环）里写死的时长字面量 `(行号, 字面量)`。
///
/// 为什么只管退场：弹层关闭后元素要留在 DOM 里等动画播完再卸载，卸载延迟由 Rust 侧读样式表里的
/// 时长 token 得到（`presence::exit_ms`）。退场动画若写成 `180ms` 这样的字面量，改了 token 它不会跟着变 ——
/// 动画比延迟长就被截断，比延迟短就留一段空壳。入场动画没有这个耦合，不在此列。
/// 路由过渡的 `*-out` 之类不走延迟卸载的，在同行写 `/* dur-ok: 理由 */`。
fn hardcoded_exit_durations(text: &str) -> Vec<(usize, String)> {
  let stripped = strip_css_comments(text);
  let orig: Vec<&str> = text.lines().collect();
  let mut out = Vec::new();
  for (i, line) in stripped.lines().enumerate() {
    // 声明可能和选择器写在同一行（`.a { animation: … }`）：不能要求它在行首。
    let Some(at) = line.find("animation:") else {
      continue;
    };
    if orig.get(i).copied().unwrap_or("").contains("dur-ok") {
      continue;
    }
    // 只取这一条声明（到分号 / 右花括号为止），别把同一行里后面的别的声明吃进来。
    let decl = line[at + "animation:".len()..]
      .split([';', '}'])
      .next()
      .unwrap_or("");
    let tokens: Vec<&str> = decl
      .split_whitespace()
      .map(|t| t.trim_end_matches(','))
      .collect();
    let is_exit = tokens
      .iter()
      .any(|t| t.ends_with("-out") || t.ends_with("-out-only"));
    // 无限循环的是氛围动画（`md-ring-out` 的「声呐环」之类），不是弹层退场。
    if !is_exit || tokens.contains(&"infinite") {
      continue;
    }
    if let Some(lit) = tokens
      .iter()
      .find(|t| ham_web_core::css_time::parse_ms(t).is_some())
    {
      out.push((i + 1, (*lit).to_owned()));
    }
  }
  out
}

/// 两套皮肤的退场动画时长必须走 token。
fn check_exit_durations(root: &Path) -> bool {
  let label = "退场时长走 token";
  let mut report = Vec::new();
  for dir in ["crates/app/style/modern", "crates/app/style/pixel"] {
    let Some(r) = scan_css_dir(
      root,
      dir,
      label,
      &[],
      hardcoded_exit_durations,
      String::clone,
    ) else {
      return false;
    };
    report.extend(r);
  }
  if report.is_empty() {
    println!("✓ {label}：退场动画的时长都走 token，卸载延迟读同一个值");
    return true;
  }
  println!("✗ {label}：{} 处退场动画写死了时长", report.len());
  for line in report.iter().take(MAX_REPORT) {
    println!("{line}");
  }
  println!(
    "    → 改用 `var(--md-dur)`（现代）/ `var(--motion-normal)`（像素）；不走延迟卸载的（路由过渡）同行补 `/* dur-ok: 理由 */`\n      （为什么：Rust 侧的卸载延迟读的是这个 token，字面量不会跟着变，退场会被截断或多留一段空壳）"
  );
  false
}

/// 定义了 `*EXIT_MS` 常量的行号（从 1 起）。
///
/// 弹层关闭后的延迟卸载只有 `presence.rs` 一份，时长读样式表里的 token。这个名字的常量曾经在
/// 六个文件里各写一份（200 / 200 / 240 / 180 / 180 / 180），注释里都写着「要和 CSS 同步」，没有任何东西保证。
fn exit_ms_consts(text: &str) -> Vec<usize> {
  text
    .lines()
    .enumerate()
    .filter(|(_, l)| {
      let t = l.trim_start();
      (t.starts_with("const ") || t.starts_with("pub const ") || t.starts_with("pub(crate) const "))
        && t
          .split(':')
          .next()
          .is_some_and(|head| head.contains("EXIT_MS"))
    })
    .map(|(i, _)| i + 1)
    .collect()
}

/// `EXIT_MS` 一类的常量只能出现在 `presence.rs`。
fn check_single_presence(root: &Path) -> bool {
  let label = "延迟卸载只有一份";
  if !require_dir(root, "crates/app/src", label) {
    return false;
  }
  let mut files = 0usize;
  let mut report = Vec::new();
  for entry in WalkDir::new(root.join("crates/app/src"))
    .sort_by_file_name()
    .into_iter()
    .filter_map(Result::ok)
  {
    let path = entry.path();
    if !entry.file_type().is_file() || path.extension().is_none_or(|e| e != "rs") {
      continue;
    }
    files += 1;
    if path.file_name().is_some_and(|n| n == "presence.rs") {
      continue;
    }
    let Ok(text) = std::fs::read_to_string(path) else {
      continue;
    };
    let rel = path
      .strip_prefix(root)
      .unwrap_or(path)
      .display()
      .to_string();
    for line in exit_ms_consts(&text) {
      report.push(format!("    {rel}:{line}"));
    }
  }
  if !scanned_ok(files, label) {
    return false;
  }
  if report.is_empty() {
    println!("✓ {label}：退场卸载延迟只有 `presence.rs` 一份");
    return true;
  }
  println!("✗ {label}：{} 处自己定义了 `EXIT_MS` 常量", report.len());
  for line in report.iter().take(MAX_REPORT) {
    println!("{line}");
  }
  println!(
    "    → 用 `crate::presence::use_presence(open)`（或 `exit_ms(ExitSpeed::…)` 读 token），不要再手写毫秒数"
  );
  false
}

/// `!important` 必须显式标注理由。
///
/// 现代风里样式该走「层顺序 / 更具体的选择器」而不是 `!important`；确有必要时
/// （压像素卡片类、降级动画），在**同一行**补 `/* important-ok: 理由 */`。
/// 这样每一条都能在 code review 里被看见，而不是靠 memchr 一个关键字。
fn check_important_marker(root: &Path) -> bool {
  if !require_dir(root, "crates/app/style/modern", "`!important` 标注") {
    return false;
  }
  let mut files = 0usize;
  let mut report: Vec<String> = Vec::new();
  // 扫目录而不是列举文件：白名单会腐化（漏掉的 `legacy-utilities.css` 就曾有裸圆角）。
  for entry in WalkDir::new(root.join("crates/app/style/modern"))
    .sort_by_file_name()
    .into_iter()
    .filter_map(Result::ok)
  {
    let path = entry.path();
    if !entry.file_type().is_file() || path.extension().is_none_or(|e| e != "css") {
      continue;
    }
    let rel = path
      .strip_prefix(root)
      .unwrap_or(path)
      .to_string_lossy()
      .replace('\\', "/");
    let Ok(text) = std::fs::read_to_string(path) else {
      continue;
    };
    files += 1;
    // 注释里的 `!important` 不算声明；`strip_css_comments` 保留换行，行号仍然对齐
    let stripped = strip_css_comments(&text);
    let orig: Vec<&str> = text.lines().collect();
    for (i, line) in stripped.lines().enumerate() {
      if !line.contains("!important") {
        continue;
      }
      let original = orig.get(i).copied().unwrap_or("");
      if !original.contains("important-ok") {
        report.push(format!(
          "    {rel}:{}  {}",
          i + 1,
          original.trim().chars().take(80).collect::<String>()
        ));
      }
    }
  }

  if !scanned_ok(files, "`!important` 标注") {
    return false;
  }
  if report.is_empty() {
    println!("✓ `!important` 标注：每一条都写了理由");
    return true;
  }
  println!("✗ `!important` 标注：{} 处没写理由", report.len());
  for line in report.iter().take(MAX_REPORT) {
    println!("{line}");
  }
  println!(
    "    → 先找别的通道（层顺序 / 更具体的选择器）；确有必要时在同一行补 `/* important-ok: 理由 */`"
  );
  false
}

/// 现代风视口断点白名单：`docs/ui/modern.md` §3 的「五档」+ 底部 dock 的 768 + 顶栏折叠的 360。
///
/// 曾漂移出 899（900 的「减一」反模式，同 639）、767（768 的减一）、860、960 这些
/// 非档位值 —— `ui_check` 只拦页面 class 里的 `min-[NNNpx]:`，拦不到裸 `@media`。
const MODERN_BREAKPOINTS: [u32; 7] = [360, 480, 640, 720, 768, 900, 1024];

/// 匹配 `@media CONDITION {`：`[^\{]*` 能跨行吃到第一个 `{` 前的条件部分。
static MEDIA_QUERY: LazyLock<Regex> =
  LazyLock::new(|| Regex::new(r"@media[^\{]*\{").expect("断点扫描正则应能编译"));
/// 匹配条件里的宽度阈值：`min-width: 640px` / `max-width: 899.98px` / `width <= 56em` /
/// `(width >= 40rem)`（范围语法）。只认 `px` / `em` / `rem`，后两者按 16px 折算。
static WIDTH_VALUE: LazyLock<Regex> = LazyLock::new(|| {
  Regex::new(r"(?:(?:min|max)-width\s*:\s*|\bwidth\s*[<>]=?\s*)(\d+(?:\.\d+)?)(px|em|rem)")
    .expect("断点值正则应能编译")
});

/// 从一份 CSS 文本里找出所有「非白名单」的视口断点，返回 (行号, 折算后的 px)。
///
/// 只扫 `@media`（视口查询），不扫 `@container`（容器查询阈值按容器宽定，如
/// `.md-menu-grid` 的 `max-width:520px`，不属于排版断点）。调用方负责先剥掉注释。
/// 生产检查与单测共用这一份：以前测试里另有一份「同样逻辑」的拷贝，生产代码改坏了
/// 测试照样是绿的。
fn off_list_breakpoints(css: &str) -> Vec<(usize, f64)> {
  let mut out = Vec::new();
  for m in MEDIA_QUERY.find_iter(css) {
    for w in WIDTH_VALUE.captures_iter(m.as_str()) {
      let Ok(value) = w[1].parse::<f64>() else {
        // 数字解析不出来（正则保证不会发生）也不能静默跳过：当成越界报出去。
        out.push((css[..m.start()].matches('\n').count() + 1, f64::NAN));
        continue;
      };
      let px = if &w[2] == "px" { value } else { value * 16.0 };
      let on_list = MODERN_BREAKPOINTS
        .iter()
        .any(|bp| (f64::from(*bp) - px).abs() < f64::EPSILON);
      if px > 0.0 && !on_list {
        out.push((css[..m.start()].matches('\n').count() + 1, px));
      }
    }
  }
  out
}

/// 同一个值上同时出现「含 X 的下界」与「含 X 的上界」：恰好 X 宽时两档同时命中。
///
/// `min-width: 900px` 与 `max-width: 900px` 都**含** 900，所以两者不能共存；互斥档要么把上界写成
/// `(width < 900px)`，要么用范围语法 `(640px <= width < 900px)`。只查值（[`off_list_breakpoints`]）
/// 看不出这类错：两个 900 都在白名单里。
fn overlapping_bounds(css: &str) -> Vec<(usize, f64)> {
  /// `min-width: X` / `width >= X`：含 X 的下界。
  static LOWER: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?:min-width\s*:\s*|width\s*>=\s*)(\d+(?:\.\d+)?)(px|em|rem)")
      .expect("下界正则应能编译")
  });
  /// `max-width: X` / `width <= X`：含 X 的上界。
  static UPPER: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?:max-width\s*:\s*|width\s*<=\s*)(\d+(?:\.\d+)?)(px|em|rem)")
      .expect("上界正则应能编译")
  });

  let px_of = |value: &str, unit: &str| -> f64 {
    let n = value.parse::<f64>().unwrap_or(f64::NAN);
    if unit == "px" { n } else { n * 16.0 }
  };
  let line_of = |at: usize| css[..at].matches('\n').count() + 1;

  let mut lowers: Vec<(f64, usize)> = Vec::new();
  for m in MEDIA_QUERY.find_iter(css) {
    for c in LOWER.captures_iter(m.as_str()) {
      lowers.push((px_of(&c[1], &c[2]), line_of(m.start())));
    }
  }
  let mut out = Vec::new();
  for m in MEDIA_QUERY.find_iter(css) {
    for c in UPPER.captures_iter(m.as_str()) {
      let px = px_of(&c[1], &c[2]);
      if lowers
        .iter()
        .any(|(low, _)| (low - px).abs() < f64::EPSILON)
      {
        out.push((line_of(m.start()), px));
      }
    }
  }
  out
}

/// C7.7：现代风 CSS 的视口断点只能取白名单里的值，且同一值的上下界不能同时含进来。
///
/// 为什么进门禁：断点规范（`docs/ui/modern.md` §3）是「五档 + dock 768」，但裸 `@media` 里的像素值
/// 没有静态检查守着，很容易又长出 899 / 767 这类「减一」相邻数字，改一处漏一处。
fn check_modern_breakpoints(root: &Path) -> bool {
  if !require_dir(root, "crates/app/style/modern", "现代风断点") {
    return false;
  }
  let dir = root.join("crates/app/style/modern");
  let mut files = 0usize;
  let mut report: Vec<String> = Vec::new();
  let mut overlaps: Vec<String> = Vec::new();
  for entry in WalkDir::new(&dir)
    .sort_by_file_name()
    .into_iter()
    .filter_map(Result::ok)
  {
    let path = entry.path();
    if !entry.file_type().is_file() || path.extension().is_none_or(|e| e != "css") {
      continue;
    }
    let rel = path
      .strip_prefix(root)
      .unwrap_or(path)
      .to_string_lossy()
      .replace('\\', "/");
    let Ok(text) = std::fs::read_to_string(path) else {
      continue;
    };
    files += 1;
    let stripped = strip_css_comments(&text);
    for (line, px) in off_list_breakpoints(&stripped) {
      report.push(format!("    {rel}:{line}  {px}px"));
    }
    for (line, px) in overlapping_bounds(&stripped) {
      overlaps.push(format!(
        "    {rel}:{line}  {px}px 上下界都含 {px}（恰好 {px}px 时两档同时命中）"
      ));
    }
  }

  if !scanned_ok(files, "现代风断点") {
    return false;
  }
  if report.is_empty() && overlaps.is_empty() {
    println!("✓ 现代风断点：视口断点都在 360 + 五档 + 768 白名单内，同一值的上下界不重叠");
    return true;
  }
  if !report.is_empty() {
    println!("✗ 现代风断点：{} 处非白名单断点", report.len());
    for line in report.iter().take(MAX_REPORT) {
      println!("{line}");
    }
    println!(
      "    → 视口断点只有 480/640/720/900/1024 五档 + 底部 dock 的 768 + 顶栏折叠的 360；窄屏档写 640 不写 639（同理不写 899/767），见 docs/ui/modern.md §3"
    );
  }
  if !overlaps.is_empty() {
    println!("✗ 现代风断点：{} 处同值上下界重叠", overlaps.len());
    for line in overlaps.iter().take(MAX_REPORT) {
      println!("{line}");
    }
    println!(
      "    → 「≥ X」与「≤ X」都含 X：上界改写成 `(width < Xpx)`，或用范围语法 `(Apx <= width < Xpx)`，见 docs/engineering/code-quality.md 的守卫表"
    );
  }
  false
}

/// 把现代风的 kit 样式读成一份文本：入口 `kit.css`（约定 + `@import`）+ `kit/` 下的分域文件。
///
/// 台账 C4 之后 `kit.css` 只剩约定与导入，规则在 `kit/*.css` 里。任何「在 kit.css 里找某条声明」
/// 的门禁都必须走这个函数，否则会退化成空转（文件里当然找不到 —— 但它找的东西其实还在）。
fn read_modern_kit(root: &Path) -> Option<String> {
  let entry = root.join("crates/app/style/modern/kit.css");
  let mut out = std::fs::read_to_string(entry).ok()?;
  let dir = root.join("crates/app/style/modern/kit");
  let mut files: Vec<std::path::PathBuf> = std::fs::read_dir(dir)
    .ok()
    .into_iter()
    .flatten()
    .filter_map(Result::ok)
    .map(|e| e.path())
    .filter(|p| p.extension().is_some_and(|x| x == "css"))
    .collect();
  files.sort();
  for f in files {
    if let Ok(text) = std::fs::read_to_string(&f) {
      out.push('\n');
      out.push_str(&text);
    }
  }
  Some(out)
}

/// 对比入口文件的 `@import` 列表与目录里的分片。
/// 返回 `(已导入的文件名, 在目录里却没导入的, 导入了却不存在的)`，三者都已排序。
fn diff_kit_imports(
  entry: &str,
  mut on_disk: Vec<String>,
) -> (Vec<String>, Vec<String>, Vec<String>) {
  static KIT_IMPORT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"@import\s+"\./kit/([^"]+)""#).expect("导入正则应能编译"));
  on_disk.sort();
  let mut imported: Vec<String> = KIT_IMPORT
    .captures_iter(entry)
    .map(|c| c[1].to_owned())
    .collect();
  imported.sort();
  let not_imported = on_disk
    .iter()
    .filter(|n| !imported.contains(n))
    .cloned()
    .collect();
  let missing_file = imported
    .iter()
    .filter(|n| !on_disk.contains(n))
    .cloned()
    .collect();
  (imported, not_imported, missing_file)
}

/// `kit.css` 的 `@import` 列表必须与 `kit/` 目录里的分片**完全一致**。
///
/// 只有 `@import` 进入口的分片才会进构建产物。新分片忘了写 `@import` 时：源码里「有」这份样式、
/// 读目录的 [`read_modern_kit`] 也读得到、单测照常绿 —— 但产物里没有它，
/// 表现为「改了半天没生效」。反方向（`@import` 指向不存在的文件）Trunk 会报错，
/// 但那时报的是一句路径错误，不如这里直接点名。
fn check_modern_kit_imports(root: &Path) -> bool {
  let Ok(entry) = std::fs::read_to_string(root.join("crates/app/style/modern/kit.css")) else {
    println!("✗ 现代风 kit 分片导入：读不到 style/modern/kit.css（改名或搬家了？）");
    return false;
  };
  let Ok(dir) = std::fs::read_dir(root.join("crates/app/style/modern/kit")) else {
    println!("✗ 现代风 kit 分片导入：读不到 style/modern/kit/");
    return false;
  };
  let on_disk: Vec<String> = dir
    .filter_map(Result::ok)
    .map(|e| e.file_name().to_string_lossy().into_owned())
    .filter(|n| n.ends_with(".css"))
    .collect();
  let (imported, not_imported, missing_file) = diff_kit_imports(&entry, on_disk.clone());

  if not_imported.is_empty() && missing_file.is_empty() {
    println!(
      "✓ 现代风 kit 分片导入：{} 个分片与 `@import` 一一对应",
      imported.len()
    );
    return true;
  }
  println!(
    "✗ 现代风 kit 分片导入：目录 {} 个、`@import` {} 个",
    on_disk.len(),
    imported.len()
  );
  for name in &not_imported {
    println!("    {name} 在 kit/ 里但没有 `@import`（样式不会进产物）");
  }
  for name in &missing_file {
    println!("    {name} 被 `@import` 但文件不存在（构建会报错）");
  }
  println!("    → kit.css 只放约定与 `@import`，分片文件与导入列表一一对应，顺序即级联");
  false
}

/// 阅读正文规范：`docs/ui/modern.md` §2.3「正文分两档」。
///
/// 这条规范的全部价值在两个关系上：字号随视口**连续**变化、每行字数**恒定**。
/// 两者都只靠 CSS 里的两行声明撑着（`--md-text-prose` 的 `clamp(…vw…)` 与 `--md-measure` 的 `em`）。
/// 把字号改回固定档、或把 measure 换成 `px` / `ch`，页面照常能看、文档照常写着规范 ——
/// 只有这里会发现两者对不上。值本身也要与文档表格逐字对齐：规范文档是唯一说明「为什么是 15~17.5」的地方。
fn check_prose_type_scale(root: &Path) -> bool {
  static PROSE_DECL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"--md-text-prose:\s*([^;]+);").expect("token 正则应能编译"));
  static MEASURE_DECL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"--md-measure:\s*([^;]+);").expect("token 正则应能编译"));
  static ARTICLE_PAGE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\.md-article-page\s*\{([^}]*)\}").expect("容器正则应能编译"));
  // 只认「数字 + em」：`36rem` 也以 `em` 结尾，但它按根字号算，不随正文缩放。
  static EM_ONLY: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[0-9.]+em$").expect("单位正则应能编译"));

  let (Ok(tokens_raw), Some(kit_raw), Ok(doc)) = (
    std::fs::read_to_string(root.join("crates/app/style/modern/tokens.css")),
    read_modern_kit(root),
    std::fs::read_to_string(root.join("docs/ui/modern.md")),
  ) else {
    println!("✗ 阅读正文规范：缺 tokens.css / kit.css / docs/ui/modern.md（改名或搬家了？）");
    return false;
  };
  let tokens = strip_css_comments(&tokens_raw);
  let kit = strip_css_comments(&kit_raw);
  let mut problems: Vec<String> = Vec::new();

  let prose = PROSE_DECL.captures(&tokens).map(|c| c[1].trim().to_owned());
  match &prose {
    Some(v) if v.contains("clamp(") && v.contains("vw") => {}
    Some(v) => problems.push(format!(
      "`--md-text-prose` 是 `{v}`：阅读正文必须用 `clamp(…vw…)`，断点跳变会让整段换行位置跟着跳"
    )),
    None => problems.push("`tokens.css` 里没有 `--md-text-prose`".to_owned()),
  }

  match MEASURE_DECL
    .captures(&tokens)
    .map(|c| c[1].trim().to_owned())
  {
    Some(v) if EM_ONLY.is_match(&v) => {}
    Some(v) => problems.push(format!(
      "`--md-measure` 是 `{v}`：行宽必须按 `em` 给（随字号缩放），`px` 会让每行字数跟着视口涨"
    )),
    None => problems.push("`tokens.css` 里没有 `--md-measure`".to_owned()),
  }

  let uses_prose = ARTICLE_PAGE.captures_iter(&kit).any(|c| {
    let body = c[1].to_ascii_lowercase();
    body.contains("font-size: var(--md-text-prose)")
  });
  if !uses_prose {
    problems.push(
      "`.md-article-page` 没有落 `font-size: var(--md-text-prose)`：阅读页回到了固定字号"
        .to_owned(),
    );
  }
  if !kit.contains("var(--md-measure)") {
    problems.push("`kit.css` 里没有用 `var(--md-measure)`：成段文本没有行宽上限".to_owned());
  }

  // 文档要写出与人眼比对的同一份值（表格里是 `clamp(15px, 0.36vw + 13.4px, 17.5px)`）。
  let squash = |s: &str| s.chars().filter(|c| !c.is_whitespace()).collect::<String>();
  for token in ["--md-text-prose", "--md-measure"] {
    if !doc.contains(token) {
      problems.push(format!("`docs/ui/modern.md` 没写 `{token}`：规范没有落纸"));
    }
  }
  if let Some(v) = &prose
    && !squash(&doc).contains(&squash(v))
  {
    problems.push(format!(
      "文档里的 `--md-text-prose` 值与 `tokens.css` 不一致（CSS 是 `{v}`）"
    ));
  }

  if problems.is_empty() {
    println!("✓ 阅读正文规范：字号随视口流体、行宽按字数恒定，且与 `docs/ui/modern.md` §2.3 一致");
    return true;
  }
  println!("✗ 阅读正文规范：{} 处不符", problems.len());
  for p in problems.iter().take(MAX_REPORT) {
    println!("    {p}");
  }
  println!(
    "    → 规范见 docs/ui/modern.md §2.3「正文分两档」；实现落点是 .md-article-page 与 :is(p, li, dd, blockquote)"
  );
  false
}

/// 现代风类名索引：`kit.css` 里声明的每个 `md-*` 类都必须在 `docs/ui/modern-classes.md` 登记。
///
/// 为什么需要：类名只增不减是这类设计系统最常见的腐化方式（本轮就清掉过
/// `.md-toggle` 与 `.md-switch` 并存、`.md-field` 被定义两遍）。有了索引，
/// 「这个类还有没有人用」变成一次搜索，而不是通读 8000 行 CSS。
fn check_modern_class_index(root: &Path) -> bool {
  static CSS_CLASS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\.(md-[a-z0-9-]+)").expect("类名正则应能编译"));
  static DOC_CLASS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"`(md-[a-z0-9-]+)`\(\d+\)").expect("索引正则应能编译"));
  if !require_dir(root, "crates/app/style/modern", "现代风类名索引") {
    return false;
  }
  let doc_path = root.join("docs/ui/modern-classes.md");
  // 缺文档不是「没有违规」，而是这条规则没法执行：类名索引是「这个类还有
  // 没有人用」的唯一入口，静默跳过等于让 CSS 重新变成只增不减。
  let Ok(doc) = std::fs::read_to_string(&doc_path) else {
    println!("✗ 现代风类名索引：缺 `docs/ui/modern-classes.md`（改名了？）");
    return false;
  };
  // 扫整个 `style/modern/`：动效层（`interaction.css`）里也有 `md-ping` / `md-sonar` 这类类，
  // 只扫两份文件的话它们永远不在索引里，「索引是唯一入口」就是句空话。
  let mut body = String::new();
  let mut files = 0usize;
  for entry in WalkDir::new(root.join("crates/app/style/modern"))
    .sort_by_file_name()
    .into_iter()
    .filter_map(Result::ok)
  {
    let path = entry.path();
    if !entry.file_type().is_file() || path.extension().is_none_or(|e| e != "css") {
      continue;
    }
    let Ok(text) = std::fs::read_to_string(path) else {
      continue;
    };
    files += 1;
    body.push_str(&strip_css_comments(&text));
    body.push('\n');
  }
  if !scanned_ok(files, "现代风类名索引") {
    return false;
  }
  let declared: std::collections::BTreeSet<String> = CSS_CLASS
    .captures_iter(&body)
    .map(|c| c[1].to_string())
    .collect();
  // 精确 token 比对：`doc.contains("md-card")` 会被已登记的 `md-card-title` 顺带放过，
  // 前缀类名（`md-btn` / `md-bar`）从此永远「已登记」。索引里的类名都写成 `md-xxx`(规则数) 的形态；只认这一形态，才不会把第一列的「组名」（`md-card` 这类前缀，本身不是类）当成类。
  let documented: std::collections::BTreeSet<String> = DOC_CLASS
    .captures_iter(&doc)
    .map(|c| c[1].to_string())
    .collect();
  let (missing, dead) = class_index_diff(&declared, &documented);
  if missing.is_empty() && dead.is_empty() {
    println!(
      "✓ 现代风类名索引：{} 个 `md-*` 类与索引一一对应",
      declared.len()
    );
    return true;
  }
  if !missing.is_empty() {
    println!(
      "✗ 现代风类名索引：{} 个类在 CSS 里声明但没登记",
      missing.len()
    );
    for c in missing.iter().take(MAX_REPORT) {
      println!("    .{c}");
    }
  }
  if !dead.is_empty() {
    println!(
      "✗ 现代风类名索引：{} 个类登记了但 CSS 里已经没有（死条目）",
      dead.len()
    );
    for c in dead.iter().take(MAX_REPORT) {
      println!("    {c}");
    }
  }
  println!(
    "    → 在 docs/ui/modern-classes.md 里登记新类、删掉已不存在的条目；索引是「这个类还有没有人用」的唯一入口"
  );
  false
}

/// 索引与 CSS 的差集：(CSS 里有、文档里没有，文档里有、CSS 里没有)。
fn class_index_diff(
  declared: &std::collections::BTreeSet<String>,
  documented: &std::collections::BTreeSet<String>,
) -> (Vec<String>, Vec<String>) {
  (
    declared.difference(documented).cloned().collect(),
    documented.difference(declared).cloned().collect(),
  )
}

/// 文档内链完整性：仓库内指向 `.md` 的相对链接必须真实存在。
///
/// 为什么进门禁：这轮把 UI 文档拆成「现代 / 像素」两份，改名会让十几处交叉引用同时失效，
/// 而失效的文档链接不会让构建报错 —— 只有下一个人点进去才发现是死链。
///
/// 覆盖 `docs/**`（递归）、根 `README.md` / `AGENTS.md` 与各 crate 的 `src/README.md`：
/// 入口文档才是最常被点的那几份，漏掉它等于没查。
fn check_doc_links(root: &Path) -> bool {
  static LINK: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\]\(([^)\s]+\.md)").expect("文档链接正则应能编译"));
  let mut report: Vec<String> = Vec::new();
  let mut targets: Vec<PathBuf> = Vec::new();

  for rel in ["README.md", "AGENTS.md"] {
    let path = root.join(rel);
    if path.is_file() {
      targets.push(path);
    }
  }
  for entry in WalkDir::new(root.join("docs"))
    .sort_by_file_name()
    .into_iter()
    .filter_map(Result::ok)
  {
    if entry.file_type().is_file() && entry.path().extension().is_some_and(|x| x == "md") {
      targets.push(entry.path().to_path_buf());
    }
  }
  if let Ok(entries) = std::fs::read_dir(root.join("crates")) {
    for c in entries.flatten() {
      let readme = c.path().join("src").join("README.md");
      if readme.is_file() {
        targets.push(readme);
      }
    }
  }
  if targets.is_empty() {
    println!("✗ 文档内链：一份 Markdown 都没找到（路径改了？）");
    return false;
  }

  let mut docs_text: Vec<(String, String)> = Vec::new();
  for path in targets {
    let Ok(text) = std::fs::read_to_string(&path) else {
      continue;
    };
    let rel = path
      .strip_prefix(root)
      .unwrap_or(&path)
      .to_string_lossy()
      .replace('\\', "/");
    let dir = path.parent().unwrap_or(root);
    docs_text.push((rel.clone(), text.clone()));
    for cap in LINK.captures_iter(&text) {
      let link = &cap[1];
      // 判 `://` 而不是 `starts_with("http")`：后者会把 `http-notes.md` 这类相对链接也跳过。
      if link.contains("://") || link.starts_with('#') {
        continue;
      }
      let target = if link.starts_with('/') {
        root.join(link.trim_start_matches('/'))
      } else {
        dir.join(link)
      };
      if !normalize_join(&target).exists() {
        report.push(format!("    {rel} → {link}"));
      }
    }
  }

  report.extend(missing_sections(root, &docs_text));
  if report.is_empty() {
    println!("✓ 文档内链：所有相对链接与 `§N` 小节引用都能解析");
    return true;
  }
  println!("✗ 文档内链：{} 处指向不存在的文件或小节", report.len());
  for line in report.iter().take(MAX_REPORT) {
    println!("{line}");
  }
  println!("    → 改名 / 拆分文档后同步更新引用；文档死链不会让构建失败，只会让下一个人踩空");
  false
}

/// 某份 Markdown 里所有「编号小节」的编号：`## 4. 标题` / `### 15.36 标题` → `4` / `15.36`。
fn section_numbers(markdown: &str) -> std::collections::BTreeSet<String> {
  static HEADING: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)^#{2,6}\s+(\d+(?:\.\d+)*)\.?(?:\s|$)").expect("小节标题正则应能编译")
  });
  HEADING
    .captures_iter(markdown)
    .map(|c| c[1].to_string())
    .collect()
}

/// 校验「`docs/xxx.md` §N」形式的引用：被引用的文件里必须真有 `N` 这一节。
///
/// 只认紧挨着文件名的那一个 `§`（`docs/a.md` §2、`[`docs/a.md`](docs/a.md) §2`），
/// 不解析「§11.2 / §11.3」里斜杠后面的第二个 —— 宁可少查，也不要把写法各异的行文误报成死链。
/// 它只能证明「这一节存在」，证明不了「这一节讲的就是你要的」（那要靠人读）。
fn missing_sections(root: &Path, docs: &[(String, String)]) -> Vec<String> {
  let mut cache: std::collections::BTreeMap<String, std::collections::BTreeSet<String>> =
    std::collections::BTreeMap::new();
  let mut out = Vec::new();
  for (rel, text) in docs {
    for (target, number) in section_refs(text) {
      let sections = cache.entry(target.clone()).or_insert_with(|| {
        std::fs::read_to_string(root.join(&target))
          .map(|t| section_numbers(&t))
          .unwrap_or_default()
      });
      if !sections.contains(&number) {
        out.push(format!(
          "    {rel} → {target} §{number}（该文件里没有这一节）"
        ));
      }
    }
  }
  out
}

/// 文本里所有「`docs/…md` §N」形式的引用：`(文件路径, 编号)`。
///
/// 路径允许子目录（`docs/ui/modern.md`）：`docs/` 按读者问题分了目录，正则若只认平铺的文件名，
/// 移进子目录的文档就整体逃出了 `§N` 校验 —— 而且是静默的（不匹配 = 不检查 = 不报错）。
fn section_refs(text: &str) -> Vec<(String, String)> {
  static REF: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(docs/[A-Za-z0-9_/-]+\.md)`?(?:\]\([^)]*\))?\s*§\s*(\d+(?:\.\d+)*)")
      .expect("小节引用正则应能编译")
  });
  REF
    .captures_iter(text)
    .map(|c| (c[1].to_owned(), c[2].to_owned()))
    .collect()
}

/// 路径归一化：消掉 `.` 与 `..`。
///
/// 用 `Path::components()` 而不是「按 `/` 切字符串、再拼一个 `/` 开头的绝对路径」：
/// 后者依赖 root 是绝对路径且分隔符是 `/`，Windows 下会把每条链接都判成死链。
fn normalize_join(path: &Path) -> PathBuf {
  let mut out = PathBuf::new();
  for comp in path.components() {
    match comp {
      Component::CurDir => {}
      Component::ParentDir => {
        out.pop();
      }
      other => out.push(other.as_os_str()),
    }
  }
  out
}

/// 现代风样式里「指向像素风 DOM」的选择器必须带 `html[data-ui-style="modern"]`。
///
/// 为什么单独一条：`data-sig` 在像素风下也会写到 `#route-content`，而地图标记与桥接页卡片
/// 今天仍是像素风 DOM。少一层门禁，像素风就会播现代风的动效 —— `interaction.css` 最初漏了
/// lab 与 map 两族（像素页凭空多出蓝色扫描带、现代蓝描边与平滑过渡），而
/// `check_modern_isolation` 只认 `pxl-`，抓不到 `bg-card` 这种跨风格的调色板工具类。
/// 选择器列表（逗号分隔）逐项判定，见 [`split_selector_list`]。
fn check_modern_css_is_gated(root: &Path) -> bool {
  if !require_dir(root, "crates/app/style/modern", "现代风样式门禁") {
    return false;
  }
  let dir = root.join("crates/app/style/modern");
  let mut files = 0usize;
  let mut report: Vec<String> = Vec::new();
  for entry in WalkDir::new(&dir)
    .sort_by_file_name()
    .into_iter()
    .filter_map(Result::ok)
  {
    let path = entry.path();
    if !entry.file_type().is_file() || path.extension().is_none_or(|e| e != "css") {
      continue;
    }
    let rel = path
      .strip_prefix(root)
      .unwrap_or(path)
      .to_string_lossy()
      .replace('\\', "/");
    let Ok(text) = std::fs::read_to_string(path) else {
      continue;
    };
    files += 1;
    for (line, selector) in selectors(&strip_css_comments(&text), needs_style_gate) {
      report.push(format!("    {rel}:{line}  {selector}"));
    }
  }

  if !scanned_ok(files, "现代风样式门禁") {
    return false;
  }
  if report.is_empty() {
    println!("✓ 现代风样式门禁：指向像素风 DOM 的选择器都带了 `html[data-ui-style=\"modern\"]`");
    return true;
  }
  println!(
    "✗ 现代风样式门禁：{} 处选择器会命中像素风 DOM 却没挂风格门禁",
    report.len()
  );
  for line in report.iter().take(MAX_REPORT) {
    println!("{line}");
  }
  println!(
    "    → 选择器提到 `data-sig` / `pxl-*` / `bg-card` 这类像素风标识时，必须以 `html[data-ui-style=\"modern\"]` 开头（像素风也会写 `data-sig`）"
  );
  false
}

/// C7.8：带 `data-modal-root=""` 的弹层必须同时装齐焦点 / 滚动锁 / Esc。
///
/// 为什么查「装配」而不是「行为」：这三件事都已在共享模块里实现，唯一会出错的地方就是
/// **忘了调**。现代风答题卡抽屉曾漏掉滚动锁与 Esc（抽屉开着时背景能滚、键盘关不掉），
/// 而所有静态规则都是绿的 —— 漏掉的东西根本不在代码里，只有这一条能拦住。
///
/// 判定用字面量 `data-modal-root=""`（属性写法）而不是常量名：`focus.rs` 里那个
/// `pub const MODAL_ROOT` 也是同样的字符串，会把定义处误判成弹层。
fn check_modal_parity(root: &Path) -> bool {
  const NEEDED: [(&str, &str); 3] = [
    ("焦点（`focus::use_modal_focus`）", "use_modal_focus"),
    ("滚动锁（`overlay::use_scroll_lock`）", "use_scroll_lock"),
    ("Esc（`focus::use_escape_close`）", "use_escape_close"),
  ];
  if !require_dir(root, "crates/app/src", "模态三件套") {
    return false;
  }
  let mut files = 0usize;
  let mut report: Vec<String> = Vec::new();
  for entry in WalkDir::new(root.join("crates/app/src"))
    .sort_by_file_name()
    .into_iter()
    .filter_map(Result::ok)
  {
    let path = entry.path();
    if !entry.file_type().is_file() || path.extension().is_none_or(|e| e != "rs") {
      continue;
    }
    let rel = path
      .strip_prefix(root)
      .unwrap_or(path)
      .to_string_lossy()
      .replace('\\', "/");
    let Ok(text) = std::fs::read_to_string(path) else {
      continue;
    };
    let code = text.lines().map(code_part).collect::<Vec<_>>().join("\n");
    if !code.contains("data-modal-root=\"\"") {
      continue;
    }
    files += 1;
    let missing: Vec<&str> = NEEDED
      .iter()
      .filter(|(_, needle)| !code.contains(needle))
      .map(|(label, _)| *label)
      .collect();
    if !missing.is_empty() {
      report.push(format!("    {rel}: 缺 {}", missing.join("、")));
    }
  }

  if !scanned_ok(files, "模态三件套") {
    return false;
  }
  if report.is_empty() {
    println!("✓ 模态三件套：{files} 个弹层都装了焦点 / 滚动锁 / Esc");
    return true;
  }
  println!("✗ 模态三件套：{} 个弹层缺件", report.len());
  for line in &report {
    println!("{line}");
  }
  println!(
    "    → 每个 `role=\"dialog\" aria-modal=\"true\"` 的弹层都要调 `crate::focus::use_modal_focus`、`crate::overlay::use_scroll_lock`、`crate::focus::use_escape_close`（AGENTS.md 硬规则）"
  );
  false
}

/// 一行代码里是不是在声明一个「贴底固定层」。
///
/// 两种写法：Tailwind 工具类同时带 `fixed` 与 `bottom-0`（像素风 `BottomBar` / `MobileTabs`、
/// 全站横幅），或现代风的 `md-bar` / `md-dock` 类（它们的 `position: fixed` 写在 `kit.css` 里）。
/// 按「类名整词」判，`md-bar-inner` / `xl:bottom-0` / `bottom-20` 都不算。
fn declares_fixed_bottom(line: &str) -> bool {
  let tokens: Vec<&str> = line
    .split(|c: char| c.is_whitespace() || c == '"' || c == '\'')
    .collect();
  let has = |t: &str| tokens.contains(&t);
  (has("fixed") && has("bottom-0")) || has("md-bar") || has("md-dock")
}

/// 贴底固定层必须带 `data-fixed-bottom`：自绘窗口滚动条（`ui::ScrollBar`）靠它量出底栏占位，
/// 少一条标记，滚动条就会一路画进那条底栏里。
///
/// 为什么按文件数而不是逐行配对：`class` 与属性通常写在不同行（rustfmt 会把长属性拆开），
/// 一个文件里「声明了几处贴底层」与「标了几处」对得上就够了；漏标的那一处恰好让两个数不等。
fn check_fixed_bottom_marker(root: &Path) -> bool {
  const MARKER: &str = "data-fixed-bottom";
  if !require_dir(root, "crates/app/src", "贴底固定层标记") {
    return false;
  }
  let mut files = 0usize;
  let mut layers = 0usize;
  let mut report: Vec<String> = Vec::new();
  for entry in WalkDir::new(root.join("crates/app/src"))
    .sort_by_file_name()
    .into_iter()
    .filter_map(Result::ok)
  {
    let path = entry.path();
    if !entry.file_type().is_file() || path.extension().is_none_or(|e| e != "rs") {
      continue;
    }
    let rel = path
      .strip_prefix(root)
      .unwrap_or(path)
      .to_string_lossy()
      .replace('\\', "/");
    let Ok(text) = std::fs::read_to_string(path) else {
      continue;
    };
    files += 1;
    let code: Vec<&str> = text.lines().map(code_part).collect();
    let declared = code.iter().filter(|l| declares_fixed_bottom(l)).count();
    if declared == 0 {
      continue;
    }
    layers += declared;
    let marked = code
      .iter()
      .map(|l| l.matches(MARKER).count())
      .sum::<usize>();
    if marked < declared {
      report.push(format!(
        "    {rel}: 声明了 {declared} 处贴底固定层，只标了 {marked} 处 `{MARKER}`"
      ));
    }
  }

  if !scanned_ok(files, "贴底固定层标记") {
    return false;
  }
  if layers == 0 {
    // 一处都没认出来 = 识别规则失效（类名改了），不能当成「全部合规」。
    println!(
      "✗ 贴底固定层标记：一处贴底固定层都没认出来（类名改了？同步改 `declares_fixed_bottom`）"
    );
    return false;
  }
  if report.is_empty() {
    println!("✓ 贴底固定层标记：{layers} 处贴底固定层都带了 `{MARKER}`");
    return true;
  }
  println!("✗ 贴底固定层标记：{} 个文件漏标", report.len());
  for line in &report {
    println!("{line}");
  }
  println!(
    "    → 新增 `fixed` 贴底的元素必须加 `{MARKER}`，否则窗口滚动条会画进它里面（docs/ui/modern.md §3、docs/ui/pixel.md 滚动模型）"
  );
  false
}

/// 跑全部守卫：全部 0 违规返回 `Ok`，否则打印证据并 `bail`。
pub fn check(root: &Path) -> Result<()> {
  println!(
    "结构与架构守卫（{} 条逐行规则 + 结构检查，见 docs/engineering/code-quality.md）",
    RULES.len()
  );
  let mut failed: Vec<String> = Vec::new();

  println!("\n── 分层与依赖 ──");
  if !check_no_reverse_deps(root) {
    failed.push("crate 依赖方向".to_owned());
  }

  println!("\n── 组件结构 ──");
  if !check_one_component_per_file(root) {
    failed.push("一文件一组件".to_owned());
  }

  println!("\n── 设计系统一致性 ──");
  if !check_modal_parity(root) {
    failed.push("模态三件套".to_owned());
  }
  if !check_fixed_bottom_marker(root) {
    failed.push("贴底固定层标记".to_owned());
  }
  if !check_modern_isolation(root) {
    failed.push("双设计系统隔离".to_owned());
  }
  if !check_pixel_isolation(root) {
    failed.push("反向隔离（像素文件里的现代类名）".to_owned());
  }
  if !check_pixel_inset_blocks(root) {
    failed.push("像素式内嵌块".to_owned());
  }
  if !check_eager_style_call(root) {
    failed.push("风格读取闭包被立即调用".to_owned());
  }
  if !check_modern_css_is_gated(root) {
    failed.push("现代风样式门禁".to_owned());
  }
  if !check_unlayered_modern_css(root) {
    failed.push("层外样式门禁".to_owned());
  }
  if !check_important_marker(root) {
    failed.push("`!important` 标注".to_owned());
  }
  if !check_modern_has_marker(root) {
    failed.push("`:has()` 降级声明".to_owned());
  }
  if !check_modern_state_colors(root) {
    failed.push("状态色字面值".to_owned());
  }
  if !check_exit_durations(root) {
    failed.push("退场时长走 token".to_owned());
  }
  if !check_single_presence(root) {
    failed.push("延迟卸载只有一份".to_owned());
  }
  if !check_modern_kit_imports(root) {
    failed.push("现代风 kit 分片导入".to_owned());
  }
  if !check_modern_class_index(root) {
    failed.push("现代风类名索引".to_owned());
  }
  if !check_modern_breakpoints(root) {
    failed.push("现代风断点".to_owned());
  }
  if !check_prose_type_scale(root) {
    failed.push("阅读正文规范".to_owned());
  }

  println!("\n── 数据与文案 ──");
  if !check_knowledge_dict_is_loaded(root) {
    failed.push("知识库译文词典挂载".to_owned());
  }
  if !check_doc_links(root) {
    failed.push("文档内链".to_owned());
  }

  println!("\n── 逐行规则 ──");
  failed.extend(run_line_rules(root));

  if !failed.is_empty() {
    bail!("结构与架构守卫未通过：{}", failed.join("、"));
  }
  Ok(())
}

#[cfg(test)]
mod tests {
  #[test]
  fn exit_animations_must_use_tokens() {
    let css = "\
.a { animation: md-fade-out 180ms ease forwards; }
.b {
  animation: md-modal-out var(--md-dur) var(--md-ease) forwards;
  animation: pop-out 0.24s steps(3, end) both;
  animation: fade-out-only var(--motion-normal) steps(3, end) both;
}
.c { animation: md-vt-out 180ms ease both; /* dur-ok: 路由过渡 */ }
.d { animation: md-ring-out 4.2s ease infinite; }
.e { animation: md-fade-in 160ms ease both; }
/* animation: md-x-out 90ms; */
";
    // 行 1 写死 180ms（和选择器同一行也要识别）；行 3 / 5 用 token（过）；行 4 的 0.24s 写死（秒也算）；
    // 行 7 有 dur-ok 豁免；行 8 无限循环不算退场；行 9 是入场；行 10 在注释里。
    let hits = hardcoded_exit_durations(css);
    assert_eq!(hits, vec![(1, "180ms".to_owned()), (4, "0.24s".to_owned())]);
  }

  #[test]
  fn only_presence_may_define_exit_ms() {
    let rs = "\
const EXIT_MS: u64 = 200;
pub const TOAST_EXIT_MS: u64 = 180;
pub(crate) const MENU_EXIT_MS: u64 = 1;
// const COMMENTED_EXIT_MS: u64 = 5;
let exit_ms = 3;
const FRAME_MS: u64 = 16;
";
    assert_eq!(exit_ms_consts(rs), vec![1, 2, 3]);
  }

  #[test]
  fn has_selector_needs_a_same_line_marker() {
    let css = "\
.a:has(.b) { color: red; }
.c:has(.d) { color: red; } /* has-ok: 装饰 */
/* .e:has(.f) 只是注释里提到 */
@supports not selector(:has(*)) {
  .g:focus-within { outline: 0; }
}
.h { color: blue; }
.i:has(.j),
.k:has(.l) { /* has-ok: 装饰 */
}
";
    // 第 1 行没标注；第 2 行有；注释和 `selector(:has(*))` 特性检测都不算在用；
    // 多行选择器的**每一行**都要标（第 8 行 `.i:has(.j),` 没标，第 9 行标了）。
    assert_eq!(unmarked_has_lines(css), vec![1, 8]);
  }

  #[test]
  fn warm_literals_catch_pink_gold_and_orange_only() {
    // 要拦的：按深色底调的浅粉红 / 亮金 / 品牌橙，含 3 位简写和 8 位带透明度
    for hex in [
      "#ff7a7a",
      "#ffb02e",
      "#ffcf6b",
      "#ff6a35",
      "#d93b3b",
      "#f55",
      "#ff5d5d99",
    ] {
      let line = format!("color: {hex};");
      assert_eq!(
        warm_hex_literals(&line).len(),
        1,
        "{hex} 应被识别为红 / 琥珀色"
      );
    }
    // 不拦的：蓝 / 绿 / 紫、黑白灰、深棕墨水字（明度低）、id 片段
    for hex in [
      "#2540ff", "#35e39f", "#a97bff", "#000", "#fff", "#888888", "#21120a", "#2a1206",
    ] {
      let line = format!("color: {hex};");
      assert!(warm_hex_literals(&line).is_empty(), "{hex} 不该被拦");
    }
    assert!(warm_hex_literals("fill: url(#md-moon);").is_empty());
    // 一行多个：只数暖色
    assert_eq!(
      warm_hex_literals("linear-gradient(#ff6a35, #a97bff, #ff4d2e)").len(),
      2
    );
  }

  #[test]
  fn warm_literals_respect_color_ok_and_comments() {
    let css = "\
.a { color: #ff7a7a; }
.b { color: #ff7a7a; } /* color-ok: 品牌 */
/* .c { color: #ff7a7a; } */
.d { color: var(--md-danger-ink); }
";
    assert_eq!(
      unmarked_warm_color_lines(css),
      vec![(1, "#ff7a7a".to_owned())]
    );
  }

  use super::*;

  #[test]
  fn kit_imports_must_match_the_directory_both_ways() {
    let entry = "@import \"./kit/01-a.css\";\n@import \"./kit/02-b.css\";\n/* @import \"x\" */";
    let names = |v: &[&str]| v.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>();

    let (imported, extra, missing) = diff_kit_imports(entry, names(&["02-b.css", "01-a.css"]));
    assert_eq!(imported, names(&["01-a.css", "02-b.css"]));
    assert!(extra.is_empty() && missing.is_empty());

    // 目录里多一个、忘了 `@import`：样式不会进产物。
    let (_, extra, missing) = diff_kit_imports(entry, names(&["01-a.css", "02-b.css", "03-c.css"]));
    assert_eq!(extra, names(&["03-c.css"]));
    assert!(missing.is_empty());

    // `@import` 指向不存在的文件。
    let (_, extra, missing) = diff_kit_imports(entry, names(&["01-a.css"]));
    assert!(extra.is_empty());
    assert_eq!(missing, names(&["02-b.css"]));
  }

  #[test]
  fn inset_blocks_are_caught_in_shared_files_in_any_order_and_across_lines() {
    let bad = [
      "use crate::kit::tile_class;",
      "view! { <div class=\"flex border bg-inset p-3\"></div> }",
      "view! { <div class=\"bg-inset px-2 border\"></div> }",
      "view! { <div class=\"flex",
      "  border bg-inset\"></div> }",
    ];
    let hits = unconditional_inset_blocks(&bad);
    assert_eq!(hits.iter().map(|h| h.0).collect::<Vec<_>>(), vec![2, 3, 4]);
    assert!(hits[2].1.contains("border bg-inset"), "{hits:?}");
  }

  #[test]
  fn inset_blocks_skip_dispatched_pixel_only_and_lookalike_classes() {
    // 按皮肤派发：else 分支的字面量不带 `class="` 前缀。
    let dispatched = [
      "use crate::kit::is_modern_at_mount;",
      "class=if is_modern_at_mount() { \"md-tile\" } else { \"border bg-inset p-3\" }",
    ];
    assert!(unconditional_inset_blocks(&dispatched).is_empty());
    // 像素专用文件：没有任何共享信号，现代风渲染不到。
    let pixel_only = ["view! { <div class=\"border bg-inset p-3 pxl-card\"></div> }"];
    assert!(unconditional_inset_blocks(&pixel_only).is_empty());
    // 只含 `border-t` / `bg-inset-foo` 这类相近类名，不是同名类。
    let lookalike = [
      "use crate::ui::Button;",
      "view! { <div class=\"border-t bg-inset-x\"></div> }",
    ];
    assert!(unconditional_inset_blocks(&lookalike).is_empty());
    // 只用共享控件、不引用 `kit::` 的页面同样算共享（M6 的漏判方向）。
    let ui_only = [
      "use crate::ui::Button;",
      "view! { <div class=\"border bg-inset\"></div> }",
    ];
    assert_eq!(unconditional_inset_blocks(&ui_only).len(), 1);
  }

  #[test]
  fn selector_lists_are_split_at_top_level_commas_only() {
    assert_eq!(split_selector_list("a, b ,c"), vec!["a", "b", "c"]);
    assert_eq!(
      split_selector_list(":is(a, b) .x, .y"),
      vec![":is(a, b) .x", ".y"]
    );
    assert_eq!(
      split_selector_list("[data-x=\"a,b\"], .z"),
      vec!["[data-x=\"a,b\"]", ".z"]
    );
    assert_eq!(split_selector_list(".only"), vec![".only"]);
    assert!(split_selector_list(" , ").is_empty());
  }

  #[test]
  fn style_gate_is_judged_per_list_item() {
    // 反例（以前漏报）：第一项带 `md-`、第二项会命中像素 DOM，整条按字符串判会被放过。
    assert!(needs_style_gate(
      "#route-content[data-sig=\"x\"] .md-a, #route-content[data-sig=\"x\"] section"
    ));
    // 每一项都带 `md-` 或门禁：不需要再加。
    assert!(!needs_style_gate(
      "#route-content[data-sig=\"x\"] .md-a, #route-content[data-sig=\"x\"] .md-b"
    ));
    assert!(!needs_style_gate(
      "html[data-ui-style=\"modern\"] .pxl-x, html[data-ui-style=\"modern\"] .bg-card"
    ));
    assert!(needs_style_gate(".bg-card, .md-x"));
  }

  #[test]
  fn layer_statements_do_not_swallow_the_next_rule() {
    // `@layer a, b;` 没有块：只丢这一句，后面的规则（这里是一条无门禁的层外规则）必须还在。
    let out = strip_layer_blocks("@layer a, b;\n.x { color: red; }\n@layer c { .y {} }\n.z {}");
    assert!(out.contains(".x"), "{out}");
    assert!(out.contains(".z"), "{out}");
    assert!(!out.contains(".y"), "{out}");
    assert!(!out.contains("@layer a"), "{out}");
    // 没有任何 `{` / `;` 的残缺输入不死循环、不 panic。
    assert_eq!(strip_layer_blocks("@layer"), "");
  }

  #[test]
  fn breakpoint_scan_handles_decimals_units_and_ranges() {
    let css = "@media (max-width: 899.98px) {}\n@media (min-width: 56em) {}\n\
               @media (width <= 899px) {}\n@media (min-width: 640px) and (max-width: 900px) {}\n\
               @media (min-width: 40rem) {}\n@container (max-width: 520px) {}\n\
               @media (min-width: 768px) {}";
    let off = off_list_breakpoints(css);
    let lines: Vec<usize> = off.iter().map(|(l, _)| *l).collect();
    // 899.98px / 56em(=896px) / `width <= 899px` 违规；640 / 900 / 40rem(=640px) / 768 在白名单；
    // `@container` 不是视口断点。
    assert_eq!(lines, vec![1, 2, 3], "{off:?}");
    assert!((off[1].1 - 896.0).abs() < f64::EPSILON);
  }

  #[test]
  fn pixel_only_files_are_found_even_when_they_mention_md() {
    // 反例（以前空转）：像素专用文件里恰好写了 `md-xxx`，这正是要抓的违规。
    let bad = ["let c = \"pxl-card md-setting-title\";"];
    assert_eq!(md_class_lines_in_pixel_only(&bad).len(), 1);
    // 双风格文件（有风格派发）里出现 `md-` 是正常的。
    let dual = ["if crate::kit::is_modern_at_mount() { \"md-x\" } else { \"pxl-x\" }"];
    assert!(md_class_lines_in_pixel_only(&dual).is_empty());
    // 没有任何 `pxl-` 的文件不是像素专用。
    assert!(md_class_lines_in_pixel_only(&["let c = \"md-x\";"]).is_empty());
  }

  #[test]
  fn class_index_is_matched_exactly_in_both_directions() {
    let set = |xs: &[&str]| xs.iter().map(|x| (*x).to_owned()).collect();
    // 前缀类名不会被更长的已登记类顺带放过。
    let (missing, dead) = class_index_diff(
      &set(&["md-card", "md-card-title"]),
      &set(&["md-card-title"]),
    );
    assert_eq!(missing, vec!["md-card".to_owned()]);
    assert!(dead.is_empty());
    // 文档里有、CSS 里已删：死条目。
    let (missing, dead) = class_index_diff(&set(&["md-a"]), &set(&["md-a", "md-gone"]));
    assert!(missing.is_empty());
    assert_eq!(dead, vec!["md-gone".to_owned()]);
  }

  #[test]
  fn section_refs_follow_subdirectories() {
    let text = "见 `docs/ui/modern.md` §2.1、[`docs/ui/decisions.md`](x) §11.4，\
                以及 docs/a.md §3。没有编号的 docs/ui/pixel.md 不算。";
    assert_eq!(
      section_refs(text),
      vec![
        ("docs/ui/modern.md".to_owned(), "2.1".to_owned()),
        ("docs/ui/decisions.md".to_owned(), "11.4".to_owned()),
        ("docs/a.md".to_owned(), "3".to_owned()),
      ],
      "子目录里的文档也要被 §N 校验认出来，否则移进子目录就等于逃过了检查"
    );
  }

  #[test]
  fn section_numbers_and_references() {
    let doc = "# 标题\n## 1. 概览\n### 3.2 子节\n### 15.36 复审\n## 附录\n正文 ## 9. 不是标题";
    let sections = section_numbers(doc);
    assert!(sections.contains("1") && sections.contains("3.2") && sections.contains("15.36"));
    assert!(!sections.contains("9"), "行内的 `##` 不是标题");
  }

  #[test]
  fn eager_calls_of_style_closures_are_flagged() {
    // 正例：`chips` 被 `{chips()}` 在组件体里立即调用（读信号那行确实在闭包里，缩进规则看不见）；
    // `grid` 被交给 `view!` 渲染，是对的，不能报。
    let code = "\
fn page() {
  let chips = {
    let rows = rows.clone();
    move || {
      if crate::kit::is_modern() {
        1
      } else {
        2
      }
    }
  };
  let grid = move || {
    if crate::kit::is_modern() { 3 } else { 4 }
  };
  view! { {chips()} {grid} }
}
";
    assert_eq!(eager_style_calls(code), vec![(15, "chips".to_owned())]);
  }

  #[test]
  fn style_closure_detector_does_not_fire_on_lookalikes() {
    // 构造期取值版不是订阅读：就算被立即调用也不算。
    let at_mount = "\
fn page() {
  let chips = move || {
    if crate::kit::is_modern_at_mount() { 1 } else { 2 }
  };
  view! { {chips()} }
}
";
    assert!(eager_style_calls(at_mount).is_empty());

    // `other.chips()` / `Foo::chips()` 不是这个局部闭包的调用。
    let qualified = "\
fn page() {
  let chips = move || { crate::kit::is_modern() };
  other.chips();
  Foo::chips();
}
";
    assert!(eager_style_calls(qualified).is_empty());
  }

  #[test]
  fn an_eager_call_in_another_function_is_not_this_closures_call() {
    // `page` 里的 `chips` 是风格闭包、被正确地交给 `view!`；`other` 里同名的 `chips` 是另一个东西，
    // 它的 `chips()` 不能算到 `page` 头上。`Thing::method` 里自己的风格闭包被立即调用，要报，
    // 而且行号是整个文件里的行号（不是片段里的）。
    let code = "\
fn page() {
  let chips = move || { crate::kit::is_modern() };
  view! { {chips} }
}

fn other() {
  let chips = || 1;
  view! { {chips()} }
}

impl Thing {
  fn method(&self) {
    let rows = move || { crate::kit::is_modern() };
    view! { {rows()} }
  }
}
";
    assert_eq!(eager_style_calls(code), vec![(14, "rows".to_owned())]);
  }

  #[test]
  fn fn_item_lines_are_recognised_with_their_qualifiers_and_only_near_the_left_edge() {
    for yes in [
      "fn a() {",
      "pub fn a() {",
      "pub(crate) fn a() {",
      "pub(super) async fn a() {",
      "  fn method(&self) {",
      "  pub const fn x() -> u8 {",
    ] {
      assert!(is_fn_item_line(yes), "{yes:?}");
    }
    for no in [
      "    fn nested() {",
      "  let f = || 1;",
      "pub struct S;",
      "pub_fn_like();",
      "// fn commented() {",
    ] {
      assert!(!is_fn_item_line(no), "{no:?}");
    }
  }

  #[test]
  fn fixed_bottom_layers_are_recognised_by_whole_class_token() {
    assert!(declares_fixed_bottom(
      "class=\"fixed inset-x-0 bottom-0 z-50 border-t\""
    ));
    assert!(declares_fixed_bottom(
      "class=\"mobile-tabs fixed inset-x-0 bottom-0 z-40 xl:hidden\""
    ));
    assert!(declares_fixed_bottom("class=\"md-dock\""));
    assert!(declares_fixed_bottom("class=\"md-bar md-bar-practice\""));
    assert!(declares_fixed_bottom("      class=\"md-bar\""));
    // 浮在底部但不贴边（toast / 更新提示）、断点前缀、同前缀的子类，都不是贴底固定层。
    assert!(!declares_fixed_bottom("class=\"fixed bottom-20 right-4\""));
    assert!(!declares_fixed_bottom("class=\"fixed xl:bottom-0\""));
    assert!(!declares_fixed_bottom("class=\"md-bar-inner md-bar-exam\""));
    assert!(!declares_fixed_bottom("class=\"md-dock-item\""));
    assert!(!declares_fixed_bottom("class=\"sticky bottom-0\""));
  }

  #[test]
  fn code_part_strips_comments() {
    assert_eq!(code_part("  // todo!()"), "  ");
    assert_eq!(code_part("  /* todo!() */"), "");
    assert_eq!(code_part("  *todo!()"), "");
    assert_eq!(code_part("  let x = 1; // todo!()"), "  let x = 1; ");
    assert_eq!(code_part("  let x = 1;"), "  let x = 1;");
  }

  #[test]
  fn view_fn_regex_matches_arrow_returns_only() {
    assert_eq!(VIEW_FN_RE.find_iter("-> impl IntoView").count(), 1);
    assert_eq!(VIEW_FN_RE.find_iter("-> AnyView").count(), 1);
    assert_eq!(VIEW_FN_RE.find_iter("-> IntoView").count(), 1);
    // 没有箭头的普通类型不匹配
    assert_eq!(VIEW_FN_RE.find_iter("let x: IntoView = y;").count(), 0);
  }

  #[test]
  fn knowledge_text_usage_and_dict_loading_are_told_apart() {
    assert!(uses_knowledge_text("{move || kt_tracked(s.name)}"));
    assert!(uses_knowledge_text("crate::data::core_text_tracked(label)"));
    // 只是 `use` 进来，不算在用；定义在 data.rs 里也不是页面。
    assert!(!uses_knowledge_text("use crate::data::kt_tracked;"));
    assert!(loads_knowledge_dict("crate::data::use_knowledge_i18n();"));
    assert!(loads_knowledge_dict(
      "<KnowledgePage title=move || t(\"x\")>"
    ));
    assert!(!loads_knowledge_dict("let x = 1;"));
  }

  #[test]
  fn rules_match_expected_text() {
    assert!(has_sticky_top("class=\"sticky top-[66px]\""));
    assert!(!has_sticky_top("class=\"sticky z-10\""));
    assert!(has_alert("window.alert(\"x\")"));
    assert!(has_alert("alert(\"x\")"));
    assert!(has_alert(
      "let _ = js_sys::eval(\"1\"); web_sys::window().alert(msg)"
    ));
    assert!(!has_alert("let alert = 1;"));
    // 自定义函数名里恰好带 `alert(`:不是浏览器原生弹窗。
    assert!(!has_alert("show_alert(msg)"));
    assert!(!has_alert("fn raise_alert(msg: &str)"));
    assert!(!has_alert("push_alert(msg)"));
    assert!(has_browser_api("use wasm_bindgen::prelude::*;"));
    assert!(has_browser_api("web_sys::window()"));
    assert!(!has_browser_api("let dom_api = 1;"));
    assert!(has_hex_color("color: \"#e11d48\","));
    assert!(has_hex_color("let c = \"#b13e53\";"));
    assert!(!has_hex_color("/// 见 `#e11d48` 的说明"));
    assert!(!has_hex_color("let id = \"q3\";"));
    assert!(has_hex_color("assert_eq!(hex, \"#b13e53ff\");"));
    assert!(!has_hex_color("let n = \"#12345\";"));
    assert!(has_panic_macro("todo!()"));
    assert!(has_panic_macro("unimplemented!()"));
    // 带说明文字的写法同样命中，`panic!` / `unreachable!` 由项目自己决定是否禁用，不在这条里。
    assert!(has_panic_macro("todo!(\"later\")"));
    assert!(has_panic_macro("unimplemented!(\"x\")"));
    assert!(!has_panic_macro("let todo = 1;"));
    // 函数体顶层（2 空格）读风格信号 = 组件体里订阅，命中。
    assert!(has_body_level_style_read("  if crate::kit::is_modern() {"));
    assert!(has_body_level_style_read(
      "  let modern = crate::kit::is_modern();"
    ));
    // 闭包里的调用缩进更深，不命中。
    assert!(!has_body_level_style_read(
      "      if crate::kit::is_modern() {"
    ));
    assert!(!has_body_level_style_read(
      "        class=move || { if crate::kit::is_modern() { \"md-root\" } else { \"\" } }"
    ));
    // 构造期那一版的名字里不含 `is_modern()`，不会误判。
    assert!(!has_body_level_style_read(
      "  if crate::kit::is_modern_at_mount() {"
    ));
  }

  #[test]
  fn style_gate_is_required_only_for_pixel_dom_selectors() {
    // 按 `data-sig` 分支但只碰现代风 DOM：`md-` 类只存在于现代风，本身就是门禁。
    assert!(!needs_style_gate(
      r#"#route-content[data-sig="read"] .md-root h2"#
    ));
    // 带门禁的选择器无论多具体都放行。
    assert!(!needs_style_gate(
      r#"html[data-ui-style="modern"] #route-content[data-sig="lab"] section.bg-card"#
    ));
    // 像素风 DOM：桥接页卡片的调色板工具类、地图的裸 svg 选择器。
    assert!(needs_style_gate(
      r#"#route-content[data-sig="lab"] section.bg-card"#
    ));
    assert!(needs_style_gate(
      r#"#route-content[data-sig="map"] svg[role="img"] :is(polygon, circle):has(> title)"#
    ));
    // 按 `data-sig` 分支却一个 `md-` 都没有：也要门禁。
    assert!(needs_style_gate(
      r#"#route-content[data-sig="board"] section"#
    ));
  }

  #[test]
  fn shared_site_footer_needs_the_style_gate() {
    // 站点页脚两套皮肤共用，像素风对它另有底部让位：裸写会盖掉像素风的值。
    assert!(needs_style_gate("body > footer"));
    assert!(needs_style_gate(
      r#"html[data-ui-style="modern"] body > footer, body > footer"#
    ));
    assert!(!needs_style_gate(
      r#"html[data-ui-style="modern"] body > footer"#
    ));
    // 现代风自己的外壳选择器不受影响。
    assert!(!needs_style_gate(
      r#"body.no-site-footer:not([data-page-mode]) [data-style-root="modern"]"#
    ));
  }

  #[test]
  fn selectors_walks_layers_and_multiline_selectors() {
    let css = r#"
@layer components {
  html[data-ui-style="modern"] #route-content[data-sig="lab"] section.bg-card {
    color: red;
  }

  #route-content[data-sig="lab"] :is(.md-panel, section.bg-card):hover {
    animation: none;
  }

  @keyframes md-lab-scan {
    to { background-position: 0 0; }
  }
}
"#;
    let hits = selectors(css, needs_style_gate);
    assert_eq!(hits.len(), 1, "只该命中未门禁的那条：{hits:?}");
    assert_eq!(hits[0].0, 7, "行号应指向选择器起始行");
    assert!(hits[0].1.contains("section.bg-card"));
  }

  #[test]
  fn normalize_join_collapses_dot_segments() {
    // 跨平台归一化靠 `Path::components()`：绝对路径与相对路径都吃，也认得 `..`。
    assert_eq!(
      normalize_join(Path::new("/a/b/../c/./d.md")),
      PathBuf::from("/a/c/d.md")
    );
    assert_eq!(
      normalize_join(Path::new("docs/./x/../y.md")),
      PathBuf::from("docs/y.md")
    );
    // 根上的 `..` 不会把根弹掉。
    assert_eq!(
      normalize_join(Path::new("/../a.md")),
      PathBuf::from("/a.md")
    );
  }

  #[test]
  fn off_list_breakpoints_catches_off_list_values_only() {
    let css = r#"
@media (min-width: 900px) { }
@media (max-width: 899px) { }
@media (min-width: 768px) and (max-width: 900px) { }
@media (pointer: coarse) and (max-width: 900px) { }
@container (max-width: 520px) { }
"#;
    // 899 违规；900 / 768 白名单内；520 是容器查询，不该被 @media 正则命中。
    let off = off_list_breakpoints(css);
    assert_eq!(off.len(), 1, "{off:?}");
    assert!((off[0].1 - 899.0).abs() < f64::EPSILON);
    // 值都在白名单里，但「≥900」与两条「≤900」同时存在：恰好 900px 两档都命中。
    assert_eq!(overlapping_bounds(css).len(), 2, "同值上下界必须被报出来");

    // 上界写成 `(width < 900px)` 就没有重叠了（这也是唯一允许的写法）。
    let fixed = r#"
@media (min-width: 900px) { }
@media (width < 900px) { }
@media (min-width: 640px) and (width < 900px) { }
"#;
    assert!(overlapping_bounds(fixed).is_empty());
    // 范围语法里的「含下界 + 不含上界」同样不重叠。
    let range = "@media (width >= 640px) and (width < 900px) { }";
    assert!(overlapping_bounds(range).is_empty());
    // 上界含、下界不含（`width > 900`）也不重叠：恰好 900 只属于上界那一档。
    let exclusive_low = "@media (width > 900px) { }\n@media (max-width: 900px) { }";
    assert!(overlapping_bounds(exclusive_low).is_empty());
  }
}
