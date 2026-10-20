//! 页内视图切换（Tabs）：同一主题下若干「分层视图」的标签栏 + 面板。
//!
//! 与 [`ChipGroup`](super::ChipGroup) 的分工是**语义**而不是密度：
//!
//! - `ChipGroup` 是**筛选 / 范围选择** —— 选中的值留下来过滤同一份数据（`20m`、`A 类`、
//!   `未寄出 / 全部`），页面结构不变。它输出 `role="radiogroup"` / `role="radio"`。
//! - `Tabs` 是**视图切换** —— 选中项决定下面**显示哪一块面板**（Koch 训练 / 呼号抄收 /
//!   发报练习），面板之间互斥且各自成块。它输出 `role="tablist"` / `role="tab"` /
//!   `role="tabpanel"`，键盘按 WAI-ARIA 的 tabs 交互走（方向键移动焦点并顺带激活、
//!   Home / End 跳到首尾、roving tabindex 让整组只占一个 Tab 位）。
//! - 一页里**并列且需要同时看**的多个分区不要用 Tabs（那会藏掉内容）：用
//!   [`StickyPanel`](crate::components::common::StickyPanel) 的目录导航 + 纵向堆叠，
//!   工具页就是这么做的。
//!
//! 判错了就是回退：把筛选器标成 tab 会让「页面结构变了」的预期落空（读屏会按 tabpanel
//! 播报），把互斥视图留在 chip 里则拿不到 tab 的键盘与语义。拿不准时看**选中后下面那块
//! 内容是「过滤过的同一块」还是「换了一块」**。
//!
//! 面板**常驻挂载**，切换只切显隐（`hidden`）：被切走的面板保留自身状态（正在进行的
//! 一轮听抄、填了一半的表单），`aria-controls` 也永远指向真实存在的节点。代价是非激活
//! 面板的副作用照常运行 —— 与迁移前「所有分区都挂载在页面上」的行为一致。
//!
//! 本目录按「一个组件一个文件」组织（见 `AGENTS.md`），共享的组上下文在 [`shared`]。
//!
//! 典型用法：
//!
//! ```text
//! <Tabs value=tab on_change=Callback::new(move |v: String| tab.set(v))>
//!   <TabsList aria_label=Signal::derive(move || t("shell.training-utilities"))>
//!     <TabsTrigger value="koch">{move || t("morse.koch")}</TabsTrigger>
//!     <TabsTrigger value="send">{move || t("morse.sending-practice")}</TabsTrigger>
//!   </TabsList>
//!   <TabsPanel value="koch"><KochTrainer /></TabsPanel>
//!   <TabsPanel value="send"><SendTrainer /></TabsPanel>
//! </Tabs>
//! ```

mod shared;
mod tabs_list;
mod tabs_panel;
mod tabs_root;
mod tabs_trigger;

pub use tabs_list::TabsList;
pub use tabs_panel::TabsPanel;
pub use tabs_root::Tabs;
pub use tabs_trigger::TabsTrigger;
