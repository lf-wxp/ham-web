//! 搜索框这类「每敲一个字都要重算一大片」的输入用的两件工具：信号防抖、值不变就不通知的写入。

use std::time::Duration;

use leptos::prelude::*;

/// 搜索框的默认防抖时长：比两次击键的典型间隔（约 100 ms）略长，停手后几乎察觉不到。
pub const SEARCH_DEBOUNCE: Duration = Duration::from_millis(150);

/// 写入 `value`，**只在值真的变了时**通知订阅者。
///
/// `RwSignal::set` 不比较新旧值，写同一个值也照样通知。搜索框的 `on_change` 里常顺手写一句
/// 「回到第一页 / 回到首批条数」，而这个值绝大多数时候本来就是它 —— 于是每敲一个字，列表都要
/// 按旧的筛选结果整页重建一遍（术语表 60 张卡、题库浏览 40 道题、日志一页 50 行）。
pub fn set_if_changed<T: PartialEq + Send + Sync + 'static>(signal: RwSignal<T>, value: T) {
  signal.maybe_update(|v| {
    if *v == value {
      return false;
    }
    *v = value;
    true
  });
}

/// 跟随 `source` 的防抖副本：`source` 停止变化 `delay` 之后才跟上；首值同步。
///
/// 输入框仍然绑 `source`（字立刻上屏），只把**重算**挂到返回值上：连续输入「antenna」只算一次，
/// 而不是七次。变成空串时立即跟上 —— 清空搜索是「回到全部」，不该让人等。
///
/// 返回可写信号：不是用户逐字输入的写入（地址里的 `?q=`、点相关词条跳转、关闭时清空）应当
/// **两个一起写**，立刻生效，不必等防抖；随后的那次定时回调发现值已一致，什么都不做。
///
/// 必须在组件体里调用（要挂 `Effect` 与 `on_cleanup`）；组件卸载时待触发的定时器一并撤销。
pub fn use_debounced(source: RwSignal<String>, delay: Duration) -> RwSignal<String> {
  let out = RwSignal::new(source.get_untracked());
  let pending: StoredValue<Option<TimeoutHandle>> = StoredValue::new(None);
  let cancel = move || {
    if let Some(Some(h)) = pending.try_update_value(Option::take) {
      h.clear();
    }
  };
  Effect::new(move |prev: Option<()>| {
    let value = source.get();
    if prev.is_none() {
      return;
    }
    cancel();
    if out.with_untracked(|o| *o == value) {
      return;
    }
    if value.trim().is_empty() {
      out.set(value);
      return;
    }
    let handle = set_timeout_with_handle(
      move || {
        if out.try_with_untracked(|o| *o != value) == Some(true) {
          let _ = out.try_set(value);
        }
      },
      delay,
    )
    .ok();
    pending.set_value(handle);
  });
  on_cleanup(cancel);
  out
}
