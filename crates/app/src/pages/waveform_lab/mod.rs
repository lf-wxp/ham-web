//! 波形实验室：调制波形与频谱、星座图与眼图、滤波器响应曲线、香农容量。
//!
//! 四块互不依赖的交互面板，全部由 `ham_web_core::waveform_lab` 的纯函数驱动，
//! 页面只负责画 SVG 与收集参数，因此离线可用、结果可复现。

mod filter_section;
mod plot;
mod shannon_section;
mod symbol_section;
mod wave_section;

use leptos::prelude::*;

use crate::components::common::{PageContainer, PageHeader};
use crate::i18n::t;
use crate::util::set_title;

use filter_section::FilterSection;
use shannon_section::ShannonSection;
use symbol_section::SymbolSection;
use wave_section::WaveSection;

/// 结果条。
pub(super) const RESULT: &str = "rounded-lg bg-muted/40 px-3 py-2 text-sm text-muted-foreground";
/// 说明文字。
pub(super) const NOTE: &str = "text-xs text-muted-foreground";
/// 绘图区外层容器。
pub(super) const PLOT: &str = "mx-auto w-full";

/// 按量级挑选小数位，避免 `0.30000000000000004` 这类浮点尾数。
pub(super) fn fmt_num(v: f64) -> String {
  if v.abs() >= 1000.0 {
    format!("{v:.0}")
  } else if v.abs() >= 100.0 {
    format!("{v:.1}")
  } else if v.abs() >= 1.0 {
    format!("{v:.2}")
  } else if v.abs() >= 0.001 {
    format!("{v:.4}")
  } else {
    format!("{v:.2e}")
  }
}

#[component]
pub fn WaveformLabPage() -> impl IntoView {
  set_title("tools.waveform-lab");
  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <PageHeader
        title=move || t("tools.waveform-lab")
        subtitle=move || t("tools.modulated-waveforms-and-spectra")
      />

      <PageContainer>
        <WaveSection />
        <SymbolSection />
        <FilterSection />
        <ShannonSection />
      </PageContainer>
    </div>
  }
}
