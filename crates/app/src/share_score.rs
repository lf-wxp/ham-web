//! 成绩分享卡片：把模拟考试成绩绘制到 Canvas，导出为 PNG 图片（完全本地完成，不上传）。

use ham_web_core::logbook::StationInfo;
use ham_web_core::rig_panel::{S_SEGMENTS, S_WEAK, S9, exam_s_units, s_reading};
use ham_web_core::{Bank, ExamScore};
use wasm_bindgen::prelude::Closure;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};

use crate::i18n::{t, tf, tp};
use crate::util::{document, js_error_message};

const W: f64 = 600.0;
const H: f64 = 400.0;

/// QSL 成绩卡的固定配色（Sweetie-16）：导出图两套皮肤共用一张，也不跟界面的配色方案走 ——
/// 它是要发出去的「卡片」，不是界面的一部分。
const QSL_STOCK: &str = "#f7f0dc";
const QSL_HEAD: &str = "#ece3c8";
const QSL_INK: &str = "#1a1c2c";
const QSL_MUTED: &str = "#566c86";
const QSL_RULE: &str = "#c9bfa3";
const QSL_BRAND: &str = "#3b5dc9";
const QSL_PASS: &str = "#257a45";
const QSL_FAIL: &str = "#b13e53";
/// 卡上的小屏幕与 S 表格子：同界面里暗色（背光）屏幕的四档（底噪 / 亮 / +dB / 偏弱）。导出图是固定配色，不随明暗变。
const QSL_SCREEN: &str = "#1a1c2c";
const QSL_SEG_OFF: &str = "#333c57";
const QSL_SEG_ON: &str = "#73eff7";
const QSL_SEG_PLUS: &str = "#a7f070";
const QSL_SEG_WEAK: &str = "#ffcd75";
const QSL_SEG_LABEL: &str = "#94b0c2";
/// 呼号与读数用等宽字：QSL 卡上的呼号本来就是这种「打字机」观感，数字也对得齐。
const QSL_MONO: &str = "ui-monospace, Menlo, Consolas, monospace";
/// 按 2 倍像素出图：分享到手机上看不糊。
const QSL_SCALE: f64 = 2.0;

fn fill(ctx: &CanvasRenderingContext2d, text: &str, x: f64, y: f64) -> Result<(), String> {
  ctx.fill_text(text, x, y).map_err(|e| js_error_message(&e))
}

/// 带最大宽度的 `fill_text`：长呼号与西语文案会被横向压缩而不是溢出卡片。
fn fill_max(
  ctx: &CanvasRenderingContext2d,
  text: &str,
  x: f64,
  y: f64,
  max_width: f64,
) -> Result<(), String> {
  ctx
    .fill_text_with_max_width(text, x, y, max_width)
    .map_err(|e| js_error_message(&e))
}

/// 本台呼号：日志里当前台站的呼号（`station-info` 镜像，RBN / PSK Reporter 读的也是它）。
pub(crate) fn own_callsign() -> Option<String> {
  crate::util::storage::get_json::<StationInfo>("station-info")
    .map(|s| s.callsign.trim().to_ascii_uppercase())
    .filter(|c| !c.is_empty())
}

/// 生成成绩分享卡片（一张 QSL 卡），返回 PNG data URL。
///
/// 卡上只写真的（`docs/ui/decisions.md` §11.6）：呼号取日志设置的本台呼号、没有就写「未设置呼号」；
/// 不编频率与模式，通联表里只有日期、题库、得分、正确率与信号报告（S9 = 合格线，`rig_panel::exam_s_units`）。
///
/// `show_call`：是否把本台呼号印在卡上。呼号能反查到个人信息，而这张图是要发出去的 ——
/// 所以调用方默认传 `false`，由用户在分享前明确勾选（没设呼号时这个参数无效）。
pub fn render_score_card(
  bank: Bank,
  score: ExamScore,
  pass_line: usize,
  show_call: bool,
) -> Result<String, String> {
  let canvas: HtmlCanvasElement = document()
    .create_element("canvas")
    .map_err(|e| js_error_message(&e))?
    .unchecked_into();
  canvas.set_width((W * QSL_SCALE) as u32);
  canvas.set_height((H * QSL_SCALE) as u32);
  let ctx: CanvasRenderingContext2d = canvas
    .get_context("2d")
    .map_err(|e| js_error_message(&e))?
    .ok_or("Canvas 2D context unavailable")?
    .unchecked_into();
  ctx
    .scale(QSL_SCALE, QSL_SCALE)
    .map_err(|e| js_error_message(&e))?;

  let passed = score.is_passed(pass_line);
  let accent = if passed { QSL_PASS } else { QSL_FAIL };
  let units = exam_s_units(score.correct, score.total, pass_line);
  let margin = 36.0;

  // 卡纸与内框。
  ctx.set_fill_style_str(QSL_STOCK);
  ctx.fill_rect(0.0, 0.0, W, H);
  ctx.set_stroke_style_str(QSL_INK);
  ctx.set_line_width(2.0);
  ctx.stroke_rect(12.0, 12.0, W - 24.0, H - 24.0);

  // 左上：本台呼号。
  ctx.set_text_align("left");
  match own_callsign() {
    Some(call) if show_call => {
      ctx.set_fill_style_str(QSL_INK);
      ctx.set_font(&format!("bold 52px {QSL_MONO}"));
      fill_max(&ctx, &call, margin, 92.0, 330.0)?;
    }
    other => {
      // 设了呼号但没勾选 = 「未公开」；压根没设 = 「未设置」，两句不能混。
      let note = if other.is_some() {
        t("exam.qsl-call-hidden")
      } else {
        t("exam.qsl-no-call")
      };
      ctx.set_fill_style_str(QSL_MUTED);
      ctx.set_font("24px sans-serif");
      fill_max(&ctx, &note, margin, 84.0, 330.0)?;
    }
  }
  ctx.set_fill_style_str(QSL_MUTED);
  ctx.set_font("14px sans-serif");
  fill_max(
    &ctx,
    &t("exam.amateur-radio-mock-exam"),
    margin,
    118.0,
    330.0,
  )?;

  // 右上：「QSL」字样与站点。
  ctx.set_text_align("right");
  ctx.set_fill_style_str(QSL_BRAND);
  ctx.set_font(&format!("bold 48px {QSL_MONO}"));
  fill(&ctx, "QSL", W - margin, 92.0)?;
  ctx.set_fill_style_str(QSL_MUTED);
  ctx.set_font(&format!("13px {QSL_MONO}"));
  fill(&ctx, "ham.onlyxp.me", W - margin, 118.0)?;

  ctx.set_fill_style_str(QSL_RULE);
  ctx.fill_rect(margin, 138.0, W - margin * 2.0, 2.0);

  // 确认语。
  ctx.set_text_align("left");
  ctx.set_fill_style_str(QSL_INK);
  ctx.set_font("17px sans-serif");
  fill_max(
    &ctx,
    &tf("exam.qsl-confirming", &[bank.as_str()]),
    margin,
    170.0,
    W - margin * 2.0,
  )?;

  // 通联表：表头一行、数据一行。
  let cells: [(String, String); 5] = [
    (t("log.date"), crate::util::local_today()),
    (t("exam.bank"), bank.as_str().to_owned()),
    (
      t("morse.score"),
      format!("{} / {}", score.correct, score.total),
    ),
    (t("learning.accuracy"), format!("{}%", score.percent())),
    (
      t("exam.signal-report"),
      units.map_or_else(|| "--".to_owned(), s_reading),
    ),
  ];
  let (table_y, head_h, row_h) = (188.0, 26.0, 44.0);
  let table_w = W - margin * 2.0;
  let col_w = table_w / cells.len() as f64;
  ctx.set_fill_style_str(QSL_HEAD);
  ctx.fill_rect(margin, table_y, table_w, head_h);
  ctx.set_fill_style_str(QSL_RULE);
  ctx.fill_rect(margin, table_y + head_h, table_w, 1.0);
  for i in 1..cells.len() {
    ctx.fill_rect(margin + col_w * i as f64, table_y, 1.0, head_h + row_h);
  }
  ctx.set_stroke_style_str(QSL_INK);
  ctx.stroke_rect(margin, table_y, table_w, head_h + row_h);
  ctx.set_text_align("center");
  for (i, (label, value)) in cells.iter().enumerate() {
    let cx = margin + col_w * (i as f64 + 0.5);
    ctx.set_fill_style_str(QSL_MUTED);
    ctx.set_font("12px sans-serif");
    fill_max(&ctx, label, cx, table_y + 18.0, col_w - 10.0)?;
    ctx.set_fill_style_str(QSL_INK);
    ctx.set_font(&format!("bold 20px {QSL_MONO}"));
    fill_max(&ctx, value, cx, table_y + head_h + 29.0, col_w - 10.0)?;
  }

  // 左下：一块小屏幕上的 S 表，读法与界面里的 `SignalMeter` 一致（S9 以上绿、S5 以下琥珀）。
  let (sx, sy, sw, sh) = (margin, 282.0, 300.0, 56.0);
  ctx.set_fill_style_str(QSL_SCREEN);
  ctx.fill_rect(sx, sy, sw, sh);
  let segs = f64::from(S_SEGMENTS);
  let gap = 4.0;
  let seg_w = ((sw - 20.0 - gap * (segs - 1.0)) / segs).floor();
  let left = sx + (sw - (seg_w * segs + gap * (segs - 1.0))) / 2.0;
  let lit = units.unwrap_or(0);
  ctx.set_font(&format!("11px {QSL_MONO}"));
  for k in 1..=S_SEGMENTS {
    let x = left + f64::from(k - 1) * (seg_w + gap);
    let color = if k > lit {
      QSL_SEG_OFF
    } else if k > S9 {
      QSL_SEG_PLUS
    } else if lit < S_WEAK {
      QSL_SEG_WEAK
    } else {
      QSL_SEG_ON
    };
    ctx.set_fill_style_str(color);
    ctx.fill_rect(x, sy + 10.0, seg_w, 14.0);
    // 刻度与界面同一套：奇数格标 1…9，S9 以上标 +20 / +40。
    let label = match k {
      1 | 3 | 5 | 7 | 9 => Some(k.to_string()),
      11 | 13 => Some(format!("+{}", u32::from(k - S9) * 10)),
      _ => None,
    };
    if let Some(label) = label {
      ctx.set_fill_style_str(if k > S9 { QSL_SEG_PLUS } else { QSL_SEG_LABEL });
      fill(&ctx, &label, x + seg_w / 2.0, sy + 44.0)?;
    }
  }

  // 右下：结论章，略微倾斜，像盖上去的。
  ctx.save();
  ctx
    .translate(462.0, 310.0)
    .map_err(|e| js_error_message(&e))?;
  ctx.rotate(-0.08).map_err(|e| js_error_message(&e))?;
  ctx.set_stroke_style_str(accent);
  ctx.set_line_width(3.0);
  ctx.stroke_rect(-92.0, -30.0, 184.0, 60.0);
  ctx.set_fill_style_str(accent);
  ctx.set_font("bold 26px sans-serif");
  let verdict = if passed {
    t("exam.passed")
  } else {
    t("exam.not-passed")
  };
  fill_max(&ctx, &verdict, 0.0, 0.0, 168.0)?;
  ctx.set_font("12px sans-serif");
  let pass_mark = tp(
    "common.class-pass-mark-questions",
    pass_line,
    &[bank.as_str(), &pass_line.to_string()],
  );
  fill_max(&ctx, &pass_mark, 0.0, 20.0, 168.0)?;
  ctx.restore();

  // 落款：QSL 卡的通行写法，不翻译。
  ctx.set_text_align("left");
  ctx.set_fill_style_str(QSL_INK);
  ctx.set_font(&format!("bold 15px {QSL_MONO}"));
  fill(&ctx, "TNX QSO · 73", margin, 370.0)?;

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
  fill(&ctx, &t("shell.weekly-report"), W / 2.0, 70.0)?;
  let today = js_sys::Date::new_0();
  let date = tf(
    "common.entry",
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
    (t("exam.answered-this-week"), answered.to_string()),
    (t("exam.new-questions-this-week"), new.to_string()),
    (
      t("learning.day-streak"),
      tp("common.days", streak, &[&streak.to_string()]),
    ),
    (t("exam.current-mistakes"), mistakes.to_string()),
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
    fill(&ctx, &t("exam.weakest-category-this-week"), W / 2.0, 262.0)?;
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

/// 把二进制字节作为文件下载（Blob + 对象 URL）。
///
/// 比 `data:` URL 更适合导出音频：几 MB 的 WAV 不经 base64（体积 +33%，且同时驻留
/// 原文 / base64 / URL 三份），也不受移动端浏览器对超长 data URL 的限制。
/// 对象 URL 延后回收——立刻 `revoke` 有概率让下载拿到已失效的 URL。
pub fn download_bytes(bytes: &[u8], mime: &str, filename: &str) {
  let array = js_sys::Uint8Array::from(bytes);
  let parts = js_sys::Array::new();
  parts.push(&array.buffer());
  let options = web_sys::BlobPropertyBag::new();
  options.set_type(mime);
  // 失败不能静默：调用方（导出 WAV / CSV）以为文件已经存下来了，其实什么都没发生。
  let Ok(blob) = web_sys::Blob::new_with_buffer_source_sequence_and_options(&parts, &options)
  else {
    crate::ui::push_error(t("common.download-failed"));
    return;
  };
  let Ok(url) = web_sys::Url::create_object_url_with_blob(&blob) else {
    crate::ui::push_error(t("common.download-failed"));
    return;
  };
  download(&url, filename);
  let revoke = url.clone();
  let _ = crate::util::window().set_timeout_with_callback_and_timeout_and_arguments_0(
    Closure::once_into_js(move || {
      let _ = web_sys::Url::revoke_object_url(&revoke);
    })
    .unchecked_ref(),
    10_000,
  );
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
  let blob = data_url_to_blob(data_url).ok_or_else(|| t("learning.image-processing-failed"))?;
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
  let blob = data_url_to_blob(data_url).ok_or_else(|| t("learning.image-processing-failed"))?;
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
#[derive(Clone, Copy, PartialEq)]
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
  fill(&ctx, &t("learning.study-report"), 300.0, 76.0)?;
  ctx.set_fill_style_str("#a1a1aa");
  ctx.set_font("16px sans-serif");
  fill(&ctx, &t(d.range), 300.0, 112.0)?;

  // 学习统计（2 列 × 3 行）。
  let stats: [(String, String); 6] = [
    (t("learning.answered-total"), d.answered.to_string()),
    (
      t("learning.accuracy"),
      tf("common.percent", &[&d.correct_rate.to_string()]),
    ),
    (
      t("learning.day-streak"),
      tp("common.days", d.streak, &[&d.streak.to_string()]),
    ),
    (t("exam.current-mistakes"), d.mistakes.to_string()),
    (t("learning.bookmarks"), d.bookmarks.to_string()),
    (t("exam.achievement-unlocked"), d.achievements.to_string()),
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
  fill(&ctx, &t("shell.logbook"), 300.0, 540.0)?;
  let log_stats: [(String, String); 2] = [
    (t("learning.qsos"), d.log_count.to_string()),
    (t("log.dxcc-entities"), d.dxcc_count.to_string()),
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
