//! QSL 卡片设计器：填写本台与 QSO 信息，实时预览并导出 PNG（完全本地完成）。

use leptos::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};

use crate::components::common::{PageContainer, PageHeader};
use crate::i18n::t;
use crate::share_score::download;
use crate::ui::{Size, Variant, button_class, input_class};
use crate::util::{document, js_error_message, set_title};

const W: f64 = 700.0;
const H: f64 = 500.0;

fn fill(ctx: &CanvasRenderingContext2d, text: &str, x: f64, y: f64) -> Result<(), String> {
  ctx.fill_text(text, x, y).map_err(|e| js_error_message(&e))
}

/// 字段集合。
#[derive(Clone, Copy)]
struct Fields {
  my_call: RwSignal<String>,
  my_grid: RwSignal<String>,
  operator: RwSignal<String>,
  my_qth: RwSignal<String>,
  to_call: RwSignal<String>,
  date: RwSignal<String>,
  time: RwSignal<String>,
  freq: RwSignal<String>,
  mode: RwSignal<String>,
  rst: RwSignal<String>,
}

impl Fields {
  fn new() -> Self {
    Self {
      my_call: RwSignal::new(String::new()),
      my_grid: RwSignal::new(String::new()),
      operator: RwSignal::new(String::new()),
      my_qth: RwSignal::new(String::new()),
      to_call: RwSignal::new(String::new()),
      date: RwSignal::new(String::new()),
      time: RwSignal::new(String::new()),
      freq: RwSignal::new(String::new()),
      mode: RwSignal::new(String::new()),
      rst: RwSignal::new(String::new()),
    }
  }

  /// 读取全部字段（用于在 Effect 中 track 与渲染）。
  fn snapshot(
    &self,
  ) -> (
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
  ) {
    (
      self.my_call.get(),
      self.my_grid.get(),
      self.operator.get(),
      self.my_qth.get(),
      self.to_call.get(),
      self.date.get(),
      self.time.get(),
      self.freq.get(),
      self.mode.get(),
      self.rst.get(),
    )
  }
}

/// 绘制 QSL 卡片到 Canvas，返回 PNG data URL。
fn render_qsl_card(
  fields: &(
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
  ),
) -> Result<String, String> {
  let (my_call, my_grid, operator, my_qth, to_call, date, time, freq, mode, rst) = fields;

  let canvas: HtmlCanvasElement = document()
    .create_element("canvas")
    .map_err(|e| js_error_message(&e))?
    .unchecked_into();
  canvas.set_width(W as u32);
  canvas.set_height(H as u32);
  let ctx: CanvasRenderingContext2d = canvas
    .get_context("2d")
    .map_err(|e| js_error_message(&e))?
    .ok_or("Canvas 2D context unavailable")?
    .unchecked_into();

  // 背景 + 边框。
  ctx.set_fill_style_str("#ffffff");
  ctx.fill_rect(0.0, 0.0, W, H);
  ctx.set_stroke_style_str("#18181b");
  ctx.set_line_width(2.0);
  ctx.stroke_rect(24.0, 24.0, W - 48.0, H - 48.0);

  // 本台呼号（大字）。
  ctx.set_fill_style_str("#18181b");
  ctx.set_text_align("left");
  ctx.set_font("bold 46px sans-serif");
  fill(&ctx, my_call, 56.0, 96.0)?;

  // 网格 + QTH。
  ctx.set_fill_style_str("#52525b");
  ctx.set_font("17px sans-serif");
  let loc = if my_qth.is_empty() {
    my_grid.clone()
  } else if my_grid.is_empty() {
    my_qth.clone()
  } else {
    format!("{my_grid} · {my_qth}")
  };
  fill(&ctx, &loc, 56.0, 132.0)?;
  fill(&ctx, &format!("OP: {operator}"), 56.0, 158.0)?;

  // 分割线。
  ctx.set_stroke_style_str("#e4e4e7");
  ctx.set_line_width(1.0);
  ctx.begin_path();
  ctx.move_to(56.0, 186.0);
  ctx.line_to(W - 56.0, 186.0);
  ctx.stroke();

  // Confirming QSO with。
  ctx.set_text_align("center");
  ctx.set_fill_style_str("#71717a");
  ctx.set_font("20px sans-serif");
  fill(&ctx, "Confirming QSO with", W / 2.0, 236.0)?;
  ctx.set_fill_style_str("#18181b");
  ctx.set_font("bold 40px sans-serif");
  fill(&ctx, to_call, W / 2.0, 292.0)?;

  // 信息行：日期 / 时间 / 频率 / 模式 / RST。
  let labels = ["DATE", "TIME", "FREQ", "MODE", "RST"];
  let values = [
    date.as_str(),
    time.as_str(),
    freq.as_str(),
    mode.as_str(),
    rst.as_str(),
  ];
  let x0 = 90.0;
  let step = (W - 180.0) / 4.0;
  for (i, (label, value)) in labels.iter().zip(values.iter()).enumerate() {
    let x = x0 + i as f64 * step;
    ctx.set_fill_style_str("#52525b");
    ctx.set_font("15px sans-serif");
    fill(&ctx, label, x, 348.0)?;
    ctx.set_fill_style_str("#18181b");
    ctx.set_font("bold 20px sans-serif");
    fill(&ctx, value, x, 378.0)?;
  }

  // 73。
  ctx.set_fill_style_str("#18181b");
  ctx.set_font("italic bold 28px sans-serif");
  fill(&ctx, "73 · TNX QSO", W / 2.0, 440.0)?;

  canvas.to_data_url().map_err(|e| js_error_message(&e))
}

#[component]
pub fn QslDesignerPage() -> impl IntoView {
  set_title(&t("QSL 卡片设计"));
  let f = Fields::new();
  let preview = RwSignal::new(String::new());
  let exporting = RwSignal::new(false);

  // 字段变化时重新生成预览。
  Effect::new(move |_| {
    let snap = f.snapshot();
    if let Ok(data_url) = render_qsl_card(&snap) {
      preview.set(data_url);
    }
  });

  let export = move || {
    exporting.set(true);
    let snap = f.snapshot();
    match render_qsl_card(&snap) {
      Ok(data_url) => download(&data_url, "qsl-card.png"),
      Err(_) => crate::util::alert(&t("生成失败，请重试。")),
    }
    exporting.set(false);
  };

  let field = |label: &'static str, signal: RwSignal<String>, placeholder: &'static str| {
    view! {
      <label class="flex flex-col gap-1.5 text-sm">
        <span class="text-xs text-muted-foreground">{move || t(label)}</span>
        <input
          type="text"
          placeholder=move || t(placeholder)
          prop:value=move || signal.get()
          on:input=move |e| signal.set(event_target_value(&e))
          class=input_class("h-9")
        />
      </label>
    }
  };

  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <PageHeader
        title=Signal::derive(move || t("QSL 卡片设计"))
        subtitle=Signal::derive(move || t("填写本台与 QSO 信息 · 本地生成 PNG"))
      />
      <PageContainer>
        <div class="grid gap-4 lg:grid-cols-2">
          // 表单
          <section class="rounded-xl border bg-card p-4">
            <h2 class="mb-3 text-sm font-semibold">{move || t("卡片信息")}</h2>
            <div class="space-y-3">
              <div class="grid grid-cols-2 gap-3">
                {field("本台呼号", f.my_call, "BG4XXX")}
                {field("本台网格", f.my_grid, "OM89")}
              </div>
              <div class="grid grid-cols-2 gap-3">
                {field("操作员", f.operator, "OP")}
                {field("本台 QTH", f.my_qth, "城市")}
              </div>
              {field("对方呼号", f.to_call, "JA1XXX")}
              <div class="grid grid-cols-3 gap-3">
                {field("日期", f.date, "2026-10-01")}
                {field("时间", f.time, "12:34")}
                {field("频率", f.freq, "14.074")}
              </div>
              <div class="grid grid-cols-2 gap-3">
                {field("模式", f.mode, "FT8")}
                {field("RST", f.rst, "599")}
              </div>
            </div>
          </section>

          // 预览 + 导出
          <section class="rounded-xl border bg-card p-4">
            <h2 class="mb-3 text-sm font-semibold">{move || t("预览")}</h2>
            <div class="overflow-hidden rounded-lg border bg-muted/30 p-2">
              <img
                src=move || preview.get()
                alt=t("QSL 卡片预览")
                class="w-full"
                style="aspect-ratio: 7/5"
              />
            </div>
            <button
              type="button"
              class=button_class(Variant::Default, Size::Default, "mt-3 w-full")
              on:click=move |_| export()
            >
              {move || if exporting.get() { t("生成中…") } else { t("导出 PNG") }}
            </button>
          </section>
        </div>
      </PageContainer>
    </div>
  }
}
