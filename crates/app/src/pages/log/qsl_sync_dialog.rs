use ham_web_core::qsl_sync::{QslSyncResult, apply_qsl_report};
use leptos::prelude::*;
use leptos::task::spawn_local;
use wasm_bindgen::JsCast;

use crate::i18n::{t, tf};
use crate::ui::{
  Dialog, DialogDescription, DialogFooter, DialogHeader, DialogTitle, Size, Variant, button_class,
};

use super::use_log_store;

const TEXTAREA: &str = "h-32 w-full rounded-lg border bg-background px-3 py-2 text-sm font-mono tabular-nums outline-none focus-visible:ring-2 focus-visible:ring-ring/50";

/// 同步 QSL 确认：粘贴或上传 LoTW / eQSL 的 ADIF 确认报告，自动更新日志确认状态。
#[component]
pub fn QslSyncDialog(open: RwSignal<bool>) -> impl IntoView {
  let store = use_log_store();
  let logbook = store.logbook;
  let text = RwSignal::new(String::new());
  let result = RwSignal::new(None::<QslSyncResult>);

  let apply = move |_| {
    let adif = text.get_untracked();
    if adif.trim().is_empty() {
      crate::util::alert(&t("请先粘贴或上传确认报告（ADIF）。"));
      return;
    }
    let mut r = QslSyncResult::default();
    logbook.update(|lb| {
      r = apply_qsl_report(&mut lb.entries, &adif);
    });
    store.persist();
    result.set(Some(r));
  };

  let upload = move |e: web_sys::Event| {
    let Some(target) = e.target() else { return };
    let input: web_sys::HtmlInputElement = target.unchecked_into();
    let Some(file) = input.files().and_then(|f| f.get(0)) else {
      return;
    };
    input.set_value("");
    spawn_local(async move {
      if let Some(s) = crate::util::read_file_text(&file).await {
        text.set(s);
      }
    });
  };

  view! {
    <Dialog open=open>
      <DialogHeader>
        <DialogTitle>{move || t("同步 QSL 确认")}</DialogTitle>
        <DialogDescription>
          {move || t("粘贴 LoTW / eQSL 下载的确认报告（ADIF），自动把匹配到的通联标记为已确认。")}
        </DialogDescription>
      </DialogHeader>
      <div class="space-y-3">
        <textarea
          class=TEXTAREA
          prop:value=move || text.get()
          on:input=move |e| text.set(event_target_value(&e))
          placeholder=move || t("在此粘贴 ADIF 报告…")
        ></textarea>
        <div class="flex flex-wrap items-center gap-2">
          <label class=button_class(Variant::Outline, Size::Sm, "cursor-pointer")>
            {move || t("上传报告文件")}
            <input type="file" accept=".adi,.adif,.txt" class="hidden" on:change=upload />
          </label>
          {move || {
            result.get().map(|r| {
              view! {
                <span class="text-xs text-muted-foreground">
                  {tf(
                    "报告 {} 条 · 匹配 {} 条 · 未匹配 {} 条",
                    &[&r.total.to_string(), &r.matched.to_string(), &r.unmatched.to_string()],
                  )}
                </span>
              }
            })
          }}
        </div>
      </div>
      <DialogFooter>
        <button type="button" class=button_class(Variant::Default, Size::Default, "") on:click=apply>
          {move || t("应用确认")}
        </button>
      </DialogFooter>
    </Dialog>
  }
}
