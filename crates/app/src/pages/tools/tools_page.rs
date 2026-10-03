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
  set_title(&t("小工具"));
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
            <h1 class="text-base font-semibold leading-tight">{move || t("小工具")}</h1>
            <div class="text-xs text-muted-foreground">{move || t("频率波长 · 功率 · 增益 · 电路计算")}</div>
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
            <nav aria-label=move || t("工具目录") class="sticky top-20 rounded-xl border bg-card p-3">
              <div class="mb-2 px-2 text-xs font-semibold text-muted-foreground">{move || t("工具导航")}</div>
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
              <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("频率 ↔ 波长")}</h2>
              <p class="px-4 pt-3 text-xs text-muted-foreground">{move || t("λ = 300 / f（MHz），真空/空气中近似。")}</p>
              <div class="p-4"><FreqWavelength /></div>
            </section>

            <section id="dbm-power" class="scroll-mt-24 rounded-xl border bg-card">
              <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("dBm ↔ 功率")}</h2>
              <p class="px-4 pt-3 text-xs text-muted-foreground">{move || t("0 dBm = 1 mW，30 dBm = 1 W。")}</p>
              <div class="p-4"><DbmConverter /></div>
            </section>

            <section id="decibel-gain" class="scroll-mt-24 rounded-xl border bg-card">
              <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("分贝增益")}</h2>
              <p class="px-4 pt-3 text-xs text-muted-foreground">{move || t("功率比 dB = 10·lg(P₁/P₀)，电压比 dB = 20·lg(V₁/V₀)。")}</p>
              <div class="p-4"><DecibelGain /></div>
            </section>

            <section id="ohms-law" class="scroll-mt-24 rounded-xl border bg-card">
              <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("欧姆定律 / 电功率")}</h2>
              <p class="px-4 pt-3 text-xs text-muted-foreground">{move || t("U = I·R，P = U·I = I²R = U²/R。")}</p>
              <div class="p-4"><OhmsLaw /></div>
            </section>

            <section id="cw-bandwidth" class="scroll-mt-24 rounded-xl border bg-card">
              <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("CW 必要带宽")}</h2>
              <p class="px-4 pt-3 text-xs text-muted-foreground">{move || t("Bn = B × K，B = WPM / 1.2，K 取 5（衰落）或 3（非衰落）。")}</p>
              <div class="p-4"><CwBandwidth /></div>
            </section>

            <section id="lc-resonance" class="scroll-mt-24 rounded-xl border bg-card">
              <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("LC 谐振频率")}</h2>
              <p class="px-4 pt-3 text-xs text-muted-foreground">{move || t("f = 1 / (2π√(LC))，用 μH、pF、MHz 单位。")}</p>
              <div class="p-4"><LcResonance /></div>
            </section>

            <section id="reactance" class="scroll-mt-24 rounded-xl border bg-card">
              <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("容抗 / 感抗")}</h2>
              <p class="px-4 pt-3 text-xs text-muted-foreground">{move || t("Xc = 1/(2πfC)，XL = 2πfL，用 MHz、pF、μH 单位。")}</p>
              <div class="p-4"><Reactance /></div>
            </section>

            <section id="antenna-length" class="scroll-mt-24 rounded-xl border bg-card">
              <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("天线长度")}</h2>
              <p class="px-4 pt-3 text-xs text-muted-foreground">{move || t("半波偶极 L = 143/f，1/4 波长 = 71.5/f，5/8 波长 = 187.5/f（米，f 为 MHz）。")}</p>
              <div class="p-4"><AntennaLength /></div>
            </section>

            <section id="antenna-matcher" class="scroll-mt-24 rounded-xl border bg-card">
              <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("天线匹配网络")}</h2>
              <p class="px-4 pt-3 text-xs text-muted-foreground">{move || t("L 型网络把负载电阻匹配到 50Ω，计算电感 / 电容元件值。")}</p>
              <div class="p-4"><AntennaMatcher /></div>
            </section>

            <section id="smith" class="scroll-mt-24 rounded-xl border bg-card">
              <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("史密斯圆图")}</h2>
              <p class="px-4 pt-3 text-xs text-muted-foreground">{move || t("输入复阻抗（R ± jX），在圆图上定位并显示驻波比 / 反射系数 / 回波损耗。")}</p>
              <div class="p-4"><SmithChart /></div>
            </section>

            <section id="swr" class="scroll-mt-24 rounded-xl border bg-card">
              <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("驻波比 / 反射系数")}</h2>
              <p class="px-4 pt-3 text-xs text-muted-foreground">{move || t("SWR = (1+|Γ|)/(1-|Γ|)，回波损耗 = -20·lg(|Γ|)。")}</p>
              <div class="p-4"><SwrConverter /></div>
            </section>

            <section id="cascade-gain" class="scroll-mt-24 rounded-xl border bg-card">
              <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("级联增益")}</h2>
              <p class="px-4 pt-3 text-xs text-muted-foreground">{move || t("各级增益 dB 直接相加。")}</p>
              <div class="p-4"><CascadeGain /></div>
            </section>

            <section id="resistor" class="scroll-mt-24 rounded-xl border bg-card">
              <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("电阻串并联")}</h2>
              <p class="px-4 pt-3 text-xs text-muted-foreground">{move || t("串联 R = ΣR，并联 1/R = Σ(1/R)。")}</p>
              <div class="p-4"><ResistorParallel /></div>
            </section>

            <section id="freq-units" class="scroll-mt-24 rounded-xl border bg-card">
              <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("频率单位换算")}</h2>
              <p class="px-4 pt-3 text-xs text-muted-foreground">{move || t("1 MHz = 1000 kHz = 1000000 Hz。")}</p>
              <div class="p-4"><FrequencyUnits /></div>
            </section>

            <section id="battery" class="scroll-mt-24 rounded-xl border bg-card">
              <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("电池续航估算")}</h2>
              <p class="px-4 pt-3 text-xs text-muted-foreground">{move || t("续航（小时）= 容量（mAh）÷ 电流（mA）。")}</p>
              <div class="p-4"><BatteryRuntime /></div>
            </section>

            <section id="dbm-dbuv" class="scroll-mt-24 rounded-xl border bg-card">
              <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("dBm ↔ dBμV")}</h2>
              <p class="px-4 pt-3 text-xs text-muted-foreground">{move || t("50Ω 阻抗下：dBμV = dBm + 107。")}</p>
              <div class="p-4"><DbmDbuv /></div>
            </section>

            <section id="feedline-loss" class="scroll-mt-24 rounded-xl border bg-card">
              <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("馈线损耗")}</h2>
              <p class="px-4 pt-3 text-xs text-muted-foreground">{move || t("总损耗 = 每百米损耗 × 长度 / 100；功率损耗 = 1 − 10^(−dB/10)。")}</p>
              <div class="p-4"><FeedlineLoss /></div>
            </section>

            <section id="filter-design" class="scroll-mt-24 rounded-xl border bg-card">
              <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("滤波器设计")}</h2>
              <p class="px-4 pt-3 text-xs text-muted-foreground">{move || t("Butterworth 低通 / 高通 / 带通 / 带阻的 L/C 元件值。")}</p>
              <div class="p-4"><FilterDesign /></div>
            </section>

            <section id="callsign-lookup" class="scroll-mt-24 rounded-xl border bg-card">
              <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("呼号查询")}</h2>
              <p class="px-4 pt-3 text-xs text-muted-foreground">{move || t("按呼号前缀识别 DXCC 实体与是否稀有实体。")}</p>
              <div class="p-4"><CallsignLookup /></div>
            </section>

            <section id="distance-bearing" class="scroll-mt-24 rounded-xl border bg-card">
              <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("两点距离 / 方位角")}</h2>
              <p class="px-4 pt-3 text-xs text-muted-foreground">{move || t("输入两个 Maidenhead 网格，计算大圆距离与初始方位角。")}</p>
              <div class="p-4"><DistanceBearing /></div>
            </section>

            <section id="propagation-muf" class="scroll-mt-24 rounded-xl border bg-card">
              <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("传播预测 MUF")}</h2>
              <p class="px-4 pt-3 text-xs text-muted-foreground">{move || t("由太阳通量 SFI 估算 foF2 / 单跳 MUF / OWF，并标注可用波段。")}</p>
              <div class="p-4"><PropagationEstimator /></div>
            </section>

            <section id="doppler" class="scroll-mt-24 rounded-xl border bg-card">
              <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("卫星多普勒")}</h2>
              <p class="px-4 pt-3 text-xs text-muted-foreground">{move || t("按频率与相对径向速度估算卫星通信中的多普勒频移。")}</p>
              <div class="p-4"><DopplerCalculator /></div>
            </section>

            <section id="fspl" class="scroll-mt-24 rounded-xl border bg-card">
              <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("自由空间路径损耗")}</h2>
              <p class="px-4 pt-3 text-xs text-muted-foreground">{move || t("按距离与频率计算电磁波在自由空间传播的损耗。")}</p>
              <div class="p-4"><FsplCalculator /></div>
            </section>

            <section id="eirp" class="scroll-mt-24 rounded-xl border bg-card">
              <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("EIRP 有效辐射功率")}</h2>
              <p class="px-4 pt-3 text-xs text-muted-foreground">{move || t("发射功率 + 天线增益 − 馈线损耗。")}</p>
              <div class="p-4"><EirpCalculator /></div>
            </section>

            <section id="link-budget" class="scroll-mt-24 rounded-xl border bg-card">
              <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("链路预算")}</h2>
              <p class="px-4 pt-3 text-xs text-muted-foreground">{move || t("EIRP − 路径损耗 + 接收增益 → 到达功率。")}</p>
              <div class="p-4"><LinkBudgetCalculator /></div>
            </section>

            <section id="receiver-sensitivity" class="scroll-mt-24 rounded-xl border bg-card">
              <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("接收机灵敏度")}</h2>
              <p class="px-4 pt-3 text-xs text-muted-foreground">{move || t("由噪声底线 + 带宽 + 噪声系数估算灵敏度。")}</p>
              <div class="p-4"><ReceiverSensitivity /></div>
            </section>

            <section id="noise-cascade" class="scroll-mt-24 rounded-xl border bg-card">
              <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("噪声系数级联")}</h2>
              <p class="px-4 pt-3 text-xs text-muted-foreground">{move || t("Friis 公式计算三级接收链的总噪声系数。")}</p>
              <div class="p-4"><NoiseCascade /></div>
            </section>

            <section id="gain-conversion" class="scroll-mt-24 rounded-xl border bg-card">
              <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("天线增益换算")}</h2>
              <p class="px-4 pt-3 text-xs text-muted-foreground">{move || t("dBi ↔ dBd 快速换算（相差 2.15 dB）。")}</p>
              <div class="p-4"><GainConversion /></div>
            </section>

            <section id="resistor-code" class="scroll-mt-24 rounded-xl border bg-card">
              <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("电阻色环")}</h2>
              <p class="px-4 pt-3 text-xs text-muted-foreground">{move || t("4 色环电阻：前两环数字 + 乘数 + 容差。")}</p>
              <div class="p-4"><ResistorColorCode /></div>
            </section>

            <section id="contest-scorer" class="scroll-mt-24 rounded-xl border bg-card">
              <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("竞赛记分")}</h2>
              <p class="px-4 pt-3 text-xs text-muted-foreground">{move || t("CQ WPX / CQ WW / ARRL DX 的分数快速计算。")}</p>
              <div class="p-4"><ContestScorer /></div>
            </section>

            <section id="coil-yagi" class="scroll-mt-24 rounded-xl border bg-card">
              <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("线圈 / Yagi 计算")}</h2>
              <p class="px-4 pt-3 text-xs text-muted-foreground">{move || t("空心线圈电感与 3 单元 Yagi 振子长度估算。")}</p>
              <div class="p-4"><CoilYagiCalculator /></div>
            </section>

            <section id="attenuator" class="scroll-mt-24 rounded-xl border bg-card">
              <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("衰减器")}</h2>
              <p class="px-4 pt-3 text-xs text-muted-foreground">{move || t("对称 T 型 / π 型电阻衰减器，按指定衰减量与特性阻抗计算元件值。")}</p>
              <div class="p-4"><AttenuatorCalculator /></div>
            </section>

            <section id="transformer" class="scroll-mt-24 rounded-xl border bg-card">
              <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("变压器阻抗")}</h2>
              <p class="px-4 pt-3 text-xs text-muted-foreground">{move || t("按初级 / 次级阻抗计算变压器匝数比与阻抗比。")}</p>
              <div class="p-4"><TransformerCalculator /></div>
            </section>

            <section id="utc-clock" class="scroll-mt-24 rounded-xl border bg-card">
              <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("UTC 时间")}</h2>
              <p class="px-4 pt-3 text-xs text-muted-foreground">{move || t("实时 UTC / Zulu 时钟与常用时区对照。")}</p>
              <div class="p-4"><UtcClock /></div>
            </section>

            <section id="tx-line" class="scroll-mt-24 rounded-xl border bg-card">
              <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("传输线阻抗")}</h2>
              <p class="px-4 pt-3 text-xs text-muted-foreground">{move || t("同轴 / 平行双线 / 微带线的特性阻抗。")}</p>
              <div class="p-4"><TxLine /></div>
            </section>

            <section id="wire-gauge" class="scroll-mt-24 rounded-xl border bg-card">
              <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("线径压降")}</h2>
              <p class="px-4 pt-3 text-xs text-muted-foreground">{move || t("按电流与长度估算直流供电线径与回路压降。")}</p>
              <div class="p-4"><WireGauge /></div>
            </section>

            <section id="oscillator" class="scroll-mt-24 rounded-xl border bg-card">
              <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("晶体振荡")}</h2>
              <p class="px-4 pt-3 text-xs text-muted-foreground">{move || t("晶体频率牵引（负载电容 → ppm）与串联谐振。")}</p>
              <div class="p-4"><Oscillator /></div>
            </section>

            <section id="rf-exposure" class="scroll-mt-24 rounded-xl border bg-card">
              <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("射频暴露评估")}</h2>
              <p class="px-4 pt-3 text-xs text-muted-foreground">{move || t("按 FCC OET-65 估算功率密度与最小安全距离。")}</p>
              <div class="p-4"><RfExposure /></div>
            </section>

            <section id="tone-squelch" class="scroll-mt-24 rounded-xl border bg-card">
              <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("亚音与中继频差")}</h2>
              <p class="px-4 pt-3 text-xs text-muted-foreground">{move || t("中继频差计算与 CTCSS / DCS 亚音码表。")}</p>
              <div class="p-4"><ToneSquelch /></div>
            </section>

            <section id="aprs-codec" class="scroll-mt-24 rounded-xl border bg-card">
              <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("APRS 编解码")}</h2>
              <p class="px-4 pt-3 text-xs text-muted-foreground">{move || t("APRS 未压缩位置报文的编解码。")}</p>
              <div class="p-4"><AprsCodec /></div>
            </section>

            <section id="mode-encoder" class="scroll-mt-24 rounded-xl border bg-card">
              <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("数字模式编码")}</h2>
              <p class="px-4 pt-3 text-xs text-muted-foreground">{move || t("文本 → 摩尔斯（CW）与 RTTY（ITA2）比特流。")}</p>
              <div class="p-4"><ModeEncoder /></div>
            </section>

            <section id="backup" class="scroll-mt-24 rounded-xl border bg-card">
              <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("数据备份")}</h2>
              <p class="px-4 pt-3 text-xs text-muted-foreground">{move || t("导出全部本地数据（进度、错题、收藏、日志等）为 JSON；可在其他设备恢复，或合并导入多台设备的数据。")}</p>
              <div class="p-4"><BackupTool /></div>
            </section>
          </div>
        </div>
      </div>
    </div>
  }
}
