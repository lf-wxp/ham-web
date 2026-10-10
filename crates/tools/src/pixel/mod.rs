//! 像素风资源的构建工具：精灵与图标、调色板校正、字体子集化、配色方案。
//!
//! 这些产物都提交入库，日常开发与 CI 不需要重新生成；改了源数据才跑对应的 `cargo make pixel-*`。

pub mod fonts;
pub mod palette;
pub mod schemes;
pub mod sprite_art;
pub mod sprites;

/// 语义 token（经典方案的亮 / 暗两套变量）。
pub const TOKENS_CSS: &str = "crates/app/style/pixel/tokens.css";
/// Tailwind 色阶的像素调色板。
pub const PALETTE_CSS: &str = "crates/app/style/pixel/palette.css";
/// 其余配色方案（生成物）。
pub const SCHEMES_CSS: &str = "crates/app/style/pixel/schemes.css";
