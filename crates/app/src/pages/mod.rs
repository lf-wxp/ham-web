//! 页面。

mod bands;
mod browse;
mod exam;
mod glossary;
mod home;
mod not_found;
mod photo;
mod practice;

pub use bands::BandsPage;
pub use browse::BrowsePage;
pub use exam::ExamPage;
pub use glossary::GlossaryPage;
pub use home::HomePage;
pub use not_found::NotFoundPage;
pub use photo::PhotoProcessorPage;
pub use practice::PracticePage;

use ham_web_core::Bank;
use leptos::prelude::*;
use leptos_router::hooks::use_query_map;

use crate::util::body_class;

/// 默认页面标题。
pub const DEFAULT_TITLE: &str = "业余无线电执照考试模拟练习";

/// 从 URL 读取 `version` 与 `bank` 参数。
pub fn use_bank_query() -> (Memo<Option<String>>, Memo<Bank>) {
  let query = use_query_map();
  let version = Memo::new(move |_| query.with(|q| q.get("version")).filter(|v| !v.is_empty()));
  let bank = Memo::new(move |_| Bank::from_param(query.with(|q| q.get("bank")).as_deref()));
  (version, bank)
}

/// 答题页隐藏站点页脚。
pub fn use_no_site_footer() {
  body_class("no-site-footer", true);
  on_cleanup(|| body_class("no-site-footer", false));
}

/// 构造带题库参数的链接。
pub fn bank_href(path: &str, version: Option<&str>, bank: Bank) -> String {
  match version {
    Some(v) => format!(
      "{path}?version={}&bank={bank}",
      String::from(js_sys::encode_uri_component(v))
    ),
    None => format!("{path}?bank={bank}"),
  }
}
