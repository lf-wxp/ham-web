//! 成绩分享卡片：把模拟考试成绩绘制到 Canvas，导出为 PNG 图片（完全本地完成，不上传）。

use ham_web_core::{Bank, ExamScore};
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};

use crate::i18n::{t, tf};
use crate::util::{document, js_error_message};

const W: f64 = 600.0;
const H: f64 = 400.0;

fn fill(ctx: &CanvasRenderingContext2d, text: &str, x: f64, y: f64) -> Result<(), String> {
  ctx.fill_text(text, x, y).map_err(|e| js_error_message(&e))
}

/// 生成成绩分享卡片，返回 PNG data URL。
pub fn render_score_card(bank: Bank, score: ExamScore, pass_line: usize) -> Result<String, String> {
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

  let passed = score.is_passed(pass_line);
  let accent = if passed { "#16a34a" } else { "#dc2626" };

  // 背景与顶部色条。
  ctx.set_fill_style_str("#ffffff");
  ctx.fill_rect(0.0, 0.0, W, H);
  ctx.set_fill_style_str(accent);
  ctx.fill_rect(0.0, 0.0, W, 8.0);

  ctx.set_text_align("center");

  // 标题。
  ctx.set_fill_style_str("#18181b");
  ctx.set_font("bold 30px sans-serif");
  fill(&ctx, &t("业余无线电模拟考试"), W / 2.0, 76.0)?;

  // 题库 + 合格线。
  ctx.set_fill_style_str("#71717a");
  ctx.set_font("18px sans-serif");
  fill(
    &ctx,
    &tf(
      "{} 类 · 合格线 {} 题",
      &[bank.as_str(), &pass_line.to_string()],
    ),
    W / 2.0,
    116.0,
  )?;

  // 得分大字。
  ctx.set_fill_style_str("#18181b");
  ctx.set_font("bold 76px sans-serif");
  fill(
    &ctx,
    &format!("{} / {}", score.correct, score.total),
    W / 2.0,
    222.0,
  )?;

  // 正确率 + 合格判定。
  ctx.set_fill_style_str(accent);
  ctx.set_font("bold 26px sans-serif");
  fill(
    &ctx,
    &tf(
      "正确率 {}% · {}",
      &[
        &score.percent().to_string(),
        &t(if passed { "合格" } else { "不合格" }),
      ],
    ),
    W / 2.0,
    266.0,
  )?;

  // 日期。
  let today = js_sys::Date::new_0();
  let date = tf(
    "{} 年 {} 月 {} 日",
    &[
      &today.get_full_year().to_string(),
      &(today.get_month() + 1).to_string(),
      &today.get_date().to_string(),
    ],
  );
  ctx.set_fill_style_str("#a1a1aa");
  ctx.set_font("16px sans-serif");
  fill(&ctx, &date, W / 2.0, 316.0)?;

  // 底部水印。
  ctx.set_fill_style_str("#d4d4d8");
  ctx.set_font("14px sans-serif");
  fill(&ctx, "ham.onlyxp.me", W / 2.0, 372.0)?;

  canvas.to_data_url().map_err(|e| js_error_message(&e))
}

/// 生成学习周报分享卡片，返回 PNG data URL。
pub fn render_weekly_card(
  answered: u32,
  new: u32,
  streak: usize,
  mistakes: usize,
  weakest: Option<&str>,
) -> Result<String, String> {
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

  // 背景与顶部色条。
  ctx.set_fill_style_str("#ffffff");
  ctx.fill_rect(0.0, 0.0, W, H);
  ctx.set_fill_style_str("#2563eb");
  ctx.fill_rect(0.0, 0.0, W, 8.0);

  ctx.set_text_align("center");

  // 标题与日期。
  ctx.set_fill_style_str("#18181b");
  ctx.set_font("bold 30px sans-serif");
  fill(&ctx, &t("学习周报"), W / 2.0, 70.0)?;
  let today = js_sys::Date::new_0();
  let date = tf(
    "{} 年 {} 月 {} 日",
    &[
      &today.get_full_year().to_string(),
      &(today.get_month() + 1).to_string(),
      &today.get_date().to_string(),
    ],
  );
  ctx.set_fill_style_str("#a1a1aa");
  ctx.set_font("16px sans-serif");
  fill(&ctx, &date, W / 2.0, 106.0)?;

  // 四个统计数字。
  let stats: [(String, String); 4] = [
    (t("本周作答"), answered.to_string()),
    (t("本周新题"), new.to_string()),
    (t("连续打卡"), tf("{} 天", &[&streak.to_string()])),
    (t("当前错题"), mistakes.to_string()),
  ];
  let xs = [W / 8.0, W * 3.0 / 8.0, W * 5.0 / 8.0, W * 7.0 / 8.0];
  for (i, (label, value)) in stats.iter().enumerate() {
    ctx.set_fill_style_str("#18181b");
    ctx.set_font("bold 32px sans-serif");
    fill(&ctx, value, xs[i], 172.0)?;
    ctx.set_fill_style_str("#71717a");
    ctx.set_font("14px sans-serif");
    fill(&ctx, label, xs[i], 206.0)?;
  }

  // 最薄弱分类（可选）。
  if let Some(w) = weakest {
    ctx.set_fill_style_str("#71717a");
    ctx.set_font("16px sans-serif");
    fill(&ctx, &t("本周最薄弱分类"), W / 2.0, 262.0)?;
    ctx.set_fill_style_str("#18181b");
    ctx.set_font("bold 22px sans-serif");
    fill(&ctx, w, W / 2.0, 294.0)?;
  }

  // 底部水印。
  ctx.set_fill_style_str("#d4d4d8");
  ctx.set_font("14px sans-serif");
  fill(&ctx, "ham.onlyxp.me", W / 2.0, 372.0)?;

  canvas.to_data_url().map_err(|e| js_error_message(&e))
}

/// 下载 data URL 图片。
pub fn download(data_url: &str, filename: &str) {
  let Ok(el) = document().create_element("a") else {
    return;
  };
  let a: web_sys::HtmlAnchorElement = el.unchecked_into();
  a.set_href(data_url);
  a.set_download(filename);
  if let Some(body) = document().body() {
    let _ = body.append_child(&a);
    a.click();
    let _ = body.remove_child(&a);
  }
}

/// 把 PNG data URL 转为 Blob（供剪贴板 / 系统分享使用）。
fn data_url_to_blob(data_url: &str) -> Option<web_sys::Blob> {
  let (header, b64) = data_url.split_once(',')?;
  let mime = header
    .strip_prefix("data:")
    .and_then(|h| h.split(';').next())
    .unwrap_or("image/png");
  let binary = crate::util::window().atob(b64).ok()?;
  let bytes = js_sys::Uint8Array::new_with_length(binary.len() as u32);
  for (i, b) in binary.bytes().enumerate() {
    bytes.set_index(i as u32, b);
  }
  let parts = js_sys::Array::new();
  parts.push(&bytes);
  let opts = web_sys::BlobPropertyBag::new();
  opts.set_type(mime);
  web_sys::Blob::new_with_u8_array_sequence_and_options(&parts, &opts).ok()
}

/// 把成绩卡片复制到剪贴板（桌面版 Chrome / Edge）。
pub async fn copy_image(data_url: &str) -> Result<(), String> {
  let blob = data_url_to_blob(data_url).ok_or_else(|| t("图片处理失败"))?;
  let record = js_sys::Object::new();
  let blob_promise = js_sys::Promise::resolve(&blob);
  js_sys::Reflect::set(&record, &JsValue::from_str("image/png"), &blob_promise)
    .map_err(|e| js_error_message(&e))?;
  let item = web_sys::ClipboardItem::new_with_record_from_str_to_blob_promise(&record)
    .map_err(|e| js_error_message(&e))?;
  let items = js_sys::Array::new();
  items.push(&item);
  let promise = crate::util::window()
    .navigator()
    .clipboard()
    .write(items.as_ref());
  wasm_bindgen_futures::JsFuture::from(promise)
    .await
    .map_err(|e| js_error_message(&e))?;
  Ok(())
}

/// 通过系统分享面板分享图片（移动端 Web Share API）。
pub async fn share_image(data_url: &str, filename: &str, title: &str) -> Result<(), String> {
  let blob = data_url_to_blob(data_url).ok_or_else(|| t("图片处理失败"))?;
  let parts = js_sys::Array::new();
  parts.push(&blob);
  let file =
    web_sys::File::new_with_blob_sequence(&parts, filename).map_err(|e| js_error_message(&e))?;
  let data = web_sys::ShareData::new();
  data.set_title(title);
  let files = js_sys::Array::new();
  files.push(&file);
  data.set_files(files.as_ref());
  let promise = crate::util::window().navigator().share_with_data(&data);
  wasm_bindgen_futures::JsFuture::from(promise)
    .await
    .map_err(|e| js_error_message(&e))?;
  Ok(())
}

/// 学习报告数据（用于生成年度 / 阶段报告卡片）。
#[derive(Clone, Copy)]
pub struct ReportData {
  pub answered: u32,
  pub correct_rate: u32,
  pub streak: usize,
  pub mistakes: usize,
  pub bookmarks: usize,
  pub achievements: usize,
  pub log_count: usize,
  pub dxcc_count: usize,
  /// 时间范围（中文原文，渲染时经 [`t`] 翻译）。
  pub range: &'static str,
}

/// 生成学习报告卡片，返回 PNG data URL。
pub fn render_report_card(d: &ReportData) -> Result<String, String> {
  let canvas: HtmlCanvasElement = document()
    .create_element("canvas")
    .map_err(|e| js_error_message(&e))?
    .unchecked_into();
  canvas.set_width(600);
  canvas.set_height(800);
  let ctx: CanvasRenderingContext2d = canvas
    .get_context("2d")
    .map_err(|e| js_error_message(&e))?
    .ok_or("Canvas 2D context unavailable")?
    .unchecked_into();

  // 背景与顶部色条。
  ctx.set_fill_style_str("#ffffff");
  ctx.fill_rect(0.0, 0.0, 600.0, 800.0);
  ctx.set_fill_style_str("#0f766e");
  ctx.fill_rect(0.0, 0.0, 600.0, 8.0);

  ctx.set_text_align("center");

  // 标题与时间范围。
  ctx.set_fill_style_str("#18181b");
  ctx.set_font("bold 36px sans-serif");
  fill(&ctx, &t("学习报告"), 300.0, 76.0)?;
  ctx.set_fill_style_str("#a1a1aa");
  ctx.set_font("16px sans-serif");
  fill(&ctx, &t(d.range), 300.0, 112.0)?;

  // 学习统计（2 列 × 3 行）。
  let stats: [(String, String); 6] = [
    (t("累计作答"), d.answered.to_string()),
    (t("正确率"), tf("{}%", &[&d.correct_rate.to_string()])),
    (t("连续打卡"), tf("{} 天", &[&d.streak.to_string()])),
    (t("当前错题"), d.mistakes.to_string()),
    (t("收藏题目"), d.bookmarks.to_string()),
    (t("解锁成就"), d.achievements.to_string()),
  ];
  let xs = [160.0, 440.0];
  let ys = [200.0, 300.0, 400.0];
  for (i, (label, value)) in stats.iter().enumerate() {
    let x = xs[i % 2];
    let y = ys[i / 2];
    ctx.set_fill_style_str("#18181b");
    ctx.set_font("bold 34px sans-serif");
    fill(&ctx, value, x, y)?;
    ctx.set_fill_style_str("#71717a");
    ctx.set_font("15px sans-serif");
    fill(&ctx, label, x, y + 30.0)?;
  }

  // 分割线。
  ctx.set_stroke_style_str("#e4e4e7");
  ctx.set_line_width(1.0);
  ctx.begin_path();
  ctx.move_to(60.0, 480.0);
  ctx.line_to(540.0, 480.0);
  ctx.stroke();

  // 通联统计。
  ctx.set_fill_style_str("#18181b");
  ctx.set_font("bold 22px sans-serif");
  fill(&ctx, &t("通联日志"), 300.0, 540.0)?;
  let log_stats: [(String, String); 2] = [
    (t("通联记录"), d.log_count.to_string()),
    (t("DXCC 实体"), d.dxcc_count.to_string()),
  ];
  for (i, (label, value)) in log_stats.iter().enumerate() {
    let x = 160.0 + i as f64 * 280.0;
    ctx.set_fill_style_str("#18181b");
    ctx.set_font("bold 30px sans-serif");
    fill(&ctx, value, x, 600.0)?;
    ctx.set_fill_style_str("#71717a");
    ctx.set_font("14px sans-serif");
    fill(&ctx, label, x, 630.0)?;
  }

  // 底部水印。
  ctx.set_fill_style_str("#d4d4d8");
  ctx.set_font("14px sans-serif");
  fill(&ctx, "ham.onlyxp.me", 300.0, 764.0)?;

  canvas.to_data_url().map_err(|e| js_error_message(&e))
}
