//! 数据表格：`<table>` 的唯一来源。
//!
//! 迁移前全站 77 个页面文件手写 `<table>`，样式各拼各的（`min-w-*`、表头底纹、行悬停
//! 三种写法互不一致）。这里把外观收口到 `style/pixel/controls.css` 的 `.pxl-table`：
//!
//! - 单元格网格线是发丝线（`--border`），表头用 muted 底纹，行悬停只在鼠标设备生效；
//! - 移动端策略内置于根组件：外层 `overflow-x-auto`，表格自身写 `min-w-[…]` 保住列宽；
//! - 卡片内贴边表格的「去最外圈双线」由 `surfaces.css` 按
//!   `:is(.pxl-card, .pxl-card > .overflow-x-auto) > table.border-collapse` 命中 ——
//!   因此 `border-collapse` 必须以**工具类**留在 DOM 上（`shared.rs`）。
//!
//! 本目录按「一个组件一个文件」组织（见 `AGENTS.md`），共享的样式 token 在 [`shared`]。
//!
//! 典型用法：
//!
//! ```text
//! <Table aria_label=… class="min-w-[640px]">
//!   <TableHead>
//!     <TableRow>
//!       <TableHeader>呼号</TableHeader>
//!       <TableHeader>国家</TableHeader>
//!     </TableRow>
//!   </TableHead>
//!   <TableBody>
//!     {rows.into_iter().map(|r| view! {
//!       <TableRow>
//!         <TableHeader scope="row">{r.callsign}</TableHeader>
//!         <TableCell>{r.country}</TableCell>
//!       </TableRow>
//!     }).collect_view()}
//!   </TableBody>
//! </Table>
//! ```

mod shared;
mod table_body;
mod table_cell;
mod table_head;
mod table_header;
mod table_root;
mod table_row;

pub use table_body::TableBody;
pub use table_cell::TableCell;
pub use table_head::TableHead;
pub use table_header::TableHeader;
pub use table_root::Table;
pub use table_row::TableRow;
