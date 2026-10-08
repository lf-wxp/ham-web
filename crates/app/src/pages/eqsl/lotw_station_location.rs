use ham_web_core::eqsl::lotw_station_fields;
use leptos::prelude::*;

use crate::i18n::t;
use crate::pages::log::use_log_store;
use crate::ui::{Button, Size, Variant};
use crate::util::copy_text;

use super::row::row;

/// TQSL 里 Station Location 要填的值：按当前台站档案 + DXCC 表算出来，照着抄即可。
///
/// 这是「不代填」这条边界内能给到的最实在的帮助：我们写不进用户本机的 TQSL 配置，
/// 但可以保证「该填什么」是算准的 —— 而这里填错**不会当场报错**，只会在 LoTW 端被判成
/// 「不匹配」，所以值得单独摆一张对照表，而不是让人去翻分区表。
#[component]
pub(super) fn LotwStationLocation() -> impl IntoView {
  let store = use_log_store();
  let fields = Memo::new(move |_| lotw_station_fields(&store.active_station()));

  let copy = move |_| {
    let f = fields.get_untracked();
    let entity = match (f.dxcc_name, f.dxcc) {
      (Some(name), Some(code)) => format!("{name} ({code})"),
      _ => String::new(),
    };
    // 顺序照着 TQSL 对话框里的字段顺序排，省得来回对。
    let text = format!(
      "{}: {}\n{}: {}\n{}: {}\n{}: {}\n{}: {}\n",
      t("log.station-callsign"),
      f.callsign,
      t("log.dxcc-entities"),
      entity,
      t("contest.cq-zone"),
      f.cq_zone.map(|z| z.to_string()).unwrap_or_default(),
      t("log.tqsl-itu-zone"),
      f.itu_zone.map(|z| z.to_string()).unwrap_or_default(),
      t("log.station-grid"),
      f.gridsquare,
    );
    copy_text(&text);
    crate::util::alert(&t("learning.copied-to-clipboard"));
  };

  view! {
    <section class="rounded-xl border bg-card">
      <h2 class="border-b px-4 py-3 text-sm font-semibold">
        {move || t("log.tqsl-station-location")}
      </h2>
      <div class="p-4">
        {move || {
          let f = fields.get();
          if !f.is_complete() {
            return view! {
              <p class="text-xs text-muted-foreground">
                {move || t("log.tqsl-set-station-first")}
                " "
                <a href="/log" class="font-medium underline underline-offset-4">
                  {move || t("log.station-info")}
                </a>
              </p>
            }
              .into_any();
          }
          let zone = |z: Option<u8>| z.map(|z| z.to_string()).unwrap_or_default();
          // 「CQ 分区」这条文案与竞赛页共用（`contest.cq-zone`）：中文释义全库唯一，
          // 同一个词只能有一份。
          let rows = vec![
            (
              t("log.station-callsign"),
              f.callsign.clone(),
              t("log.station-info"),
            ),
            (
              t("log.dxcc-entities"),
              format!(
                "{} · {}",
                f.dxcc_name.unwrap_or_default(),
                f.dxcc.unwrap_or_default(),
              ),
              t("log.tqsl-from-prefix"),
            ),
            (
              t("contest.cq-zone"),
              zone(f.cq_zone),
              t("log.tqsl-from-entity"),
            ),
            (
              t("log.tqsl-itu-zone"),
              zone(f.itu_zone),
              t("log.tqsl-from-entity"),
            ),
            (
              t("log.station-grid"),
              f.gridsquare.clone(),
              t("log.station-info"),
            ),
          ];
          view! {
            <dl>{rows.into_iter().map(|(l, v, s)| row(l, v, s)).collect_view()}</dl>
            <p class="mt-3 text-xs text-muted-foreground">
              {move || t("log.tqsl-must-match-my")}
            </p>
            <div class="mt-3">
              <Button
                variant=Variant::Outline
                size=Size::Sm
                on_click=Callback::new(copy)
              >
                {move || t("log.tqsl-copy")}
              </Button>
            </div>
          }
            .into_any()
        }}
      </div>
    </section>
  }
}
