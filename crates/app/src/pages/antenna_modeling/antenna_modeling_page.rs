use ham_web_core::antenna_modeling::{MODELING_SOFTWARE, MODELING_STEPS, MODELING_TIPS};
use leptos::prelude::*;

use crate::util::set_title;

use super::dipole_calculator::DipoleCalculator;
use super::pattern_plot::PatternPlot;
use super::vertical_calculator::VerticalCalculator;
use super::yagi_calculator::YagiCalculator;
use crate::i18n::t;

const CELL: &str = "border px-3 py-2 text-left align-top";

#[component]
pub fn AntennaModelingPage() -> impl IntoView {
  set_title(&t("天线建模软件"));
  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <header class="sticky top-0 z-20 border-b bg-background/90 backdrop-blur">
        <div class="mx-auto flex max-w-5xl flex-wrap items-center gap-3 px-4 py-3">
          <div class="mr-auto">
            <h1 class="text-base font-semibold leading-tight">{move || t("天线建模软件")}</h1>
            <div class="text-xs text-muted-foreground">"EZNEC · MMANA-GAL · 4NEC2"</div>
          </div>
        </div>
      </header>

      <div class="mx-auto max-w-5xl space-y-6 px-4 py-5">
        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("方向图可视化")}</h2>
          <p class="px-4 pt-3 text-xs text-muted-foreground">
            {move || t("极坐标图展示归一化辐射方向。方位角图：偶极子 E 面呈「8」字形（最大辐射垂直于振子）、H 面水平全向，三单元 Yagi 前向主瓣、后瓣显著被抑制，方形 / 三角环水平面近似全向（方向性很弱）。仰角图：垂直天线低仰角（适合 DX）、水平环高仰角（适合 NVIS 近距通信）；选「偶极子（架高可调）」可拖动架高滑块，观察主瓣随架高变化——低架朝上、高架压低仰角。")}
          </p>
          <div class="p-4"><PatternPlot /></div>
        </section>

        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("偶极天线尺寸估算")}</h2>
          <p class="px-4 pt-3 text-xs text-muted-foreground">
            {move || t("先估算尺寸再建模：半波偶极总长 ≈ 150/f × k（单臂为总长一半），建议架高约半波长。")}
          </p>
          <div class="p-4"><DipoleCalculator /></div>
        </section>

        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("Yagi 天线尺寸")}</h2>
          <p class="px-4 pt-3 text-xs text-muted-foreground">
            {move || t("反射器比半波长略长、激励振子略短、引向器逐个更短；单元越多主瓣越尖锐、后瓣越小。")}
          </p>
          <div class="p-4"><YagiCalculator /></div>
        </section>

        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("垂直天线尺寸")}</h2>
          <p class="px-4 pt-3 text-xs text-muted-foreground">
            {move || t("1/4λ 垂直 + 地网：辐射体约 1/4λ（乘缩短系数），地网略长，主瓣在低仰角、适合 DX。")}
          </p>
          <div class="p-4"><VerticalCalculator /></div>
        </section>

        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("关联计算工具")}</h2>
          <div class="flex flex-wrap gap-2 p-4">
            {[
              ("/tools#antenna-length", "天线长度"),
              ("/tools#coil-yagi", "线圈 / Yagi 振子"),
              ("/tools#swr", "驻波比换算"),
              ("/tools#feedline-loss", "馈线损耗"),
              ("/tools#gain-conversion", "增益换算"),
            ]
              .iter()
              .map(|(href, label)| {
                view! {
                  <a
                    href=*href
                    class="rounded-full border bg-muted/40 px-3 py-1.5 text-xs text-muted-foreground transition-colors hover:bg-accent hover:text-accent-foreground"
                  >
                    {move || t(label)}
                  </a>
                }
              })
              .collect_view()}
          </div>
        </section>

        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("常用软件")}</h2>
          <div class="overflow-x-auto">
            <table class="w-full min-w-[520px] border-collapse text-sm">
              <thead class="bg-muted/60 text-xs">
                <tr>
                  <th class=CELL>{move || t("软件")}</th>
                  <th class=CELL>{move || t("类型")}</th>
                  <th class=CELL>{move || t("说明")}</th>
                </tr>
              </thead>
              <tbody>
                {MODELING_SOFTWARE
                  .iter()
                  .map(|&(name, kind, desc)| {
                    view! {
                      <tr class="border-t transition-colors hover:bg-muted/40">
                        <td class=format!("{CELL} whitespace-nowrap font-mono font-semibold text-primary")>
                          {name}
                        </td>
                        <td class=format!("{CELL} whitespace-nowrap text-muted-foreground")>{kind}</td>
                        <td class=format!("{CELL} text-muted-foreground")>{desc}</td>
                      </tr>
                    }
                  })
                  .collect_view()}
              </tbody>
            </table>
          </div>
        </section>

        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("建模流程")}</h2>
          <div class="grid gap-1 p-4 sm:grid-cols-2">
            {MODELING_STEPS
              .iter()
              .map(|&(t, d)| {
                view! {
                  <div class="flex flex-col gap-1 rounded-lg px-3 py-2">
                    <span class="text-sm font-medium">{t}</span>
                    <span class="text-sm text-muted-foreground">{d}</span>
                  </div>
                }
              })
              .collect_view()}
          </div>
        </section>

        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("使用要点")}</h2>
          <ul class="space-y-2 p-4">
            {MODELING_TIPS
              .iter()
              .map(|tip| {
                view! {
                  <li class="flex gap-2 text-sm text-muted-foreground">
                    <span class="mt-0.5 shrink-0 text-primary">"•"</span>
                    <span>{*tip}</span>
                  </li>
                }
              })
              .collect_view()}
          </ul>
        </section>
      </div>
    </div>
  }
}
