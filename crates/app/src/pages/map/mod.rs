//! 世界地图共享绘制方案：等距圆柱投影 + 大陆轮廓 + 缩放/平移交互 + 灰线叠加。
//!
//! 灰线地图（[`GraylineOverlay`]）与网格地图（[`super::log::GridMap`]）共用本模块的
//! [`MapView`]：底图、经纬网、刻度、名称标注与缩放/平移/悬停交互在此统一实现，
//! 两者仅通过 `children` 注入各自的信息层（晨昏圈 vs 通联热力）。
//!
//! 关键决策见 [`MapView`] 的文档：等距圆柱投影（Plate Carrée）让 Maidenhead 网格
//! 退化为轴对齐矩形；`viewBox.x` 允许越过 ±180° 并在 x 方向平铺三份，实现反子午线环绕。

mod grayline_overlay;
mod map_view;
mod projection;
mod world_data;

pub use grayline_overlay::GraylineOverlay;
pub use map_view::MapView;
pub use projection::project;
