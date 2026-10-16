use ham_web_core::waveguide::{BAND_HIGH, BAND_LOW, Family, Mode, Operating, standard};
use leptos::prelude::*;

use super::fmt_num;
use super::waveguide_calculator::Report;
use crate::i18n::{t, tf};

fn mode_name(m: &Mode) -> String {
  let family = match m.family {
    Family::Te => "TE",
    Family::Tm => "TM",
  };
  format!("{family}{}{}", m.m, m.n)
}

/// 波导计算结果块。
#[component]
pub(super) fn WaveguideReport(report: Report, size: String) -> impl IntoView {
  let published = standard(&size).map(|s| s.band_ghz);
  let (lo, hi) = report.single_mode_ghz;
  let fc = report.fc_ghz;
  let operating = report.operating;
  let modes = report
    .modes
    .iter()
    .map(|m| format!("{} {} GHz", mode_name(m), fmt_num(m.fc_hz / 1e9)))
    .collect::<Vec<_>>()
    .join(" · ");

  // 组件体不是响应式上下文：t()/tf() 直接调用会在挂载时冻结，切语言后整块停在旧语言。
  // 全部文案包成派生信号，与页面其余 `move || t(...)` 的更新节奏一致。
  let cutoff = Signal::derive(move || tf("tools.waveguide-cutoff", &[&fmt_num(fc)]));
  let single_mode =
    Signal::derive(move || tf("tools.waveguide-single-mode", &[&fmt_num(lo), &fmt_num(hi)]));
  let modes_line = Signal::derive(move || tf("tools.waveguide-modes", &[&modes]));
  let band = Signal::derive(move || match published {
    Some((a, b)) => tf(
      "tools.waveguide-band-published",
      &[&fmt_num(a), &fmt_num(b)],
    ),
    None => tf(
      "tools.waveguide-band-rule",
      &[
        &BAND_LOW.to_string(),
        &BAND_HIGH.to_string(),
        &fmt_num(BAND_LOW * fc),
        &fmt_num(BAND_HIGH * fc),
      ],
    ),
  });

  let at_freq = Signal::derive(move || match operating {
    Operating::Propagating {
      wavelength_m,
      guide_wavelength_m,
      phase_velocity_ratio,
      group_velocity_ratio,
      wave_impedance_ohm,
      conductor_loss_db_per_m,
      single_mode,
      in_recommended_band,
    } => {
      let mut lines = vec![
        tf(
          "tools.waveguide-propagating",
          &[
            &fmt_num(wavelength_m * 1000.0),
            &fmt_num(guide_wavelength_m * 1000.0),
          ],
        ),
        tf(
          "tools.waveguide-velocity",
          &[
            &format!("{phase_velocity_ratio:.3}"),
            &format!("{group_velocity_ratio:.3}"),
            &fmt_num(wave_impedance_ohm),
          ],
        ),
      ];
      if let Some(l) = conductor_loss_db_per_m {
        lines.push(tf("tools.waveguide-loss", &[&fmt_num(l)]));
      }
      if !single_mode {
        lines.push(t("tools.waveguide-multimode"));
      } else if !in_recommended_band {
        lines.push(t("tools.waveguide-outside-band"));
      }
      lines
    }
    Operating::BelowCutoff {
      attenuation_db_per_m,
      db_per_cutoff_wavelength,
    } => vec![tf(
      "tools.waveguide-below-cutoff",
      &[
        &fmt_num(attenuation_db_per_m),
        &fmt_num(db_per_cutoff_wavelength),
      ],
    )],
    Operating::AtCutoff => vec![t("tools.waveguide-at-cutoff")],
  });

  view! {
    <div class="font-medium text-foreground">{move || cutoff.get()}</div>
    <div>{move || single_mode.get()}</div>
    <div>{move || band.get()}</div>
    <div>{move || modes_line.get()}</div>
    {move || at_freq.get().into_iter().map(|l| view! { <div>{l}</div> }).collect_view()}
  }
}
