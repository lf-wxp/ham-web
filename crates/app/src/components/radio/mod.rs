//! 电台组件（像素风「16 位电台前面板」）：S 表、干扰频谱、信号格、电量格、刻度尺。
//!
//! 外观在 `style/pixel/radio.css`，规则见 `docs/ui/pixel.md` §2 硬规则 14–17 与 §3.4，
//! 纯计算（七段字表、S 表映射、旋钮落点、干扰形状）在 `ham_web_core::rig_panel`。
//!
//! 这里只产出 `pxl-*` 类名、全是只读的组合显示件：只在像素风的渲染分支里用（共享页面里要先
//! `is_modern_at_mount()` 分叉）。指示灯 `Led`、七段读数 `Readout` 与旋钮 `Knob` 是 `ui/` 里的基础件 ——
//! 按键、开关、统计格这些控件也要用它们，`ui/` 不反过来依赖这里。

mod battery;
mod qrm_signature;
mod signal_bars;
mod signal_meter;
mod tuning_scale;

pub use battery::Battery;
pub use qrm_signature::{QrmSignature, qrm_name};
pub use signal_bars::SignalBars;
pub use signal_meter::SignalMeter;
pub use tuning_scale::TuningScale;
