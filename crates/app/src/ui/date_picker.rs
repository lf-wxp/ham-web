//! 日期选择器：原生 `<input type="date">` 的弹层式替代。
//!
//! 原生日期输入的日历面板由浏览器绘制，配色 / 圆角 / 阴影都无法跟随站点，与语言切换
//! （[`crate::ui::Select`]）并排时风格割裂。这里改为「触发器 + 弹层日历」，弹层的类名与
//! 开合行为取自 [`super::popover`] —— 弹出效果与语言切换完全一致。
//!
//! 对外值与原生输入保持一致：`YYYY-MM-DD`，空串表示未选择。

use leptos::html;
use leptos::prelude::*;

use crate::cn::cn;
use crate::i18n::t;
use crate::icons::{Icon, IconKind};

use super::control::{ControlSize, TextValue, invalid_attr};
use super::intl;
use super::popover;

/// 日历固定 6 行 × 7 列：任何月份都放得下，翻月时行数不跳动。
const CALENDAR_CELLS: u32 = 42;

/// 解析 `YYYY-MM-DD`（宽松：只认三段数字，但月 / 日要与**该年该月**对得上）。
///
/// 只校验 `1..=31` 是不够的：`2024-02-31` 会被当成合法值显示在触发器上，
/// 而 42 个单元格里根本没有这一天 —— 没有选中项，也没有任何提示。
fn parse_date(s: &str) -> Option<(i32, u32, u32)> {
  let mut parts = s.split('-');
  let y = parts.next()?.parse::<i32>().ok()?;
  let m = parts.next()?.parse::<u32>().ok()?;
  let d = parts.next()?.parse::<u32>().ok()?;
  let valid =
    parts.next().is_none() && (1..=12).contains(&m) && (1..=days_in_month(y, m)).contains(&d);
  valid.then_some((y, m, d))
}

/// 今天的本地日期。
fn today() -> (i32, u32, u32) {
  let d = js_sys::Date::new_0();
  (d.get_full_year() as i32, d.get_month() + 1, d.get_date())
}

/// 某年某月的天数。
fn days_in_month(y: i32, m: u32) -> u32 {
  match m {
    1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
    4 | 6 | 9 | 11 => 30,
    _ => {
      if (y % 4 == 0 && y % 100 != 0) || y % 400 == 0 {
        29
      } else {
        28
      }
    }
  }
}

/// 该月 1 日是周几（周一 = 0 … 周日 = 6）：日历按周一开头排布。
///
/// 年份要过 [`intl::js_year`]：`0..=99` 会被 `Date` 映射成 `1900 + y`。
fn leading_blanks(y: i32, m: u32) -> u32 {
  let d = js_sys::Date::new_with_year_month_day(intl::js_year(y), m as i32 - 1, 1);
  (d.get_day() + 6) % 7
}

/// 月份加减（跨年自动进位 / 借位）。
fn shift_month((y, m): (i32, u32), delta: i64) -> (i32, u32) {
  let total = y as i64 * 12 + (m as i64 - 1) + delta;
  (
    total.div_euclid(12) as i32,
    (total.rem_euclid(12) + 1) as u32,
  )
}

/// 单元格类名。
fn day_class(selected: bool, is_today: bool) -> String {
  cn(&[
    "flex size-8 items-center justify-center border-2 text-xs tabular-nums outline-none focus-visible:outline-2 focus-visible:outline-dashed focus-visible:outline-ring",
    if selected {
      "border-ink bg-primary text-primary-foreground"
    } else if is_today {
      // 今天：虚线框而不是粗体 —— 点阵字体没有粗体。
      "border-dashed border-primary text-primary hover:border-solid hover:border-ink hover:bg-accent hover:text-accent-foreground"
    } else {
      "border-transparent hover:border-ink hover:bg-accent hover:text-accent-foreground"
    },
  ])
}

const NAV_BTN: &str = "flex size-8 items-center justify-center border-2 border-transparent text-muted-foreground outline-none hover:border-ink hover:bg-accent hover:text-accent-foreground focus-visible:outline-2 focus-visible:outline-dashed focus-visible:outline-ring";

/// 弹层式日期选择器（外观与语言切换一致）。
#[component]
pub fn DatePicker(
  /// 当前值（`YYYY-MM-DD`，受控；空串表示未选择）。
  #[prop(into)]
  value: Signal<String>,
  /// 选中日期后的新值。
  on_change: Callback<String>,
  #[prop(optional)] size: ControlSize,
  /// 未选择时的占位文案（默认 `YYYY-MM-DD`）。
  #[prop(optional, into)]
  placeholder: Option<TextValue>,
  /// 无可见标签时的无障碍名称。
  #[prop(optional, into)]
  aria_label: Option<TextValue>,
  #[prop(optional, into)] id: Option<String>,
  #[prop(optional, into)] disabled: Signal<bool>,
  #[prop(optional, into)] invalid: Signal<bool>,
  #[prop(optional, into)] class: String,
) -> impl IntoView {
  let open = RwSignal::new(false);
  let mounted = popover::mount_on_open(open);
  popover::close_on_escape(open);
  // 面板引用：既作 `node_ref`，也给「焦点移出容器就关闭」的判定用
  // （见 `popover::close_on_focus_out`）。
  let panel_ref = NodeRef::<html::Div>::new();

  let placeholder = placeholder.unwrap_or_else(|| TextValue::from("YYYY-MM-DD"));
  let trigger_class = popover::trigger_class(size, &class);
  let state = popover::state(open);

  // 弹层显示的月份与选中值解耦：翻月不应改值，只在打开时对齐一次。
  let today_ymd = today();
  let view_ym = RwSignal::new(
    parse_date(&value.get_untracked()).map_or((today_ymd.0, today_ymd.1), |(y, m, _)| (y, m)),
  );

  let panel = move || {
    mounted.get().then(|| {
      let (y, m) = view_ym.get();
      let sel = parse_date(&value.get());
      let today_ymd = today();
      let lead = leading_blanks(y, m);
      let dim = days_in_month(y, m);

      let cells = (0..CALENDAR_CELLS)
        .map(|i| {
          if i < lead || i >= lead + dim {
            return view! { <div></div> }.into_any();
          }
          let d = i - lead + 1;
          let is_sel = sel == Some((y, m, d));
          let is_today = today_ymd == (y, m, d);
          let label = intl::day_label(y, m, d);
          view! {
            <button
              type="button"
              aria-label=label
              aria-pressed=is_sel.to_string()
              aria-current=is_today.then_some("date")
              class=day_class(is_sel, is_today)
              on:click=move |_| {
                on_change.run(format!("{y:04}-{m:02}-{d:02}"));
                open.set(false);
              }
            >
              {d}
            </button>
          }
          .into_any()
        })
        .collect_view();

      let title = intl::month_title(y, m);
      let panel_label = title.clone();
      view! {
        <div class=popover::OVERLAY on:click=move |e| { popover::swallow(&e); open.set(false); }></div>
        <div
          role="dialog"
          aria-label=panel_label
          data-state=state
          data-slot="date-picker"
          class=popover::panel_class("w-[18.5rem] p-2")
          node_ref=panel_ref
          on:focusout=popover::close_on_focus_out(open, panel_ref)
        >
          <div class="flex items-center justify-between">
            <button
              type="button"
              aria-label=move || t("common.previous-month")
              class=NAV_BTN
              on:click=move |_| view_ym.update(|ym| *ym = shift_month(*ym, -1))
            >
              <Icon kind=IconKind::ChevronLeft class="size-6" />
            </button>
            <div class="pxl-title text-xs" data-slot="date-picker-title">{title}</div>
            <button
              type="button"
              aria-label=move || t("common.next-month")
              class=NAV_BTN
              on:click=move |_| view_ym.update(|ym| *ym = shift_month(*ym, 1))
            >
              <Icon kind=IconKind::ChevronRight class="size-6" />
            </button>
          </div>
          <div class="mt-1 grid grid-cols-7 gap-0.5">
            {intl::weekday_labels()
              .into_iter()
              .map(|w| {
                view! {
                  <div class="flex h-6 items-center justify-center text-xs text-muted-foreground">
                    {w}
                  </div>
                }
              })
              .collect_view()}
            {cells}
          </div>
        </div>
      }
    })
  };

  view! {
    <div class="relative">
      <button
        type="button"
        role="combobox"
        id=id
        aria-haspopup="dialog"
        aria-expanded=move || open.get().to_string()
        aria-label=move || aria_label.as_ref().map(TextValue::get)
        aria-invalid=invalid_attr(invalid)
        data-state=state
        disabled=move || disabled.get()
        class=trigger_class
        on:click=move |_| {
          if !open.get_untracked()
            && let Some((y, m, _)) = parse_date(&value.get_untracked())
          {
            view_ym.set((y, m));
          }
          open.update(|o| *o = !*o);
        }
      >
        <span style="pointer-events: none;">
          {move || {
            let v = value.get();
            if v.is_empty() {
              view! { <span class="text-muted-foreground">{placeholder.get()}</span> }.into_any()
            } else {
              view! { <span class="tabular-nums">{v}</span> }.into_any()
            }
          }}
        </span>
        <Icon kind=IconKind::Calendar class="size-6 shrink-0 opacity-70" />
      </button>
      {panel}
    </div>
  }
}
