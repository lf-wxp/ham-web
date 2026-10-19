use ham_web_core::mode_audio::{
  self, CW_TONE_HZ, CW_WPM, RTTY_BAUD, RTTY_MARK_HZ, RTTY_SPACE_HZ, SAMPLE_RATE,
};
use ham_web_core::mode_encoder::{text_to_baudot_bits, text_to_morse};
use ham_web_core::psk31;
use leptos::prelude::*;

use crate::i18n::{t, tf};
use crate::share_score::download_bytes;
use crate::ui::{Button, Field, Input, NativeSelect, SelectOption, Size, Variant, push_error};
use crate::util::unique_id;

/// WSPR 基带采样率：12 kHz 下每符号正好 8192 样本，162 符号 = 110.592 秒。
///
/// 不用 48 kHz（WSPR 的标准声卡采样率）：同样的内容体积大四倍，而合成器只按
/// `rate × 符号时长` 取整，12 kHz 恰好整除，不会累积相位误差。
const WSPR_RATE: u32 = 12_000;
/// WSPR 音频基准频率（4 个音调从它开始，占 6 Hz 带宽）。
const WSPR_BASE_HZ: f64 = 1500.0;

/// 合成结果 → WAV 下载；样本为空说明一个可编码字符都没有。
///
/// CW / RTTY / Feld Hell 对不认识的字符是静默跳过的（中文、全角标点等），全军覆没时
/// 导出的是一个 44 字节（仅文件头）的 WAV —— 必须报错，而不是让用户以为导出成功。
fn download_wav(samples: Vec<f32>, rate: u32, filename: &str) {
  if samples.is_empty() {
    push_error(t("tools.mode-encoder-not-encodable"));
    return;
  }
  download_bytes(&mode_audio::to_wav(&samples, rate), "audio/wav", filename);
}

/// 数字模式报文编码：文本 → 摩尔斯（CW）与 RTTY（ITA2）比特流，并可导出基带 WAV。
#[component]
pub(super) fn ModeEncoder() -> impl IntoView {
  let text = RwSignal::new("CQ CQ DE BG1AAA".to_string());
  let text_id = unique_id("mode-encoder");

  let wspr_call = RwSignal::new("BG1AAA".to_string());
  let wspr_grid = RwSignal::new("OM89".to_string());
  let wspr_power = RwSignal::new("20".to_string());
  let wspr_power_options: Vec<SelectOption> = ham_web_wspr::POWER_LEVELS
    .iter()
    .map(|p| SelectOption::new(p.to_string(), format!("{p} dBm")))
    .collect();
  let call_id = unique_id("wspr-call");
  let grid_id = unique_id("wspr-grid");
  let power_id = unique_id("wspr-power");

  let navtex_station = RwSignal::new("F".to_string());
  let navtex_kind = RwSignal::new("A".to_string());
  let navtex_serial = RwSignal::new("79".to_string());
  let station_id = unique_id("navtex-station");
  let kind_id = unique_id("navtex-kind");
  let serial_id = unique_id("navtex-serial");

  // 四个「把报文直接合成音频」的按钮共用：空报文 / 超长报文直接提示，
  // 否则会静默导出一个 44 字节的 WAV（只有文件头）。
  let require_text = move || -> Option<String> {
    let value = text.get();
    if value.trim().is_empty() {
      push_error(t("tools.mode-encoder-empty"));
      return None;
    }
    if value.chars().count() > mode_audio::MAX_TEXT_CHARS {
      push_error(tf(
        "tools.mode-encoder-too-long",
        &[&mode_audio::MAX_TEXT_CHARS.to_string()],
      ));
      return None;
    }
    Some(value)
  };

  let download_cw = Callback::new(move |_| {
    let Some(body) = require_text() else {
      return;
    };
    download_wav(
      mode_audio::cw_samples(&body, CW_WPM, CW_TONE_HZ, SAMPLE_RATE),
      SAMPLE_RATE,
      "cw.wav",
    );
  });
  let download_rtty = Callback::new(move |_| {
    let Some(body) = require_text() else {
      return;
    };
    download_wav(
      mode_audio::rtty_samples(&body, RTTY_BAUD, RTTY_MARK_HZ, RTTY_SPACE_HZ, SAMPLE_RATE),
      SAMPLE_RATE,
      "rtty.wav",
    );
  });
  // PSK31 载波 1000 Hz（标准音频中心频率），复用 `psk31::synthesize`（往返解码已对拍）。
  let download_psk31 = Callback::new(move |_| {
    let Some(body) = require_text() else {
      return;
    };
    download_wav(
      psk31::synthesize(&body, SAMPLE_RATE, 1000.0, 0.0),
      SAMPLE_RATE,
      "psk31.wav",
    );
  });
  // WSPR：消息打包 + 4-FSK 合成都在解码器的 crate 里（`crates/wspr` 自带往返测试），
  // 这里只负责把符号变成 WAV。呼号 / 网格 / 功率任一不合法就整条不发。
  let download_wspr = Callback::new(move |_| {
    let power = wspr_power.get().parse::<u8>().ok();
    let encoded = power.and_then(|p| ham_web_wspr::encode(&wspr_call.get(), &wspr_grid.get(), p));
    let Some(symbols) = encoded else {
      push_error(t("tools.wspr-encode-failed"));
      return;
    };
    let samples = ham_web_wspr::synthesize(&symbols, WSPR_BASE_HZ, WSPR_RATE);
    download_wav(samples, WSPR_RATE, "wspr.wav");
  });

  // Feld Hell：字库与合成都在解码器 crate 里（`crates/hell` 自带 modulate → decode 往返测试），
  // 这里只把点阵变成 WAV。
  let download_hell = Callback::new(move |_| {
    let Some(body) = require_text() else {
      return;
    };
    download_wav(
      ham_web_hell::modulate(&body, SAMPLE_RATE),
      SAMPLE_RATE,
      "hell.wav",
    );
  });

  // NAVTEX：报头（台站字母 / 类型字母 / 两位序号）与正文一起合成 100 波特 FSK 基带，
  // 解码端能原样读回 `ZCZC` 里的三个字段（`crates/navtex` 有往返测试）。
  let download_navtex = Callback::new(move |_| {
    // 正文长度必须先拦：100 波特下每字符约 0.07 s，没有上限时一段长粘贴就能把
    // `synthesize_message` 撑到几百 MB。
    let Some(body) = require_text() else {
      return;
    };
    let single_letter = |value: String| {
      value
        .trim()
        .chars()
        .next()
        .filter(|c| c.is_ascii_alphabetic())
        .map(|c| c.to_ascii_uppercase())
    };
    let (Some(b1), Some(b2)) = (
      single_letter(navtex_station.get()),
      single_letter(navtex_kind.get()),
    ) else {
      push_error(t("tools.navtex-encode-failed"));
      return;
    };
    let serial = navtex_serial
      .get()
      .trim()
      .parse::<u8>()
      .ok()
      .filter(|s| *s <= 99);
    let Some(serial) = serial else {
      push_error(t("tools.navtex-encode-failed"));
      return;
    };
    download_wav(
      ham_web_navtex::synthesize_message(b1, b2, serial, &body, 2.0),
      SAMPLE_RATE,
      "navtex.wav",
    );
  });

  view! {
    <div class="space-y-4">
      <Field label=Signal::derive(move || t("tools.message-content")) r#for=text_id.clone()>
        <Input id=text_id value=text on_change=Callback::new(move |v: String| text.set(v)) />
      </Field>
      <div class="space-y-1 bg-muted/40 px-3 py-2 text-sm text-muted-foreground">
        <div class="text-xs font-medium text-foreground/70">{move || t("tools.cw-morse-code")}</div>
        <div class="break-all font-mono tabular-nums">{move || text_to_morse(&text.get())}</div>
      </div>
      <div class="space-y-1 bg-muted/40 px-3 py-2 text-sm text-muted-foreground">
        <div class="text-xs font-medium text-foreground/70">{move || t("tools.rtty-ita2-bit-stream")}</div>
        <div class="break-all font-mono tabular-nums">{move || text_to_baudot_bits(&text.get())}</div>
      </div>
      <p class="text-xs text-muted-foreground">
        {move || t("tools.rtty-coding-11111-is")}
      </p>
      <div class="grid gap-3 border-t pt-3 sm:grid-cols-3">
        <Field label=Signal::derive(|| t("tools.wspr-tx-callsign")) r#for=call_id.clone()>
          <Input
            id=call_id
            value=wspr_call
            on_change=Callback::new(move |v: String| wspr_call.set(v))
          />
        </Field>
        <Field label=Signal::derive(|| t("tools.wspr-tx-locator")) r#for=grid_id.clone()>
          <Input
            id=grid_id
            value=wspr_grid
            on_change=Callback::new(move |v: String| wspr_grid.set(v))
          />
        </Field>
        <Field label=Signal::derive(|| t("tools.wspr-tx-power")) r#for=power_id.clone()>
          <NativeSelect
            id=power_id
            value=wspr_power
            on_change=Callback::new(move |v: String| wspr_power.set(v))
            options=wspr_power_options
            aria_label=Signal::derive(|| t("tools.wspr-tx-power"))
          />
        </Field>
      </div>
      <div class="grid gap-3 border-t pt-3 sm:grid-cols-3">
        <Field label=Signal::derive(|| t("tools.navtex-station")) r#for=station_id.clone()>
          <Input
            id=station_id
            value=navtex_station
            on_change=Callback::new(move |v: String| navtex_station.set(v))
          />
        </Field>
        <Field label=Signal::derive(|| t("tools.navtex-kind")) r#for=kind_id.clone()>
          <Input
            id=kind_id
            value=navtex_kind
            on_change=Callback::new(move |v: String| navtex_kind.set(v))
          />
        </Field>
        <Field label=Signal::derive(|| t("tools.navtex-serial")) r#for=serial_id.clone()>
          <Input
            id=serial_id
            value=navtex_serial
            on_change=Callback::new(move |v: String| navtex_serial.set(v))
          />
        </Field>
      </div>
      <div class="space-y-2 border-t pt-3">
        <div class="text-xs font-medium text-foreground/70">{move || t("tools.audio-generate")}</div>
        <div class="flex flex-wrap gap-2">
          <Button variant=Variant::Outline size=Size::Sm on_click=download_cw>
            {move || t("tools.download-cw-wav")}
          </Button>
          <Button variant=Variant::Outline size=Size::Sm on_click=download_rtty>
            {move || t("tools.download-rtty-wav")}
          </Button>
          <Button variant=Variant::Outline size=Size::Sm on_click=download_psk31>
            {move || t("tools.download-psk31-wav")}
          </Button>
          <Button variant=Variant::Outline size=Size::Sm on_click=download_wspr>
            {move || t("tools.download-wspr-wav")}
          </Button>
          <Button variant=Variant::Outline size=Size::Sm on_click=download_navtex>
            {move || t("tools.download-navtex-wav")}
          </Button>
          <Button variant=Variant::Outline size=Size::Sm on_click=download_hell>
            {move || t("tools.download-hell-wav")}
          </Button>
        </div>
        <p class="text-xs text-muted-foreground">{move || t("tools.audio-note")}</p>
        <p class="text-xs text-muted-foreground">{move || t("tools.wspr-audio-note")}</p>
        <p class="text-xs text-muted-foreground">{move || t("tools.navtex-audio-note")}</p>
        <p class="text-xs text-muted-foreground">{move || t("tools.hell-audio-note")}</p>
      </div>
    </div>
  }
}
