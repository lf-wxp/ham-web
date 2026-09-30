use leptos::prelude::*;

use super::{INPUT, fmt_num};

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

  view! {
    <div class="grid gap-3 sm:grid-cols-2">
      <label class="flex flex-col gap-1.5 text-sm">
        <span class="text-xs text-muted-foreground">"负载电阻（Ω，谐振点）"</span>
        <input
          type="number"
          prop:value=move || resist.get().to_string()
          on:input=move |e| {
            if let Ok(v) = event_target_value(&e).parse::<f64>() {
              resist.set(v);
            }
          }
          class=INPUT
        />
      </label>
      <label class="flex flex-col gap-1.5 text-sm">
        <span class="text-xs text-muted-foreground">"频率（MHz）"</span>
        <input
          type="number"
          prop:value=move || freq.get().to_string()
          on:input=move |e| {
            if let Ok(v) = event_target_value(&e).parse::<f64>() {
              freq.set(v);
            }
          }
          class=INPUT
        />
      </label>
      <div class="sm:col-span-2 rounded-lg bg-muted/40 px-3 py-2 text-sm tabular-nums text-muted-foreground">
        {move || {
          match l_match(resist.get(), freq.get()) {
            Some((lp, cs, cp, ls)) => format!(
              "低通型：并联 {} μH + 串联 {} pF　｜　高通型：并联 {} pF + 串联 {} μH",
              fmt_num(lp),
              fmt_num(cs),
              fmt_num(cp),
              fmt_num(ls),
            ),
            None => "请输入正的电阻与频率（电阻不等于 50Ω）。".to_owned(),
          }
        }}
      </div>
      <p class="sm:col-span-2 text-xs text-muted-foreground">
        "L 型网络把负载电阻共轭匹配到 50Ω，消除反射。负载 <50Ω 时并联元件在负载侧，>50Ω 时串联元件在源侧。元件值按谐振点纯电阻计算，含电抗时请先将其调至谐振。"
      </p>
    </div>
  }
}
