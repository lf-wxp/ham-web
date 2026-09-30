//! 实时仪表盘：聚合太阳活动、空间天气警报、DX 热点、ISS 位置与 DXCC 稀有度。

mod alert_card;
mod dashboard_page;
mod iss_card;
mod solar_card;
mod spots_card;
mod wanted_card;

pub use dashboard_page::DashboardPage;
