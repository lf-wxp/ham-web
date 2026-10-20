use leptos::prelude::*;

use crate::components::common::{PageContainer, PageHeader, StickyPanel};
use crate::icons::IconKind;
use crate::ui::MobileNavDrawer;
use crate::util::{document, scroll_to_element, set_title};

use super::antenna_length::AntennaLength;
use super::antenna_matcher::AntennaMatcher;
use super::aprs_codec::AprsCodec;
use super::attenuator_calculator::AttenuatorCalculator;
use super::backup_tool::BackupTool;
use super::battery_runtime::BatteryRuntime;
use super::callsign_lookup::CallsignLookup;
use super::capacitor_code::CapacitorCode;
use super::cascade_gain::CascadeGain;
use super::coil_yagi_calculator::CoilYagiCalculator;
use super::contest_scorer::ContestScorer;
use super::cw_bandwidth::CwBandwidth;
use super::dbm_converter::DbmConverter;
use super::dbm_dbuv::DbmDbuv;
use super::decibel_gain::DecibelGain;
use super::distance_bearing::DistanceBearing;
use super::doppler_calculator::DopplerCalculator;
use super::duty_power::DutyPower;
use super::eirp_calculator::EirpCalculator;
use super::feedline_loss::FeedlineLoss;
use super::filter_design::FilterDesign;
use super::freq_wavelength::FreqWavelength;
use super::frequency_units::FrequencyUnits;
use super::fresnel_zone::FresnelZone;
use super::fspl_calculator::FsplCalculator;
use super::gain_conversion::GainConversion;
use super::intermod_calculator::IntermodCalculator;
use super::ip3_cascade::Ip3Cascade;
use super::lc_resonance::LcResonance;
use super::link_budget_calculator::LinkBudgetCalculator;
use super::loading_coil::LoadingCoilTool;
use super::lora_calculator::LoraCalculator;
use super::magnetic_loop::MagneticLoopTool;
use super::mode_encoder::ModeEncoder;
use super::noise_cascade::NoiseCascade;
use super::off_grid::OffGrid;
use super::ohms_law::OhmsLaw;
use super::oscillator::Oscillator;
use super::parabolic_dish::ParabolicDish;
use super::path_profile::PathProfile;
use super::propagation_estimator::PropagationEstimator;
use super::radiogram_generator::RadiogramGenerator;
use super::rain_fade::RainFade;
use super::reactance::Reactance;
use super::receiver_sensitivity::ReceiverSensitivity;
use super::resistor_color_code::ResistorColorCode;
use super::resistor_parallel::ResistorParallel;
use super::rf_exposure::RfExposure;
use super::smd_resistor_code::SmdResistorCode;
use super::smith_chart::SmithChart;
use super::soft_tnc::SoftTnc;
use super::stub_match::StubMatch;
use super::swr_converter::SwrConverter;
use super::tone_squelch::ToneSquelch;
use super::tool_section::ToolSection;
use super::toroid_winding::ToroidWinding;
use super::transformer_calculator::TransformerCalculator;
use super::trap::TrapTool;
use super::tx_line::TxLine;
use super::utc_clock::UtcClock;
use super::waveguide_calculator::WaveguideCalculator;
use super::wire_gauge::WireGauge;
use crate::i18n::t;

/// 工具目录（锚点 id + 目录标题词条）。标题必须走 i18n：这里存的是**词条 key** 而不是文字，
/// 否则 en / es 界面会露中文（i18n 检查器扫不到 const 数据）。
const TOOL_NAV: &[(&str, &str)] = &[
  ("freq-wavelength", "tools.frequency-wavelength"),
  ("dbm-power", "tools.dbm-power"),
  ("decibel-gain", "tools.decibel-gain"),
  ("ohms-law", "tools.ohm-s-law"),
  ("cw-bandwidth", "tools.cw-required-bandwidth"),
  ("lc-resonance", "tools.lc-resonance"),
  ("reactance", "tools.capacitive-inductive-reactance"),
  ("antenna-length", "tools.antenna-length"),
  ("antenna-matcher", "tools.antenna-matching"),
  ("smith", "tools.smith-chart"),
  ("swr", "tools.swr"),
  ("cascade-gain", "tools.cascaded-gain"),
  ("resistor", "tools.resistors-in-series-parallel"),
  ("freq-units", "tools.frequency-units"),
  ("battery", "tools.battery-runtime"),
  ("off-grid", "tools.offgrid"),
  ("dbm-dbuv", "tools.dbm-db-v"),
  ("feedline-loss", "tools.feedline-loss"),
  ("filter-design", "tools.filter-design"),
  ("callsign-lookup", "tools.callsign-lookup"),
  ("distance-bearing", "tools.distance-bearing"),
  ("propagation-muf", "tools.propagation-muf"),
  ("doppler", "tools.satellite-doppler"),
  ("fspl", "tools.path-loss"),
  ("fresnel-zone", "tools.fresnel-zone"),
  ("path-profile", "tools.path-profile"),
  ("intermod", "tools.intermod"),
  ("parabolic-dish", "tools.parabolic-dish"),
  ("waveguide", "tools.waveguide"),
  ("rain-fade", "tools.rain-fade"),
  ("duty-power", "tools.duty-power"),
  ("ip3-cascade", "tools.ip3-cascade"),
  // 与正文章节标题同一条词条：目录里显示成完整标题（如「EIRP 有效辐射功率」）。
  ("eirp", "tools.eirp-effective-radiated-power"),
  ("link-budget", "tools.link-budget"),
  ("lora", "tools.lora"),
  ("radiogram", "tools.radiogram"),
  ("receiver-sensitivity", "tools.receiver-sensitivity-toc"),
  ("noise-cascade", "tools.noise-cascade"),
  ("gain-conversion", "tools.gain-conversion"),
  ("resistor-code", "tools.resistor-colour-code"),
  ("smd-resistor", "tools.smd-resistor-code"),
  ("capacitor-code", "tools.capacitor-code"),
  ("contest-scorer", "tools.contest-scoring"),
  ("coil-yagi", "tools.coil-yagi-calculator"),
  ("attenuator", "tools.attenuator"),
  ("transformer", "tools.transformer-impedance"),
  ("utc-clock", "tools.utc-time-heading"),
  ("tx-line", "tools.transmission-line-impedance"),
  ("stub-match", "tools.stub-match"),
  ("toroid-winding", "tools.toroid-winding"),
  ("magnetic-loop", "tools.magloop"),
  ("loading-coil", "tools.loading-coil"),
  ("trap", "tools.trap"),
  ("wire-gauge", "tools.wire-gauge-and-voltage"),
  ("oscillator", "tools.crystal-oscillator"),
  ("rf-exposure", "tools.rf-exposure-evaluation"),
  ("tone-squelch", "tools.tone-squelch-and-repeater"),
  ("aprs-codec", "tools.aprs-codec"),
  ("soft-tnc", "tools.soft-tnc"),
  ("mode-encoder", "tools.digital-mode-encoding"),
  ("backup", "tools.data-backup"),
];

/// 滚动到指定工具（让出吸顶页头，见 `util::scroll_to_element`）。
fn scroll_to(id: &str) {
  if let Some(el) = document().get_element_by_id(id) {
    scroll_to_element(&el);
  }
}

#[component]
pub fn ToolsPage() -> impl IntoView {
  set_title("shell.calculators");
  // 移动端工具导航抽屉的开关：选中工具后要主动收起。
  let nav_open = RwSignal::new(false);
  // 路由切换后不会自动跳到锚点（如存储告警里的 `/tools#backup`），挂载后手动滚动
  let hash = leptos_router::hooks::use_location().hash;
  Effect::new(move |_| {
    if let Some(id) = hash.get().strip_prefix('#').filter(|id| !id.is_empty()) {
      let id = id.to_owned();
      request_animation_frame(move || scroll_to(&id));
    }
  });
  // 工具导航列表（桌面侧栏与移动端抽屉共用）。
  let tool_nav_items = move || {
    TOOL_NAV
      .iter()
      .map(|&(id, key)| {
        view! {
          <li>
            <button
              type="button"
              class="w-full px-2.5 py-1.5 text-left text-sm text-muted-foreground transition-colors hover:bg-accent hover:text-foreground"
              on:click=move |_| {
                scroll_to(id);
                nav_open.set(false);
              }
            >
              {move || t(key)}
            </button>
          </li>
        }
      })
      .collect_view()
  };
  view! {
    <div class="min-h-screen animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <PageHeader
        title=move || t("shell.calculators")
        subtitle=move || t("tools.frequency-wavelength-power-gain")
      />

      <PageContainer class="space-y-0">
        <MobileNavDrawer
          open=nav_open
          title=Signal::derive(move || t("tools.tool-catalogue"))
          button_label=Signal::derive(move || t("tools.tool-catalogue"))
          icon=IconKind::SlidersHorizontal
          class="lg:hidden"
        >
          <ul class="space-y-0.5">{tool_nav_items()}</ul>
        </MobileNavDrawer>

        <div class="grid gap-6 lg:grid-cols-[14rem_1fr]">
          <aside class="hidden lg:block">
            <StickyPanel class="pxl-card border bg-card p-3">
              <nav aria-label=move || t("tools.tool-catalogue")>
                <div class="mb-2 px-2 text-xs font-semibold text-muted-foreground">{move || t("tools.tool-navigation")}</div>
                <ul class="max-h-[calc(100vh-var(--sticky-top)-70px)] space-y-0.5 overflow-y-auto pr-1">
                  {tool_nav_items()}
                </ul>
              </nav>
            </StickyPanel>
          </aside>

          <div class="min-w-0 space-y-6">
            <ToolSection id="freq-wavelength" title="tools.frequency-wavelength" description="tools.300-f-mhz-approximate"><FreqWavelength /></ToolSection>

            <ToolSection id="dbm-power" title="tools.dbm-power" description="tools.0-dbm-1-mw"><DbmConverter /></ToolSection>

            <ToolSection id="decibel-gain" title="tools.decibel-gain" description="tools.power-ratio-db-10"><DecibelGain /></ToolSection>

            <ToolSection id="ohms-law" title="tools.ohm-s-law-power" description="tools.u-i-r-p"><OhmsLaw /></ToolSection>

            <ToolSection id="cw-bandwidth" title="tools.cw-required-bandwidth" description="tools.bn-b-k-b"><CwBandwidth /></ToolSection>

            <ToolSection id="lc-resonance" title="tools.lc-resonant-frequency" description="tools.f-1-2-lc"><LcResonance /></ToolSection>

            <ToolSection id="reactance" title="tools.capacitive-inductive-reactance" description="tools.xc-1-2-fc"><Reactance /></ToolSection>

            <ToolSection id="antenna-length" title="tools.antenna-length" description="tools.half-wave-dipole-l"><AntennaLength /></ToolSection>

            <ToolSection id="antenna-matcher" title="tools.antenna-matching-network" description="tools.an-l-network-matches"><AntennaMatcher /></ToolSection>

            <section id="smith" class="scroll-mt-24 pxl-card border bg-card">
              <h2 class="pxl-section-title">{move || t("tools.smith-chart")}</h2>
              <p class="px-4 pt-3 text-xs text-muted-foreground">{move || t("tools.enter-a-complex-impedance")}</p>
              <div class="p-4"><SmithChart /></div>
              <p class="px-4 pb-4 text-xs text-muted-foreground">
                {move || t("tools.for-drag-to-position")}
                <a href="/smith" class="ml-1 text-primary underline underline-offset-4">
                  {move || t("tools.smith-chart-and-matching")}
                </a>
              </p>
            </section>

            <ToolSection id="swr" title="tools.swr-reflection-coefficient" description="tools.swr-1-1-return"><SwrConverter /></ToolSection>

            <ToolSection id="cascade-gain" title="tools.cascaded-gain" description="tools.stage-gains-in-db"><CascadeGain /></ToolSection>

            <ToolSection id="resistor" title="tools.resistors-in-series-parallel" description="tools.series-r-r-parallel"><ResistorParallel /></ToolSection>

            <ToolSection id="freq-units" title="tools.frequency-unit-conversion" description="tools.1-mhz-1000-khz"><FrequencyUnits /></ToolSection>

            <ToolSection id="battery" title="tools.battery-runtime-estimate" description="tools.runtime-hours-capacity-mah"><BatteryRuntime /></ToolSection>

            <ToolSection id="off-grid" title="tools.offgrid" description="tools.offgrid-description"><OffGrid /></ToolSection>

            <ToolSection id="dbm-dbuv" title="tools.dbm-db-v" description="tools.at-50-db-v"><DbmDbuv /></ToolSection>

            <ToolSection id="feedline-loss" title="tools.feedline-loss" description="tools.total-loss-loss-per"><FeedlineLoss /></ToolSection>

            <ToolSection id="filter-design" title="tools.filter-design" description="tools.l-c-component-values"><FilterDesign /></ToolSection>

            <ToolSection id="callsign-lookup" title="tools.callsign-lookup" description="tools.identifies-the-dxcc-entity"><CallsignLookup /></ToolSection>

            <ToolSection id="distance-bearing" title="tools.distance-bearing" description="tools.enter-two-maidenhead-grids"><DistanceBearing /></ToolSection>

            <ToolSection id="propagation-muf" title="tools.propagation-muf" description="tools.estimates-fof2-single-hop"><PropagationEstimator /></ToolSection>

            <ToolSection id="doppler" title="tools.satellite-doppler" description="tools.estimates-the-doppler-shift"><DopplerCalculator /></ToolSection>

            <ToolSection id="fspl" title="tools.free-space-path-loss" description="tools.computes-the-free-space"><FsplCalculator /></ToolSection>

            <ToolSection id="fresnel-zone" title="tools.fresnel-zone-and-line-of-sight" description="tools.fresnel-zone-description"><FresnelZone /></ToolSection>

            <ToolSection id="path-profile" title="tools.path-profile" description="tools.path-profile-description"><PathProfile /></ToolSection>

            <ToolSection id="intermod" title="tools.intermod-harmonics-image" description="tools.intermod-description"><IntermodCalculator /></ToolSection>

            <ToolSection id="parabolic-dish" title="tools.parabolic-dish" description="tools.parabolic-dish-description"><ParabolicDish /></ToolSection>

            <ToolSection id="waveguide" title="tools.waveguide" description="tools.waveguide-description"><WaveguideCalculator /></ToolSection>

            <ToolSection id="rain-fade" title="tools.rain-fade" description="tools.rain-fade-description"><RainFade /></ToolSection>

            <ToolSection id="duty-power" title="tools.duty-power" description="tools.duty-power-description"><DutyPower /></ToolSection>

            <ToolSection id="ip3-cascade" title="tools.ip3-cascade" description="tools.ip3-cascade-description"><Ip3Cascade /></ToolSection>

            <ToolSection id="eirp" title="tools.eirp-effective-radiated-power" description="tools.transmit-power-antenna-gain"><EirpCalculator /></ToolSection>

            <ToolSection id="link-budget" title="tools.link-budget" description="tools.eirp-path-loss-receive"><LinkBudgetCalculator /></ToolSection>

            <ToolSection id="lora" title="tools.lora" description="tools.lora-description"><LoraCalculator /></ToolSection>

            <ToolSection id="radiogram" title="tools.radiogram" description="tools.radiogram-description"><RadiogramGenerator /></ToolSection>

            <ToolSection id="receiver-sensitivity" title="tools.receiver-sensitivity" description="tools.estimates-sensitivity-from-the"><ReceiverSensitivity /></ToolSection>

            <ToolSection id="noise-cascade" title="tools.noise-figure-cascade" description="tools.friis-formula-computes-the"><NoiseCascade /></ToolSection>

            <ToolSection id="gain-conversion" title="tools.antenna-gain-conversion" description="tools.quick-dbi-dbd-conversion"><GainConversion /></ToolSection>

            <ToolSection id="resistor-code" title="tools.resistor-colour-code" description="tools.4-band-resistor-first"><ResistorColorCode /></ToolSection>

            <ToolSection id="smd-resistor" title="tools.smd-resistor-code" description="tools.smd-resistor-code-description"><SmdResistorCode /></ToolSection>

            <ToolSection id="capacitor-code" title="tools.capacitor-code" description="tools.capacitor-code-description"><CapacitorCode /></ToolSection>

            <ToolSection id="contest-scorer" title="tools.contest-scoring" description="tools.quick-score-estimates-for"><ContestScorer /></ToolSection>

            <ToolSection id="coil-yagi" title="tools.coil-yagi-calculator" description="tools.air-core-coil-inductance"><CoilYagiCalculator /></ToolSection>

            <ToolSection id="attenuator" title="tools.attenuator" description="tools.attenuator-description"><AttenuatorCalculator /></ToolSection>

            <ToolSection id="transformer" title="tools.transformer-impedance" description="tools.compute-transformer-turns-ratio"><TransformerCalculator /></ToolSection>

            <ToolSection id="utc-clock" title="tools.utc-time-heading" description="tools.live-utc-zulu-clock"><UtcClock /></ToolSection>

            <ToolSection id="tx-line" title="tools.transmission-line-impedance" description="tools.characteristic-impedance-of-coax"><TxLine /></ToolSection>

            <ToolSection id="stub-match" title="tools.stub-match" description="tools.stub-match-description"><StubMatch /></ToolSection>

            <ToolSection id="toroid-winding" title="tools.toroid-winding" description="tools.toroid-winding-description"><ToroidWinding /></ToolSection>

            <ToolSection id="magnetic-loop" title="tools.magloop" description="tools.magloop-description"><MagneticLoopTool /></ToolSection>

            <ToolSection id="loading-coil" title="tools.loading-coil" description="tools.loading-coil-description"><LoadingCoilTool /></ToolSection>

            <ToolSection id="trap" title="tools.trap" description="tools.trap-description"><TrapTool /></ToolSection>

            <ToolSection id="wire-gauge" title="tools.wire-gauge-and-voltage" description="tools.estimate-dc-supply-wire"><WireGauge /></ToolSection>

            <ToolSection id="oscillator" title="tools.crystal-oscillator" description="tools.crystal-frequency-pulling-load"><Oscillator /></ToolSection>

            <ToolSection id="rf-exposure" title="tools.rf-exposure-evaluation" description="tools.estimate-power-density-and"><RfExposure /></ToolSection>

            <ToolSection id="tone-squelch" title="tools.tone-squelch-and-repeater" description="tools.repeater-offset-calculation-plus"><ToneSquelch /></ToolSection>

            <ToolSection id="aprs-codec" title="tools.aprs-codec" description="tools.encode-and-decode-uncompressed"><AprsCodec /></ToolSection>

            <ToolSection id="soft-tnc" title="tools.soft-tnc" description="tools.soft-tnc-description"><SoftTnc /></ToolSection>

            <ToolSection id="mode-encoder" title="tools.digital-mode-encoding" description="tools.text-to-morse-cw"><ModeEncoder /></ToolSection>

            <ToolSection id="backup" title="tools.data-backup" description="settings.export-all-local-data"><BackupTool /></ToolSection>
          </div>
        </div>
      </PageContainer>
    </div>
  }
}
