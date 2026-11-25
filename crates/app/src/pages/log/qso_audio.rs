//! 通联录音：给一条通联挂一段本机录音（站内录一段，或选一个音频文件），存 IndexedDB。
//!
//! 规则与 [`super::qsl_image`]（QSL 卡片影像）一一对应，注释里的理由同样适用：本体走
//! [`crate::kv::save_store_only`]（只有权威层、写完才算数，所以保存期间显示「保存中」、成功后才放出播放器）；
//! localStorage 里只留一个 id 索引当「提示」。
//!
//! 与影像**不同**的地方：录音**不上传、不同步**，默认也不进备份（备份里是单独勾选的「通联录音」模块）。
//! 核心里的类型 / 体积 / 值校验见 `ham_web_core::qso_audio`。

use ham_web_core::qso_audio::{self, Reject};
use leptos::prelude::*;
use leptos::task::spawn_local;
use web_sys::{Blob, File};

use crate::i18n::t;
use crate::mic_recorder::{self, Recording};
use crate::ui::{Button, FileInput, Size, Variant};
use crate::util::storage;

/// 读索引：哪些通联挂了录音。
fn index() -> std::collections::BTreeSet<u64> {
  storage::get_json::<Vec<u64>>(qso_audio::INDEX_KEY)
    .map(|ids| ids.into_iter().collect())
    .unwrap_or_default()
}

/// 把索引写回 localStorage（空集合就删掉 key，不留一个空数组）。
fn write_index(ids: &std::collections::BTreeSet<u64>) {
  if ids.is_empty() {
    if let Some(ls) = crate::util::window().local_storage().ok().flatten() {
      let _ = ls.remove_item(qso_audio::INDEX_KEY);
    }
  } else {
    let sorted: Vec<u64> = ids.iter().copied().collect();
    storage::set_json(qso_audio::INDEX_KEY, &sorted);
  }
}

fn mark(entry_id: u64, has: bool) {
  let mut ids = index();
  if has {
    ids.insert(entry_id);
  } else {
    ids.remove(&entry_id);
  }
  write_index(&ids);
}

/// 删除若干通联的录音与索引条目（删记录 / 清空日志时调用）。
///
/// 录音的 key 是 `qso-audio:<id>`，而 id 由 `Logbook::next_id()`（`max + 1`）分配——删掉 id 最大的那条之后，
/// 下一个新通联会**复用**同一个 id。不清理的话，新记录会直接挂上已删记录的录音（隐私与数据正确性都有问题）。
pub(super) fn purge(ids: &[u64]) {
  if ids.is_empty() {
    return;
  }
  let mut set = index();
  for id in ids {
    set.remove(id);
  }
  write_index(&set);
  let keys: Vec<String> = ids.iter().map(|id| qso_audio::key_for(*id)).collect();
  // 本体走异步删除：`remove_store_only` 没有快照可回退，删不掉的只是残留，不影响日志本身，所以不等它。
  spawn_local(async move {
    for key in keys {
      let _ = crate::kv::remove_store_only(&key).await;
    }
  });
}

/// 界面反馈。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Issue {
  /// 类型不支持 / 太大 / 空文件。
  Rejected(Reject),
  /// 写 IndexedDB 失败。
  StoreFailed,
  /// 删 IndexedDB 失败：录音还在，索引也得留着，不能装作已删。
  DeleteFailed,
  /// 没有麦克风 / 没授权。
  NoMic,
}

/// 按调用点字面量取文案（静态扫描靠它认条目）。
fn issue_message(issue: Issue) -> String {
  match issue {
    Issue::Rejected(Reject::UnsupportedType) => t("log.audio-unsupported"),
    Issue::Rejected(Reject::TooLarge) => t("log.audio-too-large"),
    Issue::Rejected(Reject::Empty) => t("log.audio-empty"),
    Issue::StoreFailed => t("log.audio-save-failed"),
    Issue::DeleteFailed => t("log.audio-delete-failed"),
    Issue::NoMic => t("log.audio-no-mic"),
  }
}

/// 通联录音面板：播放器 + 选文件 / 站内录音 / 替换 / 删除。
///
/// 只对**已保存**的通联渲染（录音按记录 id 关联，新记录还没有 id）。
#[component]
pub(super) fn QsoAudio(entry_id: u64) -> impl IntoView {
  let key = StoredValue::new(qso_audio::key_for(entry_id));
  let stored = RwSignal::new(None::<String>);
  let issue = RwSignal::new(None::<Issue>);
  let busy = RwSignal::new(false);
  let recording = RwSignal::new(false);
  let rec = StoredValue::new_local(None::<Recording>);
  // 代次与卸载守卫：理由同 `QslImage`——`await`（麦克风授权、文件读取、IndexedDB）期间用户可能已经
  // 离开 `/log` 或切换了编辑对象，那时这些信号已随 Owner 释放。
  let generation = StoredValue::new(0u32);
  let alive = crate::util::mount_guard();
  let alive_read = alive.clone();
  let alive_attach = alive.clone();
  let alive_record = alive.clone();
  let alive_remove = alive;

  // 离开时放掉麦克风（正在录的话）。
  on_cleanup(move || {
    generation.try_update_value(|g| *g = g.wrapping_add(1));
    if let Some(r) = rec.try_update_value(Option::take).flatten() {
      r.cancel();
    }
  });

  // 挂载时读一次已有录音；不像本站写出的数据就当没有。
  Effect::new(move |_| {
    let key = key.get_value();
    let token = generation.get_value();
    let alive = alive_read.clone();
    spawn_local(async move {
      let loaded = crate::kv::load_store_only(&key)
        .await
        .filter(|v| qso_audio::is_valid_stored_value(v));
      if !alive() || generation.try_get_value() != Some(token) {
        return;
      }
      stored.set(loaded);
    });
  });

  // 把一段音频（选的文件或刚录的）存下来：先校验、读成 dataURL、规范化，写完才放出播放器。
  let attach = Callback::new(move |blob: Blob| {
    issue.set(None);
    if let Err(reason) = qso_audio::check(&blob.type_(), blob.size()) {
      issue.set(Some(Issue::Rejected(reason)));
      return;
    }
    busy.set(true);
    let token = generation.get_value() + 1;
    generation.set_value(token);
    let alive = alive_attach.clone();
    spawn_local(async move {
      let url = crate::audio_blob::data_url_of(&blob).await;
      if !alive() || generation.try_get_value() != Some(token) {
        return;
      }
      let Some(value) = url.as_deref().and_then(qso_audio::normalize_data_url) else {
        issue.set(Some(Issue::Rejected(Reject::UnsupportedType)));
        busy.set(false);
        return;
      };
      let saved = crate::kv::save_store_only(&key.get_value(), &value).await;
      // 落盘期间用户可能点了删除 / 换了对象：此时把录音写回去就与「删除已生效」矛盾。
      if !alive() || generation.get_value() != token {
        return;
      }
      if saved {
        stored.set(Some(value));
        mark(entry_id, true);
      } else {
        issue.set(Some(Issue::StoreFailed));
      }
      busy.set(false);
    });
  });

  let on_files = Callback::new(move |files: Vec<File>| {
    if let Some(file) = files.into_iter().next() {
      attach.run(file.into());
    }
  });

  let start_record = Callback::new(move |()| {
    issue.set(None);
    let token = generation.get_value() + 1;
    generation.set_value(token);
    let alive = alive_record.clone();
    spawn_local(async move {
      let outcome = mic_recorder::start().await;
      // 授权弹窗开着的时候用户可能已经离开 / 换了对象：这次录音不要了，麦克风立刻放掉。
      if !alive() || generation.try_get_value() != Some(token) {
        if let Ok(r) = outcome {
          r.cancel();
        }
        return;
      }
      match outcome {
        Ok(r) => {
          rec.set_value(Some(r));
          recording.set(true);
        }
        Err(_) => issue.set(Some(Issue::NoMic)),
      }
    });
  });

  let stop_record = Callback::new(move |()| {
    recording.set(false);
    let Some(r) = rec.try_update_value(Option::take).flatten() else {
      return;
    };
    busy.set(true);
    spawn_local(async move {
      match r.stop_blob().await {
        Some(blob) => {
          busy.set(false);
          attach.run(blob);
        }
        None => {
          issue.set(Some(Issue::Rejected(Reject::Empty)));
          busy.set(false);
        }
      }
    });
  });

  let remove = move || {
    let guard = alive_remove.clone();
    crate::ui::confirm(
      t("log.delete-audio-confirm"),
      "",
      t("log.delete-audio"),
      crate::ui::ConfirmTone::Danger,
      Callback::new(move |_| {
        issue.set(None);
        busy.set(true);
        // 本次删除也让在途的保存作废（否则它可能刚删完就把录音写回去）。
        generation.set_value(generation.get_value() + 1);
        let alive = guard.clone();
        spawn_local(async move {
          // 删失败不能装作删掉了：录音还在 IndexedDB 里，索引与播放器都得留着。
          let removed = crate::kv::remove_store_only(&key.get_value()).await;
          if !alive() {
            return;
          }
          if removed {
            stored.set(None);
            mark(entry_id, false);
          } else {
            issue.set(Some(Issue::DeleteFailed));
          }
          busy.set(false);
        });
      }),
    );
  };

  view! {
    <div class="flex flex-col gap-2 sm:col-span-2 lg:col-span-3" data-testid="qso-audio">
      <span class="text-xs text-muted-foreground">{move || t("log.qso-audio")}</span>
      <span class="text-xs text-muted-foreground">{move || t("log.qso-audio-hint")}</span>
      {move || {
        stored
          .get()
          .map(|src| {
            view! {
              <audio
                controls=true
                src=src
                class="w-full max-w-md"
                data-testid="qso-audio-player"
              ></audio>
            }
          })
      }}
      <div class="flex flex-wrap items-center gap-2">
        {move || {
          if recording.get() {
            view! {
              <Button
                variant=Variant::Default
                size=Size::Sm
                on_click=stop_record
              >
                {t("log.stop-recording")}
              </Button>
              <span class="text-xs text-hp-text" role="status">{t("log.recording-now")}</span>
            }
              .into_any()
          } else {
            view! {
              <FileInput
                accept="audio/*"
                label=Signal::derive(move || {
                  if stored.with(Option::is_some) {
                    t("log.replace-audio-file")
                  } else {
                    t("log.attach-audio-file")
                  }
                })
                variant=Variant::Outline
                size=Size::Sm
                disabled=Signal::derive(move || busy.get())
                on_files=on_files
              />
              <Button
                variant=Variant::Outline
                size=Size::Sm
                disabled=Signal::derive(move || busy.get())
                on_click=start_record
              >
                {move || {
                  if stored.with(Option::is_some) {
                    t("log.record-audio-again")
                  } else {
                    t("log.record-audio")
                  }
                }}
              </Button>
            }
              .into_any()
          }
        }}
        {move || {
          stored
            .with(Option::is_some)
            .then(|| {
              let remove = remove.clone();
              view! {
                <Button
                  variant=Variant::Ghost
                  size=Size::Sm
                  disabled=Signal::derive(move || busy.get() || recording.get())
                  on_click=Callback::new(move |()| remove())
                >
                  {t("log.delete-audio")}
                </Button>
              }
            })
        }}
      </div>
      <Show when=move || busy.get()>
        <span class="text-xs text-muted-foreground">{move || t("log.audio-saving")}</span>
      </Show>
      {move || {
        issue
          .get()
          .map(|issue| {
            view! {
              <span class="text-xs text-hp-text" role="alert" data-testid="qso-audio-issue">
                {issue_message(issue)}
              </span>
            }
          })
      }}
    </div>
  }
}
