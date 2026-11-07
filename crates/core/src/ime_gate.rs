//! 输入法组字期间的派发门控：组字中不派发，组字结束时补派一次，并吞掉随后那次重复的 `input`。
//!
//! 搜索框每个字符都会触发一次整份重算；中文拼音输入时，「t / ti / tia / tian」这些中间串既搜不出
//! 东西，又白白多算几次。浏览器侧的事件顺序并不统一：
//!
//! - Chrome / Firefox：最后一次 `input`（`isComposing = true`）→ `compositionend`；
//! - Safari：`compositionend` → 最后一次 `input`（`isComposing = false`，值与 `compositionend` 时相同）。
//!
//! 两种顺序下组字结束都只该派发一次。这里只做判定，不碰 DOM —— 事件、`isComposing` 与时间戳
//! 由 `crates/app` 的 `composition` 模块从事件里读出来再喂进来。

/// 「紧跟在 `compositionend` 之后」的时间窗（毫秒）。
///
/// 同一次选字确认产生的 `compositionend` 与随后的 `input` 来自同一批原生事件，时间戳相差远小于 1 ms；
/// 取 50 ms 是留足浏览器与调度的余量，又远短于人手能再做出一次「值恰好相同的粘贴」的间隔。
const TRAILING_INPUT_MS: f64 = 50.0;

/// 一个输入框一份的门控状态。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CompositionGate {
  /// 最近一次 `compositionend` 派发出去的值及其时间戳。
  ended: Option<(String, f64)>,
}

impl CompositionGate {
  /// 一次 `input` 事件。返回应当派发给调用方的值，`None` 表示吞掉。
  ///
  /// `now_ms` 取事件自带的 `timeStamp`（与 [`Self::on_end`] 同一时钟）。
  pub fn on_input(&mut self, composing: bool, value: &str, now_ms: f64) -> Option<String> {
    if composing {
      // 新一轮组字开始：上一次的结束记录作废。
      self.ended = None;
      return None;
    }
    match self.ended.take() {
      Some((done, at)) if done == value && (now_ms - at).abs() <= TRAILING_INPUT_MS => None,
      _ => Some(value.to_owned()),
    }
  }

  /// 一次 `compositionend` 事件。返回要派发的值（组字结束总是派发）。
  pub fn on_end(&mut self, value: &str, now_ms: f64) -> String {
    self.ended = Some((value.to_owned(), now_ms));
    value.to_owned()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn composing_input_is_swallowed() {
    let mut g = CompositionGate::default();
    assert_eq!(g.on_input(true, "t", 1.0), None);
    assert_eq!(g.on_input(true, "tian", 2.0), None);
  }

  #[test]
  fn plain_input_passes_through() {
    let mut g = CompositionGate::default();
    assert_eq!(g.on_input(false, "a", 1.0), Some("a".to_owned()));
    // 值没变的重复 `input`（例如选中后粘贴同一段文字）也照常派发：门控不替调用方去重。
    assert_eq!(g.on_input(false, "a", 2.0), Some("a".to_owned()));
  }

  #[test]
  fn chrome_order_dispatches_once_at_end() {
    // 最后一次 input 仍在组字中 → compositionend。
    let mut g = CompositionGate::default();
    assert_eq!(g.on_input(true, "天线", 10.0), None);
    assert_eq!(g.on_end("天线", 10.2), "天线");
    // 之后继续敲字：值变了，正常派发。
    assert_eq!(g.on_input(false, "天线a", 400.0), Some("天线a".to_owned()));
  }

  #[test]
  fn safari_order_drops_the_trailing_duplicate() {
    // compositionend 在前，随后一次 isComposing=false 的 input 带着同一个值。
    let mut g = CompositionGate::default();
    assert_eq!(g.on_end("天线", 10.0), "天线");
    assert_eq!(g.on_input(false, "天线", 10.1), None);
    // 只吞一次：再来一个同值 input 就是用户的新操作。
    assert_eq!(g.on_input(false, "天线", 10.2), Some("天线".to_owned()));
  }

  #[test]
  fn same_value_later_is_not_mistaken_for_the_trailing_input() {
    // 选字完成后清空、又粘贴了同一个词：间隔远超时间窗，必须派发，否则搜索框与状态脱节。
    let mut g = CompositionGate::default();
    g.on_end("天线", 10.0);
    assert_eq!(
      g.on_input(false, "天线", 10.0 + TRAILING_INPUT_MS + 1.0),
      Some("天线".to_owned())
    );
  }

  #[test]
  fn different_value_right_after_end_is_dispatched() {
    let mut g = CompositionGate::default();
    g.on_end("天线", 10.0);
    assert_eq!(g.on_input(false, "天线a", 10.1), Some("天线a".to_owned()));
  }

  #[test]
  fn a_new_composition_clears_the_previous_end() {
    let mut g = CompositionGate::default();
    g.on_end("天线", 10.0);
    assert_eq!(g.on_input(true, "天线z", 10.1), None);
    // 上一次的结束记录已作废：即便值恰好回到「天线」，也不会被当成尾随的重复。
    assert_eq!(g.on_input(false, "天线", 10.2), Some("天线".to_owned()));
  }
}
