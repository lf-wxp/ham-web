use leptos::prelude::*;

use super::fmt_num;
use crate::i18n::{t, tf};
use crate::ui::{Field, NumberField};
use crate::util::unique_id;

/// 计算 L 型匹配网络元件值（负载纯电阻匹配到 50Ω）。
/// 返回 `(低通: 并联电感 μH, 串联电容 pF, 高通: 并联电容 pF, 串联电感 μH)`。
fn l_match(resist: f64, freq_mhz: f64) -> Option<(f64, f64, f64, f64)> {
  if resist <= 0.0 || freq_mhz <= 0.0 || (resist - 50.0).abs() < 1e-9 {
    return None;
  }
  let z0 = 50.0;
  let omega = std::f64::consts::TAU * freq_mhz * 1e6;
  if resist < z0 {
    let q = (z0 / resist - 1.0).sqrt();
    let xp = z0 / q; // 并联电抗（跨负载）
    let xs = q * resist; // 串联电抗（接源）
    let lp_uh = xp / omega * 1e6;
    let cs_pf = 1.0 / (omega * xs) * 1e12;
    let cp_pf = 1.0 / (omega * xp) * 1e12;
    let ls_uh = xs / omega * 1e6;
    Some((lp_uh, cs_pf, cp_pf, ls_uh))
  } else {
    let q = (resist / z0 - 1.0).sqrt();
    let xs = q * z0; // 串联电抗
    let xp = resist / q; // 并联电抗
    let ls_uh = xs / omega * 1e6;
    let cp_pf = 1.0 / (omega * xp) * 1e12;
    let cs_pf = 1.0 / (omega * xs) * 1e12;
    let lp_uh = xp / omega * 1e6;
    Some((lp_uh, cs_pf, cp_pf, ls_uh))
  }
}

/// 天线匹配网络：L 型网络把负载电阻匹配到 50Ω。
#[component]
pub(super) fn AntennaMatcher() -> impl IntoView {
  let resist = RwSignal::new(12.0);
  let freq = RwSignal::new(7.1);

  let resist_id = unique_id("antenna-matcher-r");
  let freq_id = unique_id("antenna-matcher-f");

  view! {
    <div class="grid gap-3 sm:grid-cols-2">
      <Field label=Signal::derive(move || t("tools.load-resistance-at-resonance")) r#for=resist_id.clone()>
        <NumberField
          id=resist_id
          value=Signal::derive(move || resist.get().to_string())
          on_change=Callback::new(move |v: String| {
            if let Ok(v) = v.trim().parse::<f64>() {
              resist.set(v);
            }
          })
          controls=false
        />
      </Field>
      <Field label=Signal::derive(move || t("log.frequency-mhz")) r#for=freq_id.clone()>
        <NumberField
          id=freq_id
          value=Signal::derive(move || freq.get().to_string())
          on_change=Callback::new(move |v: String| {
            if let Ok(v) = v.trim().parse::<f64>() {
              freq.set(v);
            }
          })
          controls=false
        />
      </Field>
      <div class="sm:col-span-2 rounded-lg bg-muted/40 px-3 py-2 text-sm tabular-nums text-muted-foreground">
        {move || {
          match l_match(resist.get(), freq.get()) {
            Some((lp, cs, cp, ls)) => tf(
              "tools.low-pass-shunt-h",
              &[&fmt_num(lp), &fmt_num(cs), &fmt_num(cp), &fmt_num(ls)],
            ),
            None => t("tools.enter-a-positive-resistance"),
          }
        }}
      </div>
      <p class="sm:col-span-2 text-xs text-muted-foreground">
        {move || t("tools.an-l-network-conjugate")}
      </p>
    </div>
  }
}
