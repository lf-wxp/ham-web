//! 导航里的「分组下拉」：左侧分组栏 + 右侧条目区（知识库、工具两个菜单共用）。
//!
//! 旧版把分组一次平铺成五列：条目最多的分组单列高度就接近一屏，直接顶出视口，
//! 而面板没有任何高度约束、连滚动都做不到（知识库上百个专题时最明显）。这里改成
//! 主从两栏：分组常驻侧栏，条目区只渲染当前分组（两列），面板再以
//! `max-h-[calc(100vh-5rem)]` 兜底 —— 窗口再矮也只是条目区内部滚动。
//!
//! 下拉开关保留原生 `<button>` + [`button_class`] 的原因见 `nav_bar.rs` 文件头与
//! `docs/ui-components.md` §5：高亮要切 `variant`、开关要输出 `aria-expanded`。

use leptos::prelude::*;
use leptos_router::hooks::use_location;

use ham_web_core::registry;

use crate::cn::cn;
use crate::i18n::t;
use crate::icons::{Icon, IconKind, icon_of};
use crate::ui::{Button, Size, TextValue, Variant, button_class};

use super::menu::MenuKind;

/// 面板的水平对齐方式（960px 面板不是贴着开关，而是贴着导航容器）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MenuAlign {
  /// 相对导航容器居中（知识库）。
  Center,
  /// 相对导航容器右对齐（工具）。
  Right,
}

impl MenuAlign {
  /// 外层定位类名。两个菜单的垂直基准与动效一致，差异只在水平锚点。
  const fn wrapper_class(self) -> &'static str {
    match self {
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
/// 知识库与工具两个菜单的结构完全相同，只有分组清单、文案、图标与对齐方式不同 ——
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
  /// 开关图标。
  icon: IconKind,
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

  view! {
    // `static`：面板宽 960px，必须相对整条导航容器定位（居中 / 贴右）才不会溢出
    // 视口。垂直方向因此与「考试中心」共享同一个基准 —— 容器高 h-16，
    // `top-full` 恒为导航下沿。
    <div class="group static">
      // 刻意的例外，见文件头：下拉开关要切换 `variant` 并输出 `aria-expanded`。
      <button
        type="button"
        data-slot="button"
        aria-expanded=move || is_open().to_string()
        aria-haspopup="true"
        aria-controls=panel_id
        class=move || {
          button_class(
            if menu_active() { Variant::Default } else { Variant::Ghost },
            Size::Sm,
            "inline-flex items-center gap-2 whitespace-nowrap",
          )
        }
        on:click=move |_| {
          open_menu.update(|m| *m = if *m == Some(kind) { None } else { Some(kind) });
        }
      >
        <Icon kind=icon class="size-6" />
        {move || label.get()}
        <Icon
          kind=IconKind::ChevronDown
          class=Signal::derive(move || {
            cn(&[
              "size-6",
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
          class="motion-popover pxl-popover origin-top flex max-h-[calc(100vh-5rem)] w-[960px] max-w-[calc(100vw-2rem)] gap-2 p-2"
        >
          // 分组栏：常驻，不参与条目区的滚动。
          <div class="flex w-64 shrink-0 flex-col gap-1 border-2 border-ink bg-muted p-1.5">
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
                      aria_label=Signal::derive(move || t(g))
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
                          "min-w-0 flex-1 truncate text-left",
                          if selected() {
                            "text-foreground"
                          } else {
                            "text-muted-foreground"
                          },
                        ])
                      }>{move || t(g)}</span>
                      // 数字只是概览，读屏名字用按钮的 `aria_label` 保持干净。
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
          <div class="min-w-0 flex-1 overflow-y-auto p-1">
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
