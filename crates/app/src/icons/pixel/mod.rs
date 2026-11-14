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

  /// 站标用的电台精灵四周要留空：它铺在 `brand-mark` 那个「2px 墨框 + 主色底」的小面板里，
  /// 机身墨框贴到画布边缘时，两块墨框会叠成一条粗黑边（2026-11-14 修过一次）。
  ///
  /// 判据只看**墨色**层的包围盒：路径是 `M{x} {y}h{w}v{h}h-Wz` 这样的矩形段，逐段取 `[x, x+w)` × `[y, y+h)`。
  /// 左右各留 >= 2 格、上下 >= 1 格。
  #[test]
  fn the_brand_sprite_leaves_room_around_the_body() {
    let data = sprite_by_name("radio").expect("精灵 radio 必须存在");
    let ink = data.layers.first().expect("第一层是墨色轮廓");
    let mut min_x = u32::MAX;
    let mut max_x = 0;
    let mut min_y = u32::MAX;
    let mut max_y = 0;
    for seg in ink.d.split('M').skip(1) {
      let (x, y, w, h) = parse_rect(seg);
      min_x = min_x.min(x);
      max_x = max_x.max(x + w);
      min_y = min_y.min(y);
      max_y = max_y.max(y + h);
    }
    let size = u32::from(data.size);
    let right = size
      .checked_sub(max_x)
      .unwrap_or_else(|| panic!("机身越出画布右边（x={max_x}，画布 {size}）"));
    let bottom = size
      .checked_sub(max_y)
      .unwrap_or_else(|| panic!("机身越出画布下边（y={max_y}，画布 {size}）"));
    assert!(min_x >= 2, "机身离左边只有 {min_x} 格，贴边了");
    assert!(right >= 2, "机身离右边只有 {right} 格，贴边了");
    assert!(min_y >= 1, "机身顶到上边（y={min_y}）");
    assert!(bottom >= 1, "机身压到底边（y={max_y}，画布 {size}）");
  }

  /// 解析 `M` 之后的一段矩形路径 `12 4h1v1h-1z` → `(x, y, w, h)`。
  ///
  /// 格式一变就让测试直接红（`expect` 带段内容），不能悄悄按默认值算出一个「看起来合格」的包围盒。
  fn parse_rect(seg: &str) -> (u32, u32, u32, u32) {
    let bad = |what: &str| -> ! { panic!("精灵路径段 `{seg}` 解析失败：{what}") };
    let num = |v: &str, what: &str| -> u32 { v.trim().parse().unwrap_or_else(|_| bad(what)) };
    let (x, rest) = seg.trim().split_once(' ').unwrap_or_else(|| bad("缺 y"));
    let (y, rest) = rest.split_once('h').unwrap_or_else(|| bad("缺 h"));
    let (w, rest) = rest.split_once('v').unwrap_or_else(|| bad("缺 v"));
    let (h, _) = rest.split_once('h').unwrap_or_else(|| bad("缺回程 h"));
    (num(x, "x"), num(y, "y"), num(w, "w"), num(h, "h"))
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
