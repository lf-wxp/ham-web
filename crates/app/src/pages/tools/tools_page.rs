use leptos::prelude::*;

use crate::util::set_title;

use super::antenna_length::AntennaLength;
use super::antenna_matcher::AntennaMatcher;
use super::aprs_codec::AprsCodec;
use super::attenuator_calculator::AttenuatorCalculator;
use super::backup_tool::BackupTool;
use super::battery_runtime::BatteryRuntime;
use super::callsign_lookup::CallsignLookup;
use super::cascade_gain::CascadeGain;
use super::coil_yagi_calculator::CoilYagiCalculator;
use super::contest_scorer::ContestScorer;
use super::cw_bandwidth::CwBandwidth;
use super::dbm_converter::DbmConverter;
use super::dbm_dbuv::DbmDbuv;
use super::decibel_gain::DecibelGain;
use super::distance_bearing::DistanceBearing;
use super::doppler_calculator::DopplerCalculator;
use super::eirp_calculator::EirpCalculator;
use super::feedline_loss::FeedlineLoss;
use super::filter_design::FilterDesign;
use super::freq_wavelength::FreqWavelength;
use super::frequency_units::FrequencyUnits;
use super::fspl_calculator::FsplCalculator;
use super::gain_conversion::GainConversion;
use super::lc_resonance::LcResonance;
use super::link_budget_calculator::LinkBudgetCalculator;
use super::mode_encoder::ModeEncoder;
use super::noise_cascade::NoiseCascade;
use super::ohms_law::OhmsLaw;
use super::oscillator::Oscillator;
use super::propagation_estimator::PropagationEstimator;
use super::reactance::Reactance;
use super::receiver_sensitivity::ReceiverSensitivity;
use super::resistor_color_code::ResistorColorCode;
use super::resistor_parallel::ResistorParallel;
use super::rf_exposure::RfExposure;
use super::smith_chart::SmithChart;
use super::swr_converter::SwrConverter;
use super::tone_squelch::ToneSquelch;
use super::transformer_calculator::TransformerCalculator;
use super::tx_line::TxLine;
use super::utc_clock::UtcClock;
use super::wire_gauge::WireGauge;
use crate::i18n::t;

/// 工具目录（锚点 id + 标题）。
const TOOL_NAV: &[(&str, &str)] = &[
  ("freq-wavelength", "频率 ↔ 波长"),
  ("dbm-power", "dBm ↔ 功率"),
  ("decibel-gain", "分贝增益"),
  ("ohms-law", "欧姆定律"),
  ("cw-bandwidth", "CW 必要带宽"),
  ("lc-resonance", "LC 谐振"),
  ("reactance", "容抗 / 感抗"),
  ("antenna-length", "天线长度"),
  ("antenna-matcher", "天线匹配"),
  ("smith", "史密斯圆图"),
  ("swr", "驻波比"),
  ("cascade-gain", "级联增益"),
  ("resistor", "电阻串并联"),
  ("freq-units", "频率单位"),
  ("battery", "电池续航"),
  ("dbm-dbuv", "dBm ↔ dBμV"),
  ("feedline-loss", "馈线损耗"),
  ("filter-design", "滤波器设计"),
  ("callsign-lookup", "呼号查询"),
  ("distance-bearing", "两点距离 / 方位角"),
  ("propagation-muf", "传播预测 MUF"),
  ("doppler", "卫星多普勒"),
  ("fspl", "路径损耗"),
  ("eirp", "EIRP"),
  ("link-budget", "链路预算"),
  ("receiver-sensitivity", "接收灵敏度"),
  ("noise-cascade", "噪声级联"),
  ("gain-conversion", "增益换算"),
  ("resistor-code", "电阻色环"),
  ("contest-scorer", "竞赛记分"),
  ("coil-yagi", "线圈 / Yagi 计算"),
  ("attenuator", "衰减器"),
  ("transformer", "变压器阻抗"),
  ("utc-clock", "UTC 时间"),
  ("tx-line", "传输线阻抗"),
  ("wire-gauge", "线径压降"),
  ("oscillator", "晶体振荡"),
  ("rf-exposure", "射频暴露"),
  ("tone-squelch", "亚音与中继频差"),
  ("aprs-codec", "APRS 编解码"),
  ("mode-encoder", "数字模式编码"),
  ("backup", "数据备份"),
];

/// 平滑滚动到指定工具。
fn scroll_to(id: &str) {
  if let Some(el) = web_sys::window()
    .and_then(|w| w.document())
    .and_then(|d| d.get_element_by_id(id))
  {
    el.scroll_into_view();
  }
}

#[component]
pub fn ToolsPage() -> impl IntoView {
  set_title("shell.calculators");
  // 路由切换后不会自动跳到锚点（如存储告警里的 `/tools#backup`），挂载后手动滚动
  let hash = leptos_router::hooks::use_location().hash;
  Effect::new(move |_| {
    if let Some(id) = hash.get().strip_prefix('#').filter(|id| !id.is_empty()) {
      let id = id.to_owned();
      request_animation_frame(move || scroll_to(&id));
    }
  });
  view! {
    <div class="min-h-screen animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <header class="sticky top-0 z-20 border-b bg-background/90 backdrop-blur">
        <div class="mx-auto flex max-w-5xl flex-wrap items-center gap-3 px-4 py-3">
          <div class="mr-auto">
            <h1 class="text-base font-semibold leading-tight">{move || t("shell.calculators")}</h1>
            <div class="text-xs text-muted-foreground">{move || t("tools.frequency-wavelength-power-gain")}</div>
          </div>
        </div>
      </header>

      <div class="mx-auto max-w-5xl px-4 py-5">
        // 移动端：横向滚动的工具导航 chip 条。
        <div class="mb-4 -mx-4 overflow-x-auto px-4 lg:hidden">
          <div class="flex gap-1.5 pb-1">
            {TOOL_NAV
              .iter()
              .map(|&(id, label)| {
                view! {
                  <button
                    type="button"
                    class="shrink-0 rounded-full border bg-card px-3 py-1.5 text-xs text-muted-foreground transition-colors hover:bg-accent hover:text-foreground"
                    on:click=move |_| scroll_to(id)
                  >
                    {move || t(label)}
                  </button>
                }
              })
              .collect_view()}
          </div>
        </div>

        <div class="grid gap-6 lg:grid-cols-[14rem_1fr]">
          <aside class="hidden lg:block">
            <nav aria-label=move || t("tools.tool-catalogue") class="sticky top-20 rounded-xl border bg-card p-3">
              <div class="mb-2 px-2 text-xs font-semibold text-muted-foreground">{move || t("tools.tool-navigation")}</div>
              <ul class="max-h-[calc(100vh-7rem)] space-y-0.5 overflow-y-auto pr-1">
                {TOOL_NAV
                  .iter()
                  .map(|&(id, label)| {
                    view! {
                      <li>
                        <button
                          type="button"
                          class="w-full rounded-md px-2.5 py-1.5 text-left text-sm text-muted-foreground transition-colors hover:bg-accent hover:text-foreground"
                          on:click=move |_| scroll_to(id)
                        >
                          {move || t(label)}
                        </button>
                      </li>
                    }
                  })
                  .collect_view()}
              </ul>
            </nav>
          </aside>

          <div class="min-w-0 space-y-6">
            <section id="freq-wavelength" class="scroll-mt-24 rounded-xl border bg-card">
              <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("tools.frequency-wavelength")}</h2>
              <p class="px-4 pt-3 text-xs text-muted-foreground">{move || t("tools.300-f-mhz-approximate")}</p>
              <div class="p-4"><FreqWavelength /></div>
            </section>

            <section id="dbm-power" class="scroll-mt-24 rounded-xl border bg-card">
              <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("tools.dbm-power")}</h2>
              <p class="px-4 pt-3 text-xs text-muted-foreground">{move || t("tools.0-dbm-1-mw")}</p>
              <div class="p-4"><DbmConverter /></div>
            </section>

            <section id="decibel-gain" class="scroll-mt-24 rounded-xl border bg-card">
              <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("tools.decibel-gain")}</h2>
              <p class="px-4 pt-3 text-xs text-muted-foreground">{move || t("tools.power-ratio-db-10")}</p>
              <div class="p-4"><DecibelGain /></div>
            </section>

            <section id="ohms-law" class="scroll-mt-24 rounded-xl border bg-card">
              <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("tools.ohm-s-law-power")}</h2>
              <p class="px-4 pt-3 text-xs text-muted-foreground">{move || t("tools.u-i-r-p")}</p>
              <div class="p-4"><OhmsLaw /></div>
            </section>

            <section id="cw-bandwidth" class="scroll-mt-24 rounded-xl border bg-card">
              <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("tools.cw-required-bandwidth")}</h2>
              <p class="px-4 pt-3 text-xs text-muted-foreground">{move || t("tools.bn-b-k-b")}</p>
              <div class="p-4"><CwBandwidth /></div>
            </section>

            <section id="lc-resonance" class="scroll-mt-24 rounded-xl border bg-card">
              <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("tools.lc-resonant-frequency")}</h2>
              <p class="px-4 pt-3 text-xs text-muted-foreground">{move || t("tools.f-1-2-lc")}</p>
              <div class="p-4"><LcResonance /></div>
            </section>

            <section id="reactance" class="scroll-mt-24 rounded-xl border bg-card">
              <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("tools.capacitive-inductive-reactance")}</h2>
              <p class="px-4 pt-3 text-xs text-muted-foreground">{move || t("tools.xc-1-2-fc")}</p>
              <div class="p-4"><Reactance /></div>
            </section>

            <section id="antenna-length" class="scroll-mt-24 rounded-xl border bg-card">
              <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("tools.antenna-length")}</h2>
              <p class="px-4 pt-3 text-xs text-muted-foreground">{move || t("tools.half-wave-dipole-l")}</p>
              <div class="p-4"><AntennaLength /></div>
            </section>

            <section id="antenna-matcher" class="scroll-mt-24 rounded-xl border bg-card">
              <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("tools.antenna-matching-network")}</h2>
              <p class="px-4 pt-3 text-xs text-muted-foreground">{move || t("tools.an-l-network-matches")}</p>
              <div class="p-4"><AntennaMatcher /></div>
            </section>

            <section id="smith" class="scroll-mt-24 rounded-xl border bg-card">
              <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("tools.smith-chart")}</h2>
              <p class="px-4 pt-3 text-xs text-muted-foreground">{move || t("tools.enter-a-complex-impedance")}</p>
              <div class="p-4"><SmithChart /></div>
              <p class="px-4 pb-4 text-xs text-muted-foreground">
                {move || t("tools.for-drag-to-position")}
                <a href="/smith" class="ml-1 text-primary underline underline-offset-4">
                  {move || t("tools.smith-chart-and-matching")}
                </a>
              </p>
            </section>

            <section id="swr" class="scroll-mt-24 rounded-xl border bg-card">
              <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("tools.swr-reflection-coefficient")}</h2>
              <p class="px-4 pt-3 text-xs text-muted-foreground">{move || t("tools.swr-1-1-return")}</p>
              <div class="p-4"><SwrConverter /></div>
            </section>

            <section id="cascade-gain" class="scroll-mt-24 rounded-xl border bg-card">
              <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("tools.cascaded-gain")}</h2>
              <p class="px-4 pt-3 text-xs text-muted-foreground">{move || t("tools.stage-gains-in-db")}</p>
              <div class="p-4"><CascadeGain /></div>
            </section>

            <section id="resistor" class="scroll-mt-24 rounded-xl border bg-card">
              <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("tools.resistors-in-series-parallel")}</h2>
              <p class="px-4 pt-3 text-xs text-muted-foreground">{move || t("tools.series-r-r-parallel")}</p>
              <div class="p-4"><ResistorParallel /></div>
            </section>

            <section id="freq-units" class="scroll-mt-24 rounded-xl border bg-card">
              <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("tools.frequency-unit-conversion")}</h2>
              <p class="px-4 pt-3 text-xs text-muted-foreground">{move || t("tools.1-mhz-1000-khz")}</p>
              <div class="p-4"><FrequencyUnits /></div>
            </section>

            <section id="battery" class="scroll-mt-24 rounded-xl border bg-card">
              <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("tools.battery-runtime-estimate")}</h2>
              <p class="px-4 pt-3 text-xs text-muted-foreground">{move || t("tools.runtime-hours-capacity-mah")}</p>
              <div class="p-4"><BatteryRuntime /></div>
            </section>

            <section id="dbm-dbuv" class="scroll-mt-24 rounded-xl border bg-card">
              <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("tools.dbm-db-v")}</h2>
              <p class="px-4 pt-3 text-xs text-muted-foreground">{move || t("tools.at-50-db-v")}</p>
              <div class="p-4"><DbmDbuv /></div>
            </section>

            <section id="feedline-loss" class="scroll-mt-24 rounded-xl border bg-card">
              <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("tools.feedline-loss")}</h2>
              <p class="px-4 pt-3 text-xs text-muted-foreground">{move || t("tools.total-loss-loss-per")}</p>
              <div class="p-4"><FeedlineLoss /></div>
            </section>

            <section id="filter-design" class="scroll-mt-24 rounded-xl border bg-card">
              <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("tools.filter-design")}</h2>
              <p class="px-4 pt-3 text-xs text-muted-foreground">{move || t("tools.l-c-component-values")}</p>
              <div class="p-4"><FilterDesign /></div>
            </section>

            <section id="callsign-lookup" class="scroll-mt-24 rounded-xl border bg-card">
              <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("tools.callsign-lookup")}</h2>
              <p class="px-4 pt-3 text-xs text-muted-foreground">{move || t("tools.identifies-the-dxcc-entity")}</p>
              <div class="p-4"><CallsignLookup /></div>
            </section>

            <section id="distance-bearing" class="scroll-mt-24 rounded-xl border bg-card">
              <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("tools.distance-bearing")}</h2>
              <p class="px-4 pt-3 text-xs text-muted-foreground">{move || t("tools.enter-two-maidenhead-grids")}</p>
              <div class="p-4"><DistanceBearing /></div>
            </section>

            <section id="propagation-muf" class="scroll-mt-24 rounded-xl border bg-card">
              <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("tools.propagation-muf")}</h2>
              <p class="px-4 pt-3 text-xs text-muted-foreground">{move || t("tools.estimates-fof2-single-hop")}</p>
              <div class="p-4"><PropagationEstimator /></div>
            </section>

            <section id="doppler" class="scroll-mt-24 rounded-xl border bg-card">
              <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("tools.satellite-doppler")}</h2>
              <p class="px-4 pt-3 text-xs text-muted-foreground">{move || t("tools.estimates-the-doppler-shift")}</p>
              <div class="p-4"><DopplerCalculator /></div>
            </section>

            <section id="fspl" class="scroll-mt-24 rounded-xl border bg-card">
              <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("tools.free-space-path-loss")}</h2>
              <p class="px-4 pt-3 text-xs text-muted-foreground">{move || t("tools.computes-the-free-space")}</p>
              <div class="p-4"><FsplCalculator /></div>
            </section>

            <section id="eirp" class="scroll-mt-24 rounded-xl border bg-card">
              <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("tools.eirp-effective-radiated-power")}</h2>
              <p class="px-4 pt-3 text-xs text-muted-foreground">{move || t("tools.transmit-power-antenna-gain")}</p>
              <div class="p-4"><EirpCalculator /></div>
            </section>

            <section id="link-budget" class="scroll-mt-24 rounded-xl border bg-card">
              <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("tools.link-budget")}</h2>
              <p class="px-4 pt-3 text-xs text-muted-foreground">{move || t("tools.eirp-path-loss-receive")}</p>
              <div class="p-4"><LinkBudgetCalculator /></div>
            </section>

            <section id="receiver-sensitivity" class="scroll-mt-24 rounded-xl border bg-card">
              <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("tools.receiver-sensitivity")}</h2>
              <p class="px-4 pt-3 text-xs text-muted-foreground">{move || t("tools.estimates-sensitivity-from-the")}</p>
              <div class="p-4"><ReceiverSensitivity /></div>
            </section>

            <section id="noise-cascade" class="scroll-mt-24 rounded-xl border bg-card">
              <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("tools.noise-figure-cascade")}</h2>
              <p class="px-4 pt-3 text-xs text-muted-foreground">{move || t("tools.friis-formula-computes-the")}</p>
              <div class="p-4"><NoiseCascade /></div>
            </section>

            <section id="gain-conversion" class="scroll-mt-24 rounded-xl border bg-card">
              <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("tools.antenna-gain-conversion")}</h2>
              <p class="px-4 pt-3 text-xs text-muted-foreground">{move || t("tools.quick-dbi-dbd-conversion")}</p>
              <div class="p-4"><GainConversion /></div>
            </section>

            <section id="resistor-code" class="scroll-mt-24 rounded-xl border bg-card">
              <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("tools.resistor-colour-code")}</h2>
              <p class="px-4 pt-3 text-xs text-muted-foreground">{move || t("tools.4-band-resistor-first")}</p>
              <div class="p-4"><ResistorColorCode /></div>
            </section>

            <section id="contest-scorer" class="scroll-mt-24 rounded-xl border bg-card">
              <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("tools.contest-scoring")}</h2>
              <p class="px-4 pt-3 text-xs text-muted-foreground">{move || t("tools.quick-score-estimates-for")}</p>
              <div class="p-4"><ContestScorer /></div>
            </section>

            <section id="coil-yagi" class="scroll-mt-24 rounded-xl border bg-card">
              <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("tools.coil-yagi-calculator")}</h2>
              <p class="px-4 pt-3 text-xs text-muted-foreground">{move || t("tools.air-core-coil-inductance")}</p>
              <div class="p-4"><CoilYagiCalculator /></div>
            </section>

            <section id="attenuator" class="scroll-mt-24 rounded-xl border bg-card">
              <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("tools.attenuator")}</h2>
              <p class="px-4 pt-3 text-xs text-muted-foreground">{move || t("tools.symmetrical-t-resistive-attenuators-2")}</p>
              <div class="p-4"><AttenuatorCalculator /></div>
            </section>

            <section id="transformer" class="scroll-mt-24 rounded-xl border bg-card">
              <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("tools.transformer-impedance")}</h2>
              <p class="px-4 pt-3 text-xs text-muted-foreground">{move || t("tools.compute-transformer-turns-ratio")}</p>
              <div class="p-4"><TransformerCalculator /></div>
            </section>

            <section id="utc-clock" class="scroll-mt-24 rounded-xl border bg-card">
              <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("tools.utc-time-2")}</h2>
              <p class="px-4 pt-3 text-xs text-muted-foreground">{move || t("tools.live-utc-zulu-clock")}</p>
              <div class="p-4"><UtcClock /></div>
            </section>

            <section id="tx-line" class="scroll-mt-24 rounded-xl border bg-card">
              <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("tools.transmission-line-impedance")}</h2>
              <p class="px-4 pt-3 text-xs text-muted-foreground">{move || t("tools.characteristic-impedance-of-coax")}</p>
              <div class="p-4"><TxLine /></div>
            </section>

            <section id="wire-gauge" class="scroll-mt-24 rounded-xl border bg-card">
              <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("tools.wire-gauge-and-voltage")}</h2>
              <p class="px-4 pt-3 text-xs text-muted-foreground">{move || t("tools.estimate-dc-supply-wire")}</p>
              <div class="p-4"><WireGauge /></div>
            </section>

            <section id="oscillator" class="scroll-mt-24 rounded-xl border bg-card">
              <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("tools.crystal-oscillator")}</h2>
              <p class="px-4 pt-3 text-xs text-muted-foreground">{move || t("tools.crystal-frequency-pulling-load")}</p>
              <div class="p-4"><Oscillator /></div>
            </section>

            <section id="rf-exposure" class="scroll-mt-24 rounded-xl border bg-card">
              <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("tools.rf-exposure-evaluation")}</h2>
              <p class="px-4 pt-3 text-xs text-muted-foreground">{move || t("tools.estimate-power-density-and")}</p>
              <div class="p-4"><RfExposure /></div>
            </section>

            <section id="tone-squelch" class="scroll-mt-24 rounded-xl border bg-card">
              <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("tools.tone-squelch-and-repeater")}</h2>
              <p class="px-4 pt-3 text-xs text-muted-foreground">{move || t("tools.repeater-offset-calculation-plus")}</p>
              <div class="p-4"><ToneSquelch /></div>
            </section>

            <section id="aprs-codec" class="scroll-mt-24 rounded-xl border bg-card">
              <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("tools.aprs-codec")}</h2>
              <p class="px-4 pt-3 text-xs text-muted-foreground">{move || t("tools.encode-and-decode-uncompressed")}</p>
              <div class="p-4"><AprsCodec /></div>
            </section>

            <section id="mode-encoder" class="scroll-mt-24 rounded-xl border bg-card">
              <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("tools.digital-mode-encoding")}</h2>
              <p class="px-4 pt-3 text-xs text-muted-foreground">{move || t("tools.text-to-morse-cw")}</p>
              <div class="p-4"><ModeEncoder /></div>
            </section>

            <section id="backup" class="scroll-mt-24 rounded-xl border bg-card">
              <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("tools.data-backup")}</h2>
              <p class="px-4 pt-3 text-xs text-muted-foreground">{move || t("settings.export-all-local-data")}</p>
              <div class="p-4"><BackupTool /></div>
            </section>
          </div>
        </div>
      </div>
    </div>
  }
}
