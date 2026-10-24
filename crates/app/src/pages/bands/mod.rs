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
const SAT_ICON: &str = "h-3.5 w-3.5 shrink-0 text-gold-text";

/// 用途徽章的配色。
///
/// 「唯一主要」与「主要」在语义 token 里同属信息档（`xp`），只靠底色时两者在同一张表里
/// 长得一模一样 —— 层级（唯一 / 之一）反而丢了。所以四档都带同宽描边：唯一主要用实描边，
/// 主要用透明描边（等同无框），层级落在这个第二通道上，布局也不因为有没有边框而抖动。
fn usage_class(u: Usage) -> &'static str {
  match u {
    Usage::Exclusive => "border border-transparent bg-win/10 text-win-text",
    Usage::SolePrimary => "border border-xp bg-xp/10 text-xp-text",
    Usage::Primary => "border border-transparent bg-xp/10 text-xp-text",
    Usage::Secondary => "border border-transparent bg-muted text-muted-foreground",
  }
}

fn footnote_id(code: &str) -> String {
  format!("fn-{}", code.replace('.', "-"))
}
