//! WEFAX 解码器页面：上传 WEFAX 录音（WAV），交由 Web Worker 后台解码出灰度图。

use leptos::prelude::*;
use wasm_bindgen::JsValue;
use web_sys::File;

use crate::components::common::{DecodeElapsed, PageContainer, PageHeader};
use crate::decode_session::{DecodeConfig, DecodeSession};
use crate::icons::{Icon, IconKind};
use crate::ui::{
  Button, ButtonLink, Field, FileDropZone, SelectOption, SimpleSelect, Size, Slider, Variant,
};
use crate::util::{rgba_to_data_url, set_title};

use super::wefax_worker::parse_message;
use crate::i18n::{t, tf};

/// 体积上限：一幅真实的气象图要发十分钟左右，11.025 kHz / 16 位单声道约 13 MB，通用的 8 MiB 装不下。
/// 解调要把整段混频成 I / Q 两份 f32（各是样本数 × 4 字节），24 MiB 的 WAV 在 worker 里约占 200 MB。
const MAX_BYTES: u64 = 24 * 1024 * 1024;

/// 页面上可选的行速率。不依赖 `ham-web-wefax`（它只进 worker，不进主包）；worker 会把不在
/// `ham_web_wefax::fax::LPM_CHOICES` 里的值当成缺省 120，e2e 逐项验过两边一致。
const LPM_CHOICES: [u32; 6] = [60, 90, 100, 120, 180, 240];

/// 灰度像素 → 离屏 canvas → PNG data URL。
///
/// 解码器的灰度是「0 = 白、255 = 黑」（传真的黑是高幅度），屏幕上要反过来：白纸显示成白色、
/// 黑线显示成黑色。不反的话整幅图是底片（白线黑底）。
///
/// 峰值内存是灰度数据的 4 倍（`ImageData` 只收 RGBA）：一幅十分钟的真实气象图约 1810 × 1200
/// = 217 万像素，4× 即 8.7 MB，可以接受；再大的图应该改走 Worker 出图。
fn render_gray(gray: &[u8], width: u32, height: u32) -> Result<String, String> {
  let mut rgba = vec![0u8; gray.len() * 4];
  for (i, &g) in gray.iter().enumerate() {
    let o = i * 4;
    let shade = 255 - g;
    rgba[o] = shade;
    rgba[o + 1] = shade;
    rgba[o + 2] = shade;
    rgba[o + 3] = 255;
  }
  rgba_to_data_url(&rgba, width, height)
}

#[component]
pub fn WefaxDecoderPage() -> impl IntoView {
  set_title("tools.wefax-decoder");

  let session = DecodeSession::new(DecodeConfig {
    codec: "wefax",
    max_bytes: MAX_BYTES,
    transfer: false,
    parse: parse_message,
    too_large: |mb, max| tf("tools.file-too-large-downsample", &[mb, max]),
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

  // 格式：真实传真是默认；简化模型是本站最初自拟的口径，只解合成录音。
  let format = RwSignal::new("fax".to_owned());
  let lpm = RwSignal::new("120".to_owned());
  let ioc = RwSignal::new("576".to_owned());
  // 简化模型的解调方式：自动判断是默认；判错（噪声很大的录音）时让用户手选。
  let modulation = RwSignal::new("auto".to_owned());
  // 没有相位线时行首位置没有依据：图可能左右错开，用滑块把它转回来（纯显示，不重新解码）。
  let shift = RwSignal::new(0.0_f64);
  let format_options = vec![
    SelectOption::new("fax", Signal::derive(move || t("tools.wefax-format-fax"))),
    SelectOption::new(
      "simplified",
      Signal::derive(move || t("tools.wefax-format-simplified")),
    ),
  ];
  // `LPM` / `IOC` 是国际通用的技术单位缩写（专有名词，语言中立），与 `watch_window_row` 里的
  // 功率单位「W」同属不进词典的豁免，这里不写死 `t(...)`。
  let lpm_options: Vec<SelectOption> = LPM_CHOICES
    .iter()
    .map(|n| SelectOption::new(n.to_string(), format!("{n} LPM")))
    .collect();
  let ioc_options = vec![
    SelectOption::new("576", "IOC 576".to_owned()),
    SelectOption::new("288", "IOC 288".to_owned()),
  ];
  let modulation_options = vec![
    SelectOption::new(
      "auto",
      Signal::derive(move || t("tools.wefax-modulation-auto")),
    ),
    SelectOption::new("fm", Signal::derive(move || t("tools.wefax-modulation-fm"))),
    SelectOption::new("am", Signal::derive(move || t("tools.wefax-modulation-am"))),
  ];

  let cancel = move |_| session.cancel();
  let reset = move |_| session.reset();
  let on_file = Callback::new(move |file: File| {
    shift.set(0.0);
    session.start(file, move || {
      let params = js_sys::Object::new();
      for (key, value) in [
        ("format", format.get_untracked()),
        ("lpm", lpm.get_untracked()),
        ("ioc", ioc.get_untracked()),
        ("modulation", modulation.get_untracked()),
      ] {
        let _ = js_sys::Reflect::set(&params, &key.into(), &JsValue::from_str(&value));
      }
      Ok(params.into())
    });
  });

  // PNG 编码是主线程同步的大活（整幅 ImageData + `to_data_url`）。放进 `Memo` 只在结果
  // 变化时算一次：下面的渲染区里 `t()` / `tf()` 会订阅语言包版本，若把编码写在渲染闭包里，
  // 切一次语言就会重编码整幅图。
  let image = Memo::new(move |_| {
    // `with` 而不是 `get`：`WorkerPayload` 里带着整幅 `gray`，每次重算都 clone 一份
    // 等于白复制几 MB。
    result.with(|slot| {
      slot
        .as_ref()
        .map(|p| render_gray(&p.gray, p.width, p.height))
    })
  });

  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-(--motion-normal) ease-out">
      <PageHeader
        title=move || t("tools.wefax-decoder")
        subtitle=move || t("tools.wefax-decoder-subtitle")
        actions=ViewFn::from(move || {
          view! {
            <ButtonLink href="/modes" variant=Variant::Outline size=Size::Sm>
              {move || t("radio.digital-modes-reference")}
            </ButtonLink>
          }
        })
      />

      <PageContainer class="space-y-5">
        <section class="border bg-card p-4 text-sm text-muted-foreground">
          <p>
            {move || t("tools.upload-a-wefax-recording")}
            {move || t("common.decoding-runs-in-a")}
            {move || t("common.the-audio-is-never")}
          </p>
        </section>

        <section class="border bg-card p-4">
          {move || {
            (result.with(Option::is_none) && !processing.get())
              .then(|| {
                let format_id = crate::util::unique_id("wefax-format");
                let format_for = format_id.clone();
                let lpm_id = crate::util::unique_id("wefax-lpm");
                let lpm_for = lpm_id.clone();
                let ioc_id = crate::util::unique_id("wefax-ioc");
                let ioc_for = ioc_id.clone();
                let modulation_id = crate::util::unique_id("wefax-modulation");
                let modulation_for = modulation_id.clone();
                let format_options = format_options.clone();
                let lpm_options = lpm_options.clone();
                let ioc_options = ioc_options.clone();
                let modulation_options = modulation_options.clone();
                view! {
                  <div class="space-y-4">
                    <Field
                      label=Signal::derive(move || t("tools.radiogram-format"))
                      r#for=format_for
                    >
                      <SimpleSelect
                        id=format_id
                        value=Signal::derive(move || format.get())
                        on_change=Callback::new(move |v: String| format.set(v))
                        options=format_options
                      />
                      <span class="text-xs text-muted-foreground">
                        {move || t("tools.wefax-format-hint")}
                      </span>
                    </Field>
                    {move || {
                      if format.get() == "fax" {
                        let (lpm_for, lpm_id, ioc_for, ioc_id) = (
                          lpm_for.clone(),
                          lpm_id.clone(),
                          ioc_for.clone(),
                          ioc_id.clone(),
                        );
                        let (lpm_options, ioc_options) = (lpm_options.clone(), ioc_options.clone());
                        view! {
                          <div class="grid gap-4 sm:grid-cols-2">
                            <Field
                              label=Signal::derive(move || t("tools.wefax-lpm"))
                              r#for=lpm_for
                            >
                              <SimpleSelect
                                id=lpm_id
                                value=Signal::derive(move || lpm.get())
                                on_change=Callback::new(move |v: String| lpm.set(v))
                                options=lpm_options
                              />
                            </Field>
                            <Field
                              label=Signal::derive(move || t("tools.wefax-ioc"))
                              r#for=ioc_for
                            >
                              <SimpleSelect
                                id=ioc_id
                                value=Signal::derive(move || ioc.get())
                                on_change=Callback::new(move |v: String| ioc.set(v))
                                options=ioc_options
                              />
                            </Field>
                            <span class="text-xs text-muted-foreground sm:col-span-2">
                              {move || t("tools.wefax-fax-hint")}
                            </span>
                          </div>
                        }
                          .into_any()
                      } else {
                        let (modulation_for, modulation_id) =
                          (modulation_for.clone(), modulation_id.clone());
                        let modulation_options = modulation_options.clone();
                        view! {
                          <Field
                            label=Signal::derive(move || t("tools.wefax-modulation"))
                            r#for=modulation_for
                          >
                            <SimpleSelect
                              id=modulation_id
                              value=Signal::derive(move || modulation.get())
                              on_change=Callback::new(move |v: String| modulation.set(v))
                              options=modulation_options
                            />
                            <span class="text-xs text-muted-foreground">
                              {move || t("tools.wefax-modulation-hint")}
                            </span>
                          </Field>
                        }
                          .into_any()
                      }
                    }}
                    <FileDropZone
                      on_file=on_file
                      accept=".wav,audio/wav,audio/x-wav,audio/*"
                      title=Signal::derive(move || t("tools.choose-a-wefax-recording"))
                      icon=IconKind::Waves
                    />
                  </div>
                }
              })
          }}

          {move || {
            processing
              .get()
              .then(|| {
                view! {
                  <div role="status" class="flex flex-col items-center justify-center gap-3 py-8">
                    <Icon kind=IconKind::Loader2 class="h-8 w-8 animate-spin text-primary" />
                    <span class="text-sm text-muted-foreground">
                      {move || {
                        if reading.get() {
                          t("tools.reading-file")
                        } else {
                          t("radio.decoding-in-the-background")
                        }
                      }}
                    </span>
                    <DecodeElapsed ms=session.elapsed_ms />
                    <Button variant=Variant::Outline on_click=cancel>
                      {move || t("exam.cancel")}
                    </Button>
                  </div>
                }
              })
          }}

          {move || {
            error.get().map(|e| {
              view! {
                <crate::components::common::ErrorState
                  title=Signal::derive(move || t("radio.decoding-failed"))
                  detail=e
                />
              }
            })
          }}

          {move || {
            image
              .get()
              .map(|rendered| {
                let (width, height) = result
                  .with(|p| p.as_ref().map_or((0, 0), |p| (p.width, p.height)));
                let (is_fax, info, used_modulation) = result.with(|p| {
                  p.as_ref()
                    .map(|p| (p.format == "fax", p.fax.clone(), p.modulation.clone()))
                    .unwrap_or_default()
                });
                match rendered {
                  // 渲染失败不能给空 src：浏览器会把空 src 当页面地址再请求一次，只显示破图。
                  // 此时 `result` 仍是 `Some`、上传区已被收起，所以必须给一个出口，
                  // 否则用户只能刷新页面。
                  Err(e) => {
                    view! {
                      <div class="space-y-4">
                        <crate::components::common::ErrorState
                          title=Signal::derive(move || t("radio.decoding-failed"))
                          detail=e
                        />
                        <div class="flex justify-center">
                          <Button variant=Variant::Outline on_click=move |_| reset(())>
                            <Icon kind=IconKind::RefreshCw class="size-6" />
                            {move || t("tools.decode-another-file")}
                          </Button>
                        </div>
                      </div>
                    }
                      .into_any()
                  }
                  Ok(url) => {
                    // 没有相位线时行首位置没有依据：把图并排放两份、用 CSS 平移来「转」，不重新编码。
                    let tiled = is_fax && info.phasing_lines == 0;
                    let shift_id = crate::util::unique_id("wefax-shift");
                    let shift_for = shift_id.clone();
                    let url_again = url.clone();
                    let used_info = info.clone();
                    view! {
                      <div class="space-y-4">
                        <div class="bg-inset px-3 py-2 text-xs text-muted-foreground">
                          {crate::i18n::tf(
                            "tools.wefax-result-meta",
                            &[
                              &file_name.get_untracked(),
                              &width.to_string(),
                              &height.to_string(),
                            ],
                          )}
                        </div>
                        <div class="space-y-1 text-xs text-muted-foreground">
                          <p data-testid="wefax-used">
                            {move || {
                              if is_fax {
                                let levels = match used_info.levels.as_str() {
                                  "phasing" => t("tools.wefax-levels-phasing"),
                                  "stretched" => t("tools.wefax-levels-stretched"),
                                  _ => t("tools.wefax-levels-nominal"),
                                };
                                tf(
                                  "tools.wefax-used-fax",
                                  &[
                                    &format!("{:.2}", used_info.lpm),
                                    &format!("{:.0}", used_info.black_hz),
                                    &format!("{:.0}", used_info.white_hz),
                                    &levels,
                                  ],
                                )
                              } else if used_modulation == "fm" {
                                t("tools.wefax-used-fm")
                              } else {
                                t("tools.wefax-used-am")
                              }
                            }}
                          </p>
                          {is_fax
                            .then(|| {
                              let lines = info.phasing_lines;
                              view! {
                                <p data-testid="wefax-phasing" data-lines=lines.to_string()>
                                  {move || {
                                    if lines == 0 {
                                      t("tools.wefax-phasing-missing")
                                    } else {
                                      tf("tools.wefax-phasing-found", &[&lines.to_string()])
                                    }
                                  }}
                                </p>
                              }
                            })}
                        </div>
                        {tiled
                          .then(|| {
                            view! {
                              <Field
                                label=Signal::derive(move || t("tools.wefax-shift"))
                                r#for=shift_for
                              >
                                <Slider
                                  id=shift_id
                                  value=Signal::derive(move || shift.get())
                                  on_change=Callback::new(move |v: f64| shift.set(v))
                                  min=0.0
                                  max=99.0
                                  step=1.0
                                  aria_valuetext=Signal::derive(move || {
                                    format!("{}%", shift.get().round())
                                  })
                                />
                              </Field>
                            }
                          })}
                        {if tiled {
                          view! {
                            <div class="overflow-hidden border bg-card">
                              <div
                                class="flex w-[200%]"
                                style=move || {
                                  format!("transform: translateX(-{:.2}%)", shift.get() / 2.0)
                                }
                              >
                                <img
                                  src=url
                                  alt=t("tools.wefax-image-alt")
                                  class="w-1/2 shrink-0"
                                />
                                <img src=url_again alt="" aria-hidden="true" class="w-1/2 shrink-0" />
                              </div>
                            </div>
                          }
                            .into_any()
                        } else {
                          view! {
                            <img
                              src=url
                              alt=t("tools.wefax-image-alt")
                              class="w-full border bg-card"
                            />
                          }
                            .into_any()
                        }}
                        <div class="flex justify-center">
                          <Button variant=Variant::Outline on_click=move |_| reset(())>
                            <Icon kind=IconKind::RefreshCw class="size-6" />
                            {move || t("tools.decode-another-file")}
                          </Button>
                        </div>
                      </div>
                    }
                      .into_any()
                  }
                }
              })
          }}
        </section>
      </PageContainer>
    </div>
  }
}
