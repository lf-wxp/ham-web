use leptos::prelude::*;

use crate::util::set_title;

use super::alert_card::AlertCard;
use super::iss_card::IssCard;
use super::solar_card::SolarCard;
use super::spots_card::SpotsCard;
use super::wanted_card::WantedCard;

#[component]
pub fn DashboardPage() -> impl IntoView {
  set_title("实时仪表盘");
  view! {
    <div class="min-h-screen animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <header class="sticky top-0 z-20 border-b bg-background/90 backdrop-blur">
        <div class="mx-auto flex max-w-5xl flex-wrap items-center gap-3 px-4 py-3">
          <div class="mr-auto">
            <div class="text-base font-semibold leading-tight">"实时仪表盘"</div>
            <div class="text-xs text-muted-foreground">"太阳活动 · 警报 · DX 热点 · ISS · 稀有度"</div>
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
          "各卡片点击可进入对应详情页。数据分别来自 HamQSL / NOAA SWPC、DXWatch、wheretheiss.at 与 Club Log。"
        </p>
      </div>
    </div>
  }
}
