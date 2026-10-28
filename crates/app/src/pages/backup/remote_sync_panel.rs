use ham_web_core::remote_sync::{RemoteSyncConfig, WriteMethod};
use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::auto_sync::AutoStatus;
use crate::i18n::{t, tf, tp};
use crate::remote_sync::{self, PullResult};
use crate::ui::{
  Button, Checkbox, ConfirmTone, Field, Input, InputType, NativeSelect, SelectOption, Textarea,
  ToastKind, Variant,
};

/// 三个同步动作。
#[derive(Clone, Copy, PartialEq, Eq)]
enum Action {
  PullOverwrite,
  PullMerge,
  Push,
}

/// 改配置并立即落盘：字段很少、体积很小，逐次保存比再配一个「保存」按钮少一种中间状态。
fn edit(cfg: RwSignal<RemoteSyncConfig>, apply: impl FnOnce(&mut RemoteSyncConfig)) {
  cfg.update(apply);
  remote_sync::save_config(&cfg.get_untracked());
}

/// 自定义远端同步：用户配一个「获取」接口和一个「写入」接口，自己管理那份备份 JSON。
///
/// 内容、合并规则都与手动导入 / 导出一致（见 `crate::remote_sync`）；这里只负责配置表单与
/// 三个动作按钮。配置不进备份文件（见 `ham_web_core::remote_sync::is_excluded_from_backup`）。
/// 写出的范围永远是完整的，不跟「导出备份」的模块勾选走（理由见 `crate::remote_sync` 模块文档）。
#[component]
pub(super) fn RemoteSyncPanel(
  /// 导入 / 导出正在进行。
  busy: Signal<bool>,
  /// 同步进行中；由父组件持有，好让导入 / 导出按钮一并禁用。
  syncing: RwSignal<bool>,
  /// 动作结束（无论成败）后的回调，父组件用它刷新存储占用。
  on_done: Callback<()>,
) -> impl IntoView {
  let cfg = RwSignal::new(remote_sync::config());
  let auto_status = crate::auto_sync::status();
  let alive = StoredValue::new(crate::util::mount_guard());
  let log_store = crate::pages::log::use_log_store();

  let get_error = Signal::derive(move || {
    cfg
      .with(|c| remote_sync::url_error(&c.get_url))
      .unwrap_or_default()
  });
  let put_error = Signal::derive(move || {
    cfg
      .with(|c| remote_sync::url_error(&c.put_url))
      .unwrap_or_default()
  });
  let get_headers_error = Signal::derive(move || {
    cfg
      .with(|c| remote_sync::headers_error(&c.get_headers))
      .unwrap_or_default()
  });
  let put_headers_error = Signal::derive(move || {
    cfg
      .with(|c| remote_sync::headers_error(&c.put_headers))
      .unwrap_or_default()
  });

  // 没填、或填了但不合格，对应动作都不可用；不合格的原因已经显示在输入框下面。
  let locked = move || busy.get() || syncing.get();
  let pull_disabled = Signal::derive(move || {
    locked()
      || !cfg.with(RemoteSyncConfig::can_pull)
      || !get_error.get().is_empty()
      || !get_headers_error.get().is_empty()
  });
  let push_disabled = Signal::derive(move || {
    locked()
      || !cfg.with(RemoteSyncConfig::can_push)
      || !put_error.get().is_empty()
      || !put_headers_error.get().is_empty()
  });

  let run = move |action: Action| {
    let Some(task) = crate::util::begin_backup_task(log_store.backup_busy) else {
      return;
    };
    syncing.set(true);
    let config = cfg.get_untracked();
    let alive = alive.get_value();
    spawn_local(async move {
      let (kind, text) = match action {
        Action::Push => match remote_sync::push(&config).await {
          Ok(pushed) => {
            // 写出的总是完整快照，远端现在就等于本机，可以当自动同步的基线。
            crate::auto_sync::note_pushed(pushed.fp);
            (
              ToastKind::Success,
              tp(
                "tools.remote-pushed",
                pushed.count as u32,
                &[&pushed.count.to_string()],
              ),
            )
          }
          Err(e) => (ToastKind::Error, e),
        },
        Action::PullOverwrite | Action::PullMerge => {
          let merge = action == Action::PullMerge;
          let outcome = remote_sync::pull(&config, merge).await;
          // 恢复可能写了一半才失败，所以无论成败都把日志 Store 对齐到存储（与手动导入一致）。
          if !matches!(outcome, Ok(PullResult::Empty)) {
            log_store.refresh_from_storage().await;
          }
          match outcome {
            Ok(PullResult::Empty) => (ToastKind::Info, t("tools.remote-no-data")),
            Ok(PullResult::Imported { count: n, fp }) => {
              crate::auto_sync::note_pulled(fp, merge);
              // 与手动导入同一套词条（走语义 key + `tp`，见 `backup_tool.rs` 的同类注释）。
              let msg = if merge {
                tp("settings.merged-items", n as u32, &[&n.to_string()])
              } else {
                tp("tools.restored-records", n as u32, &[&n.to_string()])
              };
              (ToastKind::Success, msg)
            }
            Err(e) => (ToastKind::Error, e),
          }
        }
      };
      drop(task);
      if !alive() {
        return;
      }
      syncing.set(false);
      crate::ui::push(kind, text);
      on_done.run(());
    });
  };

  // 覆盖会丢掉本机尚未同步出去的改动，而地址配错也会拉回别人的数据：先确认一次。
  let ask_overwrite = move |_| {
    crate::ui::confirm(
      t("tools.remote-pull-overwrite-title"),
      t("tools.remote-pull-overwrite-desc"),
      t("tools.remote-pull-overwrite"),
      ConfirmTone::Danger,
      Callback::new(move |()| run(Action::PullOverwrite)),
    );
  };

  let method_options: Vec<SelectOption> = WriteMethod::ALL
    .iter()
    .map(|m| SelectOption::new(m.as_str(), m.as_str()))
    .collect();

  view! {
    <div class="space-y-3 border bg-inset p-3">
      <h2 class="pxl-label text-base">{move || t("tools.remote-sync-title")}</h2>
      <p class="text-xs text-muted-foreground">{move || t("tools.remote-sync-desc")}</p>

      <Field
        label=Signal::derive(move || t("tools.remote-get-url"))
        r#for="remote-get-url"
        error=get_error
      >
        <Input
          id="remote-get-url"
          kind=InputType::Url
          value=Signal::derive(move || cfg.with(|c| c.get_url.clone()))
          on_change=Callback::new(move |v: String| {
            edit(cfg, move |c| c.get_url = v);
            crate::auto_sync::config_changed();
          })
          placeholder="https://example.com/ham-backup.json"
          autocomplete="off"
          autocapitalize="off"
          spellcheck="false"
          invalid=Signal::derive(move || !get_error.get().is_empty())
        />
      </Field>

      <Field
        label=Signal::derive(move || t("tools.remote-get-headers"))
        r#for="remote-get-headers"
        hint=Signal::derive(move || t("tools.remote-get-headers-hint"))
        error=get_headers_error
      >
        <Textarea
          id="remote-get-headers"
          rows=2
          autocomplete="off"
          autocapitalize="off"
          spellcheck="false"
          value=Signal::derive(move || cfg.with(|c| c.get_headers.clone()))
          on_change=Callback::new(move |v: String| edit(cfg, move |c| c.get_headers = v))
          placeholder="Authorization: Bearer <read-token>"
          invalid=Signal::derive(move || !get_headers_error.get().is_empty())
        />
      </Field>

      <Field
        label=Signal::derive(move || t("tools.remote-put-url"))
        r#for="remote-put-url"
        error=put_error
      >
        <Input
          id="remote-put-url"
          kind=InputType::Url
          value=Signal::derive(move || cfg.with(|c| c.put_url.clone()))
          on_change=Callback::new(move |v: String| {
            edit(cfg, move |c| c.put_url = v);
            crate::auto_sync::config_changed();
          })
          placeholder="https://example.com/ham-backup"
          autocomplete="off"
          autocapitalize="off"
          spellcheck="false"
          invalid=Signal::derive(move || !put_error.get().is_empty())
        />
      </Field>

      <Field
        label=Signal::derive(move || t("tools.remote-write-method"))
        r#for="remote-method"
        class="sm:w-40"
      >
        <NativeSelect
          id="remote-method"
          value=Signal::derive(move || cfg.with(|c| c.method.as_str().to_owned()))
          on_change=Callback::new(move |v: String| {
            edit(cfg, move |c| c.method = WriteMethod::parse(&v));
          })
          options=method_options
        />
      </Field>

      <Field
        label=Signal::derive(move || t("tools.remote-put-headers"))
        r#for="remote-put-headers"
        hint=Signal::derive(move || t("tools.remote-put-headers-hint"))
        error=put_headers_error
      >
        <Textarea
          id="remote-put-headers"
          rows=2
          autocomplete="off"
          autocapitalize="off"
          spellcheck="false"
          value=Signal::derive(move || cfg.with(|c| c.put_headers.clone()))
          on_change=Callback::new(move |v: String| edit(cfg, move |c| c.put_headers = v))
          placeholder="Authorization: Bearer <write-token>"
          invalid=Signal::derive(move || !put_headers_error.get().is_empty())
        />
      </Field>

      <div class="space-y-1">
        <div class="flex items-center gap-2">
          <Checkbox
            id="remote-exclude-qsl"
            checked=Signal::derive(move || cfg.with(|c| c.exclude_qsl_images))
            on_change=Callback::new(move |v: bool| {
              edit(cfg, move |c| c.exclude_qsl_images = v);
              // 同步范围变了：远端内容的形状也会变，让下一次自动写入按新范围重新比一次。
              crate::auto_sync::mark_dirty();
            })
            aria_label=Signal::derive(move || t("tools.remote-exclude-qsl"))
          />
          <span class="text-sm">{move || t("tools.remote-exclude-qsl")}</span>
        </div>
        <p class="text-xs text-muted-foreground">{move || t("tools.remote-exclude-qsl-hint")}</p>
      </div>

      <p class="text-xs text-muted-foreground">{move || t("tools.remote-protocol-hint")}</p>
      <p class="text-xs text-muted-foreground">{move || t("tools.remote-auto-hint")}</p>
      {move || {
        let (text, is_problem) = match auto_status.get() {
          AutoStatus::Off => return None,
          AutoStatus::Syncing => (t("tools.remote-auto-syncing"), false),
          AutoStatus::Synced(at) => (tf("tools.remote-auto-synced", &[&at]), false),
          AutoStatus::Conflict => (t("tools.remote-auto-conflict"), true),
          AutoStatus::NeedsFirstPush => (t("tools.remote-auto-first-push"), true),
          AutoStatus::PullFailed(e) => (tf("tools.remote-auto-pull-paused", &[&e]), true),
          AutoStatus::PushFailed(e) => (tf("tools.remote-auto-push-retry", &[&e]), true),
          AutoStatus::PushPaused(e) => (tf("tools.remote-auto-push-paused", &[&e]), true),
        };
        let class = if is_problem { "text-xs text-hp-text" } else { "text-xs" };
        Some(view! { <p role="status" class=class>{text}</p> })
      }}

      <div class="flex flex-wrap items-center gap-2">
        <Button variant=Variant::Outline disabled=pull_disabled on_click=Callback::new(ask_overwrite)>
          {move || t("tools.remote-pull-overwrite")}
        </Button>
        <Button
          variant=Variant::Outline
          disabled=pull_disabled
          on_click=Callback::new(move |()| run(Action::PullMerge))
        >
          {move || t("tools.remote-pull-merge")}
        </Button>
        <Button
          variant=Variant::Default
          disabled=push_disabled
          on_click=Callback::new(move |()| run(Action::Push))
        >
          {move || t("tools.remote-push")}
        </Button>
      </div>
    </div>
  }
}
