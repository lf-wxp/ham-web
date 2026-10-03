use leptos::prelude::*;

use ham_web_core::categories::TOP_CATEGORIES;
use ham_web_core::exam::CustomPaper;

use crate::i18n::t;
use crate::ui::{
  Button, Checkbox, Dialog, DialogFooter, DialogHeader, DialogTitle, Field, Label, NumberField,
  Variant,
};

/// 自定义组卷对话框：自选题量（单选 / 多选）、限时与一级分类范围。
#[component]
pub fn CustomPaperDialog(open: RwSignal<bool>, on_confirm: Callback<CustomPaper>) -> impl IntoView {
  let singles = RwSignal::new(32u32);
  let multiples = RwSignal::new(8u32);
  let minutes = RwSignal::new(40u32);
  let cats: RwSignal<Vec<&'static str>> = RwSignal::new(Vec::new());

  let confirm = move || {
    open.set(false);
    on_confirm.run(CustomPaper {
      singles: singles.get() as usize,
      multiples: multiples.get() as usize,
      categories: cats.get(),
      minutes: minutes.get(),
    });
  };

  view! {
    <Dialog open=open class="sm:max-w-[560px]">
      <DialogHeader>
        <DialogTitle>{move || t("自定义组卷")}</DialogTitle>
      </DialogHeader>
      <div class="space-y-5">
        <div class="grid grid-cols-3 gap-3">
          <Field label=t("单选题数") r#for="custom-single">
            <NumberField
              id="custom-single"
              value=Signal::derive(move || singles.get().to_string())
              on_change=Callback::new(move |v: String| singles.set(v.parse().unwrap_or(0)))
              min=0.0
              max=300.0
            />
          </Field>
          <Field label=t("多选题数") r#for="custom-multi">
            <NumberField
              id="custom-multi"
              value=Signal::derive(move || multiples.get().to_string())
              on_change=Callback::new(move |v: String| multiples.set(v.parse().unwrap_or(0)))
              min=0.0
              max=100.0
            />
          </Field>
          <Field label=t("限时（分钟，0 为不限时）") r#for="custom-minutes">
            <NumberField
              id="custom-minutes"
              value=Signal::derive(move || minutes.get().to_string())
              on_change=Callback::new(move |v: String| minutes.set(v.parse().unwrap_or(0)))
              min=0.0
              max=600.0
            />
          </Field>
        </div>
        <div class="space-y-2">
          <div class="text-sm text-muted-foreground">{t("限定分类（不选 = 全部）")}</div>
          <div class="grid grid-cols-2 gap-x-4 gap-y-1.5 sm:grid-cols-3">
            {TOP_CATEGORIES.iter().map(|c| {
              let key = c.key;
              let name = c.name;
              let checked = Signal::derive(move || cats.get().contains(&key));
              let on_change = Callback::new(move |v: bool| {
                cats.update(|list| {
                  let pos = list.iter().position(|&k| k == key);
                  match (v, pos) {
                    (true, None) => list.push(key),
                    (false, Some(p)) => {
                      list.remove(p);
                    }
                    _ => {}
                  }
                });
              });
              view! {
                <div class="flex items-center gap-2">
                  <Checkbox id=format!("custom-cat-{key}") checked=checked on_change=on_change />
                  <Label r#for=format!("custom-cat-{key}")>{name}</Label>
                </div>
              }
            }).collect_view()}
          </div>
        </div>
      </div>
      <DialogFooter>
        <Button variant=Variant::Ghost on_click=Callback::new(move |_| open.set(false))>
          {move || t("取消")}
        </Button>
        <Button
          variant=Variant::Default
          disabled=Signal::derive(move || singles.get() + multiples.get() == 0)
          on_click=Callback::new(move |_| confirm())
        >
          {move || t("开始考试")}
        </Button>
      </DialogFooter>
    </Dialog>
  }
}
