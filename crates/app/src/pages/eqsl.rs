//! 电子 QSL 与日志确认速查。

use ham_web_core::eqsl::{EQSL_NOTES, EQSL_SERVICES};
use leptos::prelude::*;

use crate::components::common::{BulletSection, KnowledgePage, TableSection};
use crate::i18n::t;
use crate::util::set_title;

#[component]
pub fn EqslPage() -> impl IntoView {
  set_title(&t("电子 QSL"));
  view! {
    <KnowledgePage title=t("电子 QSL") subtitle=t("LoTW 日志确认 · eQSL 电子卡片 · QRZ · Club Log")>
      <TableSection title="主要服务" headers=&["服务", "类型", "说明"] rows=EQSL_SERVICES min_width=640 />
      <BulletSection title="说明要点" items=EQSL_NOTES />
    </KnowledgePage>
  }
}
