use ham_web_core::waveguide::{
  Geometry, Mode, Operating, STANDARDS, WgError, lowest_modes, operate, single_mode_range_hz,
  standard,
};
use leptos::prelude::*;

use super::RESULT;
use super::waveguide_report::WaveguideReport;
use crate::i18n::t;
use crate::ui::{Field, NativeSelect, NumberField, SelectOption};
use crate::util::unique_id;

const CUSTOM: &str = "custom";
/// 列出最低几个模式。
const MODE_COUNT: usize = 5;

/// 取 3 位小数，输入框里才不会显示 22.860000000000003。
fn round3(x: f64) -> f64 {
  (x * 1000.0).round() / 1000.0
}

/// 一次计算的全部结果。
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Report {
  pub(super) fc_ghz: f64,
  pub(super) single_mode_ghz: (f64, f64),
  pub(super) modes: Vec<Mode>,
  pub(super) operating: Operating,
}

/// 矩形波导：截止频率、模式、导波波长、波阻抗、导体损耗与截止以下的衰减。
#[component]
pub(super) fn WaveguideCalculator() -> impl IntoView {
  // 默认尺寸取 WR-90（X 波段最常用）。表里万一没有它，退到表的第一项 —— 挂载期
  // `expect` 会让整页白屏，而这里能给的本来也只是「初始值」。
  let (a0, b0) = standard("WR-90")
    .or_else(|| STANDARDS.first())
    .map_or((None, None), |s| {
      (Some(round3(s.a_mm())), Some(round3(s.b_mm())))
    });
  let size = RwSignal::new("WR-90".to_owned());
  // 数值一律存 `Option<f64>`：空输入 = 未填写。计算时把 `None` 换成 `NAN` 交给
  // `Geometry::validate` / `operate` 的校验，报对应的错误 —— 输入框空着时结果区给出
  // 提示，而不是用旧值算出与界面不符的数字。
  let a_mm = RwSignal::new(a0);
  let b_mm = RwSignal::new(b0);
  let er = RwSignal::new(Some(1.0));
  let freq_ghz = RwSignal::new(Some(10.368));
  // 电导率（MS/m）：铜 58。
  let sigma = RwSignal::new(Some(58.0));

  let size_id = unique_id("waveguide-size");
  let a_id = unique_id("waveguide-a");
  let b_id = unique_id("waveguide-b");
  let er_id = unique_id("waveguide-er");
  let freq_id = unique_id("waveguide-freq");
  let sigma_id = unique_id("waveguide-sigma");

  let mut size_options: Vec<SelectOption> = STANDARDS
    .iter()
    .map(|s| SelectOption::new(s.name, s.name))
    .collect();
  size_options.push(SelectOption::new(
    CUSTOM,
    Signal::derive(move || t("tools.waveguide-custom")),
  ));

  let report = Memo::new(move |_| -> Result<Report, WgError> {
    let g = Geometry {
      a_m: a_mm.get().unwrap_or(f64::NAN) / 1000.0,
      b_m: b_mm.get().unwrap_or(f64::NAN) / 1000.0,
      er: er.get().unwrap_or(f64::NAN),
    };
    let operating = operate(
      &g,
      freq_ghz.get().unwrap_or(f64::NAN) * 1e9,
      Some(sigma.get().unwrap_or(f64::NAN) * 1e6),
    )?;
    let (lo, hi) = single_mode_range_hz(&g)?;
    Ok(Report {
      fc_ghz: lo / 1e9,
      single_mode_ghz: (lo / 1e9, hi / 1e9),
      modes: lowest_modes(&g, MODE_COUNT)?,
      operating,
    })
  });

  view! {
    <div class="grid grid-cols-1 gap-3 sm:grid-cols-2">
      <Field label=Signal::derive(move || t("tools.waveguide-size")) r#for=size_id.clone()>
        <NativeSelect
          id=size_id
          value=Signal::derive(move || size.get())
          on_change=Callback::new(move |v: String| {
            if let Some(s) = standard(&v) {
              a_mm.set(Some(round3(s.a_mm())));
              b_mm.set(Some(round3(s.b_mm())));
            }
            size.set(v);
          })
          options=size_options
        />
      </Field>
      <Field label=Signal::derive(move || t("tools.waveguide-er")) r#for=er_id.clone()>
        <NumberField
          id=er_id
          value=Signal::derive(move || er.get().map_or_else(String::new, |v| v.to_string()))
          on_change=Callback::new(move |v: String| er.set(v.trim().parse::<f64>().ok()))
          controls=false
        />
      </Field>
      <Field label=Signal::derive(move || t("tools.waveguide-a-mm")) r#for=a_id.clone()>
        <NumberField
          id=a_id
          value=Signal::derive(move || a_mm.get().map_or_else(String::new, |v| v.to_string()))
          on_change=Callback::new(move |v: String| {
            match v.trim().parse::<f64>().ok() {
              Some(v) => {
                a_mm.set(Some(v));
                size.set(CUSTOM.to_owned());
              }
              None => a_mm.set(None),
            }
          })
          controls=false
        />
      </Field>
      <Field label=Signal::derive(move || t("tools.waveguide-b-mm")) r#for=b_id.clone()>
        <NumberField
          id=b_id
          value=Signal::derive(move || b_mm.get().map_or_else(String::new, |v| v.to_string()))
          on_change=Callback::new(move |v: String| {
            match v.trim().parse::<f64>().ok() {
              Some(v) => {
                b_mm.set(Some(v));
                size.set(CUSTOM.to_owned());
              }
              None => b_mm.set(None),
            }
          })
          controls=false
        />
      </Field>
      <Field label=Signal::derive(move || t("tools.waveguide-freq-ghz")) r#for=freq_id.clone()>
        <NumberField
          id=freq_id
          value=Signal::derive(move || freq_ghz.get().map_or_else(String::new, |v| v.to_string()))
          on_change=Callback::new(move |v: String| freq_ghz.set(v.trim().parse::<f64>().ok()))
          controls=false
        />
      </Field>
      <Field
        label=Signal::derive(move || t("tools.waveguide-sigma"))
        hint=Signal::derive(move || t("tools.waveguide-sigma-hint"))
        r#for=sigma_id.clone()
      >
        <NumberField
          id=sigma_id
          value=Signal::derive(move || sigma.get().map_or_else(String::new, |v| v.to_string()))
          on_change=Callback::new(move |v: String| sigma.set(v.trim().parse::<f64>().ok()))
          controls=false
        />
      </Field>
      <div class=format!("{RESULT} space-y-1 tabular-nums") role="status">
        {move || match report.get() {
          Ok(r) => view! { <WaveguideReport report=r size=size.get() /> }.into_any(),
          Err(e) => view! { <div>{error_text(e)}</div> }.into_any(),
        }}
      </div>
      <p class="sm:col-span-2 text-xs text-muted-foreground">{move || t("tools.waveguide-note")}</p>
    </div>
  }
}

fn error_text(e: WgError) -> String {
  t(match e {
    WgError::Size => "tools.waveguide-error-size",
    WgError::HeightExceedsWidth => "tools.waveguide-error-height",
    WgError::Permittivity => "tools.waveguide-error-er",
    WgError::Frequency => "tools.waveguide-error-freq",
    WgError::Conductivity => "tools.waveguide-error-sigma",
  })
}
