//! 频谱波段划分表：以 HTML 表格展示各波段的波长/频率范围，以及业余业务、卫星业余业务频段划分与脚注。
//! 桌面端为与原表一致的合并单元格表格，移动端为按波段分组的卡片。

mod band_card;
mod band_quiz;
mod bands_page;
mod notes_view;
mod range_view;
mod table_rows;
mod usage_badge;

pub use bands_page::BandsPage;

use notes_view::notes_view;
use range_view::range_view;
use table_rows::table_rows;

use ham_web_core::bands::Usage;

const CELL: &str = "border px-2 py-1.5 text-center align-middle";
const SAT_ICON: &str = "h-3.5 w-3.5 shrink-0 text-amber-600 dark:text-amber-400";

fn usage_class(u: Usage) -> &'static str {
  match u {
    Usage::Exclusive => {
      "bg-emerald-100 text-emerald-800 dark:bg-emerald-900/40 dark:text-emerald-300"
    }
    Usage::SolePrimary => "bg-blue-100 text-blue-800 dark:bg-blue-900/40 dark:text-blue-300",
    Usage::Primary => "bg-sky-100 text-sky-800 dark:bg-sky-900/40 dark:text-sky-300",
    Usage::Secondary => "bg-muted text-muted-foreground",
  }
}

fn footnote_id(code: &str) -> String {
  format!("fn-{}", code.replace('.', "-"))
}
