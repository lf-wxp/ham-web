//! 电台频率护栏：CAT 连上电台后，按读到的频率与模式、用户选的操作证类别亮一盏灯。
//!
//! 判定全在 `ham_web_core::rig_guard`（有单测）；本组件只做呈现与「我的操作证类别」这个输入。
//! **只读**：不写电台的频率、不发 PTT，灯只回答「如果现在发射，合规吗」。
//!
//! 灯不靠颜色单独表意：每一档都有文字（「红灯 · …」），像素风里另配一颗 [`Led`]。
//! 灰灯（不在业余频段）常见于只收听，所以不画成红色。

use ham_web_core::license_compliance::Class;
use ham_web_core::rig_guard::{self, Finding, Lamp, Verdict};
use leptos::prelude::*;

use crate::i18n::{t, tf};
use crate::kit::is_modern_at_mount;
use crate::ui::{ControlSize, Led, LedState, LedTone, SelectOption, SimpleSelect};

/// 「没选类别」在下拉里的取值（空串在部分控件里不是合法的选项值）。
const UNSET: &str = "none";

/// 一盏灯在两套皮肤下的呈现：像素风文字色 token、现代风 `data-tone` 状态键与像素风指示灯取值。
#[derive(Debug, Clone, Copy)]
struct LampLook {
  /// 像素风文字色 token（`text-hp-text` / `text-gold-text` / `text-win-text` / `text-muted-foreground`）。
  pixel_text: &'static str,
  /// 现代风 `data-tone` 取值：`style/modern/kit/07-bridge.css` 的 `.md-guard-*[data-tone="…"]` 按它上色。
  tone_key: &'static str,
  /// 像素风指示灯的色调与状态。
  led_tone: LedTone,
  led_state: LedState,
}

fn lamp_look(lamp: Lamp) -> LampLook {
  match lamp {
    Lamp::Alert => LampLook {
      pixel_text: "text-hp-text",
      tone_key: "alert",
      led_tone: LedTone::Tx,
      led_state: LedState::On,
    },
    Lamp::Notice => LampLook {
      pixel_text: "text-gold-text",
      tone_key: "notice",
      led_tone: LedTone::Warn,
      led_state: LedState::On,
    },
    Lamp::Ok => LampLook {
      pixel_text: "text-win-text",
      tone_key: "ok",
      led_tone: LedTone::Rx,
      led_state: LedState::On,
    },
    Lamp::OutOfBand => LampLook {
      pixel_text: "text-muted-foreground",
      tone_key: "out",
      led_tone: LedTone::Info,
      led_state: LedState::Off,
    },
  }
}

/// 灯的名字（文字通道）。
fn lamp_label(lamp: Lamp) -> String {
  t(match lamp {
    Lamp::Alert => "log.rig-guard-lamp-alert",
    Lamp::Notice => "log.rig-guard-lamp-notice",
    Lamp::Ok => "log.rig-guard-lamp-ok",
    Lamp::OutOfBand => "log.rig-guard-lamp-out",
  })
}

/// 一条发现 → 一句话（红 / 黄各自的文字色跟着 [`Finding::lamp`]）。
fn finding_text(finding: Finding) -> String {
  match finding {
    Finding::ClassNotPermitted(class) => tf("log.rig-guard-class-not-permitted", &[class.label()]),
    Finding::PhoneOutOfSegment { band, range } => {
      tf("log.rig-guard-phone-out-of-segment", &[band, range])
    }
    Finding::NeedsCoordination => t("log.rig-guard-coordination"),
    Finding::OffPlan { band, range } => tf("log.rig-guard-off-plan", &[band, range]),
  }
}

/// 频率护栏条。
#[component]
pub fn RigGuard(
  /// 电台读到的频率（Hz）；还没读到时是 `None`，不显示灯。
  #[prop(into)]
  freq_hz: Signal<Option<u64>>,
  /// 电台读到的模式。
  #[prop(into)]
  mode: Signal<Option<&'static str>>,
  /// 用户选的操作证类别。
  #[prop(into)]
  class: Signal<Option<Class>>,
  /// 用户改了类别。
  on_class: Callback<Option<Class>>,
) -> impl IntoView {
  let verdict = Memo::new(move |_| {
    freq_hz
      .get()
      .map(|hz| rig_guard::check(class.get(), hz as f64 / 1_000_000.0, mode.get()))
  });

  let class_options: Vec<SelectOption> = std::iter::once(SelectOption::new(
    UNSET,
    Signal::derive(|| t("log.rig-guard-class-unset")),
  ))
  .chain(
    ["A", "B", "C"]
      .into_iter()
      .map(|c| SelectOption::new(c, Signal::derive(move || tf("learning.class", &[c])))),
  )
  .collect();

  // 灯的视觉在两套皮肤不同：像素风是 Led + text-* token，现代风是 data-tone 驱动的圆点 + 文字色
  // （`style/modern/kit/07-bridge.css` 的 `.md-guard-row` / `.md-guard-tone`）。
  let modern = is_modern_at_mount();
  let render = move |v: Verdict| {
    let lamp = v.lamp();
    let look = lamp_look(lamp);
    let band = v.band.map(str::to_owned);
    let power = v
      .power_limit_w
      .map(|w| tf("log.rig-guard-power-limit", &[&w.to_string()]));
    view! {
      <div
        class=if modern {
          "flex flex-wrap items-center gap-x-2 gap-y-1 md-guard-row"
        } else {
          "flex flex-wrap items-center gap-x-2 gap-y-1"
        }
        data-tone=look.tone_key
      >
        {(!modern)
          .then(|| view! { <Led tone=look.led_tone state=look.led_state small=true /> })}
        {modern.then(|| view! { <span class="md-led" aria-hidden="true"></span> })}
        <span class=if modern { "md-guard-tone" } else { look.pixel_text }>{lamp_label(lamp)}</span>
        {band.map(|b| view! { <span class="tabular-nums">{b}</span> })}
        {power.map(|p| view! { <span class="tabular-nums">{p}</span> })}
      </div>
      <ul class="space-y-0.5">
        {v
          .findings
          .iter()
          .map(|f| {
            let look = lamp_look(f.lamp());
            view! {
              <li
                class=if modern { "md-guard-tone" } else { look.pixel_text }
                data-tone=look.tone_key
              >
                {finding_text(*f)}
              </li>
            }
          })
          .collect_view()}
      </ul>
    }
  };

  view! {
    <div
      class="flex flex-col gap-1 border border-dashed px-3 py-2 text-xs text-muted-foreground"
      data-slot="rig-guard"
    >
      <div class="flex flex-wrap items-center gap-2">
        <span class="text-foreground">{move || t("radio.compliance-class-label")}</span>
        <SimpleSelect
          value=Signal::derive(move || {
            class.get().map_or_else(|| UNSET.to_owned(), |c| c.label().to_owned())
          })
          on_change=Callback::new(move |v: String| on_class.run(Class::parse(&v)))
          options=class_options
          size=ControlSize::Sm
          aria_label=Signal::derive(move || t("radio.compliance-class-label"))
          class="w-auto"
        />
        {move || class.get().is_none().then(|| view! { <span>{t("log.rig-guard-class-hint")}</span> })}
      </div>
      // `aria-live` 只放在灯与发现上：包住整条会让读屏在用户改类别时把下拉本身也念一遍。
      <div class="flex flex-col gap-1" aria-live="polite">{move || verdict.get().map(render)}</div>
      <p>{move || t("log.rig-guard-note")}</p>
    </div>
  }
}
