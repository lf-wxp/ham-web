//! 电子 QSL 与日志确认速查，含 LoTW 签名上传的完整流程。

use ham_web_core::eqsl::{EQSL_NOTES, EQSL_SERVICES, LOTW_UPLOAD_NOTES, LOTW_UPLOAD_STEPS};
use leptos::prelude::*;

use crate::components::common::{BulletSection, KnowledgePage, StepsSection, TableSection};
use crate::i18n::t;
use crate::util::set_title;

#[component]
pub fn EqslPage() -> impl IntoView {
  set_title("shell.eqsl");
  view! {
    <KnowledgePage title=t("shell.eqsl") subtitle=t("log.lotw-confirmations-eqsl-qrz")>
      <TableSection title="主要服务" headers=&["服务", "类型", "说明"] rows=EQSL_SERVICES min_width=640 />
      // 流程放在服务表后面：先知道有哪几家，再看 LoTW 这条路怎么走通。
      // 工具本身在通联日志页（导出 ADIF / 标记已上传 / 同步 QSL），这里只讲步骤与边界。
      <StepsSection title="LoTW 签名上传流程" items=LOTW_UPLOAD_STEPS />
      <BulletSection title="LoTW 的边界与常见坑" items=LOTW_UPLOAD_NOTES />
      <BulletSection title="说明要点" items=EQSL_NOTES />
    </KnowledgePage>
  }
}
