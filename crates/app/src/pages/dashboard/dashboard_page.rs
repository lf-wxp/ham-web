use leptos::prelude::*;

use crate::components::common::{PageContainer, PageHeader};
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
      <PageHeader
        title=move || t("shell.live-dashboard")
        subtitle=move || t("learning.solar-activity-alerts-dx")
      />

      <PageContainer class="space-y-4">
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
      </PageContainer>
    </div>
  }
}
