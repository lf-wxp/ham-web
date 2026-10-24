use ham_web_core::rain_fade::{
  COMMON_LIMIT_GHZ, COMMON_LIMIT_KM, FREQ_RANGE_GHZ, Input, Output, PERCENT_RANGE, Polarization,
  RainError, RainWarning, estimate,
};
use leptos::prelude::*;

use super::{RESULT, fmt_num};
use crate::i18n::{t, tf};
use crate::ui::{Field, NativeSelect, NumberField, SelectOption};
use crate::util::unique_id;

/// 雨衰：地面微波链路的单位长度雨衰、有效路径长度与指定时间百分比的雨衰（ITU-R P.838-3 / P.530-17）。
#[component]
pub(super) fn RainFade() -> impl IntoView {
  // 数值一律存 `Option<f64>`：空输入 = 未填写。计算时把 `None` 换成 `NAN` 交给
  // `estimate()` 的校验，报对应的范围错误 —— 输入框空着时结果区给出提示，而不是
  // 用旧值算出与界面不符的数字。
  let freq = RwSignal::new(Some(10.368));
  let dist = RwSignal::new(Some(20.0));
  let rain = RwSignal::new(Some(40.0));
  let pol = RwSignal::new("h".to_owned());
  let percent = RwSignal::new(Some(0.01));

  let freq_id = unique_id("rain-freq");
  let dist_id = unique_id("rain-dist");
  let rain_id = unique_id("rain-rate");
  let pol_id = unique_id("rain-pol");
  let percent_id = unique_id("rain-percent");

  let pol_options = vec![
    SelectOption::new("h", Signal::derive(move || t("tools.rain-pol-h"))),
    SelectOption::new("v", Signal::derive(move || t("tools.rain-pol-v"))),
    SelectOption::new("c", Signal::derive(move || t("tools.rain-pol-c"))),
  ];

  let outcome = Memo::new(move |_| -> Result<Output, RainError> {
    let polarization = match pol.get().as_str() {
      "v" => Polarization::Vertical,
      "c" => Polarization::Circular,
      _ => Polarization::Horizontal,
    };
    estimate(&Input {
      freq_ghz: freq.get().unwrap_or(f64::NAN),
      dist_km: dist.get().unwrap_or(f64::NAN),
      rain_rate_001: rain.get().unwrap_or(f64::NAN),
      polarization,
      percent: percent.get().unwrap_or(f64::NAN),
    })
  });

  view! {
    <div class="grid grid-cols-1 gap-3 sm:grid-cols-2">
      <Field label=Signal::derive(move || t("tools.rain-freq-ghz")) r#for=freq_id.clone()>
        <NumberField
          id=freq_id
          value=Signal::derive(move || freq.get().map_or_else(String::new, |v| v.to_string()))
          on_change=Callback::new(move |v: String| freq.set(v.trim().parse::<f64>().ok()))
          controls=false
        />
      </Field>
      <Field label=Signal::derive(move || t("tools.distance-km")) r#for=dist_id.clone()>
        <NumberField
          id=dist_id
          value=Signal::derive(move || dist.get().map_or_else(String::new, |v| v.to_string()))
          on_change=Callback::new(move |v: String| dist.set(v.trim().parse::<f64>().ok()))
          controls=false
        />
      </Field>
      <Field
        label=Signal::derive(move || t("tools.rain-rate-001"))
        hint=Signal::derive(move || t("tools.rain-rate-hint"))
        r#for=rain_id.clone()
      >
        <NumberField
          id=rain_id
          value=Signal::derive(move || rain.get().map_or_else(String::new, |v| v.to_string()))
          on_change=Callback::new(move |v: String| rain.set(v.trim().parse::<f64>().ok()))
          controls=false
        />
      </Field>
      <Field label=Signal::derive(move || t("tools.rain-pol")) r#for=pol_id.clone()>
        <NativeSelect
          id=pol_id
          value=Signal::derive(move || pol.get())
          on_change=Callback::new(move |v: String| pol.set(v))
          options=pol_options
        />
      </Field>
      <Field label=Signal::derive(move || t("tools.rain-percent")) r#for=percent_id.clone()>
        <NumberField
          id=percent_id
          value=Signal::derive(move || percent.get().map_or_else(String::new, |v| v.to_string()))
          on_change=Callback::new(move |v: String| percent.set(v.trim().parse::<f64>().ok()))
          controls=false
        />
      </Field>
      <div class=format!("{RESULT} space-y-1 tabular-nums") role="status">
        {move || match outcome.get() {
          Ok(o) => {
            let capped = o.path_factor_capped;
            let warnings = o.warnings.clone();
            view! {
              <div class="text-foreground">
                {tf(
                  "tools.rain-result",
                  &[&fmt_num(o.a_p_db), &percent.get().map_or_else(String::new, |v| v.to_string())],
                )}
              </div>
              <div>{tf("tools.rain-result-rarest", &[&fmt_num(o.a_001_db)])}</div>
              <div>
                {tf(
                  "tools.rain-specific",
                  &[&fmt_num(o.specific_db_per_km), &fmt_num(o.k), &format!("{:.3}", o.alpha)],
                )}
              </div>
              <div>
                {tf(
                  "tools.rain-effective",
                  &[&format!("{:.3}", o.path_factor), &fmt_num(o.effective_km)],
                )}
              </div>
              {capped.then(|| view! { <div>{move || t("tools.rain-capped")}</div> })}
              {warnings.into_iter().map(|w| view! { <div>{warning_text(w)}</div> }).collect_view()}
            }
            .into_any()
          }
          Err(e) => view! { <div>{error_text(e)}</div> }.into_any(),
        }}
      </div>
      <p class="sm:col-span-2 text-xs text-muted-foreground">{move || t("tools.rain-note")}</p>
    </div>
  }
}

fn warning_text(w: RainWarning) -> String {
  match w {
    RainWarning::FrequencyAboveCommonLimit => {
      tf("tools.rain-warn-freq", &[&COMMON_LIMIT_GHZ.to_string()])
    }
    RainWarning::PathAboveCommonLimit => {
      tf("tools.rain-warn-path", &[&COMMON_LIMIT_KM.to_string()])
    }
  }
}

fn error_text(e: RainError) -> String {
  match e {
    RainError::Frequency => tf(
      "tools.rain-error-freq",
      &[&FREQ_RANGE_GHZ.0.to_string(), &FREQ_RANGE_GHZ.1.to_string()],
    ),
    RainError::Distance => t("tools.rain-error-dist"),
    RainError::RainRate => t("tools.rain-error-rate"),
    RainError::Percent => tf(
      "tools.rain-error-percent",
      &[&PERCENT_RANGE.0.to_string(), &PERCENT_RANGE.1.to_string()],
    ),
  }
}
