//! AX.25 软 TNC：Bell 202 AFSK1200 的「帧 ↔ 基带音频」。
//!
//! TX 侧把 AX.25 UI 帧调制成 48 kHz 单声道 WAV（声卡馈入电台）；RX 侧上传 WAV 解调回帧。
//! 解调在专用 Worker 里跑：符号定时恢复要穷举「位相位 × NRZI 初始电平」，放在主线程
//! 会把页面冻住几十秒（见 `crates/afsk1200-worker`）。

mod soft_tnc_worker;

use ham_web_core::afsk1200::{self, SAMPLE_RATE};
use ham_web_core::kiss::{MAX_UI_INFO, ax25_ui_frame};
use ham_web_core::mode_audio;
use leptos::prelude::*;
use wasm_bindgen::JsValue;
use web_sys::File;

use crate::decode_session::{DecodeConfig, DecodeSession, MAX_UPLOAD_BYTES};
use crate::i18n::{t, tf};
use crate::share_score::download_bytes;
use crate::ui::{Button, Field, FileInput, Input, Size, Variant, push_error};
use crate::util::unique_id;

use soft_tnc_worker::{TncFrame, parse_message};

#[component]
pub(super) fn SoftTnc() -> impl IntoView {
  let dest = RwSignal::new("APRS".to_string());
  let src = RwSignal::new("BG1AAA-7".to_string());
  let info = RwSignal::new(">hello".to_string());

  let dest_id = unique_id("soft-tnc-dest");
  let src_id = unique_id("soft-tnc-src");
  let info_id = unique_id("soft-tnc-info");

  let download_wav = Callback::new(move |_| {
    let info = info.get();
    // `ax25_ui_frame` 对「呼号非法」和「整帧超长」都返回 `None`：先把长度挑出来单独报，
    // 免得用户去检查一个其实没有错的呼号。
    if info.len() > MAX_UI_INFO {
      push_error(tf("tools.kiss-info-too-long", &[&MAX_UI_INFO.to_string()]));
      return;
    }
    let Some(frame) = ax25_ui_frame(&dest.get(), &src.get(), &[], info.as_bytes()) else {
      push_error(t("tools.kiss-invalid-call"));
      return;
    };
    // 实发形态：带前导 flag 与尾部 flag，VOX / PTT 启动不会吃掉首帧（见 `afsk1200`）。
    let samples = afsk1200::modulate_frame_for_tx(&frame, SAMPLE_RATE);
    download_bytes(
      &mode_audio::to_wav(&samples, SAMPLE_RATE),
      "audio/wav",
      "afsk1200.wav",
    );
  });

  let session = DecodeSession::new(DecodeConfig {
    codec: "afsk1200",
    max_bytes: MAX_UPLOAD_BYTES,
    // 解调在 Worker 里做，主线程只持有一份 `Vec<u8>` 副本；APRS 录音不大，不值得为
    // 转移所有权把页面侧的读取路径换成 `ArrayBuffer`。
    transfer: false,
    parse: parse_message,
    too_large: |mb, max| tf("tools.file-too-large-about", &[mb, max]),
    timed_out: || t("tools.decoding-timed-out"),
    failed: || t("tools.decoding-failed-generic"),
  });
  let DecodeSession {
    processing,
    result,
    error,
    file_name,
    reading,
    ..
  } = session;

  let on_files = Callback::new(move |files: Vec<File>| {
    if let Some(file) = files.into_iter().next() {
      session.start(file, || Ok(JsValue::NULL));
    }
  });
  let reset = move |_| session.reset();
  let cancel = move |_| session.cancel();

  view! {
    <div class="space-y-4">
      <div class="grid gap-3 sm:grid-cols-3">
        <Field label=Signal::derive(move || t("tools.kiss-dest")) r#for=dest_id.clone()>
          <Input id=dest_id value=dest on_change=Callback::new(move |v: String| dest.set(v)) />
        </Field>
        <Field label=Signal::derive(move || t("tools.kiss-src")) r#for=src_id.clone()>
          <Input id=src_id value=src on_change=Callback::new(move |v: String| src.set(v)) />
        </Field>
        <Field label=Signal::derive(move || t("tools.kiss-info")) r#for=info_id.clone()>
          <Input id=info_id value=info on_change=Callback::new(move |v: String| info.set(v)) />
        </Field>
      </div>
      <div class="flex flex-wrap items-center gap-2">
        <Button variant=Variant::Outline size=Size::Sm on_click=download_wav>
          {move || t("tools.soft-tnc-download")}
        </Button>
      </div>

      <div class="space-y-2 border-t pt-3">
        <div class="text-xs font-medium text-foreground/70">{move || t("tools.soft-tnc-rx-title")}</div>
        <FileInput
          on_files=on_files
          accept=".wav,audio/wav,audio/x-wav"
          label=Signal::derive(move || t("tools.soft-tnc-rx-choose"))
          size=Size::Sm
          variant=Variant::Outline
        />
        {move || {
          processing
            .get()
            .then(|| {
              view! {
                <div class="flex items-center gap-2">
                  <p role="status" class="text-xs text-muted-foreground">
                    {move || {
                      if reading.get() {
                        t("tools.reading-file")
                      } else {
                        t("tools.soft-tnc-decoding")
                      }
                    }}
                  </p>
                  <Button variant=Variant::Text size=Size::Xs on_click=cancel>
                    {move || t("exam.cancel")}
                  </Button>
                </div>
              }
            })
        }}
        {move || {
          error.get().map(|e| {
            view! {
              <p role="alert" class="text-sm text-destructive">{e}</p>
            }
          })
        }}
        {move || {
          result.get().map(|payload| {
            let lines = payload.frames.iter().map(|frame| match frame {
              TncFrame::Ui { source, destination, info } => {
                tf("tools.soft-tnc-frame", &[source, destination, info])
              }
              TncFrame::Raw(hex) => hex.clone(),
            }).collect::<Vec<_>>().join("\n");
            view! {
              <div class="space-y-2">
                {payload
                  .truncated
                  .then(|| {
                    view! {
                      <p class="text-xs text-amber-600 dark:text-amber-400">
                        {move || t("tools.soft-tnc-truncated")}
                      </p>
                    }
                  })}
                <div class="bg-muted/40 px-3 py-2 text-xs text-muted-foreground">
                  {crate::i18n::tf("radio.decoded", &[&file_name.get_untracked()])}
                </div>
                <pre class="whitespace-pre-wrap break-words border bg-muted/40 p-3 text-sm font-mono">{lines}</pre>
                <div class="flex justify-center">
                  <Button variant=Variant::Outline on_click=move |_| reset(())>
                    {move || t("tools.decode-another-file")}
                  </Button>
                </div>
              </div>
            }
          })
        }}
      </div>
      <p class="text-xs text-muted-foreground">{move || t("tools.soft-tnc-note")}</p>
    </div>
  }
}
