//! 导航里的「分组下拉」：左侧分组栏 + 右侧条目区（考试中心、知识库、工具三个菜单共用）。
//!
//! 旧版把分组一次平铺成五列：条目最多的分组单列高度就接近一屏，直接顶出视口，
//! 而面板没有任何高度约束、连滚动都做不到（知识库上百个专题时最明显）。这里改成
//! 主从两栏：分组常驻侧栏，条目区只渲染当前分组（两列），面板再以
//! `max-h-[calc(100svh-5rem)]` 兜底 —— 窗口再矮也只是条目区内部滚动。
//!
//! 下拉开关保留原生 `<button>` 的原因见 `docs/ui/pixel.md` §10.2：它是电台面板上的波段键
//! （`.pxl-band-key`，不是 `Button` 的任何变体），「当前页面在这个分组里」要随路由动态打
//! `data-active`，而 `Button` 的类名是静态的。

use leptos::html;
use leptos::prelude::*;
use leptos_router::hooks::use_location;

use ham_web_core::registry;

use crate::cn::cn;
use crate::focus::is_escape_for_overlay;
use crate::i18n::{t, tp};
use crate::icons::{Icon, IconKind, icon_of};
use crate::ui::{Button, Led, LedState, LedTone, Size, TextValue, Variant};

use super::menu::MenuKind;

/// 面板的水平对齐方式（960px 面板不是贴着开关，而是贴着导航容器）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MenuAlign {
  /// 相对导航容器左对齐（考试中心）。
  Left,
  /// 相对导航容器居中（知识库）。
  Center,
  /// 相对导航容器右对齐（工具）。
  Right,
}

impl MenuAlign {
  /// 外层定位类名。三个菜单的垂直基准与动效一致，差异只在水平锚点。
  const fn wrapper_class(self) -> &'static str {
    match self {
      Self::Left => "absolute left-0 top-full pt-2",
      Self::Center => "absolute left-1/2 top-full -translate-x-1/2 pt-2",
      Self::Right => "absolute right-4 top-full pt-2",
    }
  }
}

/// 分组下的条目数（侧栏右侧的数字）。
fn group_count(group: &str) -> usize {
  registry::MODULES
    .iter()
    .filter(|m| m.group == Some(group))
    .count()
}

/// 分组下拉：一个开关 + 一个「分组侧栏 + 条目区」面板。
///
/// 考试中心、知识库与工具三个菜单的结构完全相同，只有分组清单、文案、图标与对齐方式不同 ——
/// `groups` 同时决定侧栏内容与「当前页面是否高亮本菜单」。
#[component]
pub(crate) fn GroupMenu(
  /// 本菜单对应 `open_menu` 的取值。
  kind: MenuKind,
  /// 分组展示顺序（`registry::KNOWLEDGE_GROUPS` / `registry::TOOL_GROUPS`）。
  groups: &'static [&'static str],
  /// 开关文案。字面量留在调用点：`check-i18n` 按调用点采集，转手进来的 key 会漏统计。
  #[prop(into)]
  label: TextValue,
  /// 面板水平对齐。
  align: MenuAlign,
  /// 展开状态由导航栏统一持有：同一时刻只允许一个下拉打开。
  open_menu: RwSignal<Option<MenuKind>>,
) -> impl IntoView {
  let location = use_location();
  let is_open = move || open_menu.get() == Some(kind);
  let active = move |href: &str| location.pathname.get() == href;
  let group_active = move |group: &str| {
    registry::MODULES
      .iter()
      .filter(|m| m.group == Some(group))
      .any(|m| active(m.path))
  };
  let menu_active = move || groups.iter().any(|g| group_active(g));
  // 当前页面所在的分组；不在本菜单的页面上时落到第一组。
  let route_group = move || {
    groups
      .iter()
      .copied()
      .find(|g| group_active(g))
      .unwrap_or(groups[0])
  };
  let group = RwSignal::new(groups[0]);
  // 每次展开都回到当前页面所在分组：上一次点击留下的选择不该跨次生效，
  // 否则重开菜单看到的内容与所在页面无关，反而更难找。
  Effect::new(move |_| {
    if is_open() {
      group.set(route_group());
    }
  });

  // 面板的 DOM id（`aria-controls` 要指向它）。显式 match 而不是 `{kind:?}`：
  // `Debug` 输出会随变体重命名而变，无障碍关联会在重构时静默失联。
  let panel_id = match kind {
    MenuKind::Knowledge => "nav-knowledge-panel",
    MenuKind::Tools => "nav-tools-panel",
    MenuKind::Exam => "nav-exam-panel",
  };
  // Esc 收起后把焦点还给波段键：面板一隐藏，焦点就从里面的链接掉回 `body`，键盘用户得从页首重新 Tab。
  let trigger = NodeRef::<html::Button>::new();

  view! {
    // `static`：面板宽 960px，必须相对整条导航容器定位（居中 / 贴右）才不会溢出
    // 视口。垂直方向因此与「考试中心」共享同一个基准 —— 容器高 h-16，
    // `top-full` 恒为导航下沿。
    <div
      class="group static"
      // 焦点在波段键或面板内任意元素上时按 Esc 收起（`is_escape_for_overlay` 排除输入法组词）。
      // 挂在元素级 keydown 而不是 `window`：焦点不在本菜单内时不该由它响应。
      on:keydown=move |e| {
        if is_escape_for_overlay(&e) && is_open() {
          e.stop_propagation();
          open_menu.set(None);
          if let Some(el) = trigger.get_untracked() {
            let _ = el.focus();
          }
        }
      }
    >
      // 刻意的例外，见文件头：下拉开关是波段键，当前分组随路由打 `data-active`。
      <button
        node_ref=trigger
        type="button"
        data-slot="button"
        aria-expanded=move || is_open().to_string()
        // 站点导航的展开键只带 `aria-expanded` + `aria-controls`（W3C APG「Disclosure Navigation」）：
        // 面板是一组导航链接，`aria-haspopup="true"` 会被读屏报成「菜单」，
        // `"dialog"` 又要求面板本身是 `role="dialog"` —— 两种都不是它。
        aria-controls=panel_id
        // 波段键：当前页面在这个分组里 = 按进去 + 顶部灯条 + 灯亮（`radio.css` 的 `.pxl-band-key`）。
        // 高度与这一行的站标 / 按钮同源（`size-10 xl:size-9`，见 `nav_bar.rs`）：`<xl` 40px、`xl` 起 36px。
        class="pxl-band-key h-10 xl:h-9"
        data-active=move || menu_active().then_some("")
        on:click=move |_| {
          open_menu.update(|m| *m = if *m == Some(kind) { None } else { Some(kind) });
        }
      >
        // 波段键的指示灯：亮 = 当前页面在这个分组里（不只靠按钮底色区分）。
        <Led tone=LedTone::Rx state=Signal::derive(move || LedState::lit(menu_active())) small=true />
        {move || label.get()}
        // 下拉箭头只在桌面档（`≥1280`）画：平板档（768–1279）要在一行里放下三个键 +
        // 站标 + 右侧动作组，西语算下来只剩几十像素余量，箭头是三个键里唯一的非文字件。
        // 「这是个下拉」由 `aria-expanded`、面板的开合与按键按下去的样子说清。
        <Icon
          kind=IconKind::ChevronDown
          class=Signal::derive(move || {
            cn(&[
              "hidden size-6 xl:block",
              if is_open() { "rotate-180" } else { "" },
            ])
          })
        />
      </button>
      // 外层只管定位与显隐（隐藏后不再参与命中测试）；
      // `motion-popover` 挂在内层面板上做位移 + 缩放 —— 外层带 `-translate-x-1/2`
      // 这类居中 transform，动效若也写 transform 会把它覆盖掉。
      <div class=move || {
        cn(&[
          align.wrapper_class(),
          if is_open() { "visible opacity-100" } else { "invisible opacity-0" },
        ])
      }>
        <div
          id=panel_id
          data-open=move || is_open().to_string()
          class="motion-popover pxl-popover origin-top flex max-h-[calc(100svh-5rem)] w-[960px] max-w-[calc(100vw-2rem)] gap-2 p-2"
        >
          // 分组栏：常驻，不参与条目区的滚动。
          // 分组名里可能出现「Equipment · Measurement」这类长串：栏宽给到 18rem，
          // 名称本身允许换行（下面的 `break-words`），否则会被 `truncate` 吃掉尾巴。
          <div class="flex w-72 shrink-0 flex-col gap-1 border-2 border-ink bg-muted p-1.5">
            {groups
              .iter()
              .map(|&g| {
                let selected = move || group.get() == g;
                view! {
                  // 悬停即预览该分组；`mouseenter`（不冒泡）而非 `mouseover` ——
                  // 后者在子元素之间移动会反复触发。`focusin` 为键盘补上同样的预览：
                  // 不补的话 Tab 到分组按钮时，条目区还停在上一次的分组上。
                  <div
                    class=move || {
                      cn(&[
                        "border-2",
                        if selected() { "border-ink bg-accent" } else { "border-transparent" },
                      ])
                    }
                    on:mouseenter=move |_| {
                      if group.get_untracked() != g {
                        group.set(g);
                      }
                    }
                    on:focusin=move |_| {
                      if group.get_untracked() != g {
                        group.set(g);
                      }
                    }
                  >
                    <Button
                      variant=Variant::Ghost
                      size=Size::Sm
                      class="w-full justify-start gap-2 has-[>svg]:px-2"
                      on_click=Callback::new(move |_| group.set(g))
                    >
                      <span class=move || {
                        cn(&[
                          "h-4 w-1 shrink-0",
                          if selected() { "bg-primary" } else { "bg-transparent" },
                        ])
                      }></span>
                      <Icon
                        kind=icon_of(registry::group_icon(g))
                        class=Signal::derive(move || {
                          cn(&[
                            "size-6 shrink-0",
                            if selected() { "text-primary" } else { "text-muted-foreground" },
                          ])
                        })
                      />
                      <span class=move || {
                        cn(&[
                          // `whitespace-normal` 必须显式写：按钮本身带 `whitespace-nowrap`，
                          // 不覆盖的话长组名会顶出侧栏而不是换行。
                          "min-w-0 flex-1 whitespace-normal break-words text-left leading-snug",
                          if selected() {
                            "text-foreground"
                          } else {
                            "text-muted-foreground"
                          },
                        ])
                      }>{move || t(g)}</span>
                      // 条目数只是概览。**不能只靠 `aria-hidden` 把它从无障碍名里摘掉**：
                      // axe 的 `label-content-name-mismatch` 按 DOM 可见文字比对，属性对它无效
                      // （实测按钮文字是「练题5」、`aria-label` 是「练题」，直接判不匹配）。
                      // 所以这里用 `sr-only` 的同义说法给读屏念「共 N 项」（en / es 按数量换词形），
                      // 可见的角标才 `aria-hidden`。
                      <span class="sr-only">
                        {move || {
                          let n = group_count(g);
                          tp("shell.group-item-count", n, &[&n.to_string()])
                        }}
                      </span>
                      <span
                        class="pxl-label text-xs tabular-nums text-muted-foreground"
                        aria-hidden="true"
                      >
                        {group_count(g).to_string()}
                      </span>
                    </Button>
                  </div>
                }
              })
              .collect_view()}
          </div>
          // 条目区：只渲染当前分组，窗口矮时在这里内部滚动。
          //
          // `-m-0.5 p-1.5`：`overflow-y-auto` 两个方向都裁剪，条目（`border-2` 链接）的焦点环
          // （2px 虚线 + 2px 偏移 = 盒外 4px）此前被 4px 内距刚好压在裁剪边界上。这里把内距提到
          // 6px（余量 2px），多出的 2px 用负外边距抵掉、内容位置不变（同一手法见 `ui::DialogBody`）。
          <div class="min-w-0 flex-1 -m-0.5 overflow-y-auto overscroll-contain p-1.5">
            <div class="grid grid-cols-2 gap-x-2 gap-y-0.5">
              {move || {
                registry::MODULES
                  .iter()
                  .filter(|m| m.group == Some(group.get()))
                  .map(|m| {
                    view! {
                      <a
                        href=m.nav_href()
                        on:click=move |_| open_menu.set(None)
                        class=move || {
                          cn(&[
                            "flex items-center gap-2 border-2 px-2.5 py-1.5 text-sm",
                            if active(m.path) {
                              "border-ink bg-accent text-accent-foreground"
                            } else {
                              "border-transparent text-muted-foreground hover:border-ink hover:bg-accent hover:text-accent-foreground"
                            },
                          ])
                        }
                      >
                        <Icon kind=icon_of(m.icon) class="size-6 shrink-0" />
                        // `break-words` 而不是 `truncate`：西语 / 英语长标题会被省略号吃掉，
                        // 换行最多多占一行，条目区本身有高度上限与内部滚动兜底。
                        <span class="min-w-0 break-words leading-snug">{move || t(m.title)}</span>
                      </a>
                    }
                  })
                  .collect_view()
              }}
            </div>
          </div>
        </div>
      </div>
    </div>
  }
}
