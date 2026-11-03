//! 成就徽章 / 怪物图鉴的现代「纹章」：圆角方块 + 渐变环 + 线性图标。
//!
//! 像素风用 16×16 点阵精灵（`PixelSprite`）表达「奖杯 / 火焰 / 星球」，
//! 放大到 48–64px 就是一块块马赛克。现代风改用同一套线性图标语言：
//! 一枚纹章 = 一层柔和渐变底 + 一圈同色描边 + 居中的 `MIcon`，
//! 尺寸与圆角跟按钮、面板同一套刻度。

use ham_web_core::emblem::{self, Glyph};
use leptos::prelude::*;

use super::{MIcon, ModernIcon};

/// 纹章色调（规则与取值在 `ham_web_core::emblem`，那边有单测）。
pub use ham_web_core::emblem::Tone as EmblemTone;

/// core 的图标种类 → 前端线性图标。
const fn icon_of(glyph: Glyph) -> MIcon {
  match glyph {
    Glyph::Flame => MIcon::Flame,
    Glyph::Trophy => MIcon::Trophy,
    Glyph::Book => MIcon::Book,
    Glyph::Doc => MIcon::Doc,
    Glyph::Map => MIcon::Map,
    Glyph::Antenna => MIcon::Antenna,
    Glyph::Signal => MIcon::Signal,
    Glyph::Gauge => MIcon::Gauge,
    Glyph::Code => MIcon::Code,
    Glyph::Shield => MIcon::Shield,
    Glyph::Star => MIcon::Star,
  }
}

/// 成就 / 怪物精灵名 → 纹章图标（漏配退到星星，不会出现空白方块）。
#[must_use]
pub fn emblem_icon(name: &str) -> MIcon {
  icon_of(emblem::glyph_for(name))
}

/// 精灵名 → 色调。
#[must_use]
pub fn emblem_tone(name: &str) -> EmblemTone {
  emblem::tone_for(name)
}

/// 按主题（可以是中文）稳定地取一组「图标 + 色调」：同一主题每次刷新都是同一枚纹章。
#[must_use]
pub fn emblem_variant(seed: &str) -> (MIcon, EmblemTone) {
  let (glyph, tone) = emblem::variant_for(seed);
  (icon_of(glyph), tone)
}

/// 现代纹章。`size` 是外框边长（px），锁定时只画虚线框与盾牌图标。
#[component]
pub fn ModernEmblem(
  icon: MIcon,
  #[prop(default = EmblemTone::Blue)] tone: EmblemTone,
  #[prop(default = 56)] size: u32,
  /// 锁定态是响应式的：成就的解锁状态可能在页面加载后才算出来。
  #[prop(optional, into)]
  locked: Signal<bool>,
) -> impl IntoView {
  view! {
    <span
      class="md-emblem"
      data-tone=tone.key()
      data-locked=move || locked.get().to_string()
      style=format!("--emblem-size: {size}px")
      aria-hidden="true"
    >
      <ModernIcon kind=icon class="md-emblem-icon" />
      <ModernIcon kind=MIcon::Shield class="md-emblem-lock" />
    </span>
  }
}
