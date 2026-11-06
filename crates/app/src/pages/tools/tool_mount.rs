//! 计算器主体的分批挂载。
//!
//! 工具页一口气挂 60 个计算器：首次进入时脚本 + 布局在中端手机上要五六百毫秒，
//! 而首屏只看得到前一两个。现在首屏先挂 [`EAGER`] 个，其余在首帧画出之后每帧放行
//! [`BATCH`] 个（`after_paint::each_frame`），一两百毫秒内全部挂完；页面要按真实高度
//! 定位时（锚点直达、点目录）立刻挂全，见 [`ToolMount::mount_all`]。
//!
//! 卡片外壳（标题 / 说明 / `id`）一开始就在：目录、滚动高亮、锚点都按 `id` 找卡片，
//! 只有主体是晚到的；没挂上的主体先占一个与中位高度相近的空位（见 `ToolBody`）。
//! 为什么不用 `content-visibility`：见 `docs/ui/motion.md`「长列表优先分页」，而且
//! 计算器里的下拉面板是卡片内的 `absolute` 定位，会被它的绘制隔离裁掉。

use leptos::prelude::*;

/// 首屏直接挂载的个数：手机首屏只看得到 1–2 个，桌面 2–3 个。
const EAGER: usize = 3;

/// 之后每帧放行的个数：单个计算器在中端手机上约 5ms，四个一批不会形成长任务（50ms）。
const BATCH: usize = 4;

/// 本页的挂载进度（`provide_context` 注入，`ToolSection` 与 Smith 圆图那一节各自取用）。
#[derive(Clone, Copy)]
pub(super) struct ToolMount {
  /// 页面顺序：计算器在这里的下标就是它的挂载次序（与目录同一份清单）。
  order: &'static [(&'static str, &'static str)],
  /// 已放行的个数：下标小于它的计算器挂主体。
  limit: RwSignal<usize>,
}

impl ToolMount {
  /// 注入并返回本页的挂载进度。`order` 传目录清单（`TOOL_NAV`）。
  pub(super) fn provide(order: &'static [(&'static str, &'static str)]) -> Self {
    let mount = Self {
      order,
      limit: RwSignal::new(EAGER),
    };
    provide_context(mount);
    mount
  }

  /// 编号为 `id` 的计算器是否该挂主体。
  ///
  /// 用 `Memo`：每放行一批，60 个订阅方里只有刚跨过门槛的那几个会收到通知。
  /// 不在清单里的 `id` 一律立刻挂（新增计算器忘了登记目录时，至少不会一直空着）。
  pub(super) fn ready(self, id: &'static str) -> Memo<bool> {
    let index = self.order.iter().position(|&(i, _)| i == id).unwrap_or(0);
    Memo::new(move |_| self.limit.get() > index)
  }

  /// 首帧画出之后开始分批放行，直到全部挂上。页面构造完再调用。
  pub(super) fn start(self) {
    let total = self.order.len();
    crate::after_paint::each_frame(move || {
      // 页面在途中被卸载时信号已释放：`try_get_untracked` 返回 `None`，循环随之结束。
      // 已被 `mount_all` 一次放满（`usize::MAX`）时也在这里停下，不再累加。
      let Some(current) = self.limit.try_get_untracked() else {
        return false;
      };
      if current >= total {
        return false;
      }
      let next = (current + BATCH).min(total);
      self.limit.set(next);
      next < total
    });
  }

  /// 立刻挂上全部：要按真实高度定位（锚点直达、点目录）之前调用。
  pub(super) fn mount_all(self) {
    if !self.all_mounted() {
      self.limit.set(usize::MAX);
    }
  }

  /// 是否已全部挂上（不订阅）。
  pub(super) fn all_mounted(self) -> bool {
    self.limit.get_untracked() >= self.order.len()
  }
}
