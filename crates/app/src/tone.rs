//! 分类标识色：把数据层的档位（`CategoryTone`）映射到调色板的 `--cat-*`。
//!
//! 色值只在这一处出现 —— 数据层（`crates/core/src/categories.rs`）只给档位，
//! 因为它写死 `#rrggbb` 会让 11 套配色方案全部失效，而且「任意底色 + 白字」的对比度
//! 无法审计（见 `docs/ui/pixel.md` §2 规则 6）。
//!
//! 用法约束：分类色**只做图形锚点**（8px 圆点、2px 描边、浅底），不铺整块文字底 ——
//! 分类名始终在标记旁边，颜色只是视觉索引，所以第 9 / 10 档允许循环到第 1 / 2 档的色相，
//! 用「空心 / 实心」这个第二通道区分（同一手法见波段用途徽章的描边）。

use ham_web_core::categories::CategoryTone;
use ham_web_core::mistake_book::heat_percent;

/// 实心标记（圆点 / 色块）的填充类。调用点自己给尺寸（`size-2` / `size-2.5`）。
///
/// 第 9 / 10 档返回的是「空心」写法（`border-2` 无底），配合调用点的尺寸正好是 8px 方块里
/// 挖掉 4px —— 与实心的 1 / 2 档同尺寸，布局不抖。
pub const fn dot(tone: CategoryTone) -> &'static str {
  match tone {
    CategoryTone::One => "bg-cat-1",
    CategoryTone::Two => "bg-cat-2",
    CategoryTone::Three => "bg-cat-3",
    CategoryTone::Four => "bg-cat-4",
    CategoryTone::Five => "bg-cat-5",
    CategoryTone::Six => "bg-cat-6",
    CategoryTone::Seven => "bg-cat-7",
    CategoryTone::Eight => "bg-cat-8",
    CategoryTone::Nine => "border-2 border-cat-1",
    CategoryTone::Ten => "border-2 border-cat-2",
  }
}

/// 徽章配色：**2px 同色描边 + 极浅同色底**，文字一律 `text-foreground`。
///
/// 不写「实色底 + 白字」：白字落在任意色上对比度不可控（`#0891b2` 只有 3.2:1），
/// 这条与 `docs/ui/pixel.md` §2 规则 7「实色填充上不要放白字」一致。
/// 调用点补 `border-2 px-2 py-0.5 text-xs text-foreground`。
pub const fn chip(tone: CategoryTone) -> &'static str {
  match tone {
    CategoryTone::One => "border-cat-1 bg-cat-1/15",
    CategoryTone::Two => "border-cat-2 bg-cat-2/15",
    CategoryTone::Three => "border-cat-3 bg-cat-3/15",
    CategoryTone::Four => "border-cat-4 bg-cat-4/15",
    CategoryTone::Five => "border-cat-5 bg-cat-5/15",
    CategoryTone::Six => "border-cat-6 bg-cat-6/15",
    CategoryTone::Seven => "border-cat-7 bg-cat-7/15",
    CategoryTone::Eight => "border-cat-8 bg-cat-8/15",
    // 空心系：底退回卡片色，靠描边区分（与 `dot` 的第二通道一致）。
    CategoryTone::Nine => "border-cat-1 bg-card",
    CategoryTone::Ten => "border-cat-2 bg-card",
  }
}

/// `--cat-*` 的 CSS 变量引用（给 `color-mix` 这类需要「色值」而非「类名」的场景用）。
///
/// 例：热力条的 `background-color: color-mix(in srgb, var(--cat-1) 85%, var(--card))`。
pub const fn var(tone: CategoryTone) -> &'static str {
  match tone {
    CategoryTone::One => "var(--cat-1)",
    CategoryTone::Two => "var(--cat-2)",
    CategoryTone::Three => "var(--cat-3)",
    CategoryTone::Four => "var(--cat-4)",
    CategoryTone::Five => "var(--cat-5)",
    CategoryTone::Six => "var(--cat-6)",
    CategoryTone::Seven => "var(--cat-7)",
    CategoryTone::Eight => "var(--cat-8)",
    CategoryTone::Nine => "var(--cat-1)",
    CategoryTone::Ten => "var(--cat-2)",
  }
}

/// 热力档位 + 分类色变量 → 一格热力的 `background-color` 声明。
///
/// 档位 → 浓度的表在 core（`heat_percent`，带单测，`pixel-check` 按它审计各套配色下的文字对比度）。
/// 这里只返回 `background-color: …` 这一段（调用点自己拼进 `style`）：浓度只有那一处定，
/// 换个页面 / 换套皮肤不会各自算一套。文字色**不在这里**——用 `text-foreground`，
/// 它随主题翻转，与浓度上限是一对。
pub fn heat_fill(level: u8, cat_var: &str) -> String {
  format!(
    "background-color: color-mix(in srgb, {cat_var} {}%, var(--card))",
    heat_percent(level)
  )
}

#[cfg(test)]
mod tests {
  use super::{CategoryTone, chip, dot, heat_fill};
  use ham_web_core::categories::TOP_CATEGORIES;

  #[test]
  fn every_tone_maps_to_palette_classes() {
    for tone in [
      CategoryTone::One,
      CategoryTone::Two,
      CategoryTone::Three,
      CategoryTone::Four,
      CategoryTone::Five,
      CategoryTone::Six,
      CategoryTone::Seven,
      CategoryTone::Eight,
      CategoryTone::Nine,
      CategoryTone::Ten,
    ] {
      // 只允许调色板类：出现十六进制 / 任意值就说明有人又把手写色塞回来了。
      for class in [dot(tone), chip(tone)] {
        assert!(
          !class.contains('#') && !class.contains('['),
          "{tone:?} 用了非 token 的颜色：{class}"
        );
      }
    }
  }

  #[test]
  fn heat_fill_uses_the_core_percent_table() {
    assert_eq!(
      heat_fill(4, "var(--cat-2)"),
      "background-color: color-mix(in srgb, var(--cat-2) 30%, var(--card))"
    );
    assert_eq!(
      heat_fill(0, "var(--cat-2)"),
      "background-color: color-mix(in srgb, var(--cat-2) 0%, var(--card))"
    );
  }

  #[test]
  fn top_categories_do_not_share_a_tone() {
    let mut seen = Vec::new();
    for c in TOP_CATEGORIES {
      assert!(
        !seen.contains(&c.tone),
        "分类 {} 与其它分类共用了同一个档位",
        c.key
      );
      seen.push(c.tone);
    }
  }
}
