use leptos::prelude::*;

use crate::components::common::{PageContainer, PageHeader};
use crate::i18n::t;
use crate::util::set_title;

use super::backup_tool::BackupTool;

/// `/backup`：导出 / 导入全部本地数据，并给出存储占用。
#[component]
pub fn BackupPage() -> impl IntoView {
  set_title("tools.data-backup");
  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <PageHeader
        title=move || t("tools.data-backup")
        subtitle=move || t("settings.export-all-local-data")
      />
      // `id="backup"`：e2e 按这个 id 定位整块备份区；旧链接 `/tools#backup` 由
      // `ToolsPage` 负责转到本页，锚点本身在这里已无意义。
      <PageContainer>
        <section id="backup" class="pxl-card border bg-card p-4">
          <BackupTool />
        </section>
      </PageContainer>
    </div>
  }
}
