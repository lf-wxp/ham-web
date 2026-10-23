//! KISS TNC 页面：Web Serial 收发 AX.25 帧。
//!
//! 编解码全部在核心 `ham_web_core::kiss`（有单测）；这里只有串口胶水与界面。
//! 帧预览不依赖串口 —— 填好呼号与信息就能离线看到 KISS 字节（与测试里的另一套
//! JS 实现逐字节对拍）。

use std::collections::VecDeque;

use ham_web_core::kiss::{KissDecoder, ax25_ui_frame, encode_frame, parse_ui_frame};
use leptos::prelude::*;
use leptos::task::spawn_local;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use web_sys::js_sys;

use crate::components::common::{EmptyState, PageContainer, PageHeader};
use crate::i18n::{t, tf};
use crate::ui::{
  Button, ButtonLink, Field, Input, NativeSelect, SelectOption, Size, TextValue, Tooltip, Variant,
};
use crate::util::{js_error_message, set_title, unique_id};
use crate::web_serial::{Conn, call, close_serial, is_user_cancel, open_serial, prop, serial};

/// 发送预览：同一份 KISS 字节既用来显示 hex，也用来真正写串口（不重复编码）。
#[derive(Clone, PartialEq)]
struct Preview {
  /// 完整 KISS 帧（含 FEND 定界与命令字节）。
  kiss: Vec<u8>,
  /// AX.25 帧长（不含 KISS 封装）。
  frame_len: usize,
}

/// 收到的一帧（显示用）。
#[derive(Clone)]
struct RxRow {
  /// 简要文本：`src>dest: info` 或 hex。
  text: String,
  /// 完整 hex（帧长 = 长度 / 2）。
  hex: String,
}

#[component]
pub fn KissTncPage() -> impl IntoView {
  set_title("tools.kiss-tnc");

  let supported = serial().is_some();
  let connected = RwSignal::new(false);
  // 端口选择对话框弹出期间置 true：防波特率在「已点击、未打开」的窗口期被改。
  let opening = RwSignal::new(false);
  let error = RwSignal::new(None::<String>);
  let rows = RwSignal::new(VecDeque::<RxRow>::new());

  let baud = RwSignal::new(9600u32);
  // 选项在视图外预生成：view! 的属性里不能直接写 `vec![…]`（逗号会被当成属性分隔）。
  let baud_options: Vec<SelectOption> = vec![1200u32, 9600, 19200, 38400, 57600, 115_200]
    .into_iter()
    .map(|b| SelectOption {
      value: b.to_string(),
      label: TextValue::from(b.to_string()),
      group: None,
    })
    .collect();
  let src = RwSignal::new(String::new());
  let dest = RwSignal::new("APRS".to_owned());
  let info = RwSignal::new(String::new());

  let conn = StoredValue::new_local(None::<Conn>);
  let writer_slot = StoredValue::new_local(None::<JsValue>);
  let generation = StoredValue::new(0u32);

  let disconnect = move || {
    // try_*：写失败回调与卸载清理可能交错，已释放的槽位直接忽略而不是 panic。
    generation.try_update_value(|g| *g = g.wrapping_add(1));
    writer_slot.try_update_value(Option::take);
    if let Some(c) = conn.try_update_value(Option::take).flatten() {
      spawn_local(close_serial(c));
    }
    connected.set(false);
  };

  let connect = move || {
    let my = generation.get_value();
    error.set(None);
    opening.set(true);
    spawn_local(async move {
      let outcome = open_serial(baud.get_untracked()).await;
      // 端口选择框可能开着很久：这期间用户切走路由或点了断开，就不能再写回信号
      // （卸载后 set 是 no-op，连接状态与提示会静默丢失），端口直接关掉。
      if generation.try_get_value() != Some(my) {
        if let Ok(c) = outcome {
          spawn_local(close_serial(c));
        }
        return;
      }
      opening.set(false);
      match outcome {
        Ok(c) => {
          // Conn 不 Clone：把读写句柄先拷出来再存（照 rotator 的模式）。
          let (reader, writer) = (c.reader.clone(), c.writer.clone());
          conn.set_value(Some(c));
          writer_slot.set_value(Some(writer));
          connected.set(true);
          // 读循环：字节喂给 KISS 流式解码器，完成的帧解析后进列表。
          let alive = move || generation.try_get_value() == Some(my);
          spawn_local(async move {
            let mut dec = KissDecoder::new();
            while alive() {
              // 读失败或流结束（拔线 / TNC 关机）都属意外断开：走 disconnect 收尾
              // 并提示，避免界面永远停在「已连接」。
              let chunk = match call(&reader, "read", &[]).await {
                Ok(r) => r,
                Err(_) => {
                  // 主动断开时 generation 已变更：`alive()` 只在循环顶部检查，
                  // 挡不住这次已在 await 中的读 —— 这里再查一次，避免把用户
                  // 自己点的「断开」报成「连接丢失」。
                  if alive() {
                    disconnect();
                    error.set(Some(t("tools.kiss-connection-lost")));
                  }
                  break;
                }
              };
              if prop(&chunk, "done").ok().is_some_and(|d| d.is_truthy()) {
                if alive() {
                  disconnect();
                  error.set(Some(t("tools.kiss-connection-lost")));
                }
                break;
              }
              let Ok(v) = prop(&chunk, "value") else {
                continue;
              };
              let Ok(arr) = v.dyn_into::<js_sys::Uint8Array>() else {
                continue;
              };
              for frame in dec.feed(&arr.to_vec()) {
                let text = match parse_ui_frame(&frame) {
                  Some(f) => format!(
                    "{}>{}:{}",
                    f.source,
                    f.destination,
                    String::from_utf8_lossy(&f.info)
                  ),
                  None => hex(&frame),
                };
                let hex_text = hex(&frame);
                rows.update(|q| {
                  if q.len() >= 50 {
                    q.pop_front();
                  }
                  q.push_back(RxRow {
                    text,
                    hex: hex_text,
                  });
                });
              }
            }
          });
        }
        Err(e) => {
          if !is_user_cancel(&e) {
            error.set(Some(js_error_message(&e)));
          }
        }
      }
    });
  };

  // 帧预览：编解码全走核心（与串口无关，离线可用）。Memo 只在呼号 / 信息变化时重算，
  // 渲染、按钮禁用态与发送共用同一份结果。
  let preview = Memo::new(move |_| {
    let frame = ax25_ui_frame(
      dest.get().trim(),
      src.get().trim(),
      &[],
      info.get().as_bytes(),
    )?;
    Some(Preview {
      kiss: encode_frame(0, &frame),
      frame_len: frame.len(),
    })
  });

  let send = move |_| {
    let Some(Preview { kiss, .. }) = preview.get_untracked() else {
      error.set(Some(t("tools.kiss-invalid-call")));
      return;
    };
    let Some(writer) = writer_slot.get_value() else {
      return;
    };
    let bytes = js_sys::Uint8Array::from(kiss.as_slice());
    let my = generation.get_value();
    spawn_local(async move {
      if call(&writer, "write", &[bytes.into()]).await.is_err() {
        // 与读循环相同的守卫：用户已主动断开 / 离开页面（代际变了或已释放）时，
        // 这次失败只是关端口的副作用，不能再报「连接丢失」。
        if generation.try_get_value() != Some(my) {
          return;
        }
        // 写失败说明端口已不可用（拔线 / TNC 关机）：与读循环一样断开并提示，
        // 否则用户会以为帧已经发出去了。
        disconnect();
        error.set(Some(t("tools.kiss-connection-lost")));
      }
    });
  };

  // 离开页面时关闭串口：读循环任务随组件卸载被取消，但端口必须显式 close，
  // 否则浏览器一直持有该串口（disconnect 幂等：take + generation 守卫）。
  on_cleanup(disconnect);

  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <PageHeader
        title=move || t("tools.kiss-tnc")
        subtitle=move || t("tools.kiss-tnc-subtitle")
        actions=ViewFn::from(move || {
          view! {
            <ButtonLink href="/packet" variant=Variant::Outline size=Size::Sm>
              {move || t("tools.kiss-reference")}
            </ButtonLink>
          }
        })
      />

      <PageContainer class="space-y-5">
        <section class="border bg-card p-4 text-sm text-muted-foreground">
          {move || t("tools.kiss-note")}
        </section>

        <section class="border bg-card p-4">
          <div class="flex flex-wrap items-end gap-3">
            {{
              let baud_id = unique_id("kiss-baud");
              let label_for = baud_id.clone();
              view! {
                <Field
                  label=Signal::derive(move || t("common.baud-rate"))
                  r#for=label_for
                  class="sm:max-w-44"
                >
                  <NativeSelect
                    id=baud_id
                    value=Signal::derive(move || baud.get().to_string())
                    on_change=Callback::new(move |v: String| {
                      if let Ok(b) = v.trim().parse::<u32>() {
                        baud.set(b);
                      }
                    })
                    options=baud_options.clone()
                    disabled=move || connected.get() || opening.get()
                  />
                </Field>
              }
            }}
            <div class="ml-auto flex items-center gap-2">
              {move || {
                if !supported {
                  view! { <span class="text-xs text-destructive">{t("common.this-browser-does-not")}</span> }
                    .into_any()
                } else if connected.get() {
                  view! {
                    <Button variant=Variant::Outline on_click=move |_| disconnect()>
                      {t("common.disconnect")}
                    </Button>
                  }
                    .into_any()
                } else {
                  view! {
                    <Button on_click=move |_| connect() disabled=move || opening.get()>
                      {t("tools.kiss-connect")}
                    </Button>
                  }
                    .into_any()
                }
              }}
            </div>
          </div>
          {move || {
            error.get().map(|e| {
              view! { <p class="mt-3 text-xs text-destructive">{e}</p> }
            })
          }}
        </section>

        <section class="border bg-card p-4">
          <h2 class="text-sm font-semibold">{move || t("tools.kiss-rx")}</h2>
          <div class="mt-3 space-y-1.5">
            {move || {
              if rows.with(VecDeque::is_empty) {
                view! {
                  <EmptyState plain=true title=t("tools.kiss-rx-empty") />
                }
                  .into_any()
              } else {
                rows.get()
                  .iter()
                  .map(|r| {
                    // Tooltip children 需要 `'static`：字段先拷成 owned。
                    let hex = r.hex.clone();
                    let text = r.text.clone();
                    view! {
                      <Tooltip label=hex.clone()>
                        <div class="flex flex-wrap items-baseline gap-2 bg-muted/40 px-3 py-2 text-xs">
                          <span class="font-mono text-muted-foreground">
                            [{tf("tools.kiss-rx-bytes", &[&(hex.len() / 2).to_string()])}]
                          </span>
                          <span class="min-w-0 flex-1 break-all font-mono">{text}</span>
                        </div>
                      </Tooltip>
                    }
                  })
                  .collect_view()
                  .into_any()
              }
            }}
          </div>
        </section>

        <section class="border bg-card p-4">
          <h2 class="text-sm font-semibold">{move || t("tools.kiss-tx")}</h2>
          <div class="mt-3 grid gap-3 sm:grid-cols-3">
            {{
              let src_id = unique_id("kiss-src");
              let label_for = src_id.clone();
              view! {
                <Field
                  label=Signal::derive(move || t("tools.kiss-src"))
                  r#for=label_for
                >
                  <Input
                    id=src_id
                    placeholder=t("tools.kiss-src-placeholder")
                    value=Signal::derive(move || src.get())
                    on_change=Callback::new(move |v: String| src.set(v))
                  />
                </Field>
              }
            }}
            {{
              let dest_id = unique_id("kiss-dest");
              let label_for = dest_id.clone();
              view! {
                <Field
                  label=Signal::derive(move || t("tools.kiss-dest"))
                  r#for=label_for
                >
                  <Input
                    id=dest_id
                    value=Signal::derive(move || dest.get())
                    on_change=Callback::new(move |v: String| dest.set(v))
                  />
                </Field>
              }
            }}
            {{
              let info_id = unique_id("kiss-info");
              let label_for = info_id.clone();
              view! {
                <Field
                  label=Signal::derive(move || t("tools.kiss-info"))
                  r#for=label_for
                >
                  <Input
                    id=info_id
                    placeholder=t("tools.kiss-info-placeholder")
                    value=Signal::derive(move || info.get())
                    on_change=Callback::new(move |v: String| info.set(v))
                  />
                </Field>
              }
            }}
          </div>

          <div class="mt-4 flex flex-wrap items-center gap-3">
            {move || {
              preview.get().map(|p| {
                let hex_text = hex(&p.kiss);
                view! {
                  <span class="text-xs text-muted-foreground">
                    {tf("tools.kiss-preview", &[&p.frame_len.to_string()])}
                    " "
                    <code class="break-all bg-muted/60 px-1.5 py-0.5 font-mono">
                      {hex_text}
                    </code>
                  </span>
                }
              })
            }}
            <Button
              class="ml-auto"
              variant=Variant::Outline
              disabled=move || !connected.get() || preview.get().is_none()
              on_click=send
            >
              {move || t("tools.kiss-send")}
            </Button>
          </div>
        </section>
      </PageContainer>
    </div>
  }
}

/// 字节 → 大写 hex 串。
fn hex(bytes: &[u8]) -> String {
  bytes.iter().map(|b| format!("{b:02X}")).collect()
}
