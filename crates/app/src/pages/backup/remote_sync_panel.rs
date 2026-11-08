use ham_web_core::remote_sync::{RemoteSyncConfig, WriteMethod};
use ham_web_core::sync_channel::SyncChannel;
use leptos::prelude::*;

use crate::i18n::t;
use crate::remote_sync;
use crate::ui::{
  Button, Checkbox, ConfirmTone, Field, Input, InputType, SelectOption, SimpleSelect, Textarea,
  Variant,
};

use super::auto_sync_switch::AutoSyncSwitch;
use super::sync_runner::{SyncAction, use_sync_runner};
use super::sync_status_line::SyncStatusLine;

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

  let run = use_sync_runner(SyncChannel::Custom, busy, syncing, on_done);

  // 覆盖会丢掉本机尚未同步出去的改动，而地址配错也会拉回别人的数据：先确认一次。
  let ask_overwrite = move |_| {
    crate::ui::confirm(
      t("tools.remote-pull-overwrite-title"),
      t("tools.remote-pull-overwrite-desc"),
      t("tools.remote-pull-overwrite"),
      ConfirmTone::Danger,
      Callback::new(move |()| run.run(SyncAction::PullOverwrite)),
    );
  };

  let method_options: Vec<SelectOption> = WriteMethod::ALL
    .iter()
    .map(|m| SelectOption::new(m.as_str(), m.as_str()))
    .collect();

  view! {
    <div
      class=crate::kit::inset_panel_class("space-y-3")
      data-testid="remote-sync-panel"
    >
      // 像素风保持 `pxl-label`（它没有标题栏，与相邻的 `bg-inset` 块是一体的）；
      // 现代风换成区块标题，否则这一屏里只有它会落回点阵字体。
      <h2 class=if crate::kit::is_modern_at_mount() { "md-section-title" } else { "pxl-label text-base" }>
        {move || t("tools.remote-sync-title")}
      </h2>
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
        <SimpleSelect
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
      <AutoSyncSwitch
        channel=SyncChannel::Custom
        disabled=Signal::derive(move || !cfg.with(|c| c.can_pull() || c.can_push()))
      />
      <SyncStatusLine channel=SyncChannel::Custom />

      <div class="flex flex-wrap items-center gap-2">
        <Button
          variant=Variant::Outline
          class=crate::kit::btn_wrap_class()
          disabled=pull_disabled
          on_click=Callback::new(ask_overwrite)
        >
          {move || t("tools.remote-pull-overwrite")}
        </Button>
        <Button
          variant=Variant::Outline
          class=crate::kit::btn_wrap_class()
          disabled=pull_disabled
          on_click=Callback::new(move |()| run.run(SyncAction::PullMerge))
        >
          {move || t("tools.remote-pull-merge")}
        </Button>
        <Button
          variant=Variant::Default
          class=crate::kit::btn_wrap_class()
          disabled=push_disabled
          on_click=Callback::new(move |()| run.run(SyncAction::Push))
        >
          {move || t("tools.remote-push")}
        </Button>
      </div>
    </div>
  }
}
