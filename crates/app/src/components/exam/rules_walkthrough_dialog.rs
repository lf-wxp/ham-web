//! 考场规则演练：机考仿真开考前的规则对照 + 考前准备一条龙。
//!
//! 只做「对照」，不做锁机 —— 本站与真实考场的差异**如实列出来**（比如允许续考），
//! 演练的意义是让用户知道真实考场的约束长什么样，而不是假装我们锁得住浏览器。
//!
//! 关闭但没点「开始」（Esc / 遮罩 / ×）由调用方处理：挂起的组卷作废并回练习页。

use leptos::prelude::*;

use crate::i18n::t;
use crate::ui::{
  Button, ButtonLink, Dialog, DialogDescription, DialogFooter, DialogHeader, DialogTitle, Variant,
};

#[component]
pub fn RulesWalkthroughDialog(
  open: RwSignal<bool>,
  /// 确认后真正开考（由调用方挂起的一次组卷）。
  on_confirm: Callback<()>,
) -> impl IntoView {
  view! {
    <Dialog open=open class="sm:max-w-[560px]">
      <DialogHeader>
        <DialogTitle>{move || t("exam.rules-title")}</DialogTitle>
        <DialogDescription>{move || t("exam.rules-hint")}</DialogDescription>
      </DialogHeader>

      <div class="space-y-4">
        // 规则对照：真实考场 vs 本站仿真（词条字面量写在调用点，检查器才认得出）。
        <div class="overflow-hidden rounded-lg border">
          <div class="grid grid-cols-2 border-b bg-muted/60 text-xs font-medium">
            <div class="px-3 py-2">{move || t("exam.rules-real-col")}</div>
            <div class="px-3 py-2">{move || t("exam.rules-sim-col")}</div>
          </div>
          <div class="grid grid-cols-2 border-b text-xs">
            <div class="px-3 py-2 text-muted-foreground">{move || t("exam.rules-real-time")}</div>
            <div class="px-3 py-2">{move || t("exam.rules-sim-time")}</div>
          </div>
          <div class="grid grid-cols-2 border-b text-xs">
            <div class="px-3 py-2 text-muted-foreground">{move || t("exam.rules-real-leave")}</div>
            <div class="px-3 py-2">{move || t("exam.rules-sim-leave")}</div>
          </div>
          <div class="grid grid-cols-2 text-xs">
            <div class="px-3 py-2 text-muted-foreground">{move || t("exam.rules-real-submit")}</div>
            <div class="px-3 py-2">{move || t("exam.rules-sim-submit")}</div>
          </div>
        </div>

        // 考前准备：证件照与练习的一条龙衔接。
        <div>
          <div class="text-xs font-medium text-muted-foreground">
            {move || t("exam.rules-prep")}
          </div>
          <ul class="mt-1 space-y-1 text-sm">
            <li>
              <a
                href="/photo-processor"
                class="text-primary underline-offset-2 hover:underline"
              >
                {move || t("exam.rules-prep-photo")}
              </a>
            </li>
            <li>
              <a href="/practice" class="text-primary underline-offset-2 hover:underline">
                {move || t("exam.rules-prep-practice")}
              </a>
            </li>
          </ul>
        </div>
      </div>

      <DialogFooter>
        <ButtonLink href="/practice" variant=Variant::Outline>
          {move || t("exam.rules-cancel")}
        </ButtonLink>
        <Button on_click=Callback::new(move |()| {
          // 先交给调用方取走挂起的组卷，再关弹层：调用方据「关闭时还有挂起组卷」判断是不是被中途关掉的。
          on_confirm.run(());
          open.set(false);
        })>{move || t("exam.rules-start")}</Button>
      </DialogFooter>
    </Dialog>
  }
}
