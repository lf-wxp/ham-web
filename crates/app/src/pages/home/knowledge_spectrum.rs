//! 首页「知识频谱」：10 个一级分类各一根柱，高度是该分类正确率折成的 S 表格数（S9 = 合格线）。
//!
//! 它替换了原来那条纯装饰的频谱条：电台元素必须承载功能（`docs/ui/pixel.md` §2 硬规则 16）。
//! 每根柱都是去该分类专项练习的链接；悬停 / 聚焦时屏幕顶部读出分类名与 S 值。

use ham_web_core::Bank;
use ham_web_core::categories::TOP_CATEGORIES;
use ham_web_core::rig_panel::{main_bank_pass, s_reading, s_units};
use leptos::prelude::*;

use crate::components::common::SectionCard;
use crate::data::core_text_tracked;
use crate::i18n::{bank_class, t};
use crate::ui::{Readout, ReadoutTone};

use super::spectrum_bar::SpectrumBar;

/// 一个分类至少作答这么多次才给读数（与首页「薄弱分类」同一门槛），少于它画成全灭的柱。
pub(super) const MIN_ANSWERED: u32 = 5;

struct Bar {
  key: &'static str,
  name: &'static str,
  units: Option<u8>,
}

#[component]
pub(super) fn KnowledgeSpectrum() -> impl IntoView {
  let stats = crate::study::stats_shared();
  let (bank, pass) = main_bank_pass(stats.as_deref());
  let categories = stats.as_deref().and_then(|s| s.categories_of(Some(bank)));
  let bars: Vec<Bar> = TOP_CATEGORIES
    .iter()
    .map(|c| {
      let tally = categories.and_then(|s| s.categories.get(c.key)).copied();
      let units = tally
        .filter(|t| t.answered >= MIN_ANSWERED)
        .and_then(|t| t.rate())
        .map(|r| s_units(r, pass));
      Bar {
        key: c.key,
        name: c.name,
        units,
      }
    })
    .collect();

  // 默认读出最弱的一类：屏幕第一眼给的是「下一步练什么」，而不是第一个分类。
  let weakest = bars
    .iter()
    .enumerate()
    .filter_map(|(i, b)| b.units.map(|u| (i, u)))
    .min_by_key(|(_, u)| *u)
    .map_or(0, |(i, _)| i);
  let active = RwSignal::new(weakest);
  let readings: Vec<(&'static str, Option<u8>)> = bars.iter().map(|b| (b.name, b.units)).collect();
  let readings = StoredValue::new(readings);
  let current = move || readings.with_value(|r| r[active.get()]);

  view! {
    <SectionCard
      title=Signal::derive(move || t("home.knowledge-spectrum"))
      meta=ViewFn::from(move || bank_name(bank))
    >
      <div class="space-y-3 p-4">
        <div class="pxl-display space-y-3 p-3">
          <div class="flex flex-wrap items-center justify-between gap-2" aria-hidden="true">
            <span class="text-xs">{move || core_text_tracked(current().0)}</span>
            {move || {
              let units = current().1;
              view! {
                <Readout
                  text=units.map_or_else(|| "--".to_owned(), s_reading)
                  label=units.map_or_else(|| t("common.no-signal"), s_reading)
                  tone=ReadoutTone::for_s_units(units)
                />
              }
            }}
          </div>
          <ul class="pxl-spectrum">
            {bars
              .into_iter()
              .enumerate()
              .map(|(i, b)| {
                view! {
                  <SpectrumBar index=i topic=b.key name=b.name units=b.units bank=bank active=active />
                }
              })
              .collect_view()}
          </ul>
        </div>
        <p class="text-xs text-muted-foreground">{move || t("home.knowledge-spectrum-hint")}</p>
      </div>
    </SectionCard>
  }
}

fn bank_name(bank: Bank) -> String {
  bank_class(bank.as_str())
}
