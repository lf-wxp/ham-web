//! 七段读数：屏幕上的数字（`docs/ui/pixel.md` §3.4）。外观在 `style/pixel/radio.css`。

use ham_web_core::rig_panel::{DIGIT_H, layout};
use leptos::prelude::*;

/// 七段读数：按像素格画成 `<rect>`，没亮的段画成幽灵段。只放在屏幕（`pxl-display`）里。
///
/// `label` 是读屏念的完整说法（「LV 7」「S9+20」）—— 七段图形本身对读屏是一张图。
/// 不传 `label` 时整块 `aria-hidden`：调用方自己在旁边放了一份文字（倒计时要让 e2e 与读屏都读到
/// `38:24` 这串字，见 `ExamClock`）。
/// `scale` 是每个像素格的屏幕像素数，只取正整数，七段边缘才落在像素格上；
/// 坐标与画布尺寸一律按 `u16` 饱和计算 —— 极端参数（超长读数 × 超大 `scale`）画出被截断的板子，但不会溢出。
/// 认得的字符见 `ham_web_core::rig_panel::segment_mask`；不认识的字符画成一格全灭的数字。
#[component]
pub fn Readout(
  #[prop(into)] text: Signal<String>,
  #[prop(optional, into)] label: Option<Signal<String>>,
  #[prop(default = 2)] scale: u16,
  #[prop(optional, into)] tone: Option<Signal<ReadoutTone>>,
) -> impl IntoView {
  let scale = scale.max(1);
  let tone = tone.unwrap_or_else(|| Signal::stored(ReadoutTone::Normal));
  let named = label.is_some();
  view! {
    <span
      class="pxl-readout"
      role=named.then_some("img")
      aria-label=move || label.map(|l| l.get())
      aria-hidden=(!named).then_some("true")
      data-tone=move || tone.get().as_str()
    >
      {move || {
        let l = layout(&text.get());
        let w = l.width.max(1).saturating_mul(scale);
        let h = u16::from(DIGIT_H).saturating_mul(scale);
        view! {
          <svg width=w height=h viewBox=format!("0 0 {w} {h}") aria-hidden="true">
            {l
              .rects
              .into_iter()
              .map(|r| {
                view! {
                  <rect
                    class="pxl-seg"
                    data-lit=r.lit.then_some("")
                    x=r.x.saturating_mul(scale)
                    y=u16::from(r.y).saturating_mul(scale)
                    width=u16::from(r.w).saturating_mul(scale)
                    height=u16::from(r.h).saturating_mul(scale)
                  />
                }
              })
              .collect_view()}
          </svg>
        }
      }}
    </span>
  }
}

/// 屏幕读数的颜色档（全部取 `--display-*`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ReadoutTone {
  /// 荧光读数色。
  #[default]
  Normal,
  /// 次要 / 没有数据（`--` 这类占位）。
  Dim,
  /// 临界：琥珀。
  Hot,
  /// 告急：橙红。
  Alarm,
  /// 锁定 / S9 以上：绿。
  Ok,
}

impl ReadoutTone {
  /// S 表读数的颜色：没数据灰、S9 以上绿、S5 以下琥珀，其余是读数色。
  ///
  /// `Some(0)` 是真实的最弱读数（考试几乎全错、某分类正确率接近 0），照样琥珀；
  /// 「还没开始、不该告警」的调用方（练习页连对为 0）自己传 `None`。
  #[must_use]
  pub const fn for_s_units(units: Option<u8>) -> Self {
    use ham_web_core::rig_panel::{S_WEAK, S9};
    match units {
      None => Self::Dim,
      Some(u) if u >= S9 => Self::Ok,
      Some(u) if u < S_WEAK => Self::Hot,
      Some(_) => Self::Normal,
    }
  }

  const fn as_str(self) -> &'static str {
    match self {
      Self::Normal => "normal",
      Self::Dim => "dim",
      Self::Hot => "hot",
      Self::Alarm => "alarm",
      Self::Ok => "ok",
    }
  }
}

#[cfg(test)]
mod tests {
  use super::ReadoutTone;
  use ham_web_core::rig_panel::{S_SEGMENTS, S_WEAK, S9};

  #[test]
  fn s_units_tone_bands() {
    assert_eq!(ReadoutTone::for_s_units(None), ReadoutTone::Dim);
    // S0 是读数而不是「没数据」：考试结果与首页最弱项靠它提醒。
    assert_eq!(ReadoutTone::for_s_units(Some(0)), ReadoutTone::Hot);
    assert_eq!(ReadoutTone::for_s_units(Some(S_WEAK - 1)), ReadoutTone::Hot);
    assert_eq!(ReadoutTone::for_s_units(Some(S_WEAK)), ReadoutTone::Normal);
    assert_eq!(ReadoutTone::for_s_units(Some(S9 - 1)), ReadoutTone::Normal);
    assert_eq!(ReadoutTone::for_s_units(Some(S9)), ReadoutTone::Ok);
    assert_eq!(ReadoutTone::for_s_units(Some(S_SEGMENTS)), ReadoutTone::Ok);
  }
}
