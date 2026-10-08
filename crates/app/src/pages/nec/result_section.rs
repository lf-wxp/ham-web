//! 求解结果卡片：三行读数 + 方位 / 仰角两张方向图。
//!
//! 方向图的**数据与画法**分开：这里只把结果里的方向列表交给
//! [`super::polar_grid`] / [`super::elevation_grid`] 画网格，路径由
//! [`super::polar_path`] / [`super::elevation_path`] 算。

use ham_web_core::nec::{NecResult, wavelength};
use leptos::prelude::*;

use crate::i18n::{t, tf};

use super::elevation_grid::elevation_grid;
use super::polar_grid::polar_grid;
use super::{CART_H, CART_W, NOTE, PLOT, POLAR, RESULT, elevation_path, fmt, fmt_z, polar_path};

/// 求解结果。`None` 表示当前输入还解不出来（几何不完整、参数越界或矩阵奇异）。
#[component]
pub(super) fn ResultSection(
  result: Memo<Option<NecResult>>,
  freq_hz: Signal<f64>,
  len_m: Signal<f64>,
  ground_kind: RwSignal<usize>,
) -> impl IntoView {
  view! {
    {move || match result.get() {
      None => view! {
        <p class=NOTE>{move || t("tools.nec-cannot-solve")}</p>
      }
      .into_any(),
      Some(r) => {
        let z = fmt_z(r.impedance.0, r.impedance.1);
        let swr = format!("{:.2}", r.swr_50);
        let gain = format!("{:.2}", r.gain_max_dbi);
        let fb = format!("{:.1}", r.front_to_back_db);
        let elev = format!("{:.0}", r.gain_max_elevation_deg);
        let segs = r.segments.to_string();
        let unknown = r.unknowns.to_string();
        let eff = format!("{:.1}", r.efficiency * 100.0);
        let loss = format!("{:.2}", r.loss_db);
        let dir = format!("{:.2}", r.directivity_dbi);
        let peak = r.gain_max_dbi;
        let lam = wavelength(freq_hz.get());
        let elec = format!("{:.3}", len_m.get() / lam);
        let lam_s = fmt(lam);
        let az_path = polar_path(&r.azimuth, peak);
        let el_path = elevation_path(&r.elevation, peak);
        // 两条方向图数据要供多个闭包与两处读数行使用：用 `StoredValue`（Copy）共享，
        // 避免每次重渲染都克隆整条向量（360 / 181 个点）。
        let azimuth = StoredValue::new(r.azimuth);
        let elevation = StoredValue::new(r.elevation);
        view! {
          <div class="space-y-2">
            <div class=RESULT>
              {tf(
                "tools.nec-feed-impedance-swr",
                &[&z, &swr, &gain, &fb, &elev],
              )}
            </div>
            <div class=RESULT>
              {tf(
                "tools.nec-efficiency-line",
                &[&eff, &loss, &dir],
              )}
            </div>
            <div class=RESULT>
              {tf(
                "tools.nec-wavelength-electrical-length",
                &[&lam_s, &elec, &segs, &unknown],
              )}
            </div>

            <div class="grid gap-4 sm:grid-cols-2">
              <div>
                <div class=NOTE>{move || t("tools.nec-azimuth-pattern")}</div>
                <svg
                  viewBox=format!("0 0 {POLAR} {POLAR}")
                  class=PLOT
                  role="img"
                  aria-label=move || t("tools.nec-azimuth-pattern-alt")
                >
                  {polar_grid()}
                  <path
                    d=az_path
                    class="fill-none stroke-primary"
                    stroke-width="1.6"
                    vector-effect="non-scaling-stroke"
                  />
                </svg>
                <div class=NOTE>
                  {move || {
                    // 每 45° 取一个读数：按角度**就近取**，而不是「每 45 个采样点」
                    // （后者等于假定分辨率恰为 1°，核心一改步长读数就错位）。
                    // 读数行面向所有语言，分隔用普通空格，不用全角空格 U+3000。
                    let mut list = String::new();
                    for target in [0.0f64, 45.0, 90.0, 135.0, 180.0, 225.0, 270.0, 315.0] {
                      let hit = azimuth.with_value(|v| {
                        v.iter()
                          .min_by(|a, b| (a.0 - target).abs().total_cmp(&(b.0 - target).abs()))
                          .map(|(az, db)| (*az, *db))
                      });
                      if let Some((_, db)) = hit {
                        list.push_str(&format!("{target:.0}°: {db:.1} dBi "));
                      }
                    }
                    list
                  }}
                </div>
              </div>
              <div>
                <div class=NOTE>{move || t("tools.nec-elevation-pattern")}</div>
                <svg
                  viewBox=format!("0 0 {CART_W} {CART_H}")
                  class=PLOT
                  role="img"
                  aria-label=move || t("tools.nec-elevation-pattern-alt")
                >
                  {elevation_grid(peak, ground_kind.get() != 0)}
                  <path
                    d=el_path
                    class="fill-none stroke-emerald-500"
                    stroke-width="1.6"
                    vector-effect="non-scaling-stroke"
                  />
                </svg>
                <div class=NOTE>
                  {move || {
                    let mut list = String::new();
                    for target in [0.0f64, 15.0, 30.0, 45.0, 60.0, 90.0] {
                      let hit = elevation.with_value(|v| {
                        v.iter()
                          .min_by(|a, b| (a.0 - target).abs().total_cmp(&(b.0 - target).abs()))
                          .map(|(el, db)| (*el, *db))
                      });
                      if let Some((_, db)) = hit {
                        list.push_str(&format!("{target:.0}°: {db:.1} dBi "));
                      }
                    }
                    list
                  }}
                </div>
              </div>
            </div>

            <p class=NOTE>
              {move || t("tools.nec-accuracy-note")}
            </p>
            <p class=NOTE>
              {move || t("tools.nec-model-assumptions")}
            </p>
          </div>
        }
        .into_any()
      }
    }}
  }
}
