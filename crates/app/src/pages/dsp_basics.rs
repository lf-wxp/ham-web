//! 数字信号处理基础速查页。

use ham_web_core::dsp_basics::{
  DSP_CONCEPTS, DSP_FORMULAS, DSP_STEPS, DSP_TIPS, FILTER_TYPES, WINDOW_TABLE,
};
use leptos::prelude::*;

use crate::components::common::{
  BulletSection, ConceptsSection, KnowledgePage, StepsSection, TableSection,
};
use crate::i18n::t;
use crate::util::set_title;

#[component]
pub fn DspBasicsPage() -> impl IntoView {
  set_title(&t("数字信号处理基础"));
  view! {
    <KnowledgePage title=t("数字信号处理基础") subtitle=t("采样 / 量化 / FFT / 滤波与 SDR 处理链")>
      <ConceptsSection title="核心概念" items=DSP_CONCEPTS />
      <TableSection
        title="采样与量化"
        headers=&["名称", "公式 / 取值", "说明"]
        rows=DSP_FORMULAS
        min_width=720
      />
      <TableSection
        title="常用窗函数"
        headers=&["窗", "主瓣宽", "旁瓣 / 适用"]
        rows=WINDOW_TABLE
        min_width=720
      />
      <TableSection
        title="滤波器类型"
        headers=&["类型", "特点", "说明"]
        rows=FILTER_TYPES
        min_width=720
      />
      <StepsSection title="典型 SDR 接收链" items=DSP_STEPS />
      <BulletSection title="要点" items=DSP_TIPS />
    </KnowledgePage>
  }
}
