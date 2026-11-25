//! 清单里的文字：分组标题、各项说明、工具名。
//!
//! 词条字面量必须写在 `t()` 的直接实参位（`check-i18n` 只采集直接调用），所以各项说明是一张 `match`，
//! 而不是「按 id 拼词条名」。

use ham_web_core::license_checklist::{Group, Tool};

use crate::i18n::t;

pub(super) fn group_title(group: Group) -> String {
  match group {
    Group::Conditions => t("knowledge.checklist-group-conditions"),
    Group::Authority => t("knowledge.checklist-group-authority"),
    Group::Materials => t("knowledge.checklist-group-materials"),
    Group::Parameters => t("knowledge.checklist-group-parameters"),
    Group::Review => t("knowledge.checklist-group-review"),
    Group::After => t("knowledge.checklist-group-after"),
  }
}

/// 每一项的说明。词条字面量必须写在 `t()` 的直接实参位，`check-i18n` 只采集直接调用。
pub(super) fn item_text(id: &str) -> String {
  match id {
    "know-rules" => t("knowledge.checklist-know-rules"),
    "passed-verification" => t("knowledge.checklist-passed-verification"),
    "device-qualifies" => t("knowledge.checklist-device-qualifies"),
    "province-authority" => t("knowledge.checklist-province-authority"),
    "national-authority" => t("knowledge.checklist-national-authority"),
    "form" => t("knowledge.checklist-form"),
    "identity" => t("knowledge.checklist-identity"),
    "device-photo" => t("knowledge.checklist-device-photo"),
    "device-statement" => t("knowledge.checklist-device-statement"),
    "guardian" => t("knowledge.checklist-guardian"),
    "unit-documents" => t("knowledge.checklist-unit-documents"),
    "class-limits" => t("knowledge.checklist-class-limits"),
    "minor-limits" => t("knowledge.checklist-minor-limits"),
    "frequency-coordination" => t("knowledge.checklist-frequency-coordination"),
    "licence-fields" => t("knowledge.checklist-licence-fields"),
    "radiation" => t("knowledge.checklist-radiation"),
    "supplements" => t("knowledge.checklist-supplements"),
    "decision-time" => t("knowledge.checklist-decision-time"),
    "technical-test" => t("knowledge.checklist-technical-test"),
    "callsign-issued" => t("knowledge.checklist-callsign-issued"),
    "renewal" => t("knowledge.checklist-renewal"),
    "changes" => t("knowledge.checklist-changes"),
    "closing" => t("knowledge.checklist-closing"),
    // 核心里新增了清单项而这里漏了词条：显示 id 本身（e2e 逐项核对不会出现这种裸 id）。
    other => other.to_owned(),
  }
}

pub(super) fn tool_label(tool: Tool) -> String {
  match tool {
    Tool::Compliance => t("shell.frequency-power-compliance"),
    Tool::LicenseClasses => t("shell.license-privileges"),
    Tool::Regulations => t("shell.regulations"),
    Tool::RfExposure => t("tools.rf-exposure-evaluation"),
    Tool::ExpiryReminder => t("settings.license-expiry-reminder"),
  }
}
