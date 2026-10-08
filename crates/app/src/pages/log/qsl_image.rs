//! QSL 卡片影像：上传扫描件 → 缩放 → 存 IndexedDB，与通联记录按 id 关联。
//!
//! 列表里的「有卡片影像」标记见 [`super::card_image_mark::CardImageMark`]。
//!
//! # 为什么单独占一块存储
//!
//! 影像本体是**大体积数据**：既不该进日志 JSON（每改一条通联就要重写整个日志、还要双写
//! localStorage 快照），也不该进 localStorage 快照本身（5MB 配额）。所以分成两层：
//!
//! - 影像走 [`crate::kv::save_store_only`]：只有 IndexedDB 权威层，key 是 `qsl-image:<id>`；
//! - 「哪些通联存了影像」只留一个 **id 集合**在 localStorage（小数据、同步可读），
//!   供列表直接渲染标记 —— 否则每行都要发一次异步查询。
//!
//! 两层可能不一致（例如清过 IndexedDB），所以索引只当**提示**：真正有没有影像，以
//! 本组件读到的进度为准；读不到就当作没有，用户重新上传即可。
//!
//! # 为什么上传要 `await` 落盘
//!
//! [`crate::kv::save_store_only`] 没有快照可回退，写完才算数；所以上传期间显示「保存中」、
//! 成功后才把缩略图放出来，绝不先亮图再写盘（那样刷新一下图就没了，用户会以为存住了）。

use std::collections::HashSet;

use ham_web_core::qsl_image::{self, QslImageReject};
use leptos::prelude::*;
use leptos::task::spawn_local;
use web_sys::File;

use crate::components::common::PreviewableImage;
use crate::i18n::t;
use crate::photo::{PhotoError, PhotoKind, is_image, process_photo};
use crate::ui::{Button, FileInput, Size, Variant};
use crate::util::{storage, window};

use super::use_log_store;

/// 「哪些通联存了卡片影像」的 id 集合（localStorage 里的小数据）。
/// `pub(super)`：日志 store 要监听它的跨标签页变化。
pub(super) const INDEX_KEY: &str = "qsl-image-index";

/// 读索引。
pub(super) fn index() -> HashSet<u64> {
  storage::get_json::<Vec<u64>>(INDEX_KEY)
    .map(|ids| ids.into_iter().collect())
    .unwrap_or_default()
}

/// 更新索引：只改内存里的响应式集合 + 同步写回 localStorage（供跨标签页与下次启动使用）。
fn mark(store: super::LogStore, entry_id: u64, has: bool) {
  store.qsl_images.update(|ids| {
    if has {
      ids.insert(entry_id);
    } else {
      ids.remove(&entry_id);
    }
  });
  let mut sorted: Vec<u64> = store.qsl_images.get_untracked().into_iter().collect();
  sorted.sort_unstable();
  storage::set_json(INDEX_KEY, &sorted);
}

/// 删除若干通联的影像与索引条目（删记录 / 清空日志时调用）。
///
/// 影像的 key 是 `qsl-image:<id>`，而 id 由 `Logbook::next_id()`（`max + 1`）分配 ——
/// 删掉 id 最大的那条之后，下一个新通联会**复用**同一个 id。不清理的话，新记录会直接
/// 挂上已删记录的扫描件（隐私与数据正确性都有问题）。
pub(super) fn purge(store: super::LogStore, ids: &[u64]) {
  if ids.is_empty() {
    return;
  }
  let keys: Vec<String> = ids.iter().map(|id| qsl_image::key_for(*id)).collect();
  store.qsl_images.update(|set| {
    for id in ids {
      set.remove(id);
    }
  });
  let mut sorted: Vec<u64> = store.qsl_images.get_untracked().into_iter().collect();
  sorted.sort_unstable();
  storage::set_json(INDEX_KEY, &sorted);
  // 影像本体走异步删除：`remove_store_only` 没有快照可回退，删不掉的只是残留，
  // 不影响日志本身，所以不等它。
  spawn_local(async move {
    for key in keys {
      let _ = crate::kv::remove_store_only(&key).await;
    }
  });
}

/// 界面反馈：比 [`QslImageReject`] 多两档本地存储相关的问题。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Issue {
  /// 类型不支持 / 图片过大。
  Unsupported(QslImageReject),
  /// 写 IndexedDB 失败。
  StoreFailed,
  /// 删 IndexedDB 失败：影像还在，索引也得留着，不能装作已删。
  DeleteFailed,
}

/// 按调用点字面量取文案（静态扫描靠它认条目）。
fn issue_message(issue: Issue) -> String {
  match issue {
    Issue::Unsupported(QslImageReject::UnsupportedType) => t("log.card-image-unsupported"),
    Issue::Unsupported(QslImageReject::TooLarge) => t("log.card-image-too-large"),
    Issue::StoreFailed => t("log.card-image-save-failed"),
    Issue::DeleteFailed => t("log.card-image-delete-failed"),
  }
}

/// 卡片影像面板：缩略图（点击放大）+ 上传 / 替换 / 删除。
///
/// 只对**已保存**的通联渲染（影像按记录 id 关联，新记录还没有 id）。
#[component]
pub(super) fn QslImage(entry_id: u64) -> impl IntoView {
  let store = use_log_store();
  let key = StoredValue::new(qsl_image::key_for(entry_id));
  let stored = RwSignal::new(None::<String>);
  let issue = RwSignal::new(None::<Issue>);
  let busy = RwSignal::new(false);
  // 上传代次：连点「换一张」会并发起多个任务，只让**最后**发起的那次写结果 ——
  // 否则先完成的会被后完成的覆盖，最终存下的可能不是用户最后选的那张。
  let generation = StoredValue::new(0u32);
  // 卸载守卫：`spawn_local` 的续体不会被取消，而 `await`（IndexedDB 读写、图片解码）
  // 期间用户可能已经离开 `/log` 或切换了编辑对象 —— 那时这些信号已随 Owner 释放，
  // 再 `set` 会直接 panic，把整个 wasm 实例带走（见 `util::mount_guard`）。
  // 守卫必须在**组件体**里取：事件回调 / 定时器回调里拿不到 Owner 上下文。
  let alive = crate::util::mount_guard();
  let alive_read = alive.clone();
  let alive_upload = alive.clone();
  let alive_remove = alive;

  // 挂载时读一次已有影像（`entry_id` 变了会整块重建，所以不需要再跟踪它）。
  Effect::new(move |_| {
    let key = key.get_value();
    let alive = alive_read.clone();
    spawn_local(async move {
      let loaded = crate::kv::load_store_only(&key).await;
      if !alive() {
        return;
      }
      stored.set(loaded);
      busy.set(false);
    });
  });

  let on_files = Callback::new(move |files: Vec<File>| {
    let Some(file) = files.into_iter().next() else {
      return;
    };
    issue.set(None);
    // 先按**原始文件**校验：宁可当场拒绝，也不要在浏览器里解码一个 50MB 的东西。
    if !is_image(&file) {
      issue.set(Some(Issue::Unsupported(QslImageReject::UnsupportedType)));
      return;
    }
    if let Err(reason) = qsl_image::check(&file.type_(), file.size()) {
      issue.set(Some(Issue::Unsupported(reason)));
      return;
    }
    busy.set(true);
    let token = generation.get_value() + 1;
    generation.set_value(token);
    let alive = alive_upload.clone();
    spawn_local(async move {
      // 更晚的一次上传已经开始：这次的结果作废（连 `busy` 也交给它收尾）。
      if generation.get_value() != token {
        return;
      }
      let photo = process_photo(PhotoKind::QslCard, file).await;
      // 解码期间组件被卸载、或用户又选了别的：不动信号，也不要把这次的图盖上去。
      if !alive() || generation.get_value() != token {
        return;
      }
      match photo {
        Ok(photo) => {
          let saved = crate::kv::save_store_only(&key.get_value(), &photo.data_url).await;
          // `generation` 必须在 `await` **之后**再比一次：落盘期间用户可能点了删除
          // （`generation += 1` + 删 IndexedDB），此时把图写回去就与「删除已生效」矛盾 ——
          // 缩略图与索引都会复活，注释里承诺的「在途上传作废」也就不成立了。
          if !alive() || generation.get_value() != token {
            return;
          }
          if saved {
            stored.set(Some(photo.data_url));
            mark(store, entry_id, true);
          } else {
            issue.set(Some(Issue::StoreFailed));
          }
        }
        // 解码后像素超限单独成一档：让用户知道是图太大，而不是「类型不支持」。
        Err(PhotoError::TooLarge) => {
          issue.set(Some(Issue::Unsupported(QslImageReject::TooLarge)));
        }
        // 解码失败（伪装成图片的坏文件、canvas 拿不到上下文……）都归到「不支持的类型」。
        Err(_) => issue.set(Some(Issue::Unsupported(QslImageReject::UnsupportedType))),
      }
      busy.set(false);
    });
  });

  let remove = move || {
    if !window()
      .confirm_with_message(&t("log.delete-card-image-confirm"))
      .unwrap_or(false)
    {
      return;
    }
    issue.set(None);
    busy.set(true);
    // 本次删除也会让在途的上传作废（否则它可能刚删完就把图写回去）：上传侧在
    // `await` 之后会比一次 `generation`，发现代次变了就直接放弃。
    generation.set_value(generation.get_value() + 1);
    let alive = alive_remove.clone();
    spawn_local(async move {
      // 删失败不能装作删掉了：影像还在 IndexedDB 里，索引与缩略图都得留着，
      // 否则会出现「索引里没有、数据其实还在」的静默残留。
      let removed = crate::kv::remove_store_only(&key.get_value()).await;
      if !alive() {
        return;
      }
      if removed {
        stored.set(None);
        mark(store, entry_id, false);
      } else {
        issue.set(Some(Issue::DeleteFailed));
      }
      busy.set(false);
    });
  };

  view! {
    <div class="flex flex-col gap-2 sm:col-span-2 lg:col-span-3">
      <span class="text-xs text-muted-foreground">{move || t("log.qsl-card-image")}</span>
      {move || {
        stored
          .get()
          .map(|src| {
            // `remove` 捕获了卸载守卫（`Clone` 但**不** `Copy`），而挂载守卫的闭包要
            // 在这里被 `move` 进 `Callback`：先克隆一份，外层渲染闭包才能保持 `FnMut`
            // （否则整块视图变成 `FnOnce`，Leptos 无法重渲染）。
            let remove = remove.clone();
            view! {
              <div class="flex flex-wrap items-start gap-3">
                <div class="w-40 shrink-0">
                  <PreviewableImage
                    src=src
                    alt=t("log.qsl-card-image")
                    title=t("log.qsl-card-image")
                  />
                </div>
                <div class="flex flex-col items-start gap-2">
                  <FileInput
                    accept="image/*"
                    label=t("log.replace-card-image")
                    variant=Variant::Outline
                    size=Size::Sm
                    disabled=Signal::derive(move || busy.get())
                    on_files=on_files
                  />
                  <Button
                    variant=Variant::Ghost
                    size=Size::Sm
                    disabled=Signal::derive(move || busy.get())
                    on_click=Callback::new(move |()| remove())
                  >
                    {move || t("log.delete-card-image")}
                  </Button>
                </div>
              </div>
            }
          })
      }}
      {move || {
        stored
          .get()
          .is_none()
          .then(|| {
            view! {
              <FileInput
                accept="image/*"
                label=t("log.upload-card-image")
                variant=Variant::Outline
                size=Size::Sm
                disabled=Signal::derive(move || busy.get())
                on_files=on_files
              />
            }
          })
      }}
      <Show when=move || busy.get()>
        <span class="text-xs text-muted-foreground">{move || t("log.card-image-saving")}</span>
      </Show>
      {move || {
        issue
          .get()
          .map(|issue| view! { <span class="text-xs text-destructive">{issue_message(issue)}</span> })
      }}
    </div>
  }
}
