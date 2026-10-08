use leptos::prelude::*;

use crate::util::set_title;

use super::alert_card::AlertCard;
use super::iss_card::IssCard;
use super::solar_card::SolarCard;
use super::spots_card::SpotsCard;
use super::wanted_card::WantedCard;
use crate::i18n::t;

#[component]
pub fn DashboardPage() -> impl IntoView {
  set_title("shell.live-dashboard");
  view! {
    <div class="min-h-screen animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <header class="sticky top-0 z-20 border-b bg-background/90 backdrop-blur">
        <div class="mx-auto flex max-w-5xl flex-wrap items-center gap-3 px-4 py-3">
          <div class="mr-auto">
            <h1 class="text-base font-semibold leading-tight">{move || t("shell.live-dashboard")}</h1>
            <div class="text-xs text-muted-foreground">{move || t("learning.solar-activity-alerts-dx")}</div>
          </div>
        </div>
      </header>

      <div class="mx-auto max-w-5xl space-y-4 px-4 py-5">
        <div class="grid gap-4 sm:grid-cols-2">
          <SolarCard />
          <AlertCard />
        </div>
        <div class="grid gap-4 sm:grid-cols-2">
          <SpotsCard />
          <IssCard />
        </div>
        <div class="grid gap-4 sm:grid-cols-2">
          <WantedCard />
        </div>
        <p class="text-xs text-muted-foreground">
          {move || t("learning.tap-each-card-for")}
        </p>
      </div>
    </div>
  }
}
