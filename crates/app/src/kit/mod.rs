//! 设计系统的派发入口。
//!
//! 页面只认这一层：它决定这次渲染走像素风（`ui/` 与页面里的像素分支）还是现代风
//! （`kit_modern`）。风格判断统一从这里取（`is_modern` / `is_modern_at_mount` 与下面的
//! 类名工厂），**不要在页面里直接读 `ui:uiStyle` 或 `data-ui-style`**（`check-structure`
//! 守着）。
//!
//! 派发点现在分布在外壳、共享控件（`ui/`）与需要两套 DOM 的页面里；
//! 只换类名的区块优先用下面的类名工厂，而不是各写一份 `if`。取舍见 `docs/ui/decisions.md` §4。

use crate::theme::use_display_prefs;

/// 当前界面风格是否为现代风（**订阅信号**）。
///
/// 只在响应式闭包（`view!` 的属性闭包 / `move ||`）里调用，让那块内容随切换重渲染；
/// 组件体里的派发请用 [`is_modern_at_mount`]，否则 dev 构建会告警。
pub fn is_modern() -> bool {
  use_display_prefs().is_modern()
}

/// 构造期派发：组件体里 `if is_modern_at_mount() { return 现代实现 }` 用这个。
///
/// 不订阅信号 —— 组件体只会跑一次，风格切换靠 `MainContent` 重建整棵路由子树覆盖，
/// 在那里订阅只会得到一条「在跟踪上下文之外读信号」的告警（见 `theme.rs` 同名方法）。
pub fn is_modern_at_mount() -> bool {
  use_display_prefs().is_modern_at_mount()
}

/// 内容卡片的容器类：现代风 `md-panel`，像素风 `pxl-card`。
///
/// 给「还没来得及写现代实现」的区块一个统一的换装入口 —— 页面里不要再手写
/// `class="pxl-card border bg-card"`，否则现代风只能靠共享工具类的取值映射兜底。
#[must_use]
pub fn card_class() -> &'static str {
  if is_modern_at_mount() {
    "md-panel md-panel-flush"
  } else {
    "pxl-card border bg-card"
  }
}

/// 允许标签换行的按钮（附加到 `Button` 的 `class`）：现代 `md-btn-wrap`，像素风为空。
///
/// 现代风的 `.md-btn` 是固定高度、强制单行的药丸，标签一长就会在窄屏里比所在行还宽，把整页撑出横向滚动
/// （西语 320px 下 `/backup` 的「Restaurar desde remoto (sobrescribir)」、`/nec` 的折叠标题就是这样）。
/// 只给**确实会遇到长标签**的按钮加；短标签的按钮加了也不变（单行时高度仍是控件高）。
/// 像素风按钮的字体与内距更窄，没出过这个问题，版式冻结，所以返回空。
#[must_use]
pub fn btn_wrap_class() -> &'static str {
  if is_modern_at_mount() {
    "md-btn-wrap"
  } else {
    ""
  }
}

/// 需要自带内距的卡片：现代 `md-panel` 已含 22px，像素风补 `p-4`。
#[must_use]
pub fn card_pad_class() -> &'static str {
  if is_modern_at_mount() {
    "md-panel"
  } else {
    "pxl-card border bg-card p-4"
  }
}

/// 卡片标题类：现代风 `md-panel-title`，像素风 `pxl-section-title`。
#[must_use]
pub fn card_title_class() -> &'static str {
  if is_modern_at_mount() {
    "md-section-title"
  } else {
    "pxl-section-title"
  }
}

/// 面板内的小节标题（无标题栏）：现代与卡片标题同一档，像素风保留无底纹的 `pxl-section-head`。
#[must_use]
pub fn section_head_class() -> &'static str {
  if is_modern_at_mount() {
    "md-section-title"
  } else {
    "pxl-section-head"
  }
}

/// 统计格里的数字读数：现代风走等宽大号（与 `ui::Stat` 同档），像素风是点阵读数。
#[must_use]
pub fn stat_class() -> &'static str {
  if is_modern_at_mount() {
    "md-stat-value"
  } else {
    "pxl-stat"
  }
}

/// 表格行内的小进度条：现代风是圆角渐变，像素风是方块主色。
#[must_use]
pub fn mini_bar_class() -> &'static str {
  if is_modern_at_mount() {
    "md-mini-bar"
  } else {
    "h-2 bg-primary/70"
  }
}

/// 战斗 / 判分反馈条：现代风是安静的内嵌块，像素风是弹窗。
#[must_use]
pub fn feedback_class() -> &'static str {
  if is_modern_at_mount() {
    "md-feedback motion-pop flex flex-wrap items-center gap-x-4 gap-y-1"
  } else {
    "pxl-window motion-pop flex flex-wrap items-center gap-x-4 gap-y-1 p-3 text-sm"
  }
}

/// 带附加工具类的卡片：`card_class_with("space-y-3 p-4")`。
///
/// 页面里不要再手写 `class="pxl-card border bg-card p-4"` —— 那样现代风只能靠
/// 共享工具类的映射兜底（`style/modern/legacy-utilities.css`）。附加类（间距、内距、布局）两种风格共用。
#[must_use]
pub fn card_class_with(extra: &str) -> String {
  let extra = extra.trim();
  if is_modern_at_mount() {
    crate::cn::cn(&["md-panel md-panel-flush", extra])
  } else {
    crate::cn::cn(&["pxl-card border bg-card", extra])
  }
}

/// 「复选框 / 单选 + 文字」的一行：现代风 `.md-check-row`，像素风沿用工具类。
#[must_use]
pub fn check_row_class() -> &'static str {
  if is_modern_at_mount() {
    "md-check-row"
  } else {
    "flex cursor-pointer items-center gap-2 pointer-coarse:min-h-11"
  }
}

/// 表格单元格：像素风靠 `border px-3 py-2` 画网格；现代风的边线与内距由 `.md-table`
/// 统一提供（紧凑档也是在那里收的），单元格上再挂工具类会把 `padding` 抢回去。
///
/// `extra` 只放与风格无关的布局类（`align-top` / `text-right` / `tabular-nums`）。
#[must_use]
pub fn table_cell_class(extra: &str) -> String {
  let extra = extra.trim();
  if is_modern_at_mount() {
    crate::cn::cn(&[extra])
  } else {
    crate::cn::cn(&["border px-3 py-2", extra])
  }
}

/// 正文里的脚注引用（如波段表的「1」「5.162A」）：现代风 `md-footnote`，像素风为空。
///
/// 现代风靠 `::after` 把 18px 高的行内小字撑到 44 的触控命中区（见 `kit.css`）；
/// 像素风的版式冻结，命中区不做放大。这个类名不能被写死在共享视图里 ——
/// 像素风 DOM 里出现 `md-*` 会被 `dual_isolation.spec.ts` 判为跨皮肤污染。
#[must_use]
pub fn footnote_class() -> &'static str {
  if is_modern_at_mount() {
    "md-footnote"
  } else {
    ""
  }
}

/// 正文里的行内链接：现代风 `.md-link`（行动色，悬停加深，不下划线），像素风 `pxl-link`。
///
/// `.md-link` 只管颜色 / 悬停 / 焦点环，**不碰盒模型**。首页入口列表里的「一行」是另一个组件
/// `.md-link-row`（`flex` + 36px 高 + 负外边距）；两者曾共用 `.md-link` 一个类名，于是这里的
/// 全部行内链接都被撑成了 36px 高的 flex 行（见 `docs/ui/modern.md` 反例 #17）。
///
/// `.pxl-link` 在像素风里**没有任何 CSS**（历史上只当标记用，链接样式来自全局 `a`），
/// 但它同样被写进了现代风页面 —— 而现代风没有全局 `a` 规则，于是这些链接看起来
/// 与正文完全一样（`/weather-sat` 的「前往 →」就是这样）。统一换装后才有可点的样子。
#[must_use]
pub fn link_class() -> &'static str {
  if is_modern_at_mount() {
    "md-link"
  } else {
    "pxl-link"
  }
}
