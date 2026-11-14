use ham_web_core::rig_panel::{
  QRM_CELL, QrmKind, qrm_canvas, qrm_cell_x, qrm_cell_y, qrm_levels, qrm_max_cols,
};
use leptos::prelude::*;

use crate::i18n::t;

/// 干扰现象的名字（「过调制溅射」）。字面量写在这里：`check-i18n` 只认调用点的直接实参。
#[must_use]
pub fn qrm_name(kind: QrmKind) -> String {
  match kind {
    QrmKind::Bursts => t("rpg.qrm-bursts"),
    QrmKind::Carrier => t("rpg.qrm-carrier"),
    QrmKind::Collision => t("rpg.qrm-collision"),
    QrmKind::Garble => t("rpg.qrm-garble"),
    QrmKind::Splatter => t("rpg.qrm-splatter"),
    QrmKind::Comb => t("rpg.qrm-comb"),
    QrmKind::Echo => t("rpg.qrm-echo"),
    QrmKind::Fade => t("rpg.qrm-fade"),
    QrmKind::Spark => t("rpg.qrm-spark"),
    QrmKind::Static => t("rpg.qrm-static"),
  }
}

/// 干扰信号的频谱形象：一列列点阵柱，形状由 `rig_panel::qrm_levels` 决定（每个分类一种真实干扰现象）。
///
/// 代替怪物精灵（`docs/ui/decisions.md` §11.6）。`strength`（0–1）把整体压低：连续答对一次，
/// 干扰就矮一截，降到 0 只剩底噪。`animate` 时画两帧交替（CSS 逐帧切换，`pxl-qrm-frame`），
/// 列表里大量出现时传 `false` 只画静帧。纯装饰（`aria-hidden`）：分类名与强度另有文字。
/// 只放在屏幕（`pxl-display`）里。
#[component]
pub fn QrmSignature(
  kind: QrmKind,
  #[prop(into, default = Signal::stored(1.0))] strength: Signal<f64>,
  #[prop(default = 16)] cols: usize,
  #[prop(default = 8)] rows: u8,
  #[prop(optional)] animate: bool,
) -> impl IntoView {
  // 列数先截到画布放得下的上限：列坐标（`列 × step`）直接用 u16 乘，不会溢出。
  let cols = cols.min(qrm_max_cols());
  let (w, h) = qrm_canvas(cols, rows);
  let frames: &[u8] = if animate { &[0, 1] } else { &[0] };
  let frame_view = move |frame: u8| {
    let s = strength.get().clamp(0.0, 1.0);
    let levels = qrm_levels(kind, cols, rows, frame);
    view! {
      <g class=animate.then_some("pxl-qrm-frame") data-frame=frame>
        {levels
          .into_iter()
          .enumerate()
          .flat_map(|(c, lv)| {
            // 强度压低时只削高出底噪的部分：底噪那一格永远亮着，干扰「消除」后屏幕上剩一条平线。
            let lit = 1 + (f64::from(lv.saturating_sub(1)) * s).round() as u8;
            let x = qrm_cell_x(c);
            (0..rows).map(move |r| {
              let y = qrm_cell_y(r, rows);
              let tier = if r + 1 > rows.saturating_sub(2) {
                "alarm"
              } else if r + 1 > rows / 2 {
                "hot"
              } else {
                "fg"
              };
              view! {
                <rect
                  class="pxl-qrm-cell"
                  data-lit=(r < lit).then_some("")
                  data-tier=(r < lit).then_some(tier)
                  x=x
                  y=y
                  width=QRM_CELL
                  height=QRM_CELL
                />
              }
            })
          })
          .collect_view()}
      </g>
    }
  };
  view! {
    // 两帧轮换是无限循环：滚出视口就停（`motion::pause_offscreen_in` 打 `data-offscreen`）。
    <svg
      class="pxl-qrm"
      data-kind=kind.key()
      data-pause-offscreen=animate.then_some("")
      width=w
      height=h
      viewBox=format!("0 0 {w} {h}")
      aria-hidden="true"
    >
      {move || frames.iter().map(|f| frame_view(*f)).collect_view()}
    </svg>
  }
}
