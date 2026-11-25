//! 练习里的文字：七种报文的名字与各种答错的说明。
//!
//! 词条字面量必须写在 `t()` 的直接实参位（`check-i18n` 只采集直接调用），所以是两张 `match`。

use ham_web_core::ft8_practice::{Kind, Mistake};

use crate::i18n::{t, tf};

pub(super) fn kind_name(kind: Kind) -> String {
  match kind {
    Kind::Cq => t("radio.practice-kind-cq"),
    Kind::Grid => t("radio.practice-kind-grid"),
    Kind::Report => t("radio.practice-kind-report"),
    Kind::RReport => t("radio.practice-kind-rreport"),
    Kind::Rr73 => t("radio.practice-kind-rr73"),
    Kind::Rrr => t("radio.practice-kind-rrr"),
    Kind::SeventyThree => t("radio.practice-kind-73"),
  }
}

pub(super) fn mistake_text(m: &Mistake) -> String {
  match m {
    Mistake::Empty => t("radio.practice-err-empty"),
    Mistake::NotStandard => t("radio.practice-err-not-standard"),
    Mistake::Swapped => t("radio.practice-err-swapped"),
    Mistake::WrongAddressee => t("radio.practice-err-addressee"),
    Mistake::WrongSender => t("radio.practice-err-sender"),
    Mistake::BadReport => t("radio.practice-err-bad-report"),
    Mistake::WrongStep { sent, expected } => tf(
      "radio.practice-err-wrong-step",
      &[&kind_name(*expected), &kind_name(*sent)],
    ),
    Mistake::GridNotMine => t("radio.practice-err-grid-not-mine"),
    Mistake::Finished => t("radio.practice-err-finished"),
  }
}
