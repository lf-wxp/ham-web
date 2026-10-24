use ham_web_core::scp::{MAX_FILE_BYTES, MAX_LINES, ScpError, ScpFile, parse_scp};
use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::i18n::{t, tf, tp};
use crate::ui::{Button, FileInput, Size, Variant};

/// 存进浏览器的 key。只写 IndexedDB 权威层（`kv::save_store_only`）：
/// 不进 localStorage 快照，所以也不会混进备份文件 —— 这份是用户自己下载的第三方数据。
pub(super) const STORAGE_KEY: &str = "scp-file";

/// 官方站（供用户自己去下载）。
const OFFICIAL_SITE: &str = "https://www.supercheckpartial.com/";

/// 呼号补全的数据来源：自己的通联日志 + 用户自己载入的 SCP 文件。
///
/// 本站不带 SCP 数据（官方站声明保留所有权利，没有再分发许可），所以这里只管「载入 / 清除」。
#[component]
pub(super) fn ScpSource(
  /// 载入的 SCP 文件（`None` = 没载入）。
  file: RwSignal<Option<ScpFile>>,
  /// 来自通联日志的不同呼号个数。
  from_log: Signal<usize>,
) -> impl IntoView {
  let on_files = Callback::new(move |files: Vec<web_sys::File>| {
    let Some(f) = files.into_iter().next() else {
      return;
    };
    // 先看大小：真实文件约 360 KB，太大多半选错了，不必读进内存。
    if f.size() > MAX_FILE_BYTES as f64 {
      crate::ui::push_error(error_text(ScpError::TooLarge));
      return;
    }
    spawn_local(async move {
      let Some(text) = crate::util::read_file_text(&f).await else {
        crate::ui::push_error(t("common.file-read-failed"));
        return;
      };
      let parsed = match parse_scp(&text) {
        Ok(p) => p,
        Err(e) => {
          crate::ui::push_error(error_text(e));
          return;
        }
      };
      let saved = crate::kv::save_store_only(STORAGE_KEY, &parsed.to_text()).await;
      let n = parsed.calls.len();
      let release = parsed.release.clone().unwrap_or_else(|| "—".to_owned());
      let loaded = tp("contest.scp-loaded", n, &[&n.to_string(), &release]);
      let msg = if parsed.skipped > 0 {
        let k = parsed.skipped;
        tf(
          "contest.scp-loaded-with-skipped",
          &[
            &loaded,
            &tp("contest.scp-skipped-lines", k, &[&k.to_string()]),
          ],
        )
      } else {
        loaded
      };
      file.set(Some(parsed));
      crate::ui::push_success(msg);
      if !saved {
        // 这次能用，但刷新后就没了：必须告诉用户，不能假装存好了。
        crate::ui::push_error(t("contest.scp-save-failed"));
      }
    });
  });

  let clear = Callback::new(move |()| {
    spawn_local(async move {
      if crate::kv::remove_store_only(STORAGE_KEY).await {
        file.set(None);
        crate::ui::push_success(t("contest.scp-cleared"));
      } else {
        crate::ui::push_error(t("contest.scp-clear-failed"));
      }
    });
  });

  view! {
    <details class="mt-3 border bg-inset px-3 py-2 text-sm" data-slot="scp-source">
      <summary class="cursor-pointer">{move || t("contest.scp-source-heading")}</summary>
      <div class="mt-2 space-y-2">
        <p>{move || tp("contest.scp-from-log", from_log.get(), &[&from_log.get().to_string()])}</p>
        <p>
          {move || match file.get() {
            Some(f) => tp(
              "contest.scp-from-file",
              f.calls.len(),
              &[&f.calls.len().to_string(), &f.release.clone().unwrap_or_else(|| "—".to_owned())],
            ),
            None => t("contest.scp-no-file"),
          }}
        </p>
        <div class="flex flex-wrap items-center gap-2">
          <FileInput
            on_files=on_files
            accept=".scp,.txt,text/plain"
            variant=Variant::Outline
            size=Size::Sm
            label=Signal::derive(move || t("contest.scp-load-file"))
          />
          <Button
            variant=Variant::Outline
            size=Size::Sm
            disabled=Signal::derive(move || file.with(Option::is_none))
            on_click=Callback::new(move |_| clear.run(()))
          >
            {move || t("contest.scp-clear")}
          </Button>
          <a
            href=OFFICIAL_SITE
            target="_blank"
            rel="noopener noreferrer"
            class="text-xs underline underline-offset-4"
          >
            {move || t("contest.scp-official-site")}
          </a>
        </div>
        <p class="text-xs text-muted-foreground">{move || t("contest.scp-note")}</p>
      </div>
    </details>
  }
}

fn error_text(e: ScpError) -> String {
  match e {
    ScpError::NoCallsigns => t("contest.scp-error-empty"),
    ScpError::MostlyNotCallsigns => t("contest.scp-error-mostly"),
    ScpError::TooLarge => tf(
      "contest.scp-error-too-large",
      &[
        &(MAX_FILE_BYTES / (1024 * 1024)).to_string(),
        &MAX_LINES.to_string(),
      ],
    ),
  }
}
