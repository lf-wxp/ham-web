use ham_web_core::duty_power::{
  CONTROLLED_MIN, DutyError, Input, MODES, Output, Pattern, UNCONTROLLED_MIN, average,
};
use leptos::prelude::*;

use super::{RESULT, fmt_num};
use crate::i18n::{t, tf};
use crate::ui::{Field, NativeSelect, NumberField, SelectOption};
use crate::util::unique_id;

const PATTERN_WINDOW: &str = "window";
const PATTERN_PERIODIC: &str = "periodic";
const PATTERN_PULSED: &str = "pulsed";

/// 占空比与平均功率：PEP 按模式占空比与发射时间折算成平均功率（射频暴露的时间平均估算）。
#[component]
pub(super) fn DutyPower() -> impl IntoView {
  // 数值一律存 `Option<f64>`：空输入 = 未填写，而不是「沿用旧值」。计算时把 `None`
  // 换成 `NAN` 交给 `average()` 的校验，报对应的「必须是正的有限数」错误 —— 输入框空着
  // 时结果区给出提示，而不是用旧值算出与界面不符的数字。
  let pep = RwSignal::new(Some(100.0));
  let mode = RwSignal::new("ssb".to_owned());
  let pattern = RwSignal::new(PATTERN_PERIODIC.to_owned());
  // 平均时间窗（分钟）：6 = 受控暴露，30 = 不受控暴露。
  let window_min = RwSignal::new("6".to_owned());
  let on_in_window = RwSignal::new(Some(240.0));
  let cycle_on = RwSignal::new(Some(120.0));
  let cycle_off = RwSignal::new(Some(120.0));
  let pulse_width_us = RwSignal::new(Some(1.0));
  let pulse_rate_hz = RwSignal::new(Some(1000.0));

  let pep_id = unique_id("duty-pep");
  let mode_id = unique_id("duty-mode");
  let pattern_id = unique_id("duty-pattern");
  let window_id = unique_id("duty-window");
  let on_in_window_id = unique_id("duty-on-in-window");
  let cycle_on_id = unique_id("duty-cycle-on");
  let cycle_off_id = unique_id("duty-cycle-off");
  let pulse_width_id = unique_id("duty-pulse-width");
  let pulse_rate_id = unique_id("duty-pulse-rate");

  let mode_options: Vec<SelectOption> = MODES
    .iter()
    .map(|m| {
      let (label, pct) = (m.label, (m.duty * 100.0).round());
      SelectOption::new(
        m.id,
        Signal::derive(move || format!("{} · {pct}%", t(label))),
      )
    })
    .collect();
  let pattern_options = vec![
    SelectOption::new(
      PATTERN_PERIODIC,
      Signal::derive(move || t("tools.duty-pattern-periodic")),
    ),
    SelectOption::new(
      PATTERN_WINDOW,
      Signal::derive(move || t("tools.duty-pattern-window")),
    ),
    SelectOption::new(
      PATTERN_PULSED,
      Signal::derive(move || t("tools.duty-pattern-pulsed")),
    ),
  ];
  let window_options = vec![
    SelectOption::new(
      "6",
      Signal::derive(move || t("tools.duty-window-controlled")),
    ),
    SelectOption::new(
      "30",
      Signal::derive(move || t("tools.duty-window-uncontrolled")),
    ),
  ];

  let outcome = Memo::new(move |_| -> Result<Output, DutyError> {
    let window_s = window_min.with(|w| w.parse::<f64>().unwrap_or(CONTROLLED_MIN)) * 60.0;
    let pattern = match pattern.get().as_str() {
      PATTERN_WINDOW => Pattern::InWindow {
        on_s: on_in_window.get().unwrap_or(f64::NAN),
        window_s,
      },
      PATTERN_PULSED => Pattern::Pulsed {
        width_s: pulse_width_us.get().unwrap_or(f64::NAN) * 1e-6,
        rate_hz: pulse_rate_hz.get().unwrap_or(f64::NAN),
      },
      _ => Pattern::Periodic {
        on_s: cycle_on.get().unwrap_or(f64::NAN),
        off_s: cycle_off.get().unwrap_or(f64::NAN),
        window_s,
      },
    };
    let mode_duty = mode.with(|id| MODES.iter().find(|m| m.id == id).map_or(1.0, |m| m.duty));
    average(&Input {
      pep_w: pep.get().unwrap_or(f64::NAN),
      mode_duty,
      pattern,
    })
  });

  view! {
    <div class="grid grid-cols-1 gap-3 sm:grid-cols-2">
      <Field label=Signal::derive(move || t("tools.duty-pep-w")) r#for=pep_id.clone()>
        <NumberField
          id=pep_id
          value=Signal::derive(move || pep.get().map_or_else(String::new, |v| v.to_string()))
          on_change=Callback::new(move |v: String| pep.set(v.trim().parse::<f64>().ok()))
          controls=false
        />
      </Field>
      <Field label=Signal::derive(move || t("tools.duty-mode")) r#for=mode_id.clone()>
        <NativeSelect
          id=mode_id
          value=Signal::derive(move || mode.get())
          on_change=Callback::new(move |v: String| mode.set(v))
          options=mode_options
        />
      </Field>
      <Field label=Signal::derive(move || t("tools.duty-pattern")) r#for=pattern_id.clone()>
        <NativeSelect
          id=pattern_id
          value=Signal::derive(move || pattern.get())
          on_change=Callback::new(move |v: String| pattern.set(v))
          options=pattern_options
        />
      </Field>
      {move || {
        (pattern.get() != PATTERN_PULSED).then(|| {
          let window_id = window_id.clone();
          let options = window_options.clone();
          view! {
            <Field label=Signal::derive(move || t("tools.duty-window")) r#for=window_id.clone()>
              <NativeSelect
                id=window_id
                value=Signal::derive(move || window_min.get())
                on_change=Callback::new(move |v: String| window_min.set(v))
                options=options
              />
            </Field>
          }
        })
      }}
      {move || match pattern.get().as_str() {
        PATTERN_WINDOW => {
          let id = on_in_window_id.clone();
          view! {
            <Field label=Signal::derive(move || t("tools.duty-on-in-window")) r#for=id.clone()>
              <NumberField
                id=id
                value=Signal::derive(move || on_in_window.get().map_or_else(String::new, |v| v.to_string()))
                on_change=Callback::new(move |v: String| on_in_window.set(v.trim().parse::<f64>().ok()))
                controls=false
              />
            </Field>
          }
          .into_any()
        }
        PATTERN_PULSED => {
          let (width_id, rate_id) = (pulse_width_id.clone(), pulse_rate_id.clone());
          view! {
            <Field label=Signal::derive(move || t("tools.duty-pulse-width-us")) r#for=width_id.clone()>
              <NumberField
                id=width_id
                value=Signal::derive(move || pulse_width_us.get().map_or_else(String::new, |v| v.to_string()))
                on_change=Callback::new(move |v: String| pulse_width_us.set(v.trim().parse::<f64>().ok()))
                controls=false
              />
            </Field>
            <Field label=Signal::derive(move || t("tools.duty-pulse-rate-hz")) r#for=rate_id.clone()>
              <NumberField
                id=rate_id
                value=Signal::derive(move || pulse_rate_hz.get().map_or_else(String::new, |v| v.to_string()))
                on_change=Callback::new(move |v: String| pulse_rate_hz.set(v.trim().parse::<f64>().ok()))
                controls=false
              />
            </Field>
          }
          .into_any()
        }
        _ => {
          let (on_id, off_id) = (cycle_on_id.clone(), cycle_off_id.clone());
          view! {
            <Field label=Signal::derive(move || t("tools.duty-cycle-on-s")) r#for=on_id.clone()>
              <NumberField
                id=on_id
                value=Signal::derive(move || cycle_on.get().map_or_else(String::new, |v| v.to_string()))
                on_change=Callback::new(move |v: String| cycle_on.set(v.trim().parse::<f64>().ok()))
                controls=false
              />
            </Field>
            <Field label=Signal::derive(move || t("tools.duty-cycle-off-s")) r#for=off_id.clone()>
              <NumberField
                id=off_id
                value=Signal::derive(move || cycle_off.get().map_or_else(String::new, |v| v.to_string()))
                on_change=Callback::new(move |v: String| cycle_off.set(v.trim().parse::<f64>().ok()))
                controls=false
              />
            </Field>
          }
          .into_any()
        }
      }}
      <div class=format!("{RESULT} tabular-nums") role="status">
        {move || match outcome.get() {
          Ok(o) => view! {
            <div class="font-medium text-foreground">
              {tf(
                "tools.duty-result-average",
                &[
                  &fmt_num(o.average_w),
                  &if o.average_dbm.is_finite() { fmt_num(o.average_dbm) } else { "—".to_owned() },
                ],
              )}
            </div>
            <div>
              {tf(
                "tools.duty-result-duty",
                &[&fmt_num(o.on_fraction * 100.0), &fmt_num(o.total_duty * 100.0)],
              )}
            </div>
            <div>{tf("tools.duty-result-distance", &[&format!("{:.2}", o.distance_factor)])}</div>
          }
          .into_any(),
          Err(e) => view! { <div>{error_text(e)}</div> }.into_any(),
        }}
      </div>
      <p class="sm:col-span-2 text-xs text-muted-foreground">
        {move || tf("tools.duty-note", &[&CONTROLLED_MIN.to_string(), &UNCONTROLLED_MIN.to_string()])}
      </p>
    </div>
  }
}

fn error_text(e: DutyError) -> String {
  t(match e {
    DutyError::Power => "tools.duty-error-power",
    DutyError::ModeDuty => "tools.duty-error-mode",
    DutyError::Time => "tools.duty-error-time",
    DutyError::OnLongerThanWindow => "tools.duty-error-on-longer",
    DutyError::PulseOverlap => "tools.duty-error-pulse-overlap",
  })
}
