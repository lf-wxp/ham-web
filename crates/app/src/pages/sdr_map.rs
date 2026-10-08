//! SDR 在线接收站地图：地图标注公开接收站，并提供接收机目录与使用提示。

use ham_web_core::sdr_map::{SDR_DIRECTORIES, SDR_MAP_TIPS, SDR_SITES};
use leptos::prelude::*;

use crate::components::common::{BulletSection, KnowledgePage};
use crate::data::{self, kt};
use crate::i18n::t;
use crate::icons::{Icon, IconKind};
use crate::pages::map::{MapView, project};
use crate::ui::{Button, ButtonLink, Size, Variant};
use crate::util::set_title;

#[component]
pub fn SdrMapPage() -> impl IntoView {
  set_title("tools.online-sdr-receiver-map");

  // 地图定位：点列表里的「定位」把视图居中到该接收站。
  let focus = RwSignal::new(None::<(f64, f64)>);

  view! {
    <KnowledgePage
      title=t("tools.online-sdr-receiver-map")
      subtitle=t("tools.no-local-hardware-needed")
    >
      <section class="rounded-xl border bg-card p-4">
        <MapView focus=Signal::derive(move || focus.get())>
          {SDR_SITES
            .iter()
            .map(|s| {
              let (x, y) = project(s.lon, s.lat);
              view! {
                <circle
                  cx=x.to_string()
                  cy=y.to_string()
                  r="3"
                  class="fill-emerald-500 stroke-emerald-500/60"
                  stroke-width="1"
                  vector-effect="non-scaling-stroke"
                >
                  <title>{s.name}</title>
                </circle>
              }
            })
            .collect_view()}
        </MapView>
        <p class="mt-2 text-xs text-muted-foreground">
          {move || {
            t("tools.green-dots-are-long")
          }}
        </p>
      </section>

      <section class="rounded-xl border bg-card">
        <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("tools.public-receivers")}</h2>
        <div class="divide-y">
          {SDR_SITES
            .iter()
            .map(|s| {
              view! {
                <div class="flex flex-col gap-2 px-4 py-3 sm:flex-row sm:items-center">
                  <div class="min-w-0 flex-1">
                    <div class="flex flex-wrap items-baseline gap-x-2">
                      <span class="text-sm font-medium">
                        {move || {
                          data::track_knowledge();
                          kt(s.name)
                        }}
                      </span>
                      <span class="rounded-full border bg-muted/50 px-2 py-0.5 text-[10px] text-muted-foreground">
                        {move || {
                          data::track_knowledge();
                          kt(s.kind)
                        }}
                      </span>
                    </div>
                    <div class="mt-0.5 text-xs text-muted-foreground">
                      {move || {
                        data::track_knowledge();
                        kt(s.location)
                      }}
                      " · "
                      {s.coverage}
                    </div>
                    <div class="mt-1 text-sm text-muted-foreground">
                      {move || {
                        data::track_knowledge();
                        kt(s.desc)
                      }}
                    </div>
                  </div>
                  <div class="flex items-center gap-2 sm:justify-end">
                    <Button
                      variant=Variant::Outline
                      size=Size::Sm
                      on_click=Callback::new(move |_| focus.set(Some((s.lat, s.lon))))
                    >
                      {move || t("log.locate")}
                    </Button>
                    <ButtonLink
                      href=s.url
                      variant=Variant::Default
                      size=Size::Sm
                      target="_blank"
                      rel="noopener noreferrer"
                    >
                      <Icon kind=IconKind::ExternalLink class="size-3.5" />
                      {move || t("tools.open-receiver")}
                    </ButtonLink>
                  </div>
                </div>
              }
            })
            .collect_view()}
        </div>
      </section>

      <section class="rounded-xl border bg-card">
        <h2 class="border-b px-4 py-3 text-sm font-semibold">
          {move || t("tools.receiver-directory-and-self")}
        </h2>
        <div class="divide-y">
          {SDR_DIRECTORIES
            .iter()
            .map(|d| {
              view! {
                <a
                  href=d.url
                  target="_blank"
                  rel="noopener noreferrer"
                  class="flex flex-col gap-0.5 px-4 py-3 transition-colors hover:bg-muted/40 sm:flex-row sm:items-baseline sm:gap-3"
                >
                  <span class="shrink-0 text-sm font-medium text-primary">
                    {move || {
                      data::track_knowledge();
                      kt(d.name)
                    }}
                  </span>
                  <span class="min-w-0 text-sm text-muted-foreground">
                    {move || {
                      data::track_knowledge();
                      kt(d.desc)
                    }}
                  </span>
                </a>
              }
            })
            .collect_view()}
        </div>
      </section>

      <BulletSection title="使用提示" items=SDR_MAP_TIPS />
    </KnowledgePage>
  }
}
