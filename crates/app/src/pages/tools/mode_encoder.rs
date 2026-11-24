use ham_web_core::mode_audio::{
  self, CW_TONE_HZ, CW_WPM, RTTY_BAUD, RTTY_MARK_HZ, RTTY_SPACE_HZ, SAMPLE_RATE,
};
use ham_web_core::mode_encoder::{olivia_text, text_to_baudot_bits, text_to_morse};
use ham_web_core::psk31;
use ham_web_core::slot_clock;
use leptos::prelude::*;
use wasm_bindgen::JsValue;

use super::picture_tx::PictureTx;
use crate::codec_modes::{DOMINOEX_MODES, MFSK_MODES, THOR_MODES, select_options};
use crate::components::slot_clock::SlotClock;
use crate::decode_worker;
use crate::i18n::{t, tf};
use crate::share_score::download_bytes;
use crate::ui::{
  Button, Checkbox, Field, Input, NumberField, NumberKind, SelectOption, SimpleSelect, Size,
  Variant, push_error,
};
use crate::util::unique_id;

/// WSPR 基带采样率：12 kHz 下每符号正好 8192 样本，162 符号 = 110.592 秒。
///
/// 不用 48 kHz（WSPR 的标准声卡采样率）：同样的内容体积大四倍，而合成器只按
/// `rate × 符号时长` 取整，12 kHz 恰好整除，不会累积相位误差。
const WSPR_RATE: u32 = 12_000;
/// WSPR 音频基准频率（4 个音调从它开始，占 6 Hz 带宽）。
const WSPR_BASE_HZ: f64 = 1500.0;

/// NAVTEX 广播前面的相位段（REP / ALPHA 交替）时长，秒。
const NAVTEX_PHASING_SECS: f64 = 2.0;

/// FT8 基带采样率：合成器内部固定 12 kHz（`ham_web_ft8::synthesize` 不接受别的值），
/// 一个 15 秒时隙正好 180 000 个样本。
const FT8_RATE: u32 = 12_000;
/// FT8 音频基频（符号 0 的音调；8 个音调占 50 Hz 带宽，1500 Hz 是电台音频窗口的常用中心）。
const FT8_BASE_HZ: f32 = 1500.0;

/// 合成结果 → WAV 下载；样本为空说明一个可编码字符都没有。
///
/// CW / RTTY / Feld Hell 对不认识的字符是静默跳过的（中文、全角标点等），全军覆没时
/// 导出的是一个 44 字节（仅文件头）的 WAV —— 必须报错，而不是让用户以为导出成功。
pub(super) fn download_wav(samples: Vec<f32>, rate: u32, filename: &str) {
  if samples.is_empty() {
    push_error(t("tools.mode-encoder-not-encodable"));
    return;
  }
  download_bytes(&mode_audio::to_wav(&samples, rate), "audio/wav", filename);
}

/// 把若干字段打成 JS 对象，作为 worker 编码请求的 `params`。
pub(super) fn params_of(fields: &[(&str, JsValue)]) -> JsValue {
  let obj = js_sys::Object::new();
  for (key, value) in fields {
    let _ = js_sys::Reflect::set(&obj, &JsValue::from_str(key), value);
  }
  obj.into()
}

/// 经独立 worker 合成并下载（可 `await`）：成功写 WAV（采样率取 worker 回包里的值），失败弹已本地化的错误。
///
/// GPL-3.0 的编码器（NAVTEX / MFSK / THOR / DominoEX）都走这里：主包不能依赖它们，
/// 见 `docs/engineering/licensing.md`；协议见 `docs/domain/decode-worker-protocol.md`「编码」。
pub(super) async fn encode_to_wav(codec: &'static str, params: JsValue, filename: &'static str) {
  let outcome = async {
    let result = decode_worker::encode_once(codec, &params).await?;
    let rate = decode_worker::get_u32(&result, "sampleRate")?;
    let samples = decode_worker::get_f32s(&result, "samples")?;
    Ok::<_, String>((samples, rate))
  }
  .await;
  match outcome {
    Ok((samples, rate)) => download_wav(samples, rate, filename),
    Err(message) => push_error(message),
  }
}

/// [`encode_to_wav`] 的即发即忘版本（按钮回调用）：`busy` 在发起时置真、结束时复位，
/// 快速连点只会发起一次，不会起一串 Worker。
fn encode_and_download(
  codec: &'static str,
  params: JsValue,
  filename: &'static str,
  busy: RwSignal<bool>,
) {
  busy.set(true);
  wasm_bindgen_futures::spawn_local(async move {
    encode_to_wav(codec, params, filename).await;
    busy.set(false);
  });
}

/// 中心频率输入框 → worker 的 `centerHz`：留空 = 不传（worker 用缺省 1500 Hz）；
/// 填了但不是数字也传，给 `NaN`，让 worker 回 `center-hz-out-of-range:NaN`，而不是悄悄换成缺省值。
pub(super) fn center_field(text: &str) -> Option<JsValue> {
  let text = text.trim();
  (!text.is_empty()).then(|| JsValue::from_f64(text.parse::<f64>().unwrap_or(f64::NAN)))
}

/// 数字模式报文编码：文本 → 摩尔斯（CW）与 RTTY（ITA2）比特流，并可导出基带 WAV。
#[component]
pub(super) fn ModeEncoder() -> impl IntoView {
  let text = RwSignal::new("CQ CQ DE BG1AAA".to_string());
  let text_id = unique_id("mode-encoder");
  // 走 worker 的合成按钮共用一个 busy：快速连点只发一次，按钮同时禁用。
  let tx_busy = RwSignal::new(false);

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

  let ft8_message = RwSignal::new("CQ BG1AAA OM89".to_string());
  let ft8_id = unique_id("ft8-message");

  // Olivia 子模式取库里的标准表（音调数 / 带宽 / 默认中心频率），选项值就是表里的名字。
  let olivia_mode = RwSignal::new("32/1000".to_string());
  let olivia_options: Vec<SelectOption> = ham_web_olivia::SUBMODES
    .iter()
    .map(|(name, ..)| SelectOption::new(name.to_string(), name.to_string()))
    .collect();
  let olivia_id = unique_id("olivia-submode");

  // MFSK / THOR / DominoEX：名单来自 `codec_modes`（解码页同一份），合成在各自的 GPL worker 里。
  let mfsk_mode = RwSignal::new(MFSK_MODES[0].0.to_owned());
  let thor_mode = RwSignal::new(THOR_MODES[0].0.to_owned());
  let dominoex_mode = RwSignal::new(DOMINOEX_MODES[0].0.to_owned());
  let dominoex_fec = RwSignal::new(false);
  let mfsk_options = select_options(MFSK_MODES);
  let thor_options = select_options(THOR_MODES);
  let dominoex_options = select_options(DOMINOEX_MODES);
  // 中心频率（MFSK / THOR 共用，留空 = 缺省 1500 Hz）与 THOR 的 ID 信标（二次信道）。
  let center = RwSignal::new(String::new());
  let thor_beacon = RwSignal::new(String::new());
  let center_id = unique_id("fldigi-tx-center");
  let thor_id_id = unique_id("thor-tx-id");
  let mfsk_id = unique_id("mfsk-tx-submode");
  let thor_id = unique_id("thor-tx-submode");
  let dominoex_id = unique_id("dominoex-tx-submode");

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
  //
  // 合成在 navtex 的独立 worker 里做，不在主包：`ham-web-navtex` 是 GPL-3.0，链进 MIT 的
  // 主包 wasm 会让整个主包带上 GPL（`docs/engineering/licensing.md`）。字段校验也只在核心那一处
  // （`ham_web_navtex::encode_message`）：不合法时 worker 回 `encode-params:<字段>`，
  // 由 `localize_error` 换成指出具体字段的提示，页面不再自己抄一份规则。
  let download_navtex = Callback::new(move |_| {
    if tx_busy.get_untracked() {
      return;
    }
    // 正文长度仍先拦：与其他按钮共用同一套空 / 过长提示，也省得发一个必然失败的请求。
    let Some(body) = require_text() else {
      return;
    };
    // 空串解析不出数字：给 NaN，worker 一样回 `encode-params:serial`。
    let serial = navtex_serial
      .get()
      .trim()
      .parse::<f64>()
      .unwrap_or(f64::NAN);
    encode_and_download(
      "navtex",
      params_of(&[
        ("station", navtex_station.get().into()),
        ("category", navtex_kind.get().into()),
        ("serial", serial.into()),
        ("text", body.into()),
        ("phasingSecs", NAVTEX_PHASING_SECS.into()),
      ]),
      "navtex.wav",
      tx_busy,
    );
  });

  // MFSK / THOR / DominoEX：同样经 worker 合成（GPL-3.0，不能链进主包）。报文原样交给 worker，
  // 可发字符的过滤与长度校验只在核心 crate 一处（`ham_web_mfsk::sendable_text`）；采样率取各模式的标称值，
  // 中心频率用缺省（DominoEX 固定 1000 Hz），解码页选同一子模式即可读回。
  let download_mfsk = Callback::new(move |_| {
    if tx_busy.get_untracked() {
      return;
    }
    let Some(body) = require_text() else {
      return;
    };
    let mut fields = vec![("mode", mfsk_mode.get().into()), ("text", body.into())];
    fields.extend(center_field(&center.get()).map(|c| ("centerHz", c)));
    encode_and_download("mfsk", params_of(&fields), "mfsk.wav", tx_busy);
  });
  let download_thor = Callback::new(move |_| {
    if tx_busy.get_untracked() {
      return;
    }
    let Some(body) = require_text() else {
      return;
    };
    let mut fields = vec![
      ("mode", thor_mode.get().into()),
      ("text", body.into()),
      ("id", thor_beacon.get().into()),
    ];
    fields.extend(center_field(&center.get()).map(|c| ("centerHz", c)));
    encode_and_download("thor", params_of(&fields), "thor.wav", tx_busy);
  });
  let download_dominoex = Callback::new(move |_| {
    if tx_busy.get_untracked() {
      return;
    }
    let Some(body) = require_text() else {
      return;
    };
    encode_and_download(
      "dominoex",
      params_of(&[
        ("mode", dominoex_mode.get().into()),
        ("text", body.into()),
        ("fec", dominoex_fec.get().into()),
      ]),
      "dominoex.wav",
      tx_busy,
    );
  });

  // FT8：77 位打包 + CRC-14 + LDPC + 格雷码 + 同步 + GFSK 都在解码器的 crate 里
  // （`crates/ft8` 有「编码 → 合成 → 解码」往返单测）。报文单独一个输入框：FT8 的标准报文
  // 是「呼号 呼号 网格」，和上面 CW / RTTY 共用的自由文本不是一回事。
  // 认不出标准格式时库会回退成自由文本（最多 13 个字符），超长或含非法字符才报错；
  // 空报文在库里是合法的全空格自由文本，要自己拦。
  let download_ft8 = Callback::new(move |_| {
    let text = ft8_message.get().trim().to_uppercase();
    let samples = (!text.is_empty())
      .then(|| {
        ham_web_ft8::synthesize(
          &text,
          FT8_BASE_HZ,
          false,
          &mut ham_web_ft8::HashTable::new(),
        )
      })
      .flatten();
    let Some(samples) = samples else {
      push_error(t("tools.ft8-encode-failed"));
      return;
    };
    download_wav(samples, FT8_RATE, "ft8.wav");
  });

  // Olivia：音调数 / 带宽 / 中心频率取子模式表，合成用解码器同一份 `encode_audio`
  // （`crates/olivia` 有「编码 → 解码」往返单测）。报文只发可打印 ASCII，库对 >127 的字符
  // 会悄悄换成 NUL，所以先过滤；过滤后为空、或合成结果超过样本上限都按「无法编码」报错，
  // 否则会导出一个只有前导的 WAV。
  let download_olivia = Callback::new(move |_| {
    let Some(body) = require_text() else {
      return;
    };
    let body = olivia_text(&body);
    let mode = olivia_mode.get();
    let samples = ham_web_olivia::SUBMODES
      .iter()
      .find(|(name, ..)| *name == mode)
      .filter(|_| !body.trim().is_empty())
      .and_then(|&(_, symbols, bandwidth, center)| {
        ham_web_olivia::encode_audio(&body, symbols, bandwidth, center, SAMPLE_RATE)
      })
      .filter(|samples| samples.len() <= mode_audio::MAX_SAMPLES);
    let Some(samples) = samples else {
      push_error(t("tools.olivia-encode-failed"));
      return;
    };
    download_wav(samples, SAMPLE_RATE, "olivia.wav");
  });

  view! {
    <div class="space-y-4">
      <Field label=Signal::derive(move || t("tools.message-content")) r#for=text_id.clone()>
        <Input id=text_id value=text on_change=Callback::new(move |v: String| text.set(v)) />
      </Field>
      <div class="space-y-1 bg-inset px-3 py-2 text-sm text-muted-foreground">
        <div class="text-xs text-foreground/70">{move || t("tools.cw-morse-code")}</div>
        <div class="break-all font-mono tabular-nums">{move || text_to_morse(&text.get())}</div>
      </div>
      <div class="space-y-1 bg-inset px-3 py-2 text-sm text-muted-foreground">
        <div class="text-xs text-foreground/70">{move || t("tools.rtty-ita2-bit-stream")}</div>
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
          <SimpleSelect
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
      <div class="border-t pt-3">
        <Field label=Signal::derive(|| t("tools.ft8-tx-message")) r#for=ft8_id.clone()>
          <Input
            id=ft8_id
            value=ft8_message
            on_change=Callback::new(move |v: String| ft8_message.set(v))
          />
        </Field>
      </div>
      <div class="border-t pt-3">
        <Field label=Signal::derive(|| t("tools.olivia-tx-submode")) r#for=olivia_id.clone()>
          <SimpleSelect
            id=olivia_id
            value=olivia_mode
            on_change=Callback::new(move |v: String| olivia_mode.set(v))
            options=olivia_options
            aria_label=Signal::derive(|| t("tools.olivia-tx-submode"))
          />
        </Field>
      </div>
      <div class="space-y-3 border-t pt-3">
        <div class="grid gap-3 sm:grid-cols-3">
          <Field label=Signal::derive(|| t("tools.mfsk-tx-submode")) r#for=mfsk_id.clone()>
            <SimpleSelect
              id=mfsk_id
              value=mfsk_mode
              on_change=Callback::new(move |v: String| mfsk_mode.set(v))
              options=mfsk_options
              aria_label=Signal::derive(|| t("tools.mfsk-tx-submode"))
            />
          </Field>
          <Field label=Signal::derive(|| t("tools.thor-tx-submode")) r#for=thor_id.clone()>
            <SimpleSelect
              id=thor_id
              value=thor_mode
              on_change=Callback::new(move |v: String| thor_mode.set(v))
              options=thor_options
              aria_label=Signal::derive(|| t("tools.thor-tx-submode"))
            />
          </Field>
          <Field label=Signal::derive(|| t("tools.dominoex-tx-submode")) r#for=dominoex_id.clone()>
            <SimpleSelect
              id=dominoex_id
              value=dominoex_mode
              on_change=Callback::new(move |v: String| dominoex_mode.set(v))
              options=dominoex_options
              aria_label=Signal::derive(|| t("tools.dominoex-tx-submode"))
            />
          </Field>
        </div>
        <div class="grid gap-3 sm:grid-cols-2">
          <Field
            label=Signal::derive(|| t("tools.fldigi-tx-center"))
            hint=Signal::derive(|| t("tools.fldigi-tx-center-hint"))
            r#for=center_id.clone()
          >
            <NumberField
              id=center_id
              value=Signal::derive(move || center.get())
              on_change=Callback::new(move |v: String| center.set(v))
              controls=false
              kind=NumberKind::Decimal
            />
          </Field>
          <Field
            label=Signal::derive(|| t("tools.thor-tx-id"))
            hint=Signal::derive(|| t("tools.thor-tx-id-hint"))
            r#for=thor_id_id.clone()
          >
            <Input
              id=thor_id_id
              value=thor_beacon
              on_change=Callback::new(move |v: String| thor_beacon.set(v))
              maxlength=Signal::derive(|| "64".to_owned())
            />
          </Field>
        </div>
        <label class="flex items-start gap-2 text-sm">
          <Checkbox
            checked=Signal::derive(move || dominoex_fec.get())
            on_change=Callback::new(move |v: bool| dominoex_fec.set(v))
          />
          <span>
            {move || t("tools.dominoex-fec")}
            <span class="block text-xs text-muted-foreground">
              {move || t("tools.dominoex-tx-fec-hint")}
            </span>
          </span>
        </label>
      </div>
      <PictureTx text=text mfsk_mode=mfsk_mode thor_mode=thor_mode center=center />
      // 时隙钟：导出的 FT8 / FT4 / WSPR 基带要在时隙起点开始播放，这里给出离下一个起点还有多久。
      <div class="border-t pt-3">
        <SlotClock specs=vec![slot_clock::FT8, slot_clock::FT4, slot_clock::WSPR] />
      </div>
      <div class="space-y-2 border-t pt-3">
        <div class="text-xs text-foreground/70">{move || t("tools.audio-generate")}</div>
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
          <Button
            variant=Variant::Outline
            size=Size::Sm
            disabled=Signal::derive(move || tx_busy.get())
            on_click=download_navtex
          >
            {move || t("tools.download-navtex-wav")}
          </Button>
          <Button variant=Variant::Outline size=Size::Sm on_click=download_hell>
            {move || t("tools.download-hell-wav")}
          </Button>
          <Button variant=Variant::Outline size=Size::Sm on_click=download_ft8>
            {move || t("tools.download-ft8-wav")}
          </Button>
          <Button variant=Variant::Outline size=Size::Sm on_click=download_olivia>
            {move || t("tools.download-olivia-wav")}
          </Button>
          <Button
            variant=Variant::Outline
            size=Size::Sm
            disabled=Signal::derive(move || tx_busy.get())
            on_click=download_mfsk
          >
            {move || t("tools.download-mfsk-wav")}
          </Button>
          <Button
            variant=Variant::Outline
            size=Size::Sm
            disabled=Signal::derive(move || tx_busy.get())
            on_click=download_thor
          >
            {move || t("tools.download-thor-wav")}
          </Button>
          <Button
            variant=Variant::Outline
            size=Size::Sm
            disabled=Signal::derive(move || tx_busy.get())
            on_click=download_dominoex
          >
            {move || t("tools.download-dominoex-wav")}
          </Button>
        </div>
        <p class="text-xs text-muted-foreground">{move || t("tools.audio-note")}</p>
        <p class="text-xs text-muted-foreground">{move || t("tools.wspr-audio-note")}</p>
        <p class="text-xs text-muted-foreground">{move || t("tools.navtex-audio-note")}</p>
        <p class="text-xs text-muted-foreground">{move || t("tools.hell-audio-note")}</p>
        <p class="text-xs text-muted-foreground">{move || t("tools.ft8-audio-note")}</p>
        <p class="text-xs text-muted-foreground">{move || t("tools.olivia-audio-note")}</p>
        <p class="text-xs text-muted-foreground">{move || t("tools.fldigi-modes-audio-note")}</p>
      </div>
    </div>
  }
}
