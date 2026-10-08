//! QSL 卡片设计器：选版式 → 从日志自动填入 → 预览 / 导出 PNG / 打印（完全本地完成）。
//!
//! 版式（文字放哪、多大、什么语气）由 `ham_web_core::qsl_card` 算，这里只负责三件事：
//! 把语义语气翻成颜色、把内容画到 canvas、把 canvas 交给下载 / 打印。
//!
//! 打印走浏览器自己的「打印 → 另存为 PDF」：A4 居中放一张 **140×89mm 标准卡**
//! （3.5 × 5.5 英寸），不自己拼 PDF 字节流 —— 中文排版与纸张由浏览器负责，
//! 自己拼要多几百 KB 依赖还容易印歪。

use ham_web_core::qsl_card::{self, Align, CardContent, CardRect, CardTemplate, RectKind, Tone};
use leptos::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};

use crate::components::common::{PageContainer, PageHeader};
use crate::i18n::t;
use crate::pages::log::use_log_store;
use crate::share_score::download;
use crate::ui::{
  Button, Field, Input, NativeSelect, RadioGroup, RadioGroupItem, SelectOption, Size, Variant,
};
use crate::util::{
  cancel_debounce, debounce, document, js_error_message, set_title, storage, window,
};

/// 预览 / 导出的画布尺寸：140×89mm 的 10 倍 ≈ 254 DPI。
const CARD_PX: (f64, f64) = (1400.0, 890.0);

/// 记住上次选的版式。
const TEMPLATE_KEY: &str = "qsl-card-template";

/// 打印时覆盖全局页边距：卡片自己带留白，纸张留 0 才能精确居中。
const PRINT_CSS: &str = "@media print { @page { size: A4; margin: 0; } }";

/// 版式主色。卡片是打印物，用固定的纸面配色，不跟站点明暗主题走。
fn accent(template: CardTemplate) -> &'static str {
  match template {
    CardTemplate::Classic => "#0f766e",
    CardTemplate::Minimal => "#3f3f46",
    CardTemplate::Bold => "#0e7490",
  }
}

/// 语气 → 墨色。
fn ink(tone: Tone, template: CardTemplate) -> &'static str {
  match tone {
    Tone::Strong => "#18181b",
    Tone::Muted => "#52525b",
    Tone::Accent => accent(template),
    Tone::Inverse => "#ffffff",
  }
}

/// 版式名称（`t()` 的字面量写在调用点，静态扫描才认得出来）。
fn template_label(template: CardTemplate) -> String {
  match template {
    CardTemplate::Classic => t("log.card-template-classic"),
    CardTemplate::Minimal => t("log.card-template-minimal"),
    CardTemplate::Bold => t("log.card-template-bold"),
  }
}

/// 画一块装饰。
fn draw_decor(ctx: &CanvasRenderingContext2d, r: &CardRect, template: CardTemplate, scale: f64) {
  match r.kind {
    RectKind::TopBand => {
      ctx.set_fill_style_str(accent(template));
      ctx.fill_rect(r.x, r.y, r.w, r.h);
    }
    RectKind::InfoBand => {
      ctx.set_fill_style_str("#f4f4f5");
      ctx.fill_rect(r.x, r.y, r.w, r.h);
    }
    RectKind::Frame => {
      ctx.set_stroke_style_str("#18181b");
      ctx.set_line_width((scale * 2.0).max(1.0));
      ctx.stroke_rect(r.x, r.y, r.w, r.h);
    }
    RectKind::Rule => {
      ctx.set_stroke_style_str("#d4d4d8");
      ctx.set_line_width(r.h.max(1.0));
      ctx.begin_path();
      ctx.move_to(r.x, r.y);
      ctx.line_to(r.x + r.w, r.y);
      ctx.stroke();
    }
  }
}

/// 字体串。`sans-serif` 交给系统挑：中文与呼号都要能印出来。
fn font(size: f64, bold: bool, italic: bool) -> String {
  format!(
    "{}{}{size}px sans-serif",
    if italic { "italic " } else { "" },
    if bold { "bold " } else { "" },
  )
}

thread_local! {
  /// 预览/导出共用的离屏画布（`(canvas, 2d 上下文)`）。
  ///
  /// 每次重画都新建一块 1400×890 的 canvas 会让浏览器反复分配后台缓冲；复用同一块
  /// 只需清屏重画。全局只有一块，容量有界。
  static CARD_CANVAS: std::cell::RefCell<
    Option<(HtmlCanvasElement, CanvasRenderingContext2d)>,
  > = const { std::cell::RefCell::new(None) };
}

/// 取（首次调用时创建）共用的画布与 2D 上下文。
fn card_canvas() -> Result<(HtmlCanvasElement, CanvasRenderingContext2d), String> {
  if let Some(pair) = CARD_CANVAS.with(|c| c.borrow().clone()) {
    return Ok(pair);
  }
  let canvas: HtmlCanvasElement = document()
    .create_element("canvas")
    .map_err(|e| js_error_message(&e))?
    .unchecked_into();
  canvas.set_width(CARD_PX.0 as u32);
  canvas.set_height(CARD_PX.1 as u32);
  let ctx: CanvasRenderingContext2d = canvas
    .get_context("2d")
    .map_err(|e| js_error_message(&e))?
    .ok_or("Canvas 2D context unavailable")?
    .unchecked_into();
  CARD_CANVAS.with(|c| *c.borrow_mut() = Some((canvas.clone(), ctx.clone())));
  Ok((canvas, ctx))
}

/// 把卡片画到 canvas，返回 PNG data URL。
fn render(template: CardTemplate, content: &CardContent) -> Result<String, String> {
  let (canvas, ctx) = card_canvas()?;
  let (w, h) = CARD_PX;
  // 给 `width`/`height` 赋值会清空画布并重分配后台缓冲，值没变就别赋。
  if canvas.width() != w as u32 || canvas.height() != h as u32 {
    canvas.set_width(w as u32);
    canvas.set_height(h as u32);
  }

  // 纸底 + 装饰（先铺装饰，反白文字才压得上去）。
  ctx.set_fill_style_str("#ffffff");
  ctx.fill_rect(0.0, 0.0, w, h);
  for r in qsl_card::decor(template, CARD_PX) {
    draw_decor(&ctx, &r, template, w / 400.0);
  }

  for item in qsl_card::layout(template, CARD_PX, content) {
    // 空字段整段跳过：位置是固定的，没填就不该留下空行。
    if item.text.trim().is_empty() {
      continue;
    }
    ctx.set_fill_style_str(ink(item.tone, template));
    ctx.set_font(&font(item.size, item.bold, item.italic));
    ctx.set_text_align(match item.align {
      Align::Left => "left",
      Align::Center => "center",
      Align::Right => "right",
    });
    ctx
      .fill_text(&item.text, item.x, item.y)
      .map_err(|e| js_error_message(&e))?;
  }

  canvas.to_data_url().map_err(|e| js_error_message(&e))
}

/// 字段集合（全部是字符串信号，允许中途为空）。
#[derive(Clone, Copy)]
struct Fields {
  my_call: RwSignal<String>,
  my_grid: RwSignal<String>,
  operator: RwSignal<String>,
  my_qth: RwSignal<String>,
  to_call: RwSignal<String>,
  to_grid: RwSignal<String>,
  date: RwSignal<String>,
  time: RwSignal<String>,
  freq: RwSignal<String>,
  mode: RwSignal<String>,
  rst: RwSignal<String>,
}

impl Fields {
  fn new() -> Self {
    let s = || RwSignal::new(String::new());
    Self {
      my_call: s(),
      my_grid: s(),
      operator: s(),
      my_qth: s(),
      to_call: s(),
      to_grid: s(),
      date: s(),
      time: s(),
      freq: s(),
      mode: s(),
      rst: s(),
    }
  }

  /// 当前内容（读全部字段，天然被 `Effect` 跟踪）。
  fn content(&self) -> CardContent {
    CardContent {
      my_call: self.my_call.get(),
      my_grid: self.my_grid.get(),
      operator: self.operator.get(),
      my_qth: self.my_qth.get(),
      to_call: self.to_call.get(),
      to_grid: self.to_grid.get(),
      date: self.date.get(),
      time: self.time.get(),
      freq: self.freq.get(),
      mode: self.mode.get(),
      rst: self.rst.get(),
    }
  }
}

#[component]
pub fn QslDesignerPage() -> impl IntoView {
  set_title("shell.qsl-card-design");
  let store = use_log_store();
  let f = Fields::new();
  let template = RwSignal::new(
    storage::get(TEMPLATE_KEY).map_or(CardTemplate::Classic, |id| CardTemplate::from_id(&id)),
  );
  let preview = RwSignal::new(String::new());
  let picked = RwSignal::new(String::new());

  // 本台信息只带一次（空着才填）：之后用户手改的不会被覆盖。
  let station = store.station.get_untracked();
  if !station.callsign.trim().is_empty() {
    f.my_call.set(station.callsign.clone());
  }
  if !station.gridsquare.trim().is_empty() {
    f.my_grid.set(station.gridsquare.clone());
  }
  if !station.operator.trim().is_empty() {
    f.operator.set(station.operator.clone());
  }

  // 版式或字段变化 → 重画预览。
  //
  // 11 个字段都是逐字符输入的，而一次重画要写满 1400×890 像素再做 PNG 编码 ——
  // 逐键重画会让输入明显发涩，所以防抖到「停止输入 150 ms 后」再画一次。
  Effect::new(move |_| {
    let t = template.get();
    let content = f.content();
    debounce("qsl-preview", 150, move || {
      if let Ok(data_url) = render(t, &content) {
        preview.set(data_url);
      }
    });
  });
  // 定时器回调会在离开页面后触发，那时信号已释放 —— 必须撤销。
  on_cleanup(|| cancel_debounce("qsl-preview"));

  // 从日志挑一条通联：呼号 / 网格 / 日期 / 时间 / 频率 / 模式 / RST 一次填好。
  let pick = Callback::new(move |id: String| {
    picked.set(id.clone());
    let Some(e) = store
      .logbook
      .with_untracked(|lb| lb.entries.iter().find(|e| e.id.to_string() == id).cloned())
    else {
      return;
    };
    f.to_call.set(e.callsign.clone());
    f.to_grid.set(e.gridsquare.clone());
    f.date.set(e.date.clone());
    f.time.set(e.time.clone());
    f.freq.set(e.freq.clone());
    f.mode.set(e.mode.clone());
    // 卡片上印「我发给对方」的报告；只有收到的就先借用一下，别留空。
    f.rst.set(if e.rst_sent.trim().is_empty() {
      e.rst_rcvd.clone()
    } else {
      e.rst_sent.clone()
    });
  });

  let export = move || match render(template.get(), &f.content()) {
    Ok(data_url) => {
      // 文件名里的呼号来自用户输入：只保留字母 / 数字 / `-` / `_`，避免 `/`、`..`、
      // 控制字符混进 `download` 属性（浏览器大多会清洗，但不该指望它）。
      let call: String = f
        .to_call
        .get()
        .trim()
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
        .collect();
      let name = if call.is_empty() {
        "qsl-card.png".to_owned()
      } else {
        format!("qsl-card-{call}.png")
      };
      download(&data_url, &name);
    }
    Err(_) => crate::util::alert(&t("log.rendering-failed-please-retry")),
  };

  let field = |label: &'static str, signal: RwSignal<String>, placeholder: &'static str| {
    let id = crate::util::unique_id("qsl-designer-field");
    let label_for = id.clone();
    view! {
      <Field label=Signal::derive(move || t(label)) r#for=label_for>
        <Input
          id=id
          placeholder=Signal::derive(move || t(placeholder))
          value=signal
          on_change=Callback::new(move |v: String| signal.set(v))
        />
      </Field>
    }
  };

  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <style>{PRINT_CSS}</style>
      // 屏幕上的一切都不打印；打印时只留下面那张 A4 卡片。
      <div class="print-hide">
        <PageHeader
          title=Signal::derive(move || t("shell.qsl-card-design"))
          subtitle=Signal::derive(move || t("log.pick-a-template-then"))
        />
        <PageContainer>
          <div class="grid gap-4 lg:grid-cols-2">
            // 表单
            <section class="rounded-xl border bg-card p-4">
              <h2 class="mb-3 text-sm font-semibold">{move || t("log.card-template")}</h2>
              <RadioGroup
                value=Signal::derive(move || template.get().id().to_owned())
                on_change=Callback::new(move |v: String| {
                  if let Some(tpl) = CardTemplate::ALL.into_iter().find(|tpl| tpl.id() == v) {
                    storage::set(TEMPLATE_KEY, tpl.id());
                    template.set(tpl);
                  }
                })
                aria_label=Signal::derive(move || t("log.card-template"))
                class="mb-4 flex flex-wrap items-center gap-4"
              >
                {CardTemplate::ALL
                  .into_iter()
                  .map(|tpl| {
                    view! {
                      <label class="flex cursor-pointer items-center gap-2 text-sm">
                        <RadioGroupItem value=tpl.id().to_owned() />
                        {move || template_label(tpl)}
                      </label>
                    }
                  })
                  .collect_view()}
              </RadioGroup>

              {move || {
                let options: Vec<SelectOption> = store
                  .logbook
                  .with(|lb| {
                    lb.entries
                      .iter()
                      .rev()
                      .take(50)
                      .map(|e| {
                        SelectOption::new(
                          e.id.to_string(),
                          format!(
                            "{} · {} {} · {}",
                            e.callsign,
                            e.date,
                            e.time,
                            e.band_label()
                          ),
                        )
                      })
                      .collect()
                  });
                if options.is_empty() {
                  return view! {
                    <p class="mb-4 text-xs text-muted-foreground">
                      {move || t("log.no-records-yet-add")}
                    </p>
                  }
                    .into_any();
                }
                view! {
                  <div class="mb-4 flex flex-col gap-1.5">
                    <span class="text-xs text-muted-foreground">
                      {move || t("log.fill-qso-from-the")}
                    </span>
                    <NativeSelect
                      value=Signal::derive(move || picked.get())
                      on_change=pick
                      options=options
                      placeholder=Signal::derive(move || t("log.pick-a-qso"))
                      aria_label=Signal::derive(move || t("log.fill-qso-from-the"))
                      class="w-full"
                    />
                  </div>
                }
                  .into_any()
              }}

              <h2 class="mb-3 text-sm font-semibold">{move || t("log.card-info")}</h2>
              <div class="space-y-3">
                <div class="grid grid-cols-2 gap-3">
                  {field("本台呼号", f.my_call, "BG4XXX")}
                  {field("本台网格", f.my_grid, "OM89")}
                </div>
                <div class="grid grid-cols-2 gap-3">
                  {field("操作员", f.operator, "OP")}
                  {field("本台 QTH", f.my_qth, "城市")}
                </div>
                <div class="grid grid-cols-2 gap-3">
                  {field("对方呼号", f.to_call, "JA1XXX")}
                  {field("log.their-grid", f.to_grid, "PM95")}
                </div>
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

            // 预览 + 导出 / 打印
            <section class="rounded-xl border bg-card p-4">
              <h2 class="mb-3 text-sm font-semibold">{move || t("log.preview")}</h2>
              <div class="overflow-hidden rounded-lg border bg-muted/30 p-2">
                <img
                  src=move || preview.get()
                  alt=t("log.qsl-card-preview")
                  class="w-full"
                  style="aspect-ratio: 140/89"
                />
              </div>
              <div class="mt-3 flex flex-wrap gap-2">
                <Button
                  variant=Variant::Default
                  size=Size::Default
                  class="flex-1"
                  on_click=Callback::new(move |_| export())
                >
                  {move || t("log.export-png")}
                </Button>
                <Button
                  variant=Variant::Outline
                  size=Size::Default
                  class="flex-1"
                  on_click=Callback::new(move |_| {
                                    let _ = window().print();
                                  })
                >
                  // 「打印」这个词与 QSL 标签页共用一条文案：中文释义全库唯一，
                  // 反向索引（中文原文 → key）不允许两个 key 用同一个词。
                  {move || t("learning.print")}
                </Button>
              </div>
              <p class="mt-2 text-xs text-muted-foreground">
                {move || t("log.print-at-140")
                }
              </p>
            </section>
          </div>
        </PageContainer>
      </div>

      // 打印页：A4 正中一张标准卡（140×89mm）。高度留 1mm 余量，免得溢出到第二页。
      <div class="print-sheet hidden print:block">
        <div
          class="mx-auto flex items-center justify-center"
          style="width: 210mm; height: 296mm;"
        >
          <img
            src=move || preview.get()
            alt=t("log.qsl-card-preview")
            style="width: 140mm; height: 89mm;"
          />
        </div>
      </div>
    </div>
  }
}
