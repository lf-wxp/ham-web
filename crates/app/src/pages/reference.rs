//! 考试速查页：操作证权限、分区号、RST 信号报告、发射类别标识、通联英语短句。

use ham_web_core::reference::{
  CALL_AREAS, EMISSION_DESIGNATION, EMISSION_TYPES, KNOWLEDGE_SOURCES, LICENSE_CLASSES, PHRASES,
  RST_SCALES, call_area_color,
};
use leptos::prelude::*;

use crate::components::call_area_map::CallAreaMap;
use crate::components::common::{PageContainer, PageHeader, TableSection};
use crate::data;
use crate::data::kt_tracked;
use crate::i18n::{t, tf};
use crate::ui::{Chip, ChipGroup, Table, TableBody, TableCell, TableHead, TableHeader, TableRow};
use crate::util::set_title;

#[component]
pub fn ReferencePage() -> impl IntoView {
  set_title("shell.exam-reference-title");
  // 分区号与省名是知识库正文（`crates/core` 的数据表），译文按模块维护；本页不是
  // `KnowledgePage`，译文要自己挂一次，否则英文 / 西语界面里这一节仍是中文。
  data::use_knowledge_i18n();
  // 地图与色块共用同一个选区：`None` 表示「不分分区，全部同色」。
  let selected = RwSignal::new(None::<String>);
  // 点已选中的分区（或「全部」传空串）→ 取消选择。
  let toggle_area = Callback::new(move |area: String| {
    selected.update(|cur| {
      *cur = if area.is_empty() || cur.as_deref() == Some(area.as_str()) {
        None
      } else {
        Some(area.clone())
      };
    });
  });
  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-(--motion-normal) ease-out">
      <PageHeader title=move || t("shell.exam-reference-title") subtitle=move || t("exam.licence-privileges-zones-rst") />
      <PageContainer>
        // 操作证类别与权限
        <section class="pxl-card border bg-card">
          <h2 class="pxl-section-title">{move || t("exam.operator-licence-classes-and")}</h2>
          <Table
            aria_label=Signal::derive(move || t("exam.operator-licence-classes-and"))
            class="min-w-[560px]"
          >
            <TableHead>
              <TableRow>
                <TableHeader>{move || t("exam.class-column")}</TableHeader>
                <TableHeader>{move || t("exam.frequency-range")}</TableHeader>
                <TableHeader>{move || t("exam.power-limit")}</TableHeader>
                <TableHeader>{move || t("radio.notes")}</TableHeader>
              </TableRow>
            </TableHead>
            <TableBody>
              {LICENSE_CLASSES
                .iter()
                .map(|c| {
                  view! {
                    <TableRow>
                      <TableCell class="align-top font-mono">
                        {move || tf("learning.class", &[c.class])}
                      </TableCell>
                      <TableCell class="align-top">{move || kt_tracked(c.freq)}</TableCell>
                      <TableCell class="align-top">{move || kt_tracked(c.power)}</TableCell>
                      <TableCell class="align-top text-muted-foreground">{move || kt_tracked(c.note)}</TableCell>
                    </TableRow>
                  }
                })
                .collect_view()}
            </TableBody>
          </Table>
        </section>

        // 分区号：地图 + 色块图例 + 分区号表
        <section class="pxl-card border bg-card">
          <h2 class="pxl-section-title">{move || t("exam.amateur-radio-zone-numbers")}</h2>
          <p class="px-4 pt-3 text-xs leading-5 text-muted-foreground">
            {move || t("exam.call-area-map-hint")}
          </p>
          <div class="px-4 py-3">
            <CallAreaMap selected=selected on_select=toggle_area />
          </div>
          <ChipGroup
            value=Signal::derive(move || selected.get().unwrap_or_default())
            on_change=toggle_area
            aria_label=Signal::derive(move || t("exam.call-area-map"))
            class="gap-1.5 border-t px-4 py-3"
          >
            {CALL_AREAS
              .iter()
              .map(|a| {
                let color = call_area_color(a.digit);
                let label = view! {
                  <span
                    class="inline-block h-3 w-3"
                    style=format!("background-color: {color}")
                  ></span>
                  <span class="font-mono">{a.digit}</span>
                }
                  .into_any();
                view! { <Chip value=a.digit.to_string()>{label}</Chip> }
              })
              .collect_view()}
            // 空串 = 清空选区（与 `toggle_area` 的约定一致），默认就停在「全部」。
            <Chip value=String::new()>{move || t("exam.all")}</Chip>
          </ChipGroup>
          <div class="grid grid-cols-2 gap-2 border-t p-4 sm:grid-cols-3 lg:grid-cols-5">
            {move || {
              let sel = selected.get();
              CALL_AREAS
                .iter()
                .map(|a| {
                  let is_sel = sel.as_deref() == Some(a.digit);
                  let cell = if is_sel {
                    "flex items-start gap-2 border border-gold bg-inset p-3"
                  } else {
                    "flex items-start gap-2 border bg-inset p-3"
                  };
                  let digit_class = if is_sel {
                    "font-mono text-lg text-gold-text"
                  } else {
                    "font-mono text-lg text-primary"
                  };
                  view! {
                    <div class=cell>
                      <span class=digit_class>{a.digit}</span>
                      <span class="text-xs leading-5 text-muted-foreground">
                        {move || data::kt_tracked(a.regions)}
                      </span>
                    </div>
                  }
                })
                .collect_view()
            }}
          </div>
        </section>

        // RST 信号报告
        <section class="pxl-card border bg-card">
          <h2 class="pxl-section-title">{move || t("shell.rst-report")}</h2>
          <div class="grid grid-cols-1 gap-3 p-4 md:grid-cols-3">
            {RST_SCALES
              .iter()
              .map(|s| {
                view! {
                  <div class="border">
                    <div class="border-b bg-inset px-3 py-2">
                      <span class="font-mono text-primary">{s.key}</span>
                      <span class="ml-2 text-sm">{move || kt_tracked(s.name)}</span>
                      <div class="text-xs text-muted-foreground">{move || kt_tracked(s.note)}</div>
                    </div>
                    <dl class="divide-y text-sm">
                      {s
                        .levels
                        .iter()
                        .map(|&(num, desc)| {
                          view! {
                            <div class="flex gap-2 px-3 py-1.5">
                              <dt class="font-mono tabular-nums">{num}</dt>
                              <dd class="text-muted-foreground">{move || kt_tracked(desc)}</dd>
                            </div>
                          }
                        })
                        .collect_view()}
                    </dl>
                  </div>
                }
              })
              .collect_view()}
          </div>
        </section>

        // 发射类别
        <section class="pxl-card border bg-card">
          <h2 class="pxl-section-title">{move || t("exam.emission-designators")}</h2>
          <Table
            aria_label=Signal::derive(move || t("exam.emission-designators"))
            class="min-w-[480px]"
          >
            <TableHead>
              <TableRow>
                <TableHeader>{move || t("exam.designator")}</TableHeader>
                <TableHeader>{move || t("exam.name")}</TableHeader>
                <TableHeader>{move || t("radio.notes")}</TableHeader>
              </TableRow>
            </TableHead>
            <TableBody>
              {EMISSION_TYPES
                .iter()
                .map(|e| {
                  view! {
                    <TableRow>
                      <TableCell class="align-top font-mono">{e.code}</TableCell>
                      <TableCell class="align-top">{move || kt_tracked(e.name)}</TableCell>
                      <TableCell class="align-top text-muted-foreground">{move || kt_tracked(e.desc)}</TableCell>
                    </TableRow>
                  }
                })
                .collect_view()}
            </TableBody>
          </Table>
        </section>

        // 发射类别的构成
        <TableSection
          title="发射类别标识的构成"
          headers=&["组成", "含义", "示例"]
          rows=EMISSION_DESIGNATION
          class="min-w-[720px]"
        />

        // 通联英语
        <section class="pxl-card border bg-card">
          <h2 class="pxl-section-title">{move || t("exam.contact-english-phrases")}</h2>
          <dl class="divide-y">
            {PHRASES
              .iter()
              .map(|p| {
                view! {
                  <div class="grid gap-1 px-4 py-3 sm:grid-cols-[7rem_1fr]">
                    <dt class="text-xs text-muted-foreground">{move || kt_tracked(p.usage)}</dt>
                    <dd>
                      <div class="font-mono text-sm text-foreground">{p.en}</div>
                      <div class="mt-0.5 text-sm text-muted-foreground">{move || kt_tracked(p.zh)}</div>
                    </dd>
                  </div>
                }
              })
              .collect_view()}
          </dl>
        </section>
        // 知识来源与时效
        <TableSection
          title="知识来源与时效"
          headers=&["领域", "来源", "说明"]
          rows=KNOWLEDGE_SOURCES
          class="min-w-[720px]"
        />
      </PageContainer>
    </div>
  }
}
