//! 页面里的文字：七个必说项的名字。
//!
//! 词条字面量必须写在 `t()` 的直接实参位（`check-i18n` 只采集直接调用），所以是一张 `match`。

use ham_web_core::voice_qso::Item;

use crate::i18n::t;

pub(super) fn item_label(item: Item) -> String {
  match item {
    Item::Callsigns => t("radio.voice-item-callsigns"),
    Item::Report => t("radio.voice-item-report"),
    Item::Name => t("radio.voice-item-name"),
    Item::Qth => t("radio.voice-item-qth"),
    Item::Rig => t("radio.voice-item-rig"),
    Item::Antenna => t("radio.voice-item-antenna"),
    Item::SignOff => t("radio.voice-item-signoff"),
  }
}

/// 稳定的 id（给测试与 `data-*` 用，不随语言变）。
pub(super) const fn item_id(item: Item) -> &'static str {
  match item {
    Item::Callsigns => "callsigns",
    Item::Report => "report",
    Item::Name => "name",
    Item::Qth => "qth",
    Item::Rig => "rig",
    Item::Antenna => "antenna",
    Item::SignOff => "signoff",
  }
}
