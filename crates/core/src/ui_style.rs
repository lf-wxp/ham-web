//! 界面风格：像素风（`pixel`）与现代化（`modern`）。
//!
//! 这是**两套独立设计系统**的开关，不是配色方案那样的「同一骨架换 token」：
//! 像素风的外观与交互在 `style/pixel/` + `ui/`（及页面里的像素分支），现代风在 `style/modern/` + `kit_modern/`，
//! 两者不共享类名、不共享 token、不共享 DOM 结构（见 `docs/ui/decisions.md`）。
//!
//! 取值落在 `<html data-ui-style>` 上，由 `index.html` 的内联脚本在首屏前写入，避免闪烁；
//! 持久化在 `localStorage["ui:uiStyle"]`（设备本地偏好，与「像素动效 / 易读字体」同族）。

/// 界面风格。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum UiStyle {
  /// 像素风：16 位掌机 + 复古电台。默认值，也是既有用户看到的形态。
  #[default]
  Pixel,
  /// 现代风：圆角、柔和阴影、线性图标、平滑动效。
  Modern,
}

impl UiStyle {
  /// 全部风格，顺序即设置界面的顺序（默认项在前）。
  pub const ALL: [Self; 2] = [Self::Pixel, Self::Modern];

  /// 存储与 `<html data-ui-style>` 上使用的字符串。
  #[must_use]
  pub const fn as_str(self) -> &'static str {
    match self {
      Self::Pixel => "pixel",
      Self::Modern => "modern",
    }
  }

  /// 解析存储值；不认识的一律回退到默认风格，避免旧数据 / 手改值把界面弄成空白。
  ///
  /// **与 `crates/app/index.html` 的首屏内联脚本是两份实现**（脚本要赶在样式表之前跑，
  /// 不能等 wasm）：那边只认严格的 `=== "modern"`，这里同样**不 trim、区分大小写**。
  /// 曾经这里多做一次 `trim()`，于是手改出的 `" modern "` 首屏是 pixel、挂载后又翻成 modern，
  /// 界面闪一下。存储值只由 [`Self::as_str`] 写入，trim 本来就没有用处。
  /// 改取值 / 改回退策略时两处必须一起改。
  #[must_use]
  pub fn parse(s: &str) -> Self {
    match s {
      "modern" => Self::Modern,
      _ => Self::Pixel,
    }
  }

  /// 是否为现代风格（调用点读起来比 `== UiStyle::Modern` 短）。
  #[must_use]
  pub const fn is_modern(self) -> bool {
    matches!(self, Self::Modern)
  }
}

#[cfg(test)]
mod tests {
  use super::UiStyle;

  #[test]
  fn default_is_pixel() {
    assert_eq!(UiStyle::default(), UiStyle::Pixel);
    assert!(!UiStyle::default().is_modern());
  }

  #[test]
  fn parse_round_trips_and_falls_back() {
    for style in UiStyle::ALL {
      assert_eq!(UiStyle::parse(style.as_str()), style);
    }
    // 手改 localStorage / 版本回退留下的陌生值不能把界面弄坏。
    assert_eq!(UiStyle::parse(""), UiStyle::Pixel);
    assert_eq!(UiStyle::parse("MODERN"), UiStyle::Pixel);
    // 与首屏脚本逐字一致：带空白的值两边都回退像素风，不会出现首屏与挂载后不一致。
    assert_eq!(UiStyle::parse("  modern  "), UiStyle::Pixel);
  }
}
