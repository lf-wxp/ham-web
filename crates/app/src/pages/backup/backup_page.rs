use leptos::prelude::*;

use crate::components::common::{PageContainer, PageHeader};
use crate::i18n::t;
use crate::util::set_title;

use super::backup_tool::BackupTool;

/// `/backup`：导出 / 导入全部本地数据，并给出存储占用。
#[component]
pub fn BackupPage() -> impl IntoView {
  set_title("tools.data-backup");
  if crate::kit::is_modern_at_mount() {
    return view! {
      <div class="md-page">
        <crate::kit_modern::ModernPageHead
          title=Signal::derive(move || t("tools.data-backup"))
          subtitle=Signal::derive(move || t("settings.export-all-local-data"))
        />
        // 现代风**不套外层卡片**：页面底 + 一串 `.md-panel`（导出 / 在线账号 / 自定义远端 /
        // 存储占用）。套一层卡片再往里塞 inset 块，就是「卡片里的卡片」，一屏三层底色。
        //
        // 外面这层 `<div>` 不是多余的：`#route-content .md-page > section` 会给文章式
        // 「章节编号 + 刻度尺」，而 `section#backup` 必须保留标签与 id —— 三份 e2e
        // （`backup.spec.ts` / `account_sync.spec.ts`）按 `section#backup` 定位文件输入框，
        // 改成 `<div>` 会让它们全部超时。
        <div>
          <section id="backup" class="flex flex-col gap-5">
            <BackupTool />
          </section>
        </div>
      </div>
    }
    .into_any();
  }
  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-(--motion-normal) ease-out">
      <PageHeader
        title=move || t("tools.data-backup")
        subtitle=move || t("settings.export-all-local-data")
      />
      // `id="backup"`：e2e 按这个 id 定位整块备份区；旧链接 `/tools#backup` 由
      // `ToolsPage` 负责转到本页，锚点本身在这里已无意义。
      <PageContainer>
        <section id="backup" class=crate::kit::card_pad_class()>
          <BackupTool />
        </section>
      </PageContainer>
    </div>
  }
  .into_any()
}
