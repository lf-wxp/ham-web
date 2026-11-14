use leptos::prelude::*;

use crate::cn::cn;
use crate::i18n::t;
use crate::icons::{Icon, IconKind};
use crate::ui::{Button, Size, Variant};

use super::band_tag::BandTag;

/// 吸顶页头骨架类名：吸顶位置、层级、墨色下沿、底色。
///
/// 全站吸顶页头**只有这一份骨架**：`PageHeader` 用它，`print_page` 这类「刻意的例外」
/// （纸宽坐标系，见 `docs/ui/pixel.md` §4）也用它 —— 偏移只在这里改一处，
/// 不会出现组件与手写骨架各写各的、吸顶偏移漂移（见 `print_page` 曾与导航错位 2px）。
pub const PAGE_HEADER_BAR: &str = "sticky top-[66px] z-20 border-b-2 border-ink bg-card";

/// 窄屏折叠区外壳：`grid-rows` 0fr → 1fr 做高度过渡（与 `.ui-collapse` 同一手法，
/// 但这里要的是「`md` 起彻底不参与布局」，所以用 `md:contents` 而不是另立一个 CSS 类）。
///
/// `visibility` 一起进过渡是有意的：`visibility` 是离散动画，收起时在**动画结束**才切
/// `hidden`（正文不会先消失再收高度），展开时立即可见；同时它也挡掉了被折起来的按钮的
/// Tab 焦点与读屏。`md:visible` 让桌面端（`display: contents`）照常显示。
const FOLD: &str = "grid grid-rows-[0fr] invisible transition-[grid-template-rows,visibility] duration-[var(--motion-normal)] ease-[var(--ease-out-expo)] data-[open=true]:grid-rows-[1fr] data-[open=true]:visible md:visible md:contents";
/// 折叠区正文：`overflow: hidden` 让 0fr 那一行真的收成 0（否则子项按 min-content 撑住）；
/// `md:contents` 让它连盒子一起消失，内容回到折叠前的原位置。
const FOLD_BODY: &str = "min-h-0 overflow-hidden md:contents";
/// 操作区那一块的折叠正文：多出一圈 6px 的「焦点环余量」。
///
/// 像素风的焦点环是 2px 虚线外加 3px 偏移，共 5px 伸在控件**外面**，而 `overflow: hidden`
/// 的裁剪边界就是正文盒子本身 —— 控件贴着边时，左 / 右 / 下三侧的环会被裁掉。负外边距把
/// 盒子向两侧撑出 6px（落在容器的 `px-4` 里，不会顶出视口），内层补等量内边距把控件推回
/// 原位，版式不变。内边距必须放在**内层**：放在这个 0fr 的盒子上，收起时它的内边距会让
/// 这一行撑不到 0。工具条那块（`children`）自带 `px-4 pb-3`，余量已经够，不用这一圈。
const FOLD_BODY_ACTIONS: &str = "-mx-1.5 min-h-0 overflow-hidden md:contents";

/// 页面头部：吸顶标题栏（标题 + 副标题），右侧可放操作区（如搜索框、筛选器）。
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
/// **窄屏（`< md`）只有标题留在外面**，副标题 / 操作区 / 工具条全折进标题行右侧的
/// 「更多选项」，默认收起：页头是吸顶的，而这几块横向铺开的控件窄屏要折成三四行，
/// 66px 顶栏之外会再钉死 60～270px（390×844 实测 `/browse` 折叠前 215px、
/// `/qsl-labels` 331px）。折叠后页头只剩一行标题（45～85px）。
///
/// `md` 起折叠区退化成 `display: contents`：操作区回到标题行右侧、工具条回到页头下方、
/// 副标题回到标题底下，桌面端版式与折叠前逐像素一致。判定与实测见
/// `docs/ui/pixel.md` §4「窄屏折叠页头」。
///
/// 不该折的两种东西各有一个出口：`children` 里的**实时状态**传 `keep_rows`，`actions` 里的
/// **唯一主操作**传 `keep_actions`。判据与现有使用者见同一节。
///
/// 元素上带 `data-page-header` 标记：`StickyPanel` 靠它定位页头（页面内容里也有
/// `<header>`，按元素名会量错）。改这里时别把标记去掉。
#[component]
pub fn PageHeader(
  #[prop(into)] title: Signal<String>,
  #[prop(into)] subtitle: Signal<String>,
  #[prop(optional, into)] class: String,
  #[prop(optional, into)] actions: Option<ViewFn>,
  #[prop(optional)] children: Option<Children>,
  /// 窄屏也把 `children` 留在折叠区**外面**（常显）：只给它承载**实时状态**时用
  /// （`/daily-challenge` 的倒计时 + 交卷 —— 折进去就看不见还剩多少时间）。
  /// 其余页面不要开：页头正是因为它太占地方才折的。
  #[prop(optional)]
  keep_rows: bool,
  /// 窄屏也把 `actions` 留在折叠区**外面**（常显，排在标题行下面）：只给页面**唯一的主操作**、
  /// 运行中要随时能点的开关用（`/sdr-waterfall` 的「开始 / 停止」—— 折进去就得先点
  /// 「更多选项」才停得下来）。搜索 / 筛选 / 导出 / 跳转链接这类工具不要开。
  /// 折叠区里这时只剩副标题。
  #[prop(optional)]
  keep_actions: bool,
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

  // 折叠态：窄屏默认收起，与页面无关。
  let fold = RwSignal::new(false);
  // 「桌面一套、窄屏一套」只建当前宽度那一套（`docs/engineering/performance.md` §2；与 `/bands` 同一条
  // 断点、同一个 hook）：副标题只有一份、「更多选项」只在窄屏存在。首帧按 `matchMedia` 同步取值，不闪；
  // 其余版式（折叠区、操作区归位）仍由 `md:` 类名负责，不依赖这个信号。
  let wide = crate::media::use_media_query(crate::media::MD_UP);
  // 操作区常显（`keep_actions`）时它不进折叠区，折叠区里只剩副标题。
  let (folded_actions, kept_actions) = if keep_actions {
    (None, actions)
  } else {
    (actions, None)
  };
  // 窄屏排成一行（与折叠前同一套 wrap 节奏），`md:contents` 让操作区本体的根节点直接成为
  // 标题行的 flex 项 —— 桌面端版式与折叠前完全一致。折叠与常显两处共用这一份外壳。
  let actions_row = |a: ViewFn| {
    view! { <div class="mt-2 flex flex-wrap items-center gap-2 md:contents">{a.run()}</div> }
  };
  // `aria-controls` 只列**真的渲染了**的面板 id：工具条那块没有 children、或它常显
  // （`keep_rows`）时不折叠面板。操作区那块的折叠区恒在（至少装着副标题）。
  let actions_id = crate::util::unique_id("page-head-actions");
  let rows_id =
    (children.is_some() && !keep_rows).then(|| crate::util::unique_id("page-head-rows"));
  let controls = std::iter::once(actions_id.clone())
    .chain(rows_id.clone())
    .collect::<Vec<_>>()
    .join(" ");
  let chevron = Signal::derive(move || {
    cn(&[
      "h-3 w-3 shrink-0 transition-[rotate] duration-[var(--motion-normal)] ease-[var(--ease-out-expo)]",
      if fold.get() { "rotate-180" } else { "" },
    ])
  });

  view! {
    // `pxl-tick-edge`：下沿的刻度线画在底部内边距里（`radio.css`），不占布局。
    <header data-page-header class=cn(&[PAGE_HEADER_BAR, "pxl-tick-edge", &class])>
      // 窄屏是一列（标题行 / 折叠区），`md` 起回到「一行 + 折行」：
      // `gap-0` 是必须的 —— 收起态的折叠区高度为 0，行间距会把它变成一条白边。
      <div class="mx-auto flex max-w-5xl flex-col gap-0 px-4 py-3 md:flex-row md:flex-wrap md:items-center md:gap-3">
        <div class="flex items-center gap-3 md:contents">
          // 主色左轨 + 抬到页级字号的标题：内页的「这是哪一页」全靠这一行，
          // 12px 的 `text-sm` 标题在 18px 正文旁边读起来只是又一行小字。
          // `-ml-3` 让标题文字与下方的工具条 / 正文左边缘对齐，主色轨挂在容器的内边距里。
          // `min-w-0`：窄屏这一行还塞着「更多选项」，标题块的 min-content 撑住就会把按钮
          // 顶出视口（320px 的英文长标题实测溢出 28px）；允许它收窄、让文字自己折行。
          <div class="mr-auto -ml-3 min-w-0 border-l-4 border-primary pl-3">
            <h1 class="pxl-title text-base leading-tight sm:text-lg">{move || title.get()}</h1>
            // 副标题只建**一份**：`md` 起在标题底下（这里），窄屏在折叠区里。
            // 不能两份都建、再用 `hidden md:block` / `md:hidden` 藏一份 —— 同一段文字在 DOM 里出现两次，
            // 按文字定位的用例会撞上严格模式，页内搜索脚本也会多命中一份。
            // 桌面端副标题前挂一块「波段铭牌」（这一页属于考试中心 / 知识库 / 工具）：与副标题同一行，
            // 不加页头高度；窄屏页头只留标题，不挂。
            {move || {
              wide
                .get()
                .then(|| {
                  view! {
                    <div class="flex flex-wrap items-center gap-x-2 text-xs text-muted-foreground">
                      <BandTag />
                      <span>{move || subtitle.get()}</span>
                    </div>
                  }
                })
            }}
          </div>
          // 「更多选项」同理：桌面端根本不建（不是建出来再 `md:hidden`）。
          {move || {
            (!wide.get())
              .then(|| {
                view! {
                  <Button
                    variant=Variant::Text
                    size=Size::Xs
                    class="shrink-0"
                    aria_expanded=Signal::derive(move || fold.get())
                    aria_controls=controls.clone()
                    on_click=Callback::new(move |_| fold.update(|v| *v = !*v))
                  >
                    {move || t("common.more-options")}
                    <Icon kind=IconKind::ChevronDown class=chevron />
                  </Button>
                }
              })
          }}
        </div>
        <div
          id=actions_id
          class=FOLD
          data-open=move || fold.get().to_string()
        >
          <div class=FOLD_BODY_ACTIONS>
            <div class="px-1.5 pt-2 pb-1.5 md:contents">
              {move || {
                (!wide.get())
                  .then(|| {
                    view! { <div class="text-xs text-muted-foreground">{move || subtitle.get()}</div> }
                  })
              }}
              {folded_actions.map(actions_row)}
            </div>
          </div>
        </div>
        {kept_actions.map(actions_row)}
      </div>
      {children
        .map(|c| match rows_id.clone() {
          Some(id) => {
            view! {
              <div id=id class=FOLD data-open=move || fold.get().to_string()>
                <div class=FOLD_BODY>
                  <div class="pt-2 md:contents">{c()}</div>
                </div>
              </div>
            }
              .into_any()
          }
          // `keep_rows`：常显，不套折叠区（`children` 与折叠前一样直接挂在页头里）。
          None => view! { {c()} }.into_any(),
        })}
    </header>
  }
  .into_any()
}
