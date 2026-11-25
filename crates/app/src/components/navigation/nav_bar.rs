//! 顶部导航栏。
//!
//! 结构照现代风顶栏（`kit_modern/nav.rs`）来：品牌在左 → 导航项紧跟其后 → 动作组用
//! `ml-auto` 顶到最右。区别只在皮肤与收纳方式（像素风是三个分组下拉，现代风是一串文字链接）。
//!
//! 顶栏里**没有单独的「首页」项**：左边那枚站标就是首页链接。导航里也没有语言 / 明暗 / 备份 ——
//! 这些配置统一收进齿轮打开的设置对话框（`settings_dialog.rs`），顶栏只留
//! 「搜索 + 设置 + HUD」三个常驻入口。顶栏多摆两个切换按钮，就要在 1024–1280 那档与 HUD 抢位置，
//! 而这些操作在设置里本来就有完整形态（尤其「跟随系统」只有三选一给得出来）。
//!
//! 导航项本身只剩三个分组下拉，开关与面板都在 `group_menu.rs`（那里保留原生 `<button>` 的理由
//! 见 `docs/ui/pixel.md` §10.2）；本文件的汉堡、搜索、齿轮都是 [`Button`]。

use leptos::ev;
use leptos::prelude::*;
use send_wrapper::SendWrapper;
use wasm_bindgen::JsCast;

use ham_web_core::registry::{EXAM_GROUPS, KNOWLEDGE_GROUPS, TOOL_GROUPS};

use crate::components::search_dialog::SearchOpen;
use crate::icons::{Icon, IconKind, PixelSprite};
use crate::ui::{Button, Led, LedState, LedTone, Size, Variant};

use super::band_scale::BandScale;
use super::group_menu::{GroupMenu, MenuAlign};
use super::hud_badge::HudBadge;
use super::settings_dialog::SettingsDialog;

use super::menu::MenuKind;
use super::mobile_menu::{MOBILE_NAV_PANEL_ID, use_mobile_menu};
use super::mobile_nav_panel::MobileNavPanel;
use crate::i18n::t;

#[component]
pub(crate) fn PixelNav() -> impl IntoView {
  // 移动端菜单状态与底部 Tab 共享：底栏跳转时要顺手把可能开着的面板收起来。
  let menu_open = use_mobile_menu().open;
  // 跨过 `md`（手机横屏 / 平板旋转）时收起手机菜单：面板在 `≥md` 只是被 CSS 藏起，状态仍是「开」，
  // 转回竖屏它会自己弹出来，汉堡的 `aria-expanded` 也是过期的。
  let wide = crate::media::use_media_query(crate::media::MD_UP);
  Effect::new(move |_| {
    if wide.get() && menu_open.get_untracked() {
      menu_open.set(false);
    }
  });
  let open_menu = RwSignal::new(None::<MenuKind>);
  let settings_open = RwSignal::new(false);

  // 点击导航外部时关闭桌面端下拉。
  let outside_handle = window_event_listener(ev::click, move |e| {
    if let Some(el) = e
      .target()
      .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
    {
      let inside = el.closest("[data-nav]").ok().flatten().is_some();
      if !inside {
        open_menu.set(None);
      }
    }
  });
  let outside_handle = SendWrapper::new(Some(outside_handle));
  on_cleanup(move || {
    if let Some(h) = outside_handle.take() {
      h.remove();
    }
  });

  // 全局「/」快捷键：非输入框内按下打开站内搜索面板。
  let SearchOpen(search_open) = expect_context::<SearchOpen>();
  let search_handle = window_event_listener(ev::keydown, move |e| {
    if let Some(el) = e
      .target()
      .and_then(|t| t.dyn_into::<web_sys::HtmlElement>().ok())
    {
      let tag = el.tag_name().to_lowercase();
      if tag == "input" || tag == "textarea" || el.is_content_editable() {
        return;
      }
    }
    if e.key() == "/" {
      e.prevent_default();
      search_open.set(true);
    }
  });
  let search_handle = SendWrapper::new(Some(search_handle));
  on_cleanup(move || {
    if let Some(h) = search_handle.take() {
      h.remove();
    }
  });

  view! {
    // 顶栏是一条实心的「HUD 栏」：底色用 `--card`（不透明）+ 2px 墨色下沿 + 硬偏移投影，
    // 不模糊、不渐变；吸顶时下面滚过的内容被它实实在在地盖住，不需要任何磨砂兜底。
    <nav data-nav aria-label=move || t("shell.main-navigation") class="sticky top-0 z-[var(--z-nav)] border-b-2 border-ink bg-card shadow-[0_2px_0_0_var(--pxl-shadow)]">
      // 内容层与正文同一个版心：`max-w-5xl`（值 = `--page-max`，见 `style/input.css`）+ `px-4`，
      // 与 `PageContainer` / `PageHeader` 逐字一致，所以顶栏两端与正文边缘永远对齐。
      // 以前这里用 `container`（宽度随断点长到 1280）且 <xl 铺满整宽，于是顶栏比正文宽出一截；
      // 现在桌面导航只剩三段下拉、右侧只剩两个图标，1200 的版心在三种语言下都放得下
      // （最长的西语整行约 996px）。`relative` 留给导航项的下拉定位。
      <div class="relative mx-auto max-w-5xl px-4">
        // 排布对齐现代风：品牌在最左，导航项紧跟其后（不是居中），右侧动作组用 `ml-auto` 顶到
        // 最右。三块之间不再用 `justify-between` —— 那样导航项会随两侧宽度浮动，换语言或
        // HUD 数字变长时整组横向漂移。
        // 桌面档这一行压到 40px，下面 24px 是换台刻度尺（`BandScale`）；`<1280` 行是 64px、没有刻度。
        // 两档总高都是 66px，这是页头吸顶偏移（`PAGE_HEADER_BAR`）的前提：别只给某一档加第二条行。
        // 规则与回归见 `docs/ui/pixel.md` §3「顶栏总高」。
        //
        // 一行控件等高：`<xl`（行高 64px）全部 40px —— 站标 `size-10`、波段键与 HUD `h-10`、
        // 搜索 / 设置 `Size::Icon`；`xl` 起整行收缩到 40px、全体控件统一 36px（`xl:h-9` / `xl:size-9`，
        // 上下各留 2px）。断点一律用 Tailwind 的 `xl:`，别在 CSS 里另写 px 媒体查询：两套断点在小数宽度
        // 与非 16px 默认字号下会错开。`ui_spec.spec.ts` 的 H3 在手机 / 平板 / 桌面三档量顶边与底边。
        <div class="flex h-16 items-center gap-1 sm:gap-3 xl:h-10 xl:gap-4">
          // 品牌标识
          // 电台顶面板的左端：电源灯 + 电台站标 + 站名（`docs/ui/pixel.md` §3.4）。
          // 站名在手机上从 360px（`2xs`）才出现、用丝印字：320px 放不下，点阵标题字又太宽。
          //
          // 站名按宽度有四种形态（西语最长，768px 下三个波段键就要 370px）：
          // - `<360` 只有站标（点阵标题放不下）；
          // - 360–767 单行丝印站名（`2xs:block sm:hidden`）；
          // - 768–1279 只留站标 —— 这一档要给三个波段键腾位置，站名是唯一舍得的装饰；
          // - `≥1280` 两行标题（点阵标题 + 装饰副标题），桌面版心放得下。
          <a
            href="/"
            class="group flex min-w-0 shrink items-center gap-1.5 sm:gap-2 pointer-coarse:min-h-11"
            // 768–1279 只留站标：站标本身是装饰精灵，文字（`sm` 起的标题块 / 手机档的单行站名）
            // 都被藏起来了，链接就没有可访问名。`aria-label` 与可见站名同源，
            // 有可见文字的那两档里它只是多一遍同样的名字（axe 的 label-content-name 要求
            // 可见文字出现在名字里，这里逐字相同）。
            aria_label=move || t("shell.amateur-radio")
            on:click=move |_| menu_open.set(false)
          >
            <Led tone=LedTone::Rx state=LedState::On small=true />
            <div class="brand-mark flex size-10 shrink-0 items-center justify-center group-hover:-translate-y-0.5 xl:size-9">
              <PixelSprite name="radio" scale=2 />
            </div>
            <span class="pxl-label hidden min-w-0 truncate text-xs text-foreground 2xs:block sm:hidden">
              {move || t("shell.amateur-radio")}
            </span>
            <div class="hidden flex-col whitespace-nowrap leading-tight xl:flex">
              <span class="pxl-title text-xs text-foreground">{move || t("shell.amateur-radio")}</span>
              // 副标题是装饰，宽度让位于功能：点阵标题字体比正文宽，西语副标题又最长（≈ 240px），
              // 桌面整行展开、右侧还有 HUD 时会把它挤出屏幕。只在 2xl 给。
              <span class="hidden text-xs text-muted-foreground 2xl:block">
                {move || t("shell.exams-knowledge-tools")}
              </span>
            </div>
          </a>

          // 分组下拉。没有单独的「首页」项：左边那枚站标本身就是首页链接
          // （与现代风的 `.md-brand` 同一套语义），再摆一个「首页」按钮等于同一目标两个入口。
          // `self-stretch`：撑满 h-16 的导航行，让内部下拉的 `top-full` 能落在导航下沿。
          //
          // **平板起（`md`）就摆出来，不再收进汉堡**：平板横屏最宽 1279px，宽到足以放下三个键，
          // 而菜单藏起来的正是这三段导航 —— 能一眼看到的入口不该要求先点一下。
          // 768–1279 是紧凑档：按键不带下拉箭头（`chevron_class` 由这里传，见 `group_menu.rs`），
          // 每个键省下约 30px，三种语言都能落在同一行里。
          <div class="hidden items-center gap-1 self-stretch md:flex">
            // 考试中心下拉：17 条平铺会拉成一条长列表，与知识库 / 工具共用
            // 「分组侧栏 + 条目区」面板（见 `group_menu.rs`）。
            <GroupMenu
              kind=MenuKind::Exam
              groups=EXAM_GROUPS
              label=Signal::derive(move || t("shell.exam-center"))
              align=MenuAlign::Left
              open_menu=open_menu
            />

            // 知识库下拉：分组侧栏 + 条目区（上百个专题平铺会顶出视口，
            // 结构与高度约束见 `group_menu.rs`）
            <GroupMenu
              kind=MenuKind::Knowledge
              groups=KNOWLEDGE_GROUPS
              label=Signal::derive(move || t("shell.knowledge"))
              align=MenuAlign::Center
              open_menu=open_menu
            />

            // 工具下拉：与知识库同一套面板，只是相对导航容器右对齐
            <GroupMenu
              kind=MenuKind::Tools
              groups=TOOL_GROUPS
              label=Signal::derive(move || t("shell.tools"))
              align=MenuAlign::Right
              open_menu=open_menu
            />
          </div>

          // 右侧：搜索 + 设置 + HUD + 移动端菜单按钮。`ml-auto` 把它顶到最右（布局对齐现代风，见上）。
          // `shrink-0`：按键缩不动，窄屏时由左边的站名让位（截断 / 360px 以下隐藏）；这一组要是允许收缩，
          // 按键就会溢出到视口外。齿轮不分宽度常驻：它是语言与全部外观设置的唯一入口，手机档也不能少。
          <div class="ml-auto flex shrink-0 items-center gap-0.5 sm:gap-1">
            // 同一个按钮两种外形：手机档是图标，桌面档是面板上的丝印字按键（琥珀灯 + 名字 + 快捷键 `/`）。
            // 丝印字就是按钮的名字本身（本地化），可见文字与无障碍名一致（WCAG 2.5.3）。
            <Button
              variant=Variant::Ghost
              size=Size::Icon
              class="xl:h-9 xl:w-auto xl:gap-2 xl:border-ink xl:bg-card xl:px-3"
              aria_label=Signal::derive(move || t("shell.search"))
              title=Signal::derive(move || t("shell.search-with-shortcut"))
              on_click=Callback::new(move |_| search_open.set(true))
            >
              <Icon kind=IconKind::Search class="size-6 xl:hidden" />
              <Led tone=LedTone::Warn state=LedState::Off small=true class="hidden xl:inline-block" />
              <span class="pxl-label hidden text-xs xl:inline">{move || t("shell.search")}</span>
              <kbd class="pxl-kbd hidden xl:inline" aria-hidden="true">"/"</kbd>
            </Button>
            <Button
              variant=Variant::Ghost
              size=Size::Icon
              class="xl:h-9 xl:w-auto xl:border-ink xl:bg-card xl:px-3"
              aria_label=Signal::derive(move || t("shell.settings"))
              title=Signal::derive(move || t("shell.settings"))
              on_click=Callback::new(move |_| settings_open.set(true))
            >
              <Icon kind=IconKind::Settings class="size-6 xl:hidden" />
              <span class="pxl-label hidden text-xs xl:inline">{move || t("shell.settings")}</span>
            </Button>
            <HudBadge />
            // 汉堡只在小屏（`<md`，即手机）出现：平板起三个波段键已经摆在顶栏里
            // （见上面的分组下拉），再放一个开同样内容的汉堡就是两个入口。
            <Button
              variant=Variant::Ghost
              size=Size::Icon
              class="md:hidden"
              aria_label=Signal::derive(move || {
                if menu_open.get() { t("shell.close-menu") } else { t("shell.open-menu") }
              })
              aria_expanded=Signal::derive(move || menu_open.get())
              aria_controls=MOBILE_NAV_PANEL_ID
              on_click=Callback::new(move |_| menu_open.update(|v| *v = !*v))
            >
              {move || {
                if menu_open.get() {
                  view! { <Icon kind=IconKind::X class="size-6" /> }
                } else {
                  view! { <Icon kind=IconKind::Menu class="size-6" /> }
                }
              }}
            </Button>
          </div>
        </div>
        // 在定位容器里、行的下面：下拉面板以整个容器的下沿为基准（`top-full`），自然落在刻度尺下方。
        <BandScale />
      </div>

      // 移动端菜单。挂在 `<nav>` 上、不放进上面那层内容 div 里：那层是 `max-w-5xl`
      // 版心，面板以它为定位祖先，`inset-x-0` 就会被那条宽度收窄，左右各空出一条、
      // 露出下面的页面。挂在 `<nav>` 下（`sticky` 即定位祖先）铺满整宽。
      // 结构（标签栏 + 段内容）与限高都在 `mobile_nav_panel.rs`。
      {move || menu_open.get().then(|| view! { <MobileNavPanel /> })}
      <SettingsDialog open=settings_open />
    </nav>
  }
}
