use leptos::prelude::*;
use leptos_router::components::{Route, Routes};
use leptos_router::hooks::use_location;
use leptos_router::path;

use crate::pages::{
  AchievementsPage, AmplifierPage, AnalogModesPage, AntennaAnalyzerPage, AntennaArrayPage,
  AntennaDiyPage, AntennaFarmPage, AntennaInstallationPage, AntennaModelingPage, AntennaTuningPage,
  AntennasPage, AprsPage, AptDecoderPage, ArdfPage, AtvPage, AuroraPage, AwardsPage, BalunPage,
  BandPlanPage, BandsPage, BeaconsPage, BeginnerPage, BookmarksPage, BrowsePage, CabrilloPage,
  CallsignCopyPage, CallsignPage, CardsPage, CheatSheetPage, CommunityPage, ConfusablesPage,
  ContestCalendarPage, ContestLogPage, ContestPage, CoordinationPage, CountdownPage, CwOpPage,
  DailyChallengePage, DashboardPage, DevelopersPage, DiyProjectsPage, DvNetworkPage, DxPage,
  DxSpotsPage, DxccMapPage, DxpeditionPage, ElectronicsPage, EmcommPage, EmePage, EqslPage,
  EventsPage, ExamPage, ExamReviewPage, FeedlinePage, FiltersPage, FlashcardsPage, FormulasPage,
  FrequenciesPage, Ft8Page, GearPage, GlossaryPage, GnuradioPage, GraylinePage, GridMapPage,
  GridSystemPage, GroundingPage, HistoryPage, HomePage, IotaPage, LearningPathPage,
  LearningResourcesPage, LicenseClassesPage, LicensePage, ListenPage, LogPage, LoggingSoftwarePage,
  MeteorScatterPage, MetersPage, MicrowavePage, MistakeTopicsPage, MistakesPage, MobilePage,
  ModesPage, MorsePage, MostWantedPage, MufPage, NoisePage, NotFoundPage, NotificationsPage,
  NvisPage, OpenSourcePage, OperatingPage, OrganizationsPage, PacketPage, PhoneticPage,
  PhotoProcessorPage, PolarizationPage, PortableMapPage, PortablePage, PowerPage, PowerSupplyPage,
  PracticalAntennasPage, PracticePage, PrefixesPage, PrintPage, ProgressPage, PropagationPage,
  PskDecodePage, PskReporterPage, QCodePage, QrpPage, QslCardPage, QslDesignerPage, QslLabelsPage,
  RbnPage, ReceiverPage, ReferencePage, RegulationsPage, RemotePage, RepeaterBuildPage,
  RepeaterPage, ReportPage, RfiPage, RstPage, RttyPage, SafetyPage, SatOperationPage,
  SatellitesPage, SdrMapPage, SdrPage, SdrWaterfallPage, SolarPage, SpecialPropPage,
  SstvDecoderPage, SstvPage, StatsPage, StudyCalendarPage, SwlPage, ToolsPage, TransceiverPage,
  VnaPage, WeatherSatPage, WeeklyPage, WinlinkPage, WsprDecoderPage, WsprPage, ZoneMapPage,
};

use crate::components::related_topics::RelatedTopics;
use crate::components::topic_quiz::TopicQuiz;
use crate::motion::ROUTE_CONTENT_ID;

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
    // 路由内容的过渡容器：入场动画由 `RouteTransition` 在路径变化时重放。
    // 单独包一层而不是直接动 `<main>`，是因为 `<main>` 里还有 TopicQuiz / RelatedTopics
    // 这类 `position: fixed` 的挂件 —— 祖先只要有 transform，fixed 就退化成 absolute。
    <main id="main-content" tabindex="-1" class="flex-1 outline-none">
      <div id=ROUTE_CONTENT_ID>
        <Routes fallback=|| view! { <NotFoundPage /> }>
        <Route path=path!("/") view=HomePage />
        <Route path=path!("/practice") view=PracticePage />
        <Route path=path!("/print") view=PrintPage />
        <Route path=path!("/qsl-labels") view=QslLabelsPage />
        <Route path=path!("/listen") view=ListenPage />
        <Route path=path!("/cards") view=CardsPage />
        <Route path=path!("/exam") view=ExamPage />
        <Route path=path!("/daily-challenge") view=DailyChallengePage />
        <Route path=path!("/browse") view=BrowsePage />
        <Route path=path!("/glossary") view=GlossaryPage />
        <Route path=path!("/q-code") view=QCodePage />
        <Route path=path!("/morse") view=MorsePage />
        <Route path=path!("/phonetic") view=PhoneticPage />
        <Route path=path!("/callsign-copy") view=CallsignCopyPage />
        <Route path=path!("/bands") view=BandsPage />
        <Route path=path!("/beacons") view=BeaconsPage />
        <Route path=path!("/reference") view=ReferencePage />
        <Route path=path!("/cheat-sheet") view=CheatSheetPage />
        <Route path=path!("/confusables") view=ConfusablesPage />
        <Route path=path!("/formulas") view=FormulasPage />
        <Route path=path!("/antennas") view=AntennasPage />
        <Route path=path!("/bandplan") view=BandPlanPage />
        <Route path=path!("/prefixes") view=PrefixesPage />
        <Route path=path!("/callsign") view=CallsignPage />
        <Route path=path!("/modes") view=ModesPage />
        <Route path=path!("/analog-modes") view=AnalogModesPage />
        <Route path=path!("/frequencies") view=FrequenciesPage />
        <Route path=path!("/coordination") view=CoordinationPage />
        <Route path=path!("/satellites") view=SatellitesPage />
        <Route path=path!("/sat-operation") view=SatOperationPage />
        <Route path=path!("/operating") view=OperatingPage />
        <Route path=path!("/rst") view=RstPage />
        <Route path=path!("/propagation") view=PropagationPage />
        <Route path=path!("/log") view=LogPage />
        <Route path=path!("/contest-log") view=ContestLogPage />
        <Route path=path!("/countdown") view=CountdownPage />
        <Route path=path!("/mistakes") view=MistakesPage />
        <Route path=path!("/mistake-topics") view=MistakeTopicsPage />
        <Route path=path!("/bookmarks") view=BookmarksPage />
        <Route path=path!("/flashcards") view=FlashcardsPage />
        <Route path=path!("/contest") view=ContestPage />
        <Route path=path!("/contest-calendar") view=ContestCalendarPage />
        <Route path=path!("/events") view=EventsPage />
        <Route path=path!("/solar") view=SolarPage />
        <Route path=path!("/safety") view=SafetyPage />
        <Route path=path!("/license") view=LicensePage />
        <Route path=path!("/electronics") view=ElectronicsPage />
        <Route path=path!("/feedline") view=FeedlinePage />
        <Route path=path!("/balun") view=BalunPage />
        <Route path=path!("/meters") view=MetersPage />
        <Route path=path!("/power") view=PowerPage />
        <Route path=path!("/awards") view=AwardsPage />
        <Route path=path!("/aprs") view=AprsPage />
        <Route path=path!("/sdr") view=SdrPage />
        <Route path=path!("/sdr-map") view=SdrMapPage />
        <Route path=path!("/sdr-waterfall") view=SdrWaterfallPage />
        <Route path=path!("/emcomm") view=EmcommPage />
        <Route path=path!("/winlink") view=WinlinkPage />
        <Route path=path!("/beginner") view=BeginnerPage />
        <Route path=path!("/organizations") view=OrganizationsPage />
        <Route path=path!("/ardf") view=ArdfPage />
        <Route path=path!("/special-prop") view=SpecialPropPage />
        <Route path=path!("/meteor-scatter") view=MeteorScatterPage />
        <Route path=path!("/aurora") view=AuroraPage />
        <Route path=path!("/antenna-diy") view=AntennaDiyPage />
        <Route path=path!("/transceiver") view=TransceiverPage />
        <Route path=path!("/gear") view=GearPage />
        <Route path=path!("/dx") view=DxPage />
        <Route path=path!("/eqsl") view=EqslPage />
        <Route path=path!("/grid") view=GridSystemPage />
        <Route path=path!("/history") view=HistoryPage />
        <Route path=path!("/muf") view=MufPage />
        <Route path=path!("/portable") view=PortablePage />
        <Route path=path!("/portable-map") view=PortableMapPage />
        <Route path=path!("/cw-operating") view=CwOpPage />
        <Route path=path!("/antenna-installation") view=AntennaInstallationPage />
        <Route path=path!("/ft8") view=Ft8Page />
        <Route path=path!("/repeater") view=RepeaterPage />
        <Route path=path!("/wspr") view=WsprPage />
        <Route path=path!("/wspr-decode") view=WsprDecoderPage />
        <Route path=path!("/logging-software") view=LoggingSoftwarePage />
        <Route path=path!("/open-source") view=OpenSourcePage />
        <Route path=path!("/microwave") view=MicrowavePage />
        <Route path=path!("/remote") view=RemotePage />
        <Route path=path!("/qsl-card") view=QslCardPage />
        <Route path=path!("/qsl-designer") view=QslDesignerPage />
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
        <Route path=path!("/study-calendar") view=StudyCalendarPage />
        <Route path=path!("/achievements") view=AchievementsPage />
        <Route path=path!("/weekly") view=WeeklyPage />
        <Route path=path!("/report") view=ReportPage />
        <Route path=path!("/exam-review") view=ExamReviewPage />
        <Route path=path!("/grid-map") view=GridMapPage />
        <Route path=path!("/dxcc-map") view=DxccMapPage />
        <Route path=path!("/zone-map") view=ZoneMapPage />
        <Route path=path!("/grayline") view=GraylinePage />
        <Route path=path!("/stats") view=StatsPage />
        <Route path=path!("/notifications") view=NotificationsPage />
        <Route path=path!("/psk-reporter") view=PskReporterPage />
        <Route path=path!("/psk-decode") view=PskDecodePage />
        <Route path=path!("/rbn") view=RbnPage />
        <Route path=path!("/grounding") view=GroundingPage />
        <Route path=path!("/antenna-analyzer") view=AntennaAnalyzerPage />
        <Route path=path!("/power-supply") view=PowerSupplyPage />
        <Route path=path!("/tools") view=ToolsPage />
        <Route path=path!("/photo-processor") view=PhotoProcessorPage />
        <Route path=path!("/sstv") view=SstvPage />
        <Route path=path!("/sstv-decode") view=SstvDecoderPage />
        <Route path=path!("/weather-sat") view=WeatherSatPage />
        <Route path=path!("/apt-decoder") view=AptDecoderPage />
        <Route path=path!("/packet") view=PacketPage />
        <Route path=path!("/mobile") view=MobilePage />
        <Route path=path!("/license-classes") view=LicenseClassesPage />
        <Route path=path!("/receiver") view=ReceiverPage />
        <Route path=path!("/antenna-array") view=AntennaArrayPage />
        <Route path=path!("/noise") view=NoisePage />
        <Route path=path!("/learning-path") view=LearningPathPage />
        <Route path=path!("/practical-antennas") view=PracticalAntennasPage />
        <Route path=path!("/vna") view=VnaPage />
        <Route path=path!("/diy-projects") view=DiyProjectsPage />
        <Route path=path!("/developers") view=DevelopersPage />
        <Route path=path!("/learning-resources") view=LearningResourcesPage />
        <Route path=path!("/community") view=CommunityPage />
      </Routes>
      </div>
      <TopicQuiz />
      <RelatedTopics />
    </main>
  }
}
