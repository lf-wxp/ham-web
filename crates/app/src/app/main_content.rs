use leptos::prelude::*;
use leptos_router::components::{Route, Routes};
use leptos_router::hooks::use_location;
use leptos_router::path;

use crate::pages::{
  AmplifierPage, AnalogModesPage, AntennaAnalyzerPage, AntennaArrayPage, AntennaDiyPage,
  AntennaFarmPage, AntennaInstallationPage, AntennaModelingPage, AntennaTuningPage, AntennasPage,
  AprsPage, ArdfPage, AtvPage, AwardsPage, BandPlanPage, BandsPage, BeginnerPage, BookmarksPage,
  BrowsePage, CabrilloPage, ContestPage, CountdownPage, CwOpPage, DashboardPage, DvNetworkPage,
  DxPage, DxSpotsPage, DxpeditionPage, ElectronicsPage, EmcommPage, EmePage, EqslPage, ExamPage,
  FeedlinePage, FiltersPage, FlashcardsPage, FrequenciesPage, Ft8Page, GlossaryPage, GnuradioPage,
  GraylinePage, GridMapPage, GridSystemPage, GroundingPage, HistoryPage, HomePage, IotaPage,
  LicenseClassesPage, LicensePage, LogPage, LoggingSoftwarePage, MetersPage, MicrowavePage,
  MistakesPage, MobilePage, ModesPage, MorsePage, MostWantedPage, MufPage, NotFoundPage, NvisPage,
  OperatingPage, OrganizationsPage, PacketPage, PhoneticPage, PhotoProcessorPage, PolarizationPage,
  PortablePage, PowerPage, PowerSupplyPage, PracticePage, PrefixesPage, ProgressPage,
  PropagationPage, QCodePage, QrpPage, QslCardPage, ReceiverPage, ReferencePage, RegulationsPage,
  RemotePage, RepeaterBuildPage, RepeaterPage, RfiPage, RstPage, RttyPage, SafetyPage,
  SatellitesPage, SdrPage, SolarPage, SpecialPropPage, SstvPage, StatsPage, SwlPage, ToolsPage,
  TransceiverPage, WeatherSatPage, WsprPage,
};

use crate::components::related_topics::RelatedTopics;

/// 主体内容。
#[component]
pub(super) fn MainContent() -> impl IntoView {
  let location = use_location();
  Effect::new(move |_| {
    let _ = location.pathname.get();
    // 路由切换后立即复位滚动（覆盖 html 的 smooth 行为，避免导航菜单与内容不同步）。
    if let Some(el) = web_sys::window()
      .and_then(|w| w.document())
      .and_then(|d| d.document_element())
    {
      el.set_scroll_top(0);
    }
  });

  view! {
    <main class="flex-1">
      <Routes fallback=|| view! { <NotFoundPage /> }>
        <Route path=path!("/") view=HomePage />
        <Route path=path!("/practice") view=PracticePage />
        <Route path=path!("/exam") view=ExamPage />
        <Route path=path!("/browse") view=BrowsePage />
        <Route path=path!("/glossary") view=GlossaryPage />
        <Route path=path!("/q-code") view=QCodePage />
        <Route path=path!("/morse") view=MorsePage />
        <Route path=path!("/phonetic") view=PhoneticPage />
        <Route path=path!("/bands") view=BandsPage />
        <Route path=path!("/reference") view=ReferencePage />
        <Route path=path!("/antennas") view=AntennasPage />
        <Route path=path!("/bandplan") view=BandPlanPage />
        <Route path=path!("/prefixes") view=PrefixesPage />
        <Route path=path!("/modes") view=ModesPage />
        <Route path=path!("/analog-modes") view=AnalogModesPage />
        <Route path=path!("/frequencies") view=FrequenciesPage />
        <Route path=path!("/satellites") view=SatellitesPage />
        <Route path=path!("/operating") view=OperatingPage />
        <Route path=path!("/rst") view=RstPage />
        <Route path=path!("/propagation") view=PropagationPage />
        <Route path=path!("/log") view=LogPage />
        <Route path=path!("/countdown") view=CountdownPage />
        <Route path=path!("/mistakes") view=MistakesPage />
        <Route path=path!("/bookmarks") view=BookmarksPage />
        <Route path=path!("/flashcards") view=FlashcardsPage />
        <Route path=path!("/contest") view=ContestPage />
        <Route path=path!("/solar") view=SolarPage />
        <Route path=path!("/safety") view=SafetyPage />
        <Route path=path!("/license") view=LicensePage />
        <Route path=path!("/electronics") view=ElectronicsPage />
        <Route path=path!("/feedline") view=FeedlinePage />
        <Route path=path!("/meters") view=MetersPage />
        <Route path=path!("/power") view=PowerPage />
        <Route path=path!("/awards") view=AwardsPage />
        <Route path=path!("/aprs") view=AprsPage />
        <Route path=path!("/sdr") view=SdrPage />
        <Route path=path!("/emcomm") view=EmcommPage />
        <Route path=path!("/beginner") view=BeginnerPage />
        <Route path=path!("/organizations") view=OrganizationsPage />
        <Route path=path!("/ardf") view=ArdfPage />
        <Route path=path!("/special-prop") view=SpecialPropPage />
        <Route path=path!("/antenna-diy") view=AntennaDiyPage />
        <Route path=path!("/transceiver") view=TransceiverPage />
        <Route path=path!("/dx") view=DxPage />
        <Route path=path!("/eqsl") view=EqslPage />
        <Route path=path!("/grid") view=GridSystemPage />
        <Route path=path!("/history") view=HistoryPage />
        <Route path=path!("/muf") view=MufPage />
        <Route path=path!("/portable") view=PortablePage />
        <Route path=path!("/cw-operating") view=CwOpPage />
        <Route path=path!("/antenna-installation") view=AntennaInstallationPage />
        <Route path=path!("/ft8") view=Ft8Page />
        <Route path=path!("/repeater") view=RepeaterPage />
        <Route path=path!("/wspr") view=WsprPage />
        <Route path=path!("/logging-software") view=LoggingSoftwarePage />
        <Route path=path!("/microwave") view=MicrowavePage />
        <Route path=path!("/remote") view=RemotePage />
        <Route path=path!("/qsl-card") view=QslCardPage />
        <Route path=path!("/eme") view=EmePage />
        <Route path=path!("/antenna-tuning") view=AntennaTuningPage />
        <Route path=path!("/rfi") view=RfiPage />
        <Route path=path!("/qrp") view=QrpPage />
        <Route path=path!("/dxpedition") view=DxpeditionPage />
        <Route path=path!("/regulations") view=RegulationsPage />
        <Route path=path!("/antenna-farm") view=AntennaFarmPage />
        <Route path=path!("/nvis") view=NvisPage />
        <Route path=path!("/rtty") view=RttyPage />
        <Route path=path!("/iota") view=IotaPage />
        <Route path=path!("/gnuradio") view=GnuradioPage />
        <Route path=path!("/swl") view=SwlPage />
        <Route path=path!("/amplifier") view=AmplifierPage />
        <Route path=path!("/atv") view=AtvPage />
        <Route path=path!("/filters") view=FiltersPage />
        <Route path=path!("/antenna-modeling") view=AntennaModelingPage />
        <Route path=path!("/most-wanted") view=MostWantedPage />
        <Route path=path!("/repeater-build") view=RepeaterBuildPage />
        <Route path=path!("/cabrillo") view=CabrilloPage />
        <Route path=path!("/polarization") view=PolarizationPage />
        <Route path=path!("/dv-network") view=DvNetworkPage />
        <Route path=path!("/dx-spots") view=DxSpotsPage />
        <Route path=path!("/dashboard") view=DashboardPage />
        <Route path=path!("/progress") view=ProgressPage />
        <Route path=path!("/grid-map") view=GridMapPage />
        <Route path=path!("/grayline") view=GraylinePage />
        <Route path=path!("/stats") view=StatsPage />
        <Route path=path!("/grounding") view=GroundingPage />
        <Route path=path!("/antenna-analyzer") view=AntennaAnalyzerPage />
        <Route path=path!("/power-supply") view=PowerSupplyPage />
        <Route path=path!("/tools") view=ToolsPage />
        <Route path=path!("/photo-processor") view=PhotoProcessorPage />
        <Route path=path!("/sstv") view=SstvPage />
        <Route path=path!("/weather-sat") view=WeatherSatPage />
        <Route path=path!("/packet") view=PacketPage />
        <Route path=path!("/mobile") view=MobilePage />
        <Route path=path!("/license-classes") view=LicenseClassesPage />
        <Route path=path!("/receiver") view=ReceiverPage />
        <Route path=path!("/antenna-array") view=AntennaArrayPage />
      </Routes>
      <RelatedTopics />
    </main>
  }
}
