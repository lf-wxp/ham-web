//! 单短截线匹配：给定负载阻抗与频率，给出接入点与短截线长度。

use ham_web_core::stub_match::{self, StubKind};
use ham_web_core::unit_math::format_distance_m;
use ham_web_core::waveguide::C;
use leptos::prelude::*;

use super::RESULT;
use crate::i18n::{t, tf};
use crate::ui::{Field, NumberField, RadioGroup, RadioGroupItem};
use crate::util::unique_id;

#[component]
pub(super) fn StubMatch() -> impl IntoView {
  let r = RwSignal::new(String::from("100"));
  let x = RwSignal::new(String::from("0"));
  let z0 = RwSignal::new(String::from("50"));
  let f = RwSignal::new(String::from("14.2"));
  let vf = RwSignal::new(String::from("0.66"));
  let short = RwSignal::new(true);

  let r_id = unique_id("stub-r");
  let x_id = unique_id("stub-x");
  let z0_id = unique_id("stub-z0");
  let f_id = unique_id("stub-f");
  let vf_id = unique_id("stub-vf");
  let open_id = unique_id("stub-open");
  let short_id = unique_id("stub-short");

  view! {
    <div class="grid gap-3 sm:grid-cols-2">
      // 标签与 NEC 的负载电阻同一条词条：中文释义要求全站唯一（i18n 门禁），同义文字不能各建一条 key。
      <Field label=Signal::derive(move || t("tools.nec-load-r")) r#for=r_id.clone()>
        <NumberField id=r_id value=Signal::derive(move || r.get()) on_change=Callback::new(move |v: String| r.set(v)) controls=false />
      </Field>
      <Field label=Signal::derive(move || t("tools.stub-match-x")) r#for=x_id.clone()>
        <NumberField id=x_id value=Signal::derive(move || x.get()) on_change=Callback::new(move |v: String| x.set(v)) controls=false />
      </Field>
      <Field label=Signal::derive(move || t("tools.stub-match-z0")) r#for=z0_id.clone()>
        <NumberField id=z0_id value=Signal::derive(move || z0.get()) on_change=Callback::new(move |v: String| z0.set(v)) controls=false />
      </Field>
      <Field label=Signal::derive(move || t("log.frequency-mhz")) r#for=f_id.clone()>
        <NumberField id=f_id value=Signal::derive(move || f.get()) on_change=Callback::new(move |v: String| f.set(v)) controls=false />
      </Field>
      <Field
        label=Signal::derive(move || t("tools.vna-tdr-vf-label"))
        hint=Signal::derive(move || t("tools.vna-tdr-vf-hint"))
        r#for=vf_id.clone()
      >
        <NumberField id=vf_id value=Signal::derive(move || vf.get()) on_change=Callback::new(move |v: String| vf.set(v)) step=0.01 min=0.01 max=1.0 controls=false />
      </Field>

      <RadioGroup
        value=Signal::derive(move || if short.get() { "short" } else { "open" }.to_string())
        on_change=Callback::new(move |v: String| short.set(v == "short"))
        aria_label=Signal::derive(move || t("tools.stub-match-kind"))
        class="flex flex-wrap items-end gap-4 pb-2"
      >
        <label class="flex cursor-pointer items-center gap-2 text-sm">
          <RadioGroupItem value="short" id=short_id />
          {move || t("tools.stub-match-short")}
        </label>
        <label class="flex cursor-pointer items-center gap-2 text-sm">
          <RadioGroupItem value="open" id=open_id />
          {move || t("tools.stub-match-open")}
        </label>
      </RadioGroup>

      <div class=format!("{RESULT} space-y-1 tabular-nums")>
        {move || {
          let (Ok(rr), Ok(xx), Ok(zz), Ok(ff), Ok(vv)) = (
            r.get().trim().parse::<f64>(),
            x.get().trim().parse::<f64>(),
            z0.get().trim().parse::<f64>(),
            f.get().trim().parse::<f64>(),
            vf.get().trim().parse::<f64>(),
          ) else {
            return view! { <p>{t("tools.stub-match-invalid")}</p> }.into_any();
          };
          // 负电阻不是「纯电抗负载」，是填错了：归到输入非法，别让用户以为自己的负载只是
          // 电抗太大。`R = 0` 仍交给内核走「纯电抗」那一支（有专门文案）。
          if ![rr, xx, zz, ff, vv].iter().all(|v| v.is_finite())
            || rr < 0.0
            || zz <= 0.0
            || ff <= 0.0
            || vv <= 0.0
            || vv > 1.0
          {
            return view! { <p>{t("tools.stub-match-invalid")}</p> }.into_any();
          }
          let kind = if short.get() { StubKind::Short } else { StubKind::Open };
          match stub_match::single_stub(rr / zz, xx / zz, kind) {
            None => view! { <p>{t("tools.stub-match-unmatchable")}</p> }.into_any(),
            // 长度 0 只可能来自「已经匹配」的早退分支（浮点上是精确的 0）。
            Some(m) if m.stub_length_wl <= f64::EPSILON => {
              view! { <p>{t("tools.stub-match-already")}</p> }.into_any()
            }
            Some(m) => {
              let wl = vv * C / (ff * 1e6);
              view! {
                <p>
                  {tf(
                    "tools.stub-match-result",
                    &[
                      &format!("{:.3}", m.distance_wl),
                      &format_distance_m(m.distance_wl * wl),
                      &format!("{:.3}", m.stub_length_wl),
                      &format_distance_m(m.stub_length_wl * wl),
                    ],
                  )}
                </p>
              }
                .into_any()
            }
          }
        }}
      </div>
    </div>
  }
}
