//! TDR 故障定位面板：速率因子输入 + 距离域回波曲线 + 反射点列表。
//!
//! 计算在 [`ham_web_core::tdr::tdr`]（有单测），这里只画图与展示：把归一化回波
//! （最强 0 dB）画成「距离 – 相对幅度」，并在检测到的反射点上打标记。

use std::fmt::Write as _;

use ham_web_core::tdr::{self, TdrError};
use ham_web_core::touchstone::Sweep;
use ham_web_core::unit_math::format_distance_m;
use leptos::prelude::*;

use super::NOTE;
use crate::i18n::{t, tf};
use crate::ui::{ControlSize, Field, NumberField};
use crate::util::unique_id;

const W: f64 = 320.0;
const H: f64 = 180.0;
const PAD: f64 = 28.0;
/// 纵轴下沿（dB，相对最强反射）。
const DB_MIN: f64 = -50.0;

/// 纵轴刻度文案：`0` 不带符号，其余用真正的减号（U+2212，与排版规范一致）。
fn db_tick(db: f64) -> String {
  if db >= 0.0 {
    "0".to_owned()
  } else {
    format!("−{:.0}", -db)
  }
}

/// 测得的 dB 值（一位小数）：与 [`db_tick`] 同一套减号，且不会出现「−0.0」。
fn db_value(db: f64) -> String {
  // 先取整到一位小数：`-0.04` 直接格式化会得到「−0.0」，读起来像负数噪声。
  let tenths = (db * 10.0).round() / 10.0;
  if tenths == 0.0 {
    // `-0.0 == 0.0` 为真，`{:.1}` 却会打印成「-0.0」，所以先归零。
    "0.0".to_owned()
  } else if tenths > 0.0 {
    format!("{tenths:.1}")
  } else {
    format!("−{:.1}", -tenths)
  }
}

/// TDR 失败原因 → 界面文案。
fn tdr_error_text(e: TdrError) -> String {
  match e {
    TdrError::TooFewPoints => t("tools.vna-tdr-err-few"),
    // 上限与坏点号都来自内核，文案里的数字必须是同一份事实，不要在这里另写。
    TdrError::TooManyPoints => tf("tools.vna-tdr-err-many", &[&tdr::MAX_POINTS.to_string()]),
    TdrError::NotUniform(index) => tf("tools.vna-tdr-err-uniform", &[&index.to_string()]),
    // 内核防御：速率因子在下面已滤到 `(0, 1]`，正常走不到这里（越界由输入框旁的 alert 提示）。
    TdrError::BadVelocityFactor => t("tools.vna-tdr-err-vf"),
  }
}

#[component]
pub(super) fn TdrPanel(sweep: Memo<Option<Sweep>>, vf_text: RwSignal<String>) -> impl IntoView {
  // 速率因子由页面持有：面板所在的结果区在一个响应式闭包里，切语言会整段重建，
  // 状态放在这里就会被重置成默认值（见 `vna_reader_page` 的注释）。
  // 同一份 id 既当 `r#for` 又当控件 `id`：面板出现两次时会撞，所以走 unique_id。
  let vf_id = unique_id("vna-tdr-vf");
  let vf = Memo::new(move |_| {
    vf_text
      .get()
      .trim()
      .parse::<f64>()
      .ok()
      .filter(|v| v.is_finite() && *v > 0.0 && *v <= 1.0)
  });
  // 结果收进 `Memo`：同一份扫频 + 同一个速率因子只跑一次 IFFT。没有它的话，下面视图里读
  // 语言包（`t` / `tf`）的每一次重渲都会把上万点的 FFT 再算一遍。
  //
  // 用 `with` 借用而不是 `sweep.get()`：后者会把整份 `Vec<S11Point>` 克隆一份（6.5 万点约
  // 1.5 MB），而速率因子是逐键输入的，每敲一位就白拷一次。
  let plot =
    Memo::new(move |_| sweep.with(|s| s.as_ref().zip(vf.get()).map(|(s, vf)| tdr::tdr(s, vf))));

  view! {
    <div class="space-y-2" data-slot="vna-tdr">
      <div class="flex flex-wrap items-end gap-x-4 gap-y-2">
        <h3 class="text-sm font-semibold">{move || t("tools.vna-tdr-title")}</h3>
        <Field
          label=Signal::derive(move || t("tools.vna-tdr-vf-label"))
          hint=Signal::derive(move || t("tools.vna-tdr-vf-hint"))
          r#for=vf_id.clone()
          class="max-w-32"
        >
          <NumberField
            id=vf_id
            value=Signal::derive(move || vf_text.get())
            on_change=Callback::new(move |v: String| vf_text.set(v))
            step=0.01
            min=0.01
            max=1.0
            size=ControlSize::Sm
          />
        </Field>
      </div>

      {move || {
        // 这一层只做「有没有数据 / 输入对不对」的判断，下面用 `with` 借用结果：
        // `Memo::get()` 会把整份数据克隆一份（`Tdr` 有两条上万点的 Vec），每次重渲都拷一遍没必要。
        if sweep.with(Option::is_none) {
          return view! { <p class=NOTE>{move || t("tools.vna-empty")}</p> }.into_any();
        }
        if vf.with(Option::is_none) {
          // 越界要直说「输错了」：`Field` 的 hint 已经说明了合理取值，不能再把同一句提示当结果重复一遍。
          return view! {
            <p class=NOTE role="alert">{move || t("tools.vna-tdr-err-vf")}</p>
          }
            .into_any();
        }
        plot.with(|plot| match plot {
          Some(Err(e)) => view! { <p class=NOTE role="alert">{tdr_error_text(*e)}</p> }.into_any(),
          Some(Ok(r)) => {
            let x = move |d: f64| PAD + d / r.max_distance_m * (W - 2.0 * PAD);
            // 夹到绘图框内：比 −50 dB 更低的点不该跑到框外（SVG 虽会裁掉，但曲线会在底部凭空消失）。
            let y = |db: f64| {
              let db = db.clamp(DB_MIN, 0.0);
              H - PAD - (db - DB_MIN) / (0.0 - DB_MIN) * (H - 2.0 * PAD)
            };
            // 抽稀后一列最多两个点（谷 + 峰），一次 `write!` 写进预分配的缓冲：
            // 逐 bin 拼字符串会为上千个点各分配一次，且这段字符串每次重渲都要重建。
            let columns = ((W - 2.0 * PAD) / 2.0).max(1.0) as usize;
            let points = r.plot_points(columns);
            let mut path = String::with_capacity(points.len() * 16);
            for p in &points {
              path.push_str(if p.segment_start { "M" } else { "L" });
              let _ = write!(path, "{:.2},{:.2} ", x(p.distance_m), y(p.magnitude_db));
            }
            let peak_marks = r
              .peaks
              .iter()
              .map(|p| {
                let (cx, cy) = (x(p.distance_m), y(p.magnitude_db));
                view! {
                  <circle cx=cx cy=cy r="2.2" class="fill-destructive" />
                }
              })
              .collect_view();
            // 门限虚线读内核常量，别在这里另写一个 −30：改了门限图会跟着走。
            let threshold = tdr::PEAK_THRESHOLD_DB;
            view! {
              <div class="space-y-1">
                <svg
                  viewBox=format!("0 0 {W} {H}")
                  class="w-full"
                  role="img"
                  aria-label=move || t("tools.vna-tdr-curve")
                  data-slot="vna-tdr-plot"
                >
                  <rect x=PAD y=PAD width=W - 2.0 * PAD height=H - 2.0 * PAD fill="none" class="stroke-muted-foreground/50" stroke-width="0.5" />
                  <line x1=PAD y1=y(threshold) x2=W - PAD y2=y(threshold) class="stroke-amber-500" stroke-width="0.8" stroke-dasharray="4 3" />
                  <text x=PAD - 4.0 y=y(0.0) + 3.0 text-anchor="end" class="fill-muted-foreground" font-size="9">{db_tick(0.0)}</text>
                  <text x=PAD - 4.0 y=y(threshold) + 3.0 text-anchor="end" class="fill-muted-foreground" font-size="9">{db_tick(threshold)}</text>
                  <text x=PAD - 4.0 y=H - PAD + 3.0 text-anchor="end" class="fill-muted-foreground" font-size="9">{db_tick(DB_MIN)}</text>
                  <path d=path fill="none" class="stroke-primary" stroke-width="1.5" />
                  {peak_marks}
                </svg>
                <div class=format!("flex justify-between {NOTE}")>
                  <span>{"0"}</span>
                  <span>{move || t("tools.vna-tdr-distance")}</span>
                  <span>{format_distance_m(r.max_distance_m)}</span>
                </div>
                <p class=NOTE>
                  {tf("tools.vna-tdr-resolution", &[&format_distance_m(r.resolution_m), &format_distance_m(r.max_distance_m)])}
                </p>
                {if r.peaks.is_empty() {
                  // 门限数字来自内核常量：改了门限，文案跟着走。
                  view! { <p class=NOTE>{tf("tools.vna-tdr-no-peaks", &[&db_tick(threshold)])}</p> }.into_any()
                } else {
                  view! {
                    <ul class="space-y-0.5">
                      {r.peaks.iter().map(|p| {
                        let dist = format_distance_m(p.distance_m);
                        view! {
                          <li class="text-sm tabular-nums">
                            {tf("tools.vna-tdr-peak-item", &[&dist, &db_value(p.magnitude_db)])}
                          </li>
                        }
                      }).collect_view()}
                    </ul>
                  }
                    .into_any()
                }}
              </div>
            }
              .into_any()
          }
          // 上面两支已排除「缺数据 / 缺速率因子」，这里只是类型兜底。
          None => view! { <p class=NOTE>{move || t("tools.vna-tdr-vf-hint")}</p> }.into_any(),
        })
      }}
    </div>
  }
}
