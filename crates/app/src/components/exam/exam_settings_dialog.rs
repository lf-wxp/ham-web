use leptos::prelude::*;

use super::shortcut_row::ShortcutRow;
use crate::i18n::t;
use crate::ui::{Checkbox, Dialog, DialogDescription, DialogHeader, DialogTitle, Label, Separator};

#[component]
pub fn ExamSettingsDialog(
  open: RwSignal<bool>,
  #[prop(into)] show_explanation: Signal<bool>,
  on_change_show_explanation: Callback<bool>,
  #[prop(into)] weighted: Signal<bool>,
  on_change_weighted: Callback<bool>,
  #[prop(into)] strict: Signal<bool>,
  on_change_strict: Callback<bool>,
) -> impl IntoView {
  view! {
    <Dialog open=open class="sm:max-w-[520px]">
      <DialogHeader>
        <DialogTitle>{move || t("设置")}</DialogTitle>
        <DialogDescription>{move || t("抽题方式、快捷键与考试说明")}</DialogDescription>
      </DialogHeader>
      <div class="space-y-5">
        <div class="flex items-center gap-2">
          <Checkbox id="exam-show-expl" checked=show_explanation on_change=on_change_show_explanation />
          <Label r#for="exam-show-expl">{move || t("显示答案解析（仅交卷后）")}</Label>
        </div>
        <div class="flex items-start gap-2">
          <Checkbox id="exam-weighted" checked=weighted on_change=on_change_weighted />
          <Label r#for="exam-weighted">
            {move || t("按分类占比抽题（贴近真实大纲分布，减少随机波动）")}
          </Label>
        </div>
        <div class="flex items-start gap-2">
          <Checkbox id="exam-strict" checked=strict on_change=on_change_strict />
          <Label r#for="exam-strict">
            {move || t("机考仿真（进入全屏，不可回看上一题）")}
          </Label>
        </div>
        <div class="space-y-2 text-sm">
          <div class="text-muted-foreground">{move || t("快捷键")}</div>
          <ShortcutRow label=t("上一题 / 下一题") keys="← / →" />
          <ShortcutRow label=t("选择 / 切换选项（单选/多选）") keys="1-9" />
          <ShortcutRow label=t("严格选择（多选，仅该项）") keys="Shift 或 Cmd（macOS） + 1-9" />
        </div>
        <Separator />
        <div class="text-xs text-muted-foreground">
          {move || {
            t("考试规则：A 类 40 题（单选 32，多选 8），40 分钟，30 题合格；B 类 60 题（单选 45，多选 15），60 分钟，45 题合格；C 类 90 题（单选 70，多选 20），90 分钟，70 题合格。多选题需与标准答案完全一致，否则不得分。")
          }}
        </div>
      </div>
    </Dialog>
  }
}
