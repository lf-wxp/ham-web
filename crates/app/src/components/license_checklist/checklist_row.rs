//! 清单里的一行：勾选框 + 说明 + 条文链接 + 工具链接。

use std::collections::BTreeSet;

use ham_web_core::license_checklist::{Audience, Item, article_href};
use leptos::prelude::*;

use super::texts::{item_text, tool_label};
use crate::i18n::{t, tf};
use crate::ui::Checkbox;

#[component]
pub(super) fn ChecklistRow(
  item: &'static Item,
  checked: RwSignal<BTreeSet<String>>,
  persist: impl Fn() + Copy + Send + Sync + 'static,
) -> impl IntoView {
  let id = item.id;
  let is_on = Signal::derive(move || checked.with(|c| c.contains(id)));
  let audience = match item.audience {
    Audience::Everyone => None,
    Audience::Minor => Some(t("knowledge.checklist-minor")),
    Audience::Unit => Some(t("knowledge.checklist-unit")),
  };
  view! {
    <li class="space-y-1 py-2.5" data-checklist-item=id>
      <label class="flex items-start gap-2 text-sm">
        <Checkbox
          checked=is_on
          on_change=Callback::new(move |on: bool| {
            checked.update(|c| {
              if on {
                c.insert(id.to_owned());
              } else {
                c.remove(id);
              }
            });
            persist();
          })
        />
        <span class="min-w-0 break-words">
          {move || item_text(id)}
          {audience
            .clone()
            .map(|a| view! { <span class="ml-2 border px-1.5 py-0.5 text-xs text-muted-foreground">{a}</span> })}
        </span>
      </label>
      <p class="flex flex-wrap items-baseline gap-x-2 gap-y-1 pl-6 text-xs text-muted-foreground">
        <span>{move || t("knowledge.checklist-law")}</span>
        {item
          .cites
          .iter()
          .map(|c| c.article)
          .collect::<BTreeSet<_>>()
          .into_iter()
          .map(|n| {
            view! {
              <a href=article_href(n) class="text-primary underline underline-offset-2" data-law-article=n>
                {move || tf("knowledge.radio-law-article-n", &[&n.to_string()])}
              </a>
            }
          })
          .collect_view()}
        {item
          .tool
          .map(|tool| {
            view! {
              <span class="ml-2">{move || t("knowledge.checklist-tool")}</span>
              <a href=tool.href() class="text-primary underline underline-offset-2" data-checklist-tool=tool.href()>
                {move || tool_label(tool)}
              </a>
            }
          })}
      </p>
    </li>
  }
}
