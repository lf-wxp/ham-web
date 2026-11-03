use leptos::prelude::*;

use crate::cn::cn;

/// 吸顶页头骨架类名：吸顶位置、层级、墨色下沿、底色。
///
/// 全站吸顶页头**只有这一份骨架**：`PageHeader` 用它，`print_page` 这类「刻意的例外」
/// （纸宽坐标系，见 `docs/ui/pixel.md` §4）也用它 —— 偏移只在这里改一处，
/// 不会出现组件与手写骨架各写各的、吸顶偏移漂移（见 `print_page` 曾与导航错位 2px）。
pub const PAGE_HEADER_BAR: &str = "sticky top-[66px] z-20 border-b-2 border-ink bg-card";

/// 页面头部：粘性标题栏（标题 + 副标题），右侧可放操作区（如搜索框、筛选器）。
///
/// `title` / `subtitle` 为 [`Signal<String>`]，既接受静态 [`String`]（`From<String>`），
/// 也接受响应式信号（如 `Signal::derive(move || t("common.ellipsis"))`），切语言时即时刷新。
/// 吸顶偏移默认 `top-[66px]`（导航栏 64px 行高 + 2px 底描边；与导航下沿精确对齐，
/// 吸顶瞬间页头与导航之间才没有错位滑动）；`class` 可覆盖偏移
/// （如页面上方还有别的粘性栏时改更大值）。
///
/// `actions` 放在标题行右侧；`children` 渲染在标题行下方，用于「多行页头」
/// （如书签页的分组工具条、错题页的清空确认条）。
///
/// 元素上带 `data-page-header` 标记：`StickyPanel` 靠它定位页头（页面内容里也有
/// `<header>`，按元素名找会量错）。改这里时别把标记去掉。
#[component]
pub fn PageHeader(
  #[prop(into)] title: Signal<String>,
  #[prop(into)] subtitle: Signal<String>,
  #[prop(optional, into)] class: String,
  #[prop(optional, into)] actions: Option<ViewFn>,
  #[prop(optional)] children: Option<Children>,
) -> impl IntoView {
  // 现代风：直接交给 `ModernPageHead` —— 同一份 title / subtitle / actions / children，
  // 只是换成「标题进正文流 + 展示字体 + 1px 墨线」。这样桥接页与迁移页拿到的是
  // **同一个页头组件**，首屏内容起点、页头高度、操作区位置都不会再分叉。
  // `class` 里塞的多是像素侧吸顶偏移（`top-[…]`），现代风不需要，故忽略。
  if crate::kit::is_modern_at_mount() {
    // 可选槽位在这里补成「渲染空视图」的占位：Leptos 的可选 prop 只接受值本身，
    // 不接受 `Option`，所以不能把 `actions` / `children` 直接透传。
    let actions = actions.unwrap_or_else(|| ViewFn::from(|| ().into_any()));
    let children: Children = children.unwrap_or_else(|| Box::new(|| ().into_any()));
    return view! {
      <crate::kit_modern::ModernPageHead title=title subtitle=subtitle actions=actions children=children />
    }
    .into_any();
  }
  view! {
    <header data-page-header class=cn(&[PAGE_HEADER_BAR, &class])>
      <div class="mx-auto flex max-w-5xl flex-wrap items-center gap-3 px-4 py-3">
        // 主色左轨 + 抬到页级字号的标题：内页的「这是哪一页」全靠这一行，
        // 12px 的 `text-sm` 标题在 18px 正文旁边读起来只是又一行小字。
        // `-ml-3` 让标题文字与下方的工具条 / 正文左边缘对齐，主色轨挂在容器的内边距里。
        <div class="mr-auto -ml-3 border-l-4 border-primary pl-3">
          <h1 class="pxl-title text-base leading-tight sm:text-lg">{move || title.get()}</h1>
          <div class="text-xs text-muted-foreground">{move || subtitle.get()}</div>
        </div>
        {actions.map(|a| a.run())}
      </div>
      {children.map(|c| c())}
    </header>
  }
  .into_any()
}
