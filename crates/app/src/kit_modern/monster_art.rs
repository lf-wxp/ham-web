//! 现代风的怪物插画：五种「信号故障」怪 + Boss + 玩家。
//!
//! 像素风里怪物是 16×16 点阵精灵（`icons/pixel/`），现代风不能直接放大点阵 ——
//! 放大就是一块马赛克。这里按现代风的语言重画一遍：圆角外框 + 同色辉光 + 硬边色块 +
//! 一只表情，仍然是纯 SVG（随主题变色、无位图资产、无额外请求）。
//!
//! 颜色全部走 `--mon-*`（定义在 `style/modern/kit.css`）：一个 `data-tone` 同时换掉主体
//! （`--mon-a`）、暗部（`--mon-b`）、描线（`--mon-ink`）、辉光与外框；亮面（`--mon-face`，
//! 眼睛 / 屏幕 / 面罩）不随色调变，取 `--md-surface`，在两种主题里都与彩色身体拉开一档。
//! 插画里只写 `var()`，不写死色值。形态名与像素精灵名对齐，
//! 调用点仍然只认一个精灵名（`mon_static` / `boss` / `hero` …）。

use leptos::prelude::*;

/// 现代风怪物形态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MonsterArt {
  /// 静电：屏幕头 + 两根天线，代表「收到的只有雪花」。
  Static,
  /// 驻波：四臂 + 中央躯体，代表天线阻抗失配。
  Swr,
  /// 噪声：圆滚滚的斑点团，代表背景噪声。
  Noise,
  /// 杂散：尖刺爆发，代表杂散辐射。
  Spur,
  /// 衰落：正在淡出的幽魂，代表信号衰减。
  Fade,
  /// Boss：长角的红色魔王。
  Boss,
  /// 玩家：天线小机器人。
  Hero,
}

impl MonsterArt {
  /// 精灵名 → 现代风形态。
  ///
  /// 精灵名是编译期常量（`crate::rpg::monster_sprite`），所以未知名字只可能是
  /// 「有人传了成就徽章名」这类编码错误；退到静电怪而不是画空框 —— 至少看得出是只怪。
  #[must_use]
  pub fn from_sprite(sprite: &str) -> Self {
    match sprite {
      "mon_swr" => Self::Swr,
      "mon_noise" => Self::Noise,
      "mon_spur" => Self::Spur,
      "mon_fade" => Self::Fade,
      "boss" => Self::Boss,
      "hero" => Self::Hero,
      _ => Self::Static,
    }
  }

  /// 写进 `data-kind` 的取值（也用于测试与调试定位）。
  #[must_use]
  pub const fn key(self) -> &'static str {
    match self {
      Self::Static => "static",
      Self::Swr => "swr",
      Self::Noise => "noise",
      Self::Spur => "spur",
      Self::Fade => "fade",
      Self::Boss => "boss",
      Self::Hero => "hero",
    }
  }

  /// 写进 `data-tone` 的取值：`style/modern/kit.css` 里 `.md-mon[data-tone="…"]`
  /// 逐个给出 `--mon-a` / `--mon-b` / `--mon-ink` / `--mon-glow` / `--mon-border`。
  #[must_use]
  pub const fn tone(self) -> &'static str {
    match self {
      Self::Static => "blue",
      Self::Swr => "fire",
      Self::Noise => "green",
      Self::Spur => "violet",
      Self::Fade => "gold",
      Self::Boss => "danger",
      Self::Hero => "green",
    }
  }
}

/// 一枚现代风怪物。
///
/// - `sprite`：像素精灵名（`mon_static` / `mon_swr` / `mon_noise` / `mon_spur` /
///   `mon_fade` / `boss` / `hero`），未知名字退到静电怪。
/// - `size`：外框边长（px），插画按 78% 缩放。
///
/// 怪物总是紧挨着名字或血条出现，所以整枚插画是装饰（`aria-hidden`）；
/// 读屏要念的是「哪只怪」，那由旁边的大标题 / 敌人名承担。
#[component]
pub fn ModernMonster(
  #[prop(into)] sprite: Signal<&'static str>,
  #[prop(default = 56)] size: u32,
) -> impl IntoView {
  let art = Signal::derive(move || MonsterArt::from_sprite(sprite.get()));
  view! {
    <span
      class="md-mon"
      data-kind=move || art.get().key()
      data-tone=move || art.get().tone()
      style=format!("--mon-size: {size}px")
      aria-hidden="true"
    >
      <svg class="md-mon-svg" viewBox="0 0 64 64" xmlns="http://www.w3.org/2000/svg">
        {move || match art.get() {
          MonsterArt::Static => {
            view! {
              <g fill="none" stroke="var(--mon-ink)" stroke-width="2" stroke-linecap="round">
                <path d="M23 14 L17 6" />
                <path d="M41 14 L47 6" />
              </g>
              <circle cx="17" cy="5" r="2.6" fill="var(--mon-a)" />
              <circle cx="47" cy="5" r="2.6" fill="var(--mon-a)" />
              <path
                d="M20 47 v5"
                stroke="var(--mon-a)"
                stroke-width="5"
                stroke-linecap="round"
              />
              <path
                d="M44 47 v5"
                stroke="var(--mon-a)"
                stroke-width="5"
                stroke-linecap="round"
              />
              <rect x="10" y="12" width="44" height="36" rx="12" fill="var(--mon-a)" />
              <rect x="16" y="18" width="32" height="18" rx="5" fill="var(--mon-face)" />
              <rect x="21" y="22" width="7" height="8" rx="2.4" fill="var(--mon-ink)" />
              <rect x="36" y="22" width="7" height="8" rx="2.4" fill="var(--mon-ink)" />
              <path
                d="M19.5 32.5 h25"
                stroke="var(--mon-ink)"
                stroke-width="1.8"
                stroke-dasharray="3 3"
                opacity="0.45"
              />
              <g fill="var(--mon-ink)" opacity="0.55">
                <rect x="19" y="40" width="9" height="2.6" rx="1.3" />
                <rect x="29.5" y="40" width="6" height="2.6" rx="1.3" />
                <rect x="37" y="40" width="8" height="2.6" rx="1.3" />
              </g>
            }
              .into_any()
          }
          MonsterArt::Swr => {
            view! {
              <g stroke="var(--mon-a)" stroke-width="8" stroke-linecap="round">
                <path d="M9 9 L23 23" />
                <path d="M55 9 L41 23" />
                <path d="M9 55 L23 41" />
                <path d="M55 55 L41 41" />
              </g>
              <g fill="var(--mon-b)">
                <circle cx="8" cy="8" r="5.4" />
                <circle cx="56" cy="8" r="5.4" />
                <circle cx="8" cy="56" r="5.4" />
                <circle cx="56" cy="56" r="5.4" />
              </g>
              <circle cx="32" cy="32" r="16" fill="var(--mon-a)" />
              <circle
                cx="32"
                cy="32"
                r="16"
                fill="none"
                stroke="var(--mon-ink)"
                stroke-width="1.6"
                opacity="0.45"
              />
              <path
                d="M21 23 L27 25"
                stroke="var(--mon-ink)"
                stroke-width="2"
                stroke-linecap="round"
              />
              <path
                d="M43 23 L37 25"
                stroke="var(--mon-ink)"
                stroke-width="2"
                stroke-linecap="round"
              />
              <circle cx="26" cy="29" r="4" fill="var(--mon-face)" />
              <circle cx="38" cy="29" r="4" fill="var(--mon-face)" />
              <circle cx="26.6" cy="29.8" r="1.8" fill="var(--mon-ink)" />
              <circle cx="38.6" cy="29.8" r="1.8" fill="var(--mon-ink)" />
              <path
                d="M26 39 l3 3 l3 -3 l3 3 l3 -3"
                fill="none"
                stroke="var(--mon-ink)"
                stroke-width="2"
                stroke-linecap="round"
                stroke-linejoin="round"
              />
              <path
                d="M32 14 v4 M32 46 v4"
                stroke="var(--mon-ink)"
                stroke-width="1.6"
                opacity="0.35"
              />
            }
              .into_any()
          }
          MonsterArt::Noise => {
            view! {
              <path
                d="M10 32 C10 19 19 10 32 10 C45 10 54 19 54 32 C54 40 50 45 45 47 C43 51 38 54 32 54 C26 54 21 51 19 47 C14 45 10 40 10 32 Z"
                fill="var(--mon-a)"
              />
              <g fill="var(--mon-ink)" opacity="0.28">
                <circle cx="20" cy="24" r="2.2" />
                <circle cx="45" cy="22" r="2.6" />
                <circle cx="49" cy="35" r="1.8" />
                <circle cx="15" cy="37" r="1.6" />
                <circle cx="28" cy="19" r="1.4" />
                <circle cx="38" cy="45" r="1.8" />
                <circle cx="22" cy="44" r="1.3" />
              </g>
              <circle cx="25" cy="32" r="5.2" fill="var(--mon-face)" />
              <circle cx="39" cy="32" r="5.2" fill="var(--mon-face)" />
              <circle cx="26" cy="33" r="2.2" fill="var(--mon-ink)" />
              <circle cx="40" cy="33" r="2.2" fill="var(--mon-ink)" />
              <path
                d="M26 43 l2.5 -3 l2.5 3 l2.5 -3 l2.5 3"
                fill="none"
                stroke="var(--mon-ink)"
                stroke-width="2"
                stroke-linecap="round"
                stroke-linejoin="round"
              />
            }
              .into_any()
          }
          MonsterArt::Spur => {
            view! {
              <path
                d="M56 33 L42.5 25.4 L40 8.3 L28.3 21.6 L11.8 18.3 L18 33 L11 48.3 L28.3 44.4 L39.4 55.8 L42.5 40.6 Z"
                fill="var(--mon-a)"
                stroke="var(--mon-ink)"
                stroke-width="1.4"
                stroke-linejoin="round"
                stroke-opacity="0.4"
              />
              <circle cx="32" cy="33" r="10" fill="var(--mon-face)" opacity="0.94" />
              <path d="M26.5 29 L32 31.5 L26.5 34 Z" fill="var(--mon-ink)" />
              <path d="M37.5 29 L32 31.5 L37.5 34 Z" fill="var(--mon-ink)" />
              <path
                d="M29 38 h6"
                stroke="var(--mon-ink)"
                stroke-width="2"
                stroke-linecap="round"
              />
              <g fill="var(--mon-a)">
                <circle cx="56" cy="12" r="1.6" opacity="0.8" />
                <circle cx="50" cy="6" r="1.1" opacity="0.6" />
                <circle cx="59" cy="20" r="1" opacity="0.5" />
              </g>
            }
              .into_any()
          }
          MonsterArt::Fade => {
            view! {
              <path
                d="M16 50 V30 C16 19 23 12 32 12 C41 12 48 19 48 30 V50 L42 46 L36 51 L32 45 L28 51 L22 46 Z"
                fill="var(--mon-a)"
                opacity="0.88"
              />
              <ellipse cx="26" cy="29" rx="3.2" ry="4.2" fill="var(--mon-ink)" />
              <ellipse cx="38" cy="29" rx="3.2" ry="4.2" fill="var(--mon-ink)" />
              <path
                d="M28 38 q4 3.4 8 0"
                fill="none"
                stroke="var(--mon-ink)"
                stroke-width="2"
                stroke-linecap="round"
              />
              <g fill="var(--mon-a)">
                <circle cx="52" cy="48" r="2.4" opacity="0.55" />
                <circle cx="56" cy="55" r="1.6" opacity="0.4" />
                <circle cx="50" cy="59" r="1.1" opacity="0.3" />
                <circle cx="12" cy="54" r="1.4" opacity="0.35" />
              </g>
              <path
                d="M53 38 h6"
                stroke="var(--mon-ink)"
                stroke-width="1.6"
                stroke-linecap="round"
                opacity="0.3"
              />
            }
              .into_any()
          }
          MonsterArt::Boss => {
            view! {
              <path d="M15 27 L25 21 L4 6 Z" fill="var(--mon-b)" />
              <path d="M49 27 L39 21 L60 6 Z" fill="var(--mon-b)" />
              <path d="M9 56 C8 34 18 16 32 16 C46 16 56 34 55 56 Z" fill="var(--mon-a)" />
              <path
                d="M18 31 L29 35"
                stroke="var(--mon-ink)"
                stroke-width="3.4"
                stroke-linecap="round"
              />
              <path
                d="M46 31 L35 35"
                stroke="var(--mon-ink)"
                stroke-width="3.4"
                stroke-linecap="round"
              />
              <path d="M20 36 L30 39 L20 44 Z" fill="var(--mon-face)" />
              <path d="M44 36 L34 39 L44 44 Z" fill="var(--mon-face)" />
              <circle cx="25" cy="40" r="2" fill="var(--mon-ink)" />
              <circle cx="39" cy="40" r="2" fill="var(--mon-ink)" />
              <path
                d="M21 47 q11 9 22 0"
                fill="none"
                stroke="var(--mon-ink)"
                stroke-width="3"
                stroke-linecap="round"
              />
              <path d="M25 47.5 l1.8 5 l2 -4.4 Z" fill="var(--mon-face)" />
              <path d="M39 47.5 l-1.8 5 l-2 -4.4 Z" fill="var(--mon-face)" />
              <path d="M30.8 51 l1.2 4 l1.2 -4 Z" fill="var(--mon-face)" />
            }
              .into_any()
          }
          MonsterArt::Hero => {
            view! {
              <path
                d="M32 16 V8"
                stroke="var(--mon-ink)"
                stroke-width="2.2"
                stroke-linecap="round"
              />
              <circle cx="32" cy="6" r="3" fill="var(--mon-a)" />
              <rect x="12" y="21" width="5" height="10" rx="2.5" fill="var(--mon-b)" />
              <rect x="47" y="21" width="5" height="10" rx="2.5" fill="var(--mon-b)" />
              <rect x="17" y="15" width="30" height="22" rx="8" fill="var(--mon-a)" />
              <rect x="21" y="21" width="22" height="10" rx="5" fill="var(--mon-face)" />
              <circle cx="27" cy="26" r="2.2" fill="var(--mon-ink)" />
              <circle cx="38" cy="26" r="2.2" fill="var(--mon-ink)" />
              <path
                d="M24 43 L17 50"
                stroke="var(--mon-a)"
                stroke-width="4.5"
                stroke-linecap="round"
              />
              <path
                d="M40 43 L47 50"
                stroke="var(--mon-a)"
                stroke-width="4.5"
                stroke-linecap="round"
              />
              <rect x="24" y="39" width="16" height="13" rx="5" fill="var(--mon-a)" />
              <circle cx="32" cy="45" r="2.4" fill="var(--mon-face)" />
              <rect x="24" y="51" width="7" height="4.5" rx="2.2" fill="var(--mon-b)" />
              <rect x="33" y="51" width="7" height="4.5" rx="2.2" fill="var(--mon-b)" />
            }
              .into_any()
          }
        }}
      </svg>
    </span>
  }
}

#[cfg(test)]
mod tests {
  use super::MonsterArt;
  use crate::icons::sprite_exists;

  /// `kit.css` 里的配色块与本文件的 `tone()` 必须一一对得上：
  /// 少了 `[data-tone="…"]` 那条，插画会整块透明（`var()` 取不到值不会报错，只会静默消失）。
  #[test]
  fn every_tone_is_declared_in_kit_css() {
    const KIT_CSS: &str = include_str!("../../style/modern/kit.css");
    for art in [
      MonsterArt::Static,
      MonsterArt::Swr,
      MonsterArt::Noise,
      MonsterArt::Spur,
      MonsterArt::Fade,
      MonsterArt::Boss,
      MonsterArt::Hero,
    ] {
      let selector = format!(".md-mon[data-tone=\"{}\"]", art.tone());
      assert!(KIT_CSS.contains(&selector), "kit.css 缺 {selector}");
    }
  }

  #[test]
  fn sprite_names_map_to_their_own_art() {
    let pairs = [
      ("mon_static", MonsterArt::Static),
      ("mon_swr", MonsterArt::Swr),
      ("mon_noise", MonsterArt::Noise),
      ("mon_spur", MonsterArt::Spur),
      ("mon_fade", MonsterArt::Fade),
      ("boss", MonsterArt::Boss),
      ("hero", MonsterArt::Hero),
    ];
    for (sprite, art) in pairs {
      assert!(sprite_exists(sprite), "{sprite} 不是有效精灵名");
      assert_eq!(MonsterArt::from_sprite(sprite), art);
    }
    // 未知名字（例如成就徽章名）退到静电怪，不画空框。
    assert_eq!(MonsterArt::from_sprite("badge_trophy"), MonsterArt::Static);
    assert_eq!(MonsterArt::from_sprite(""), MonsterArt::Static);
  }

  /// 图鉴 / 地图 / 战斗里实际会出现的每一种怪都得有自己的插画：
  /// 新增精灵名却忘了在 `from_sprite` 里配图，它会静默退到静电怪，而静电怪之外的名字都该是别的形态。
  #[test]
  fn every_rpg_monster_sprite_has_its_own_art() {
    for sprite in crate::rpg::MONSTER_SPRITES {
      let art = MonsterArt::from_sprite(sprite);
      assert_eq!(
        art == MonsterArt::Static,
        sprite == "mon_static",
        "{sprite} 没有配现代风插画（落到了静电怪的兜底）"
      );
    }
  }

  /// `data-kind` 是调试与 e2e 的定位依据，不能两只怪共用一个值。
  #[test]
  fn kinds_are_unique() {
    let all = [
      MonsterArt::Static,
      MonsterArt::Swr,
      MonsterArt::Noise,
      MonsterArt::Spur,
      MonsterArt::Fade,
      MonsterArt::Boss,
      MonsterArt::Hero,
    ];
    let keys: std::collections::HashSet<_> = all.iter().map(|a| a.key()).collect();
    assert_eq!(keys.len(), all.len());
  }
}
