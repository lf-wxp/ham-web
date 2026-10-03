//! 结果视图：把灰度 / 假彩色图像渲染为 PNG data URL 并展示，支持下载与模式切换。

use leptos::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, ImageData};

use crate::i18n::{t, tf};
use crate::icons::{Icon, IconKind};
use crate::util::{document, js_error_message};

use super::channel_view::ChannelView;

/// 渲染结果：两通道灰度 + 假彩色合成（PNG data URL）+ 元信息。
#[derive(Clone)]
pub(super) struct AptRender {
  pub width: u32,
  pub height: u32,
  pub lines: usize,
  pub source_sample_rate: u32,
  pub duration_seconds: f64,
  pub channel_a_url: String,
  pub channel_b_url: String,
  pub false_color_url: String,
}

impl AptRender {
  /// 从解码出的像素（两通道灰度 + 假彩色）渲染为 PNG data URL。
  #[allow(clippy::too_many_arguments)]
  pub fn new(
    width: u32,
    height: u32,
    lines: u32,
    source_sample_rate: u32,
    duration_seconds: f64,
    channel_a: Vec<u8>,
    channel_b: Vec<u8>,
    false_color: Vec<u8>,
  ) -> Result<Self, String> {
    Ok(Self {
      width,
      height,
      lines: lines as usize,
      source_sample_rate,
      duration_seconds,
      channel_a_url: render_channel(&channel_a, width, height)?,
      channel_b_url: render_channel(&channel_b, width, height)?,
      false_color_url: render_rgba(&false_color, width, height)?,
    })
  }
}

/// 灰度像素 → 离屏 canvas → PNG data URL。
fn render_channel(pixels: &[u8], width: u32, height: u32) -> Result<String, String> {
  let mut rgba = vec![0u8; pixels.len() * 4];
  for (i, &g) in pixels.iter().enumerate() {
    let o = i * 4;
    rgba[o] = g;
    rgba[o + 1] = g;
    rgba[o + 2] = g;
    rgba[o + 3] = 255;
  }
  render_rgba(&rgba, width, height)
}

/// RGBA 像素 → 离屏 canvas → PNG data URL。
fn render_rgba(rgba: &[u8], width: u32, height: u32) -> Result<String, String> {
  let canvas: HtmlCanvasElement = document()
    .create_element("canvas")
    .map_err(|e| js_error_message(&e))?
    .unchecked_into();
  canvas.set_width(width);
  canvas.set_height(height);
  let ctx: CanvasRenderingContext2d = canvas
    .get_context("2d")
    .map_err(|e| js_error_message(&e))?
    .ok_or_else(|| t("Canvas 2D 上下文不可用"))?
    .unchecked_into();

  let clamped = wasm_bindgen::Clamped(rgba);
  let img = ImageData::new_with_u8_clamped_array_and_sh(clamped, width, height)
    .map_err(|e| js_error_message(&e))?;
  ctx
    .put_image_data(&img, 0.0, 0.0)
    .map_err(|e| js_error_message(&e))?;
  canvas.to_data_url().map_err(|e| js_error_message(&e))
}

/// 显示模式。
#[derive(Clone, Copy, PartialEq, Eq)]
enum ViewMode {
  /// 通道 A / B 灰度。
  Gray,
  /// 假彩色合成。
  FalseColor,
}

#[component]
pub(super) fn AptResultView(
  render: AptRender,
  file_name: String,
  on_reset: Callback<()>,
) -> impl IntoView {
  let mode = RwSignal::new(ViewMode::Gray);
  let AptRender {
    width,
    height,
    lines,
    source_sample_rate,
    duration_seconds,
    channel_a_url,
    channel_b_url,
    false_color_url,
  } = render;

  let tab_class = move |m: ViewMode| {
    if mode.get() == m {
      "rounded-md bg-primary px-2.5 py-1 text-xs font-medium text-primary-foreground"
    } else {
      "rounded-md px-2.5 py-1 text-xs font-medium text-muted-foreground hover:bg-accent"
    }
  };

  view! {
    <div class="space-y-4">
      <div class="flex flex-wrap gap-x-1.5 gap-y-1 rounded-lg bg-muted/40 px-3 py-2 text-xs text-muted-foreground">
        <span class="max-w-[16rem] truncate font-medium text-foreground">{file_name}</span>
        <span>"·"</span>
        <span>{format!("{width} × {height}")}</span>
        <span>"·"</span>
        <span>{tf("{} 行", &[&(lines).to_string()])}</span>
        <span>"·"</span>
        <span>{format!("{source_sample_rate} Hz")}</span>
        <span>"·"</span>
        <span>{format!("{duration_seconds:.1} s")}</span>
      </div>

      <div class="flex items-center gap-1.5">
        <button
          type="button"
          class=move || tab_class(ViewMode::Gray)
          on:click=move |_| mode.set(ViewMode::Gray)
        >
          {move || t("灰度（A / B）")}
        </button>
        <button
          type="button"
          class=move || tab_class(ViewMode::FalseColor)
          on:click=move |_| mode.set(ViewMode::FalseColor)
        >
          {move || t("假彩色（IR 增强）")}
        </button>
      </div>

      {move || {
        if mode.get() == ViewMode::Gray {
          view! {
            <div class="grid gap-5 md:grid-cols-2">
              <ChannelView label=t("通道 A") url=channel_a_url.clone() download_name="apt-channel-a.png" />
              <ChannelView label=t("通道 B") url=channel_b_url.clone() download_name="apt-channel-b.png" />
            </div>
          }
          .into_any()
        } else {
          view! {
            <ChannelView label=t("假彩色") url=false_color_url.clone() download_name="apt-false-color.png" />
          }
          .into_any()
        }
      }}

      <div class="flex justify-center">
        <button
          type="button"
          class="inline-flex items-center gap-2 rounded-md bg-primary px-4 py-2 text-sm font-medium text-primary-foreground transition-colors hover:bg-primary/90"
          on:click=move |_| on_reset.run(())
        >
          <Icon kind=IconKind::RefreshCw class="h-4 w-4" />
          {move || t("解码其他文件")}
        </button>
      </div>
    </div>
  }
}
