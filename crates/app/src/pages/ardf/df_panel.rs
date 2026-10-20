//! 测向交汇定位面板：几行「坐标 + 方位角」算出一个位置与 95% 误差椭圆。
//!
//! 固定 4 行而不是可增删的列表：交汇只需要 2–4 条方位线，动态列表在每次输入时
//! 重建行节点会丢焦点，收益抵不上代价。

use ham_web_core::df_fix::{self, DfBearing, DfError, DfFix};
use ham_web_core::grid::grid_from_lat_lon;
use leptos::prelude::*;

use super::df_row::DfRow;
use crate::i18n::{t, tf, tp};

/// 面板里的测向行数。
const ROWS: usize = 4;

/// 超过这个长半轴（米）就不值得按「一个点」用，页面提示重新测向。
const UNRELIABLE_MAJOR_M: f64 = 50_000.0;

#[derive(Clone, Copy, PartialEq)]
enum Outcome {
  NeedMore,
  BadInput,
  Parallel,
  Fix(DfFix),
}

#[component]
pub(super) fn DfPanel() -> impl IntoView {
  let lat: [RwSignal<String>; ROWS] = std::array::from_fn(|_| RwSignal::new(String::new()));
  let lon: [RwSignal<String>; ROWS] = std::array::from_fn(|_| RwSignal::new(String::new()));
  let azimuth: [RwSignal<String>; ROWS] = std::array::from_fn(|_| RwSignal::new(String::new()));
  let sigma: [RwSignal<String>; ROWS] = std::array::from_fn(|_| RwSignal::new(String::from("2")));

  // `Memo` 而不是 `Signal::derive`：下面的结果区挂在 `aria-live` 上，每敲一个键都重算并重建
  // 的话，读屏会反复念同一段；`Memo` 在结果没变时不通知，DOM 就不动。
  let outcome = Memo::new(move |_| {
    let mut list: Vec<DfBearing> = Vec::new();
    for i in 0..ROWS {
      match df_fix::parse_row(
        &lat[i].get(),
        &lon[i].get(),
        &azimuth[i].get(),
        &sigma[i].get(),
      ) {
        Ok(Some(bearing)) => list.push(bearing),
        Ok(None) => {}
        Err(_) => return Outcome::BadInput,
      }
    }
    if list.len() < 2 {
      return Outcome::NeedMore;
    }
    match df_fix::solve(&list) {
      Ok(fix) => Outcome::Fix(fix),
      Err(DfError::Parallel) => Outcome::Parallel,
      Err(DfError::TooFewBearings) => Outcome::NeedMore,
      Err(DfError::InvalidInput) => Outcome::BadInput,
    }
  });

  view! {
    <section class="space-y-4" aria-labelledby="df-fix-heading">
      <h2 id="df-fix-heading" class="pxl-section-head">
        {move || t("ardf.df-title")}
      </h2>
      <p class="text-sm text-muted-foreground">{move || t("ardf.df-desc")}</p>

      <div class="space-y-3">
        {(0..ROWS)
          .map(|i| {
            view! {
              <DfRow
                index=i
                lat=lat[i]
                lon=lon[i]
                azimuth=azimuth[i]
                sigma=sigma[i]
              />
            }
          })
          .collect_view()}
      </div>

      <div class="bg-muted/40 px-3 py-2 text-sm text-muted-foreground" aria-live="polite">
        {move || {
          match outcome.get() {
            Outcome::NeedMore => {
              view! { <p>{move || t("ardf.df-need-more")}</p> }.into_any()
            }
            Outcome::BadInput => {
              view! { <p>{move || t("ardf.df-bad-input")}</p> }.into_any()
            }
            Outcome::Parallel => {
              view! { <p>{move || t("ardf.df-parallel")}</p> }.into_any()
            }
            Outcome::Fix(fix) => {
              // 网格换算不出来时给破折号，而不是让「网格：」后面留空（`core::solve` 已经
              // 保证经纬度在范围内，这里只是兜底）。
              let grid = grid_from_lat_lon(fix.lat, fix.lon)
                .filter(|g| !g.is_empty())
                .unwrap_or_else(|| "—".to_owned());
              let wide = fix.semi_major_m > UNRELIABLE_MAJOR_M;
              view! {
                <div class="space-y-1 tabular-nums">
                  <p>
                    {tf(
                      "ardf.df-fix",
                      &[&format!("{:.5}", fix.lat), &format!("{:.5}", fix.lon)],
                    )}
                  </p>
                  <p>{tf("ardf.df-grid", &[&grid])}</p>
                  <p>
                    {tf(
                      "ardf.df-ellipse",
                      &[
                        &format!("{:.0}", fix.semi_major_m),
                        &format!("{:.0}", fix.semi_minor_m),
                        &format!("{:.0}", fix.major_bearing_deg),
                      ],
                    )}
                  </p>
                  <p>
                    {tp(
                      "ardf.df-rms",
                      fix.used,
                      &[&format!("{:.2}", fix.rms_deg), &fix.used.to_string()],
                    )}
                  </p>
                  {wide
                    .then(|| {
                      view! {
                        <p class="text-amber-600 dark:text-amber-400">
                          {move || t("ardf.df-wide")}
                        </p>
                      }
                    })}
                  {(fix.behind > 0)
                    .then(|| {
                      view! {
                        <p class="text-amber-600 dark:text-amber-400">
                          {tp(
                            "ardf.df-behind",
                            fix.behind,
                            &[&fix.behind.to_string()],
                          )}
                        </p>
                      }
                    })}
                </div>
              }
                .into_any()
            }
          }
        }}
      </div>
    </section>
  }
}
