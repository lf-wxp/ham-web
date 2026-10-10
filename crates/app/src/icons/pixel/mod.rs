//! 像素图标与精灵。
//!
//! - `icon_paths`：[`crate::icons::IconKind`] → 24×24 网格里的像素路径（生成文件）；
//! - `sprite_data`：精灵字符画编译出的分色路径（生成文件）；
//! - [`PixelSprite`]：渲染精灵的组件。

mod icon_paths;
mod pixel_sprite;
pub(in crate::icons) mod sprite_data;

pub(in crate::icons) use icon_paths::icon_path;
pub use pixel_sprite::PixelSprite;

#[cfg(test)]
mod tests {
  use super::sprite_data::sprite_by_name;
  use crate::icons::icon_of;

  /// 每个注册表里出现过的图标都必须有非空的像素路径：缺一个就会渲染成空白方块。
  #[test]
  fn every_icon_has_a_pixel_path() {
    for key in crate::icons::icon_lookup::ICON_KEYS {
      let path = super::icon_path(icon_of(key));
      assert!(path.starts_with('M'), "图标 {key} 没有像素路径");
    }
  }

  /// 精灵名写错只会渲染成空白，运行时不报错，所以在这里把名字表过一遍。
  #[test]
  fn sprites_resolve_and_are_square() {
    for name in [
      "hero",
      "boss",
      "mon_static",
      "mon_swr",
      "mon_noise",
      "mon_spur",
      "mon_fade",
      "badge_star",
      "badge_medal",
      "badge_trophy",
      "badge_flame",
      "badge_antenna",
      "badge_globe",
      "badge_book",
      "badge_log",
      "badge_lock",
      "node_flag",
      "node_heart",
      "node_coin",
    ] {
      let data = sprite_by_name(name).unwrap_or_else(|| panic!("精灵 {name} 不存在"));
      assert_eq!(data.size, 16, "{name} 应是 16×16");
      assert!(!data.layers.is_empty(), "{name} 没有图层");
    }
    assert!(sprite_by_name("不存在").is_none());
  }
}
