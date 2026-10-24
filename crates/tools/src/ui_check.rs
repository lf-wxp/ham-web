//! 界面规范的机械守卫：把 `docs/ui-components.md` §11 里「应为 0」的几条搬进 `cargo make check`。
//!
//! 为什么落成工具：`rounded-*` 这类「写了也没效果」的类曾被 `surfaces.css` 当存量卡片皮肤的
//! 钩子用（2026-10-13 才换成 `pxl-card` 标记清理干净），手写 `<input>` 与 `input_class()` 类名
//! 工厂也都是收口之后容易回潮的东西 —— 靠人记得手跑 §11 的命令不可靠。
//!
//! 覆盖 §11 的 ①②③④⑤⑦，以及 ⑥ 与 ⑧ 的机械部分：
//! - ①②③④⑤⑦ 命中即失败；
//! - ⑥ 的任意值阴影只允许「第三个长度（模糊半径）为 0」的像素硬阴影（模糊位写成 `var(…)`
//!   判不出来时放过，宁可漏报）；
//! - ⑧ 的页面级宽度容器是**封闭清单** —— 清单外命中、清单内计数漂移都算失败。
//!
//! 2026-10-24 界面评审补上「文档写了、门禁没拦」的一批（见 `docs/ui-components.md` §11），全部是
//! class token 级判定（先取字符串字面量、再按空白切 token、剥掉变体前缀）：
//! - ⑪ 字重类（`font-semibold` / `font-bold` / `font-medium`）：点阵字体下是空操作，
//!   易读字体下却会生效，两种模式层级不一致；
//! - ⑫ 缩放 / 悬停旋转（`scale-*` / `zoom-*` / `hover:rotate-*`）：中间帧会让点阵文字变糊；
//! - ⑬ `transition-all` 与写死毫秒的 `duration-<数字>`：时长只取 `--motion-*` 四档；
//! - ⑭ 实线焦点环（`ring-*` / `ring-offset-*`）：全站焦点语言是 2px 虚线；
//! - ⑮ 非 6 倍数的任意像素字号（`text-[10px]` 等）：点阵字体按整数倍缩放才不糊；
//! - ⑯ Tailwind 原始色阶（`bg-emerald-500` 等）：绕过 11 套配色方案，状态色走语义 token；
//! - ⑰ 手写的悬停反馈（`hover:-translate-y-1`、`hover:shadow-*`、`shadow-<颜色>/NN`）：
//!   整卡可点的悬停反馈只有 `motion-lift`（`SectionCard href` 自带）一份，首页追台卡曾手写
//!   4px 上抬 + 主色软阴影，和同页其它卡片是两套语言。
//! - ⑱ 空的 `class=""` / `style=""`：批量删类名（字重 / 色阶）后留下的空属性，56 处，
//!   没有语义也不生成任何样式，是纯噪声。
//! - ⑲ `text-destructive`：`destructive` 是按钮的填充色，当文字色用时暗色下对弹层表面
//!   不足 4.5:1（axe 在 `rpg_flow` 抓到过）；写字用 `text-hp-text`。
//! - ⑳ 页面里手写 `<h1>` 且没带 `pxl-title` / `sr-only`：页面标题的字体族会漂成正文点阵
//!   （2026-10-24 的第一轮整改里，5 个页面的标题就是这样和别处不一样）。
//! - ㉑ 卡片标题 `<h2>` 用 `text-sm`（12px）：比它管的 18px 正文还小，卡片就「一片平字」；
//!   区段标题统一走 `card_title_class(...)`（`pxl-label` + 18px）。
//! - ㉒ 任意值断点 `min-[NNNpx]:`：断点词汇只有 `xs`（400px）/ `sm` / `md` / `lg`，
//!   新增中间态写在 `tokens.css` 的 `--breakpoint-*` 里，别在页面里现造。
//! - ㉓ `<Icon>` 写 `h-3/3.5/4/5`（16 / 20px）：像素图标是 24px 网格，这些值会被
//!   `surfaces.css` 隐式抬到 24px，源码与实际渲染不符；直接写 `size-6`。
//!
//! 判不了、仍留给人工的：`mm` 尺寸的打印页（⑧ 的例外）、内联 `style` 里的阴影、
//! ⑨ 的例外注释计数。
//!
//! 判定刻意保守：
//! - 每行先剥掉 `//` 之后的内容，`/*` 与 `*` 开头的行整行跳过（历史说明不算违规）；
//!   字符串里含 `//`（URL）的行会被截断，只可能漏报、不会误报。
//! - `cn.rs`（类名合并器，字符串里出现工具类名是它的本职工作）按文件名跳过。

use std::collections::BTreeSet;
use std::path::Path;

use anyhow::{Result, bail};
use walkdir::WalkDir;

/// 每条规则最多打印多少条命中，避免一次刷屏。
const MAX_REPORT: usize = 10;

/// 一条守卫规则。
struct Rule {
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

const RULES: &[Rule] = &[
  Rule {
    name: "已失效的圆角类（`rounded-*`）",
    roots: &["crates/app/src"],
    skip: &["cn.rs"],
    hits: has_rounded_class,
    advice: "卡片皮肤用 `pxl-card` 标记（§2「可用像素元素」），其余圆角一律不要写",
  },
  Rule {
    name: "本地控件样式常量（`const INPUT:` / `const TEXTAREA:`）",
    roots: &["crates/app/src"],
    skip: &["/ui/"],
    hits: |line| line.contains("const INPUT:") || line.contains("const TEXTAREA:"),
    advice: "控件样式只在 ui/ 里定义；页面要局部调整就用组件的 `class` 参数",
  },
  Rule {
    name: "原生表单控件（`<input>` / `<select>` / `<textarea>`）",
    roots: &["crates/app/src/pages", "crates/app/src/components"],
    skip: &[],
    hits: has_native_control,
    advice: "用 ui/ 的 Input / NativeSelect / Textarea / FileInput / FileDropZone（§3 决策表）",
  },
  Rule {
    name: "类名工厂 `input_class(`（已删除）",
    roots: &["crates/app/src"],
    skip: &["/ui/"],
    hits: |line| line.contains("input_class("),
    advice: "表单控件已全部走 ui/，不要再造「让未迁移输入框长得一样」的类名工厂",
  },
  Rule {
    name: "手写 chip 样式常量（`CHIP_ON` 等）",
    roots: &["crates/app/src"],
    skip: &["/ui/"],
    hits: |line| line.contains("CHIP_ON"),
    advice: "用 ui/ 的 ChipGroup / Chip / ChipToggle",
  },
  Rule {
    name: "把填充色 token 当文字色（对比度不够）",
    roots: &["crates/app/src"],
    skip: &[],
    hits: has_fill_token_as_text,
    advice: "文字用 `--pxl-ink` / 语义前景色，填充色只做背景（§9）",
  },
  Rule {
    name: "模糊 / 渐变（零模糊零渐变）",
    roots: &["crates/app/src"],
    skip: &["/ui/", "cn.rs"],
    hits: has_blur_or_gradient,
    advice: "去掉 `backdrop-blur` / `blur-*` / `drop-shadow-*` / `bg-gradient-*` / `bg-linear-*`（§2 硬规则 1）",
  },
  Rule {
    name: "带模糊半径的任意值阴影（`shadow-[…]`）",
    roots: &["crates/app/src"],
    skip: &["cn.rs"],
    hits: has_soft_arbitrary_shadow,
    advice: "任意值阴影只写像素硬阴影 `x y 0 0 <色>`（模糊位必须是 0）；要层次感就加粗描边 / 换色，命名档位（`shadow-sm` 等）在 tokens.css 里已经是硬阴影、可以直接用",
  },
  Rule {
    name: "字重类（`font-semibold` / `font-bold` / `font-medium`）",
    roots: &["crates/app/src"],
    skip: &["cn.rs"],
    hits: has_font_weight,
    advice: "点阵字体没有粗体，这些类默认模式下是空操作、易读字体下却会生效；强调靠颜色 / `pxl-label` / `pxl-title` / `pxl-section-title`（§2 硬规则 4）",
  },
  Rule {
    name: "缩放 / 悬停旋转（`scale-*` / `zoom-*` / `hover:rotate-*`）",
    roots: &["crates/app/src"],
    skip: &["cn.rs"],
    hits: has_scale_motion,
    advice: "不做缩放动画（中间帧让点阵文字变糊）；按压反馈用 `pxl-btn` 自带的下沉 / `motion-press`（§2 硬规则 10）",
  },
  Rule {
    name: "`transition-all` / 写死毫秒的 `duration-<数字>`",
    roots: &["crates/app/src"],
    skip: &["cn.rs"],
    hits: has_loose_motion,
    advice: "`transition-all` 会动画布局属性，写 `transition`；时长取 `duration-(--motion-fast|normal|slow)`，不要 `duration-300`（§2 硬规则 10）",
  },
  Rule {
    name: "实线焦点环（`ring-*` / `ring-offset-*`）",
    roots: &["crates/app/src"],
    skip: &["cn.rs"],
    hits: has_solid_ring,
    advice: "全站焦点是 2px 虚线（`:focus-visible` 全局规则 + `pxl-*` 控件自带）；不要用 `ring-*` 顶替它，也不用为它加 `outline-none`（§2 / MOTION.md）",
  },
  Rule {
    name: "非 6 倍数的任意像素字号（`text-[10px]` 等）",
    roots: &["crates/app/src"],
    // nvis.rs：SVG `<text>` 的字号是 viewBox 用户单位、随图缩放，不是 CSS 像素；
    // print_sheet：打印页量的是纸（§4「为什么打印类页面不用 max-w-5xl」同理）。
    skip: &["cn.rs", "pages/nec/nvis.rs", "pages/print_sheet/"],
    hits: has_off_grid_font_px,
    advice: "点阵字体按 6 的倍数（12 / 18 / 24px）缩放才不糊：小字用 `text-xs`（§2 字号阶梯）",
  },
  Rule {
    name: "Tailwind 原始色阶（`bg-emerald-500` / `text-red-600` 等）",
    roots: &["crates/app/src"],
    // 打印 / 导出画布按白纸固定配色，不随配色方案变（§2 硬规则 7 的括注）。
    skip: &[
      "cn.rs",
      "pages/print_sheet/",
      "pages/qsl_labels/",
      "pages/qsl_designer.rs",
    ],
    hits: has_raw_palette_color,
    advice: "状态色走语义 token：文字 `text-win-text` / `text-hp-text` / `text-gold-text` / `text-xp-text`（别用 `text-destructive`，那是填充色），底色 `bg-win/10` 等；原始色阶会跳出 11 套配色方案（§2 硬规则 7）",
  },
  Rule {
    name: "手写的悬停反馈（`hover:-translate-y-*` / `hover:shadow-*` / `shadow-<颜色>/NN`）",
    roots: &["crates/app/src"],
    skip: &["cn.rs"],
    hits: has_ad_hoc_hover_lift,
    advice: "整卡可点的悬停反馈只有一份：`SectionCard href=…`（自带 `motion-lift`：上抬 2px + 硬投影）；单个元素用 `motion-lift` / `motion-press`。不要手写位移 / 带色软阴影（§2 硬规则 1、10）",
  },
  Rule {
    name: "空属性（`class=\"\"` / `style=\"\"`）",
    roots: &["crates/app/src"],
    skip: &[],
    hits: has_empty_attribute,
    advice: "批量删类名后把属性一起删掉：空 `class` / `style` 不产生任何样式，只会让人以为这里本该有样式（§11 ⑱）",
  },
  Rule {
    name: "图标写 16/20px（`<Icon class=\"h-4 w-4\">`）",
    roots: &["crates/app/src"],
    skip: &[],
    hits: has_small_icon,
    advice: "像素图标是 24px 网格：写 `size-6`（源码 = 渲染）。16/20px 会被 `surfaces.css` 隐式抬到 24px",
  },
  Rule {
    name: "任意值断点（`min-[NNNpx]:`）",
    roots: &["crates/app/src"],
    skip: &[],
    hits: has_arbitrary_breakpoint,
    advice: "断点只有 `xs`（400px）/ `sm` / `md` / `lg` 四档；新增中间态写进 `tokens.css` 的 `--breakpoint-*`",
  },
  Rule {
    name: "卡片标题用 12px（`<h2>` + `text-sm`）",
    roots: &["crates/app/src"],
    skip: &["print_page.rs"],
    hits: has_small_heading,
    advice: "区段标题走 `card_title_class(...)`（`pxl-label` + 18px）；12px 的标题会比它管的正文还小",
  },
  Rule {
    name: "页面标题没走 `pxl-title`（手写 `<h1>`）",
    roots: &["crates/app/src/pages"],
    skip: &["print_page.rs"],
    hits: has_bare_h1,
    advice: "页头用 `PageHeader`（`pxl-title` + 吸顶 + 主色左轨的唯一来源）；确需手写 `<h1>` 时带 `pxl-title`，纯无障碍标题用 `sr-only`",
  },
  Rule {
    name: "把 `destructive` 填充色当文字色（`text-destructive`）",
    roots: &["crates/app/src"],
    skip: &["cn.rs"],
    hits: has_destructive_as_text,
    advice: "`text-destructive` 是按钮填充色，暗色下落在弹层 / 卡片表面上不足 4.5:1；写字用 `text-hp-text`（§2 硬规则 6）",
  },
];

/// 一行里有没有圆角**类**：只看字符串字面量内部的 `rounded` token。
///
/// 必须限定在字面量里 —— `let rounded: f64` / `rounded.abs()` 这种标识符（`dbm_dbuv.rs`）
/// 不是类名，按整行匹配会误报。
fn has_rounded_class(line: &str) -> bool {
  has_rounded_token(&string_contents(line))
}

/// 收集一行里所有字符串字面量的内容（`\"` / `\\` 转义按字面处理），字面量之间补空格，
/// 避免相邻两个字面量拼成一个 token 而误判。未闭合的字面量按「到行尾」处理。
fn string_contents(line: &str) -> String {
  let mut out = String::new();
  let mut rest = line;
  while let Some(start) = rest.find('"') {
    let after = &rest[start + 1..];
    let mut escaped = false;
    let mut end = None;
    for (i, c) in after.char_indices() {
      if escaped {
        escaped = false;
      } else if c == '\\' {
        escaped = true;
      } else if c == '"' {
        end = Some(i);
        break;
      }
    }
    match end {
      Some(i) => {
        out.push_str(&after[..i]);
        rest = &after[i + 1..];
      }
      None => {
        out.push_str(after);
        break;
      }
    }
    out.push(' ');
  }
  out
}

/// `rounded` / `rounded-*` 的 token 判定（在「类名文本」上跑，不含引号）。
///
/// 前后做标识符边界：`roundedness`、`x_rounded_y` 不算；变体前缀（`focus:rounded-md`）算 ——
/// 它同样是被全局零圆角规则压平的类。
fn has_rounded_token(line: &str) -> bool {
  const NEEDLE: &str = "rounded";
  let bytes = line.as_bytes();
  let mut from = 0;
  while let Some(pos) = line[from..].find(NEEDLE) {
    let start = from + pos;
    let end = start + NEEDLE.len();
    let boundary = |c: u8| !c.is_ascii_alphanumeric() && c != b'_';
    let prev_ok = start == 0 || boundary(bytes[start - 1]);
    let next_ok = end == bytes.len() || boundary(bytes[end]);
    if prev_ok && next_ok {
      return true;
    }
    from = end;
  }
  false
}

fn has_native_control(line: &str) -> bool {
  line.contains("<input") || line.contains("<select") || line.contains("<textarea")
}

/// §11 ⑱：空的 `class=""` / `style=""`。
///
/// 只认这两个属性名 + 紧邻的空双引号 —— `cn(&["", ""])`、`format!("")` 这类空字符串不是空属性。
/// 属性名前后不设边界：`data-class=""` 这种同样没有意义，一并拦。
fn has_empty_attribute(line: &str) -> bool {
  line.contains("class=\"\"") || line.contains("style=\"\"")
}

/// §2 硬规则 6：`text-destructive` 把填充色当文字色用。
///
/// `text-destructive-foreground`（实色按钮上的字）是另一个 token，不归这条管 —— 它由
/// `.pxl-btn-destructive` 在 `controls.css` 里成套使用，调用点不会单独写。
/// ㉓ `<Icon>` 上用 16/20px 的尺寸类（`h-4 w-4` / `size-4` …）。
fn has_small_icon(line: &str) -> bool {
  let Some(start) = line.find("<Icon") else {
    return false;
  };
  let tail = &line[start..];
  let end = tail.find('>').map_or(tail.len(), |i| i + 1);
  class_tokens(&tail[..end]).iter().any(|(_, base)| {
    matches!(
      base.as_str(),
      "h-3" | "h-3.5" | "h-4" | "h-5" | "size-3" | "size-3.5" | "size-4" | "size-5"
    )
  })
}

/// ㉒ 任意值断点：`min-[560px]:` 这类现造的媒体查询断点。
fn has_arbitrary_breakpoint(line: &str) -> bool {
  let needle = "min-[";
  let mut from = 0;
  while let Some(i) = line[from..].find(needle) {
    let at = from + i + needle.len();
    let digits = line[at..].chars().take_while(char::is_ascii_digit).count();
    if digits > 0 && line[at + digits..].starts_with("px]:") {
      return true;
    }
    from = at;
  }
  false
}

/// ㉑ 卡片标题 `<h2>` 带 `text-sm`（12px）：比正文（18px）还小的标题。
fn has_small_heading(line: &str) -> bool {
  line.contains("<h2") && line.contains("text-sm")
}

/// ⑳ 手写 `<h1>` 且既不是 `pxl-title` 也不是 `sr-only`：标题字体就会退化成正文点阵。
fn has_bare_h1(line: &str) -> bool {
  line.contains("<h1") && !line.contains("pxl-title") && !line.contains("sr-only")
}

fn has_destructive_as_text(line: &str) -> bool {
  class_tokens(line)
    .iter()
    .any(|(_, base)| base == "text-destructive" || base.starts_with("text-destructive/"))
}

/// §11 ⑦：把填充色 token（hp / xp / gold / win）当文字色用，对比度不达标。
fn has_fill_token_as_text(line: &str) -> bool {
  ["hp", "xp", "gold", "win"]
    .iter()
    .any(|t| line.contains(&format!("text-[color:var(--pxl-{t})]")))
}

/// §11 ⑥：模糊 / 渐变。
///
/// 命名档位（`blur` / `blur-sm` … `blur-3xl` / Tailwind v4 的 `blur-xs`）、任意值
/// （`blur-[…]`）、变体（`hover:blur`）与 `drop-shadow-*` 都算 —— 像素规范里只能靠
/// 描边 / 换色做层次；内联 `style="filter: drop-shadow(0 1px 0 …)"`（`bubble.rs` 的
/// 对话气泡箭头，模糊 0）不在这条规则的范围内。
///
/// 裸 `blur` 用子串 `blur` 一并覆盖（它同时覆盖上面那些档位）：这条规则跳过 `cn.rs`
/// 与 `/ui/`，而 `cn.rs` 里那处 `("backdrop-blur", "backdrop-blur")` 是 cn 的冲突组配置，
/// 所以全仓不会误伤；漏掉裸档位则会让 `class="blur"` 整行溜过去。
fn has_blur_or_gradient(line: &str) -> bool {
  const NEEDLES: &[&str] = &["blur", "drop-shadow-", "bg-gradient", "bg-linear"];
  NEEDLES.iter().any(|n| line.contains(n))
}

/// 把一行里所有字符串字面量切成 class token，并拆成 `(变体前缀, 基础类)`。
///
/// 变体前缀里的冒号可能落在方括号 / 圆括号里（`data-[state=open]:`、`[&:not(:disabled)]:`），
/// 只认**最外层**的最后一个冒号。例：`dark:hover:bg-red-500/10` -> (`dark:hover:`, `bg-red-500/10`)。
fn class_tokens(line: &str) -> Vec<(String, String)> {
  let mut out = Vec::new();
  for token in string_contents(line).split_whitespace() {
    let mut depth = 0i32;
    let mut cut = 0usize;
    for (i, c) in token.char_indices() {
      match c {
        '[' | '(' => depth += 1,
        ']' | ')' => depth -= 1,
        ':' if depth == 0 => cut = i + 1,
        _ => {}
      }
    }
    out.push((token[..cut].to_owned(), token[cut..].to_owned()));
  }
  out
}

/// §2 硬规则 4：字重类。
fn has_font_weight(line: &str) -> bool {
  class_tokens(line).iter().any(|(_, base)| {
    matches!(
      base.as_str(),
      "font-semibold" | "font-bold" | "font-medium" | "font-extrabold"
    )
  })
}

/// §2 硬规则 10：缩放 / 悬停（或按下）旋转。静态的 `rotate-180`（展开箭头翻转）不算动画。
fn has_scale_motion(line: &str) -> bool {
  class_tokens(line).iter().any(|(variant, base)| {
    let b = base.strip_prefix('-').unwrap_or(base);
    let is_scale = b.strip_prefix("scale-").is_some_and(|rest| {
      let rest = rest
        .strip_prefix("x-")
        .or_else(|| rest.strip_prefix("y-"))
        .unwrap_or(rest);
      rest.starts_with('[') || rest.chars().next().is_some_and(|c| c.is_ascii_digit())
    });
    let is_zoom = b.starts_with("zoom-in-") || b.starts_with("zoom-out-");
    let is_interactive_rotate = b.starts_with("rotate-")
      && (variant.contains("hover:") || variant.contains("active:") || variant.contains("focus"));
    is_scale || is_zoom || is_interactive_rotate
  })
}

/// §2 硬规则 10：`transition-all` 与写死毫秒的 `duration-<数字>`。
/// `duration-(--motion-fast)` 这种引用 token 的写法合规。
fn has_loose_motion(line: &str) -> bool {
  class_tokens(line).iter().any(|(_, base)| {
    base == "transition-all"
      || base
        .strip_prefix("duration-")
        .is_some_and(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()))
  })
}

/// 实线焦点环：`ring`、`ring-2`、`ring-ring/60`、`ring-offset-2`、`ring-offset-background`。
/// `focus-visible:outline-ring`（虚线焦点的颜色）不是它。
fn has_solid_ring(line: &str) -> bool {
  class_tokens(line).iter().any(|(_, base)| {
    base == "ring"
      || base.starts_with("ring-offset")
      || base.strip_prefix("ring-").is_some_and(|rest| {
        rest == "ring"
          || rest.starts_with("ring/")
          || rest.chars().next().is_some_and(|c| c.is_ascii_digit())
      })
  })
}

/// 任意像素字号 `text-[Npx]`，N 不是 6 的倍数。
fn has_off_grid_font_px(line: &str) -> bool {
  class_tokens(line).iter().any(|(_, base)| {
    base
      .strip_prefix("text-[")
      .and_then(|rest| rest.strip_suffix("px]"))
      .and_then(|n| n.parse::<f64>().ok())
      .is_some_and(|n| (n % 6.0).abs() > f64::EPSILON)
  })
}

/// Tailwind 默认色阶的色族。
const PALETTE_FAMILIES: &[&str] = &[
  "slate", "gray", "zinc", "neutral", "stone", "red", "orange", "amber", "yellow", "lime", "green",
  "emerald", "teal", "cyan", "sky", "blue", "indigo", "violet", "purple", "fuchsia", "pink",
  "rose",
];

/// 原始色阶：`<属性>-<色族>-<两三位数字>[/不透明度]`，如 `bg-emerald-500/10`、`fill-sky-600`。
/// 带 `print:` 变体的放过（打印是白纸，不随配色方案变）。
fn has_raw_palette_color(line: &str) -> bool {
  const PROPS: &[&str] = &[
    "bg",
    "text",
    "border",
    "ring",
    "fill",
    "stroke",
    "from",
    "to",
    "via",
    "divide",
    "outline",
    "decoration",
    "shadow",
    "accent",
    "caret",
  ];
  class_tokens(line).iter().any(|(variant, base)| {
    if variant.contains("print:") {
      return false;
    }
    let base = base.split('/').next().unwrap_or(base);
    let mut parts = base.splitn(3, '-');
    let (Some(prop), Some(family), Some(shade)) = (parts.next(), parts.next(), parts.next()) else {
      return false;
    };
    PROPS.contains(&prop)
      && PALETTE_FAMILIES.contains(&family)
      && (2..=3).contains(&shade.len())
      && shade.chars().all(|c| c.is_ascii_digit())
  })
}

/// 手写的悬停反馈：悬停 / 按下时的位移、悬停时才出现的阴影、带颜色的阴影。
///
/// 放过：居中用的 `-translate-y-1/2`（没有悬停变体、也不是位移反馈）；`group-hover:-translate-y-0.5`
/// 这种 2px 以内的位移（品牌图标上抬）按「一个 art-pixel」算合规。
fn has_ad_hoc_hover_lift(line: &str) -> bool {
  class_tokens(line).iter().any(|(variant, base)| {
    let b = base.strip_prefix('-').unwrap_or(base);
    let interactive =
      variant.contains("hover:") || variant.contains("active:") || variant.contains("focus");
    // 位移：只认 `translate-y-<整数 >= 1>`，0.5（2px）以内放过。
    let lifts = interactive
      && b
        .strip_prefix("translate-y-")
        .and_then(|n| n.parse::<f64>().ok())
        .is_some_and(|n| n >= 1.0);
    // 悬停才出现的阴影：命名档位（`hover:shadow-lg`）与颜色（`hover:shadow-primary/10`）都算。
    let hover_shadow = interactive && b.starts_with("shadow-");
    // 带颜色的阴影：`shadow-primary/25`；命名档位（`shadow-sm`）与任意值（另有规则）不是它。
    let colored = b.strip_prefix("shadow-").is_some_and(|rest| {
      let name = rest.split('/').next().unwrap_or(rest);
      !matches!(
        name,
        "none" | "2xs" | "xs" | "sm" | "md" | "lg" | "xl" | "2xl" | "inner"
      ) && !name.starts_with('[')
        && !name.starts_with('(')
        && name.chars().next().is_some_and(|c| c.is_ascii_alphabetic())
    });
    lifts || hover_shadow || colored
  })
}

/// §11 ⑥：任意值阴影 `shadow-[…]` 里有没有模糊半径。
///
/// 命名档位不用判 —— `tokens.css` 把 `--shadow-*` 重定义成了 `x y 0 0 色` 的硬阴影，
/// `shadow-sm` / `shadow-lg` 本身就是合规的像素硬阴影；要拦的是用任意值绕过 token 系统
/// 写出来的软阴影（`shadow-[0_4px_12px_…]`）。
fn has_soft_arbitrary_shadow(line: &str) -> bool {
  arbitrary_values(line, "shadow-[").iter().any(|value| {
    split_shadow_list(value)
      .iter()
      .any(|seg| shadow_segment_has_blur(seg))
  })
}

/// 取出一行里所有 `<prefix>[…]` 的方括号内容（`shadow-[…]`、`blur-[…]` 这类）。
fn arbitrary_values(line: &str, prefix: &str) -> Vec<String> {
  let mut out = Vec::new();
  let mut from = 0usize;
  while let Some(pos) = line[from..].find(prefix) {
    let start = from + pos + prefix.len();
    let mut depth = 1i32;
    let mut end = None;
    for (i, c) in line[start..].char_indices() {
      match c {
        '[' => depth += 1,
        ']' => {
          depth -= 1;
          if depth == 0 {
            end = Some(i);
            break;
          }
        }
        _ => {}
      }
    }
    match end {
      Some(i) => {
        out.push(line[start..start + i].to_owned());
        from = start + i + 1;
      }
      None => break,
    }
  }
  out
}

/// 按顶层逗号切分多层阴影（`a, b`），`rgb(0,0,0,.2)` 里的逗号不算。
fn split_shadow_list(value: &str) -> Vec<&str> {
  let mut out = Vec::new();
  let mut depth = 0i32;
  let mut start = 0usize;
  for (i, c) in value.char_indices() {
    match c {
      '(' => depth += 1,
      ')' => depth -= 1,
      ',' if depth == 0 => {
        out.push(&value[start..i]);
        start = i + 1;
      }
      _ => {}
    }
  }
  out.push(&value[start..]);
  out
}

/// 单个阴影段（`0_2px_0_0_var(--pxl-shadow)`）的模糊位（第 3 个长度）是不是**明确的非 0 长度**。
///
/// - 长度位不足 3 个（`x y`、`x y 色`）：模糊半径按 CSS 默认取 0 → 合规；
/// - 模糊位写成 `var(…)` / `calc(…)`（如 `0_2px_var(--pxl-shadow)`，那其实是颜色）：
///   静态判不出来 → **放过**（宁可漏报也不拦人的合法写法）；
/// - 明确写死非 0（`0_0_10px_#000`）：拦。
fn shadow_segment_has_blur(segment: &str) -> bool {
  let mut tokens = segment.split('_').filter(|t| !t.is_empty());
  // `inset` 只是关键字，跳过它再数长度
  while tokens.clone().next().is_some_and(|t| t == "inset") {
    tokens.next();
  }
  let mut lengths: Vec<&str> = Vec::new();
  for token in tokens {
    if is_shadow_length(token) {
      lengths.push(token);
    } else {
      break; // 到颜色 / 其它关键字为止
    }
  }
  lengths.get(2).is_some_and(|blur| {
    !blur.starts_with("var(") && !blur.starts_with("calc(") && *blur != "0" && *blur != "0px"
  })
}

/// CSS 长度：`0`、`2px`、`-2px`、`.5rem`、`50%`、`var(…)`、`calc(…)`。
fn is_shadow_length(token: &str) -> bool {
  let t = token.strip_prefix('-').unwrap_or(token);
  if t == "0" || t.starts_with("var(") || t.starts_with("calc(") {
    return true;
  }
  let mut chars = t.chars();
  chars.next().is_some_and(|c| c.is_ascii_digit() || c == '.')
    && t
      .chars()
      .all(|c| c.is_ascii_digit() || c == '.' || c == '%' || c.is_ascii_alphabetic())
}

/// §4 的「页面级宽度容器」封闭清单：允许出现这类宽度的文件 + 各自的说明。
///
/// 键 = 相对项目根的路径。**新增一处就得同时改这里与 §4 的清单** —— 这正是「封闭清单」的意思。
const WIDTH_ALLOWED: &[(&str, &str)] = &[
  ("crates/app/src/app/footer.rs", "页脚"),
  ("crates/app/src/app/storage_warning.rs", "存储告警横幅"),
  (
    "crates/app/src/components/common/bottom_bar.rs",
    "底部工具栏对齐行",
  ),
  (
    "crates/app/src/components/common/page_header.rs",
    "页头对齐行（多行页头）",
  ),
  (
    "crates/app/src/components/navigation/nav_bar.rs",
    "顶栏 container",
  ),
  (
    "crates/app/src/pages/bookmarks/bookmarks_page.rs",
    "多行页头工具条（§4 表里登记的对齐行）",
  ),
  (
    "crates/app/src/pages/daily_challenge.rs",
    "多行页头吸顶计时条（§4 表里登记的对齐行）",
  ),
  (
    "crates/app/src/pages/mistakes/mistakes_page.rs",
    "多行页头工具条（§4 表里登记的对齐行）",
  ),
  (
    "crates/app/src/pages/qsl_labels/qsl_labels_page.rs",
    "打印说明行",
  ),
];

/// 清单内的命中数 / 文件数（复核日期 2026-10-16）。漂移了就要人看一眼。
const WIDTH_EXPECT_HITS: usize = 11;
const WIDTH_EXPECT_FILES: usize = 9;

/// §11 ⑧：页面里写页面级宽度容器（`max-w-2xl…6xl`，或类名开头的 `container`）。
///
/// 与手工命令同口径：只认 `class="…"` 字面量；含 `sm:max-w` 的行、`<Dialog` 的行放过
/// （`dialog` 宽度是控件自身的、断点覆盖不是页面宽度容器）。`max-w-[210mm]` 这类打印纸尺寸
/// 也不在范围内。
///
/// 曾经连 `min-w-` 也放过：`class="min-w-0 max-w-4xl"` 于是能整行溜过去 —— 现在只认
/// 上面两个明确的例外。
fn has_page_width_container(line: &str) -> bool {
  if line.contains("sm:max-w") || line.contains("<Dialog") {
    return false;
  }
  const WIDTHS: &[&str] = &[
    "max-w-2xl",
    "max-w-3xl",
    "max-w-4xl",
    "max-w-5xl",
    "max-w-6xl",
  ];
  class_literal_values(line).iter().any(|value| {
    WIDTHS.iter().any(|w| value.contains(w))
      || value
        .split_whitespace()
        .next()
        .is_some_and(|t| t == "container")
  })
}

/// 一行里所有 `class="…"` 字面量的取值。
fn class_literal_values(line: &str) -> Vec<String> {
  let mut out = Vec::new();
  let mut from = 0usize;
  while let Some(pos) = line[from..].find("class=\"") {
    let start = from + pos + "class=\"".len();
    let end = line[start..].find('"').map_or(line.len(), |i| start + i);
    out.push(line[start..end].to_owned());
    from = end;
  }
  out
}

/// 剥掉行注释，得到「参与判定的代码部分」。
///
/// `/*` 与 `*` 开头的整行按块注释续行处理（`*slot = …` 这种解引用语句因此不参与扫描，
/// 代价是漏报 —— 它本来也不该出现类名字符串）。
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

/// §11 ⑧ 的封闭清单：清单外的命中要修；清单内的计数漂移要人确认（同时改 §4 与本表）。
fn check_width_containers(root: &Path) -> bool {
  let mut inside_hits = 0usize;
  let mut inside_files: BTreeSet<String> = BTreeSet::new();
  let mut outside_hits = 0usize;
  let mut report: Vec<String> = Vec::new();

  for dir in [
    "crates/app/src/pages",
    "crates/app/src/components",
    "crates/app/src/app",
  ] {
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
      let Ok(text) = std::fs::read_to_string(path) else {
        continue;
      };
      for (i, line) in text.lines().enumerate() {
        let code = code_part(line);
        if code.is_empty() || !has_page_width_container(code) {
          continue;
        }
        if WIDTH_ALLOWED.iter().any(|(p, _)| *p == rel) {
          inside_hits += 1;
          inside_files.insert(rel.clone());
        } else {
          outside_hits += 1;
          if report.len() < MAX_REPORT {
            report.push(format!("    {rel}:{}  {}", i + 1, code.trim()));
          }
        }
      }
    }
  }

  let drift = (inside_hits, inside_files.len()) != (WIDTH_EXPECT_HITS, WIDTH_EXPECT_FILES);
  if outside_hits == 0 && !drift {
    println!(
      "✓ 页面级宽度容器（`max-w-2xl…6xl` / `container`）：清单内 {inside_hits} 处 / {} 文件（= §4）",
      inside_files.len()
    );
    return true;
  }
  if drift {
    println!(
      "✗ 页面级宽度容器：清单内计数变了（{} 处 / {} 文件，期望 {} 处 / {} 文件）",
      inside_hits,
      inside_files.len(),
      WIDTH_EXPECT_HITS,
      WIDTH_EXPECT_FILES
    );
    let list = WIDTH_ALLOWED
      .iter()
      .map(|(p, what)| format!("{p}（{what}）"))
      .collect::<Vec<_>>()
      .join("、");
    println!("    清单：{list}");
  }
  if outside_hits > 0 {
    println!("✗ 页面级宽度容器：清单外 {outside_hits} 处");
    for line in &report {
      println!("{line}");
    }
    println!(
      "    → 内容页只有一种宽度（§4）：用 `PageHeader` + `PageContainer`；确实要新增对齐行，就在 §4 清单与本工具的白名单里同时登记"
    );
  }
  false
}

/// 跑全部规则：全部 0 违规返回 `Ok`，否则打印证据并 `bail`。
pub fn check(root: &Path) -> Result<()> {
  println!(
    "界面规范守卫：{} 条规则 + 1 份封闭清单（对应 docs/ui-components.md §11）",
    RULES.len()
  );
  let mut failed: Vec<String> = Vec::new();

  for rule in RULES {
    let mut found = 0usize;
    let mut report: Vec<String> = Vec::new();
    for dir in rule.roots {
      let base = root.join(dir);
      for entry in WalkDir::new(&base)
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
        if rule.skip.iter().any(|s| rel.contains(s)) {
          continue;
        }
        let Ok(text) = std::fs::read_to_string(path) else {
          continue;
        };
        for (i, line) in text.lines().enumerate() {
          let code = code_part(line);
          if code.is_empty() || !(rule.hits)(code) {
            continue;
          }
          found += 1;
          if report.len() < MAX_REPORT {
            report.push(format!("    {rel}:{}  {}", i + 1, code.trim()));
          }
        }
      }
    }

    if found == 0 {
      println!("✓ {}", rule.name);
    } else {
      println!("✗ {}：{found} 处", rule.name);
      for line in &report {
        println!("{line}");
      }
      if found > report.len() {
        println!("    …（另有 {} 处，先修上面这些）", found - report.len());
      }
      println!("    → {}", rule.advice);
      failed.push(format!("{}（{found} 处）", rule.name));
    }
  }

  if !check_width_containers(root) {
    failed.push("页面级宽度容器（§11 ⑧）".to_owned());
  }

  if !failed.is_empty() {
    bail!("界面规范守卫未通过：{}", failed.join("、"));
  }
  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn rounded_class_only_matches_inside_string_literals() {
    // 标识符（dbm_dbuv.rs 的局部变量）不是类名
    assert!(!has_rounded_class(
      "  let rounded: f64 = format!(\"{v:.5e}\").parse().unwrap_or(v);"
    ));
    assert!(!has_rounded_class(
      "  if rounded.abs() < 1e-5 { 0.0 } else { rounded }"
    ));
    // 类是写在字面量里的；`\"` 转义（HTML 字符串）仍算同一个字面量
    assert!(has_rounded_class("  class=\"px-2 rounded-xl border\""));
    assert!(has_rounded_class(
      "    \"<mark class=\\\"rounded-sm bg-primary/20\\\">\""
    ));
    assert!(has_rounded_class(
      "  let x = format!(\"focus:rounded-md {}\", extra);"
    ));
  }

  #[test]
  fn string_contents_skips_escapes_and_keeps_separate_literals_apart() {
    assert_eq!(string_contents("a \"b c\" d"), "b c ");
    assert_eq!(string_contents("\"a\\\"b\""), "a\\\"b ");
    assert_eq!(string_contents("no literals here"), "");
  }

  #[test]
  fn rounded_token_needs_identifier_boundaries() {
    assert!(has_rounded_token("class=\"px-2 rounded-xl border\""));
    assert!(has_rounded_token("focus:rounded-md focus:bg-primary"));
    assert!(has_rounded_token("\"rounded\""));
    assert!(!has_rounded_token("let roundedness = 1;"));
    assert!(!has_rounded_token("let x_rounded_y = 1;"));
    assert!(!has_rounded_token("class=\"pxl-card border bg-card\""));
  }

  #[test]
  fn comments_are_not_scanned() {
    assert_eq!(code_part("  // class=\"rounded-xl\""), "  ");
    assert_eq!(code_part("  /// 原先写的是 `rounded-full`"), "  ");
    assert_eq!(code_part("  /* rounded-xl */"), "");
    assert_eq!(code_part("  *slot = -*slot;"), "");
    assert_eq!(
      code_part("  class=\"rounded-xl\" // 说明"),
      "  class=\"rounded-xl\" "
    );
  }

  #[test]
  fn other_rules_match_expected_text() {
    assert!(has_native_control("    <input type=\"file\" />"));
    assert!(!has_native_control("    <FileInput on_files=cb />"));
    assert!(has_fill_token_as_text(
      "class=\"text-[color:var(--pxl-gold)]\""
    ));
    assert!(!has_fill_token_as_text(
      "class=\"text-[color:var(--pxl-ink)]\""
    ));
    assert!(has_blur_or_gradient("class=\"backdrop-blur-sm\""));
    assert!(has_blur_or_gradient("class=\"blur-lg\""));
    assert!(has_blur_or_gradient("class=\"drop-shadow-md\""));
    // Tailwind v4 的 `blur-xs` 与裸 `blur` / 带变体的 `hover:blur` 曾经溜得过去（§11 ⑥）
    assert!(has_blur_or_gradient("class=\"blur-xs\""));
    assert!(has_blur_or_gradient("class=\"blur\""));
    assert!(has_blur_or_gradient("class=\"hover:blur\""));
    // 内联 `filter: drop-shadow(…)` 不是工具类，模糊 0 的那处（bubble.rs）也不算
    assert!(!has_blur_or_gradient(
      "style=\"filter: drop-shadow(0 1px 0 rgba(0,0,0,.06));\""
    ));
  }

  #[test]
  fn soft_arbitrary_shadow_sees_the_blur_slot() {
    // 仓库里现有的三处（页头 / 移动菜单 / 底部栏）都是像素硬阴影
    assert!(!has_soft_arbitrary_shadow(
      "class=\"border-b-2 bg-card shadow-[0_2px_0_0_var(--pxl-shadow)]\""
    ));
    assert!(!has_soft_arbitrary_shadow(
      "class=\"shadow-[0_-2px_0_0_var(--pxl-shadow)]\""
    ));
    assert!(!has_soft_arbitrary_shadow(
      "class=\"shadow-[inset_2px_2px_0_0_#000]\""
    ));
    assert!(!has_soft_arbitrary_shadow(
      "class=\"shadow-[0_2px_var(--pxl-shadow)]\""
    ));
    // 命名档位由 tokens.css 定义成硬阴影，不归这条规则管
    assert!(!has_soft_arbitrary_shadow(
      "class=\"shadow-lg shadow-primary/25\""
    ));
    // 带模糊半径的要拦
    assert!(has_soft_arbitrary_shadow(
      "class=\"shadow-[0_4px_12px_rgba(0,0,0,.4)]\""
    ));
    assert!(has_soft_arbitrary_shadow(
      "class=\"shadow-[0_0_10px_#000]\""
    ));
    assert!(has_soft_arbitrary_shadow(
      "class=\"shadow-[inset_0_2px_4px_#000]\""
    ));
    // 多层阴影：任一段有模糊就算
    assert!(has_soft_arbitrary_shadow(
      "class=\"shadow-[0_2px_0_0_#000,0_0_12px_rgba(0,0,0,.5)]\""
    ));
  }

  #[test]
  fn class_tokens_split_variants_at_the_outermost_colon() {
    let t = class_tokens(
      r#"class="dark:hover:bg-red-500/10 data-[state=open]:font-medium [&:not(:disabled)]:ring-2 px-2""#,
    );
    let got: Vec<(&str, &str)> = t.iter().map(|(v, b)| (v.as_str(), b.as_str())).collect();
    assert_eq!(
      got,
      vec![
        ("dark:hover:", "bg-red-500/10"),
        ("data-[state=open]:", "font-medium"),
        ("[&:not(:disabled)]:", "ring-2"),
        ("", "px-2"),
      ]
    );
  }

  #[test]
  fn font_weight_rule_sees_variants_but_not_font_normal() {
    assert!(has_font_weight(r#"class="text-sm font-semibold""#));
    assert!(has_font_weight(r#"class="group-hover:font-medium""#));
    assert!(!has_font_weight(r#"class="font-normal font-mono""#));
  }

  #[test]
  fn scale_rule_ignores_static_rotation() {
    assert!(has_scale_motion(r#"class="active:scale-95""#));
    assert!(has_scale_motion(r#"class="hover:scale-[1.02]""#));
    assert!(has_scale_motion(r#"class="animate-in zoom-in-95""#));
    assert!(has_scale_motion(r#"class="transition hover:rotate-12""#));
    assert!(!has_scale_motion(r#"class="rotate-180 transition""#));
    assert!(!has_scale_motion(r#"class="scale-text""#));
  }

  #[test]
  fn motion_rule_accepts_token_durations_only() {
    assert!(has_loose_motion(r#"class="transition-all""#));
    assert!(has_loose_motion(r#"class="animate-in duration-300""#));
    assert!(has_loose_motion(
      r#"class="data-[state=open]:duration-200""#
    ));
    assert!(!has_loose_motion(
      r#"class="duration-(--motion-normal) transition""#
    ));
    assert!(!has_loose_motion(r#"class="transition-colors""#));
  }

  #[test]
  fn empty_attribute_rule_ignores_empty_string_arguments() {
    assert!(has_empty_attribute(r#"<span class="">{v}</span>"#));
    assert!(has_empty_attribute(r#"<div class="px-2" style="">x</div>"#));
    assert!(has_empty_attribute(r#"<p class=""></p>"#));
    // 空字符串作为实参 / 值不是空属性
    assert!(!has_empty_attribute(r#"cn(&["", "px-2"])"#));
    assert!(!has_empty_attribute(r#"let s = format!("");"#));
    assert!(!has_empty_attribute(r#"<p class="text-sm">{s}</p>"#));
  }

  #[test]
  fn small_icon_rule_looks_at_icon_only() {
    assert!(has_small_icon(
      r#"<Icon kind=IconKind::X class="h-4 w-4" />"#
    ));
    assert!(has_small_icon(
      r#"<Icon class="size-4 text-muted-foreground" />"#
    ));
    assert!(!has_small_icon(r#"<Icon class="size-6" />"#));
    assert!(!has_small_icon(r#"<span class="h-4 w-4" />"#));
  }

  #[test]
  fn arbitrary_breakpoint_rule_ignores_table_min_widths() {
    assert!(has_arbitrary_breakpoint(r#"class="min-[560px]:flex""#));
    assert!(!has_arbitrary_breakpoint(r#"class="min-w-[560px]""#));
    assert!(!has_arbitrary_breakpoint(r#"class="sm:flex""#));
  }

  #[test]
  fn small_heading_rule_sees_h2_only() {
    assert!(has_small_heading(r#"<h2 class="mr-auto text-sm">"#));
    assert!(!has_small_heading(
      r#"<h2 class=card_title_class("mr-auto")>"#
    ));
    assert!(!has_small_heading(r#"<p class="text-sm">"#));
  }

  #[test]
  fn bare_h1_rule_accepts_title_and_sr_only() {
    assert!(has_bare_h1(r#"<h1 class="text-base leading-tight">"#));
    assert!(!has_bare_h1(r#"<h1 class="pxl-title text-lg">"#));
    assert!(!has_bare_h1(r#"<h1 class="sr-only">"#));
    assert!(!has_bare_h1(r#"<h2 class="text-base">"#));
  }

  #[test]
  fn destructive_as_text_rule_keeps_foreground_token() {
    assert!(has_destructive_as_text(
      r#"class="text-xs text-destructive""#
    ));
    assert!(has_destructive_as_text(
      r#"class="hover:text-destructive/80""#
    ));
    assert!(!has_destructive_as_text(
      r#"class="text-destructive-foreground""#
    ));
    assert!(!has_destructive_as_text(
      r#"class="bg-destructive/10 text-hp-text border-destructive/40""#
    ));
  }

  #[test]
  fn ring_rule_does_not_confuse_dashed_outline_colors() {
    assert!(has_solid_ring(r#"class="focus-visible:ring-2""#));
    assert!(has_solid_ring(
      r#"class="focus-visible:ring-ring/60 ring-offset-2""#
    ));
    assert!(has_solid_ring(r#"class="ring""#));
    assert!(!has_solid_ring(
      r#"class="focus-visible:outline-2 focus-visible:outline-dashed focus-visible:outline-ring""#
    ));
    assert!(!has_solid_ring(r#"class="string-width spring-load""#));
  }

  #[test]
  fn font_px_rule_wants_multiples_of_six() {
    assert!(has_off_grid_font_px(r#"class="text-[10px]""#));
    assert!(has_off_grid_font_px(
      r#"class="text-[9px] text-muted-foreground""#
    ));
    assert!(!has_off_grid_font_px(r#"class="text-[12px]""#));
    assert!(!has_off_grid_font_px(
      r#"class="text-[clamp(1.1rem,5.5vw,2rem)]""#
    ));
    assert!(!has_off_grid_font_px(r#"class="text-xs""#));
  }

  #[test]
  fn ad_hoc_hover_lift_rule_allows_centering_and_one_art_pixel() {
    // 追台卡当年的写法
    assert!(has_ad_hoc_hover_lift(
      r#"class="block transition hover:-translate-y-1 hover:border-primary/40 hover:shadow-lg hover:shadow-primary/10""#
    ));
    assert!(has_ad_hoc_hover_lift(
      r#"class="shadow-lg shadow-primary/25""#
    ));
    assert!(has_ad_hoc_hover_lift(r#"class="active:translate-y-2""#));
    // 居中、2px 以内的位移、命名档位的静态阴影都合规
    assert!(!has_ad_hoc_hover_lift(
      r#"class="absolute top-1/2 -translate-y-1/2 -translate-x-1/2""#
    ));
    assert!(!has_ad_hoc_hover_lift(
      r#"class="group-hover:-translate-y-0.5""#
    ));
    assert!(!has_ad_hoc_hover_lift(r#"class="shadow-sm shadow-xs""#));
    assert!(!has_ad_hoc_hover_lift(
      r#"class="shadow-[0_2px_0_0_var(--pxl-shadow)]""#
    ));
  }

  #[test]
  fn palette_rule_targets_default_scales_only() {
    assert!(has_raw_palette_color(
      r#"class="bg-emerald-500/10 text-red-600""#
    ));
    assert!(has_raw_palette_color(r#"class="dark:text-amber-300""#));
    assert!(has_raw_palette_color(
      r#"class="fill-sky-600 stroke-sky-600""#
    ));
    // 语义 token 与打印变体合规
    assert!(!has_raw_palette_color(
      r#"class="bg-win/10 text-win-text border-destructive""#
    ));
    assert!(!has_raw_palette_color(r#"class="print:border-zinc-300""#));
    assert!(!has_raw_palette_color(
      r#"class="text-muted-foreground bg-card""#
    ));
    // 名字撞车但不是色阶
    assert!(!has_raw_palette_color(
      r#"class="text-xs border-2 bg-primary/70""#
    ));
  }

  #[test]
  fn page_width_container_matches_the_documented_shape() {
    assert!(has_page_width_container(
      "  <div class=\"mx-auto flex max-w-5xl flex-wrap items-center gap-2 px-4 pb-3\">"
    ));
    assert!(has_page_width_container(
      "  <div class=\"container relative mx-auto px-1\">"
    ));
    // §4 的例外与排除项。`min-w-` 不再整行放过：`min-w-0 max-w-5xl` 仍然是页面级宽度容器
    // （这条曾能绕过去，§11 ⑧）。
    assert!(has_page_width_container(
      "  <div class=\"mx-auto min-w-0 max-w-5xl px-4\">"
    ));
    assert!(!has_page_width_container(
      "  <Dialog class=\"max-w-5xl sm:max-w-lg px-4\">"
    ));
    assert!(!has_page_width_container(
      "  <div class=\"print-sheet mx-auto mt-6 max-w-[210mm] bg-white\">"
    ));
    // 用变量 / 组件传宽度的不在这条规则里（PageContainer 自己就是 `class=cn(&[…])`）
    assert!(!has_page_width_container(
      "    <div class=cn(&[\"mx-auto max-w-5xl px-4\", &class])>"
    ));
  }
}
