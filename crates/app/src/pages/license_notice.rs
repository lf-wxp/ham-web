use crate::components::common::{PageContainer, PageHeader};
use crate::i18n::t;
use crate::ui::{ButtonLink, Variant};
use crate::util::set_title;
use leptos::prelude::*;

#[component]
pub fn LicenseNoticePage() -> impl IntoView {
  set_title("shell.software-license-title");
  view! {
    <PageHeader
      title=move || t("shell.software-license-title")
      subtitle=move || t("shell.software-license-summary")
    />
    <PageContainer>
      <section class="pxl-window bg-card text-card-foreground">
        <h2 class="border-b-2 border-ink px-4 py-3 text-sm text-primary">{move || t("shell.software-license-components")}</h2>
        <div class="space-y-4 p-4 text-sm leading-7">
          <p>{move || t("shell.software-license-mit")}</p>
          <p class="font-mono">"Copyright (c) 2026 lf-wxp"</p>
          <p>{move || t("shell.software-license-gpl")}</p>
          <div class="flex flex-wrap gap-3">
            <a href="/licenses/LICENSE" rel="external" class="text-primary underline underline-offset-4">{move || t("shell.software-license-mit-text")}</a>
            <a href="/licenses/GPL-3.0-only.txt" rel="external" class="text-primary underline underline-offset-4">{move || t("shell.software-license-gpl-text")}</a>
            <a href="/licenses/MIT-lrpt-encoder.txt" rel="external" class="text-primary underline underline-offset-4">{move || t("shell.software-license-mit-lrpt-encoder")}</a>
            <a href="/licenses/MIT-olivia-python.txt" rel="external" class="text-primary underline underline-offset-4">{move || t("shell.software-license-mit-olivia-python")}</a>
          </div>
        </div>
      </section>
      <section class="pxl-window bg-card text-card-foreground">
        <h2 class="border-b-2 border-ink px-4 py-3 text-sm text-primary">{move || t("shell.software-license-source")}</h2>
        <div class="space-y-4 p-4 text-sm leading-7">
          <p>{move || t("shell.software-license-source-details")}</p>
          <p>{move || t("shell.software-license-source-version")}</p>
          <ButtonLink href="https://github.com/lf-wxp/ham-web" variant=Variant::Outline>
            {move || t("shell.github-repository")}
          </ButtonLink>
        </div>
      </section>
      <section class="pxl-window bg-card text-card-foreground">
        <h2 class="border-b-2 border-ink px-4 py-3 text-sm text-primary">{move || t("shell.software-license-provenance")}</h2>
        <ul class="space-y-4 p-4 text-sm leading-7">
          <li>
            <a href="https://github.com/w1hkj/fldigi" class="text-primary underline underline-offset-4">"DominoEX · fldigi"</a>
            <p>{move || t("shell.software-license-dominoex")}</p>
            <p class="font-mono">"crates/dominoex/reference/README.md"</p>
          </li>
          <li>
            <a href="https://github.com/artlav/meteor_decoder" class="text-primary underline underline-offset-4">"LRPT · artlav/meteor_decoder"</a>
            <p>{move || t("shell.software-license-lrpt")}</p>
            <p class="font-mono">"crates/lrpt/reference/README.md"</p>
          </li>
          <li>
            <span class="text-primary">"NAVTEX · AI5GW/CCIR476"</span>
            <p>{move || t("shell.software-license-navtex")}</p>
            <p class="font-mono">"crates/navtex/src/codec.rs"</p>
          </li>
          <li>{move || t("shell.software-license-third-party")}</li>
        </ul>
      </section>
      <ButtonLink href="/" variant=Variant::Outline>{move || t("shell.home")}</ButtonLink>
    </PageContainer>
  }
}
