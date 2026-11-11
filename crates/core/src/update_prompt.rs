//! 「发现新版本」提示的生命周期判定（纯逻辑）。
//!
//! 浏览器侧的接线（`statechange` 监听、`controllerchange` 后刷新、兜底定时器）在
//! `crates/app/src/pwa/update_notices.rs`，这里只放不依赖浏览器、能单测的那一条判定：
//! 提示所指的 waiting worker 状态变了，提示该不该关。
//!
//! 提示指向的 worker 一旦不再是 `installed`（开始激活、已激活、被更新的版本顶替），提示就过时了，
//! 不关的话它会一直挂在页面上、点了也没有效果。但有两个例外，都是「关了反而坏」：
//!
//! - **提示指向的已经不是这个 worker**：连续两次部署时，旧 worker 变成 `redundant` 的事件可能晚于
//!   新 worker 的提示，无条件关闭会把新版本的提示一并关掉；
//! - **用户刚点了「刷新以更新」**：此时 worker 进入 `activating` 正是预期，页面马上会在
//!   `controllerchange` 里刷新。这时关掉提示，用户会在等新版接管的几秒里看不到任何反馈
//!   （个别浏览器不派发该事件，要等兜底定时器才刷新）。

/// 提示所指 worker 的一次 `statechange`，以及当时提示这边的处境。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StateChange {
  /// 提示当前指向的是否正是这个 worker。
  pub is_current_target: bool,
  /// 这个 worker 此刻是否仍是 `installed`（等待激活）。
  pub still_installed: bool,
  /// 用户是否已经点了「刷新以更新」，正在等新版接管页面。
  pub user_applying: bool,
}

/// 这次 `statechange` 该不该关掉提示。
#[must_use]
pub fn should_dismiss(change: StateChange) -> bool {
  change.is_current_target && !change.still_installed && !change.user_applying
}

#[cfg(test)]
mod tests {
  use super::*;

  fn change(is_current_target: bool, still_installed: bool, user_applying: bool) -> StateChange {
    StateChange {
      is_current_target,
      still_installed,
      user_applying,
    }
  }

  #[test]
  fn closes_only_when_the_current_worker_left_installed_on_its_own() {
    // 八种组合里只有一种该关：指向它、它已不是 installed、用户没点更新。
    let mut closing = Vec::new();
    for current in [false, true] {
      for installed in [false, true] {
        for applying in [false, true] {
          if should_dismiss(change(current, installed, applying)) {
            closing.push((current, installed, applying));
          }
        }
      }
    }
    assert_eq!(closing, [(true, false, false)]);
  }

  #[test]
  fn a_stale_worker_never_closes_the_prompt_for_a_newer_one() {
    // 连续两次部署：旧 worker 变 redundant，提示已指向新 worker。
    assert!(!should_dismiss(change(false, false, false)));
    assert!(!should_dismiss(change(false, false, true)));
  }

  #[test]
  fn the_prompt_stays_while_the_user_is_waiting_for_the_new_version() {
    // 点了更新之后 worker 进入 activating / activated：提示留着（按钮转圈），等页面刷新。
    assert!(!should_dismiss(change(true, false, true)));
  }

  #[test]
  fn a_worker_that_is_still_waiting_keeps_its_prompt() {
    assert!(!should_dismiss(change(true, true, false)));
  }
}
