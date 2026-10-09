//! `/gear` 里的共享映射：轴名文案。
//!
//! 雷达图与得分列表都要给六条轴取名字，两处各写一份迟早会不一致；`t()` 的字面量 key
//! 必须写在**调用点**（静态扫描靠它认条目），所以这里集中一处、两边都用它。

use ham_web_core::gear_score::Axis;

use crate::i18n::t;

/// 轴名（界面文案）。
pub(super) fn axis_label(axis: Axis) -> String {
  match axis {
    Axis::ImdNarrow => t("knowledge.axis-imd-narrow"),
    Axis::ImdWide => t("knowledge.axis-imd-wide"),
    Axis::NoiseFloor => t("knowledge.axis-noise-floor"),
    // 「功率」「模式数」这两个词词典里已经有了（`contest.power` / `radio.modes`）：
    // 中文释义全库唯一，同义的 key 只能复用一个，不能各造一份。
    Axis::Power => t("contest.power"),
    Axis::TopBand => t("knowledge.axis-top-band"),
    Axis::Modes => t("radio.modes"),
  }
}
