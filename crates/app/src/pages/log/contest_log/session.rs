//! 竞赛场次的类型现在定义在 `ham_web_core::contest::ContestSession`（账号同步的服务端要存它）。
//! core 不读时钟，所以「新场次从现在开始」由这里补上。

use crate::pages::log::{utc_now_time, utc_today};

pub(super) use ham_web_core::contest::ContestSession as Session;

/// 新场次：第一个竞赛，从此刻（UTC）开始。
pub(super) fn fresh() -> Session {
  Session::starting(format!("{} {}", utc_today(), utc_now_time()))
}
