//! 分区号地图共用的小类型：省所处的「层」，以及一个省在当前选区下的绘制信息。

use ham_web_core::province_map::ProvinceShape;
use ham_web_core::reference::Province;

/// 省份在图上所处的层：决定绘制顺序、不透明度与描边。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Layer {
  /// 没有选区：全部同色同一层。
  All,
  /// 选中的分区：画在最上层。
  Selected,
  /// 选区之外：淡出，画在下层。
  Dimmed,
}

impl Layer {
  pub(super) fn of(selected: Option<&str>, area: &str) -> Self {
    match selected {
      None => Self::All,
      Some(sel) if sel == area => Self::Selected,
      Some(_) => Self::Dimmed,
    }
  }

  /// 供样式与 e2e 使用的稳定标记（不要让测试去数 `fill-opacity` 的具体数值）。
  pub(super) fn state(self) -> &'static str {
    match self {
      Self::All => "all",
      Self::Selected => "selected",
      Self::Dimmed => "dimmed",
    }
  }

  pub(super) fn fill_opacity(self) -> &'static str {
    match self {
      Self::All => "0.5",
      Self::Selected => "0.8",
      Self::Dimmed => "0.1",
    }
  }

  pub(super) fn stroke_class(self) -> &'static str {
    match self {
      Self::Selected => "stroke-foreground",
      Self::All | Self::Dimmed => "stroke-foreground/40",
    }
  }

  /// 省名 / 分区号 / 引线 是否用「醒目」的色（没有选区，或本省就在选区里）。
  pub(super) fn is_active(self) -> bool {
    !matches!(self, Self::Dimmed)
  }

  /// 绘制的遍次：第 0 遍画 `All` / `Dimmed`，第 1 遍画 `Selected`（同一次渲染里 `All` 不会和
  /// 另外两种混用）。
  pub(super) fn pass(self) -> u8 {
    match self {
      Self::All | Self::Dimmed => 0,
      Self::Selected => 1,
    }
  }
}

/// 一个省在当前选区下的绘制所需信息（省份表里查不到的省不会被构造出来）。
pub(super) struct Drawn<'a> {
  pub(super) shape: &'a ProvinceShape,
  pub(super) province: &'static Province,
  pub(super) layer: Layer,
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn layers_follow_the_selection() {
    assert_eq!(Layer::of(None, "3"), Layer::All);
    assert_eq!(Layer::of(Some("3"), "3"), Layer::Selected);
    assert_eq!(Layer::of(Some("3"), "1"), Layer::Dimmed);
    // 空串不是合法分区号：不会被当成「选中了某个分区」。
    assert_eq!(Layer::of(Some("3"), ""), Layer::Dimmed);
  }

  #[test]
  fn selected_is_painted_after_everything_else() {
    assert!(Layer::Selected.pass() > Layer::Dimmed.pass());
    assert_eq!(Layer::All.pass(), Layer::Dimmed.pass());
  }

  /// `data-state` 是 e2e 与样式的契约：改名会让用例静默失效，这里钉死。
  #[test]
  fn state_markers_are_the_documented_contract() {
    assert_eq!(Layer::All.state(), "all");
    assert_eq!(Layer::Selected.state(), "selected");
    assert_eq!(Layer::Dimmed.state(), "dimmed");
    assert!(Layer::All.is_active() && Layer::Selected.is_active());
    assert!(!Layer::Dimmed.is_active());
  }

  /// 选中最醒目、淡出最淡：不透明度必须随层单调。
  #[test]
  fn opacity_is_monotonic_in_prominence() {
    let o = |l: Layer| l.fill_opacity().parse::<f64>().expect("数值");
    assert!(o(Layer::Selected) > o(Layer::All));
    assert!(o(Layer::All) > o(Layer::Dimmed));
  }
}
