//! 线天线的矩量法求解（混合位积分方程 + 三角基 + Galerkin）。
//!
//! 覆盖偶极、垂直、八木、折角与方框这类由直线导线拼成的天线；只做理想导体
//! 地面，不做面结构、不做有耗地面。
//!
//! # 为什么是这个离散方式
//!
//! 前两次尝试都用**脉冲基**（段上电流为常数）。脉冲基的电流在节点跳变，于是电荷
//! 是节点上的**脉冲对**（点电荷），自位势只被导线半径 `a` 正则化；而物理上电荷是
//! 沿段分布的。实测后果是把天线当成了一根细得多的导线：λ/2 偶极阻抗
//! `122 + j288 Ω`（教科书 `73 + j42.5`），把半径强行放大 8 倍才回到 `87 + j53`。
//!
//! 这里改用**三角基**：未知量取节点电流，段内电流在两端节点之间线性过渡，
//! 电荷于是**分段连续**（`ρ = −(1/jω)·ΔI/Δz`），自项被段长而不是半径正则化。
//! 测试函数取同一组三角基（**Galerkin**），矩阵对称、功率泛函自洽。
//!
//! # 公式
//!
//! 导线表面切向电场为零，取弱形式（试验函数 `Λ_m` 在支撑端点为 0，分部积分无边界项）：
//!
//! ```text
//! −jω ∫ Λ_m A_s ds + ∫ (∂Λ_m/∂s) Φ ds = −∫ Λ_m E^i ds
//! ```
//!
//! 展开成矩阵：
//!
//! ```text
//! Z_mj = −jω(μ/4π) ∫∫ Λ_m Λ_j (û·û') G ds ds'
//!        + (1/4πε)·(−1/jω) ∫∫ (∂Λ_m/∂s)(∂Λ_j/∂s') G ds ds'
//! ```
//!
//! 两式都只需 `G = e^{−jkR}/R`（对数是唯一奇性，可积），比 Pocklington 的 `1/R⁵`
//! 核好处理得多。段对上的双积分用「外层 Gauss × 内层 sinh 换元」求：
//! 内层把 `s' = s_c + a·sinh t` 换元后 `R² = a²cosh²t + d⊥²` 光滑，
//! 面板数取 `span/1` 即可（对数核比 `1/R⁵` 温和得多）。
//!
//! # 边界条件与拓扑
//!
//! - 自由导线端（度为 1 的节点）电流固定为 0；
//! - 其余节点电流连续（折角、环路都成立）；
//! - 三根以上导线交汇成 T 型时，本实现会强制各支路电流相同 —— 这类拓扑暂不支持。
//!
//! # 精度
//!
//! 对照教科书值（`a/λ = 1e-4`、自由空间、40 段）：
//!
//! | 量 | 本实现 | 教科书 | 偏差 |
//! | --- | --- | --- | --- |
//! | λ/2 偶极 Z | `80.2 + j44.6 Ω` | `73.1 + j42.5 Ω` | 电阻 +10%、电抗 +5% |
//! | λ/2 偶极方向性系数 | `2.17 dBi` | `2.15 dBi` | +1% |
//! | 谐振长度 | `0.480λ` | `0.4787λ` | +0.3% |
//! | λ/4 单极 | 恰为偶极的一半 | 同 | < 1% |
//! | 三单元八木 | `6.7 dBi` / F/B `14 dB` | 约 `7 dBi` | 约 0.3 dB |
//!
//! 也就是说**相对量（方向图、增益、前后比、谐振长度、镜像关系）可信**，
//! 阻抗的绝对值有约 10% 的系统偏差；天线越短、越接近纯电容区，阻抗偏差越大
//! （0.2λ 偶极电阻准确到 1%，电抗偏大数十个百分点）。求积精度已排除：
//! 把面板数提高一个量级、远场对换成 8 点 Gauss，结果**完全不变**；偏差来自
//! 离散方式本身（三角基 + 点/线匹配的残差），不是数值误差。
//!
//! 用途定位：**比较不同几何谁更好**（加一根引向器增益涨多少、架高多少仰角更低），
//! 以及看方向图；不要把阻抗绝对值当作实测值。
//!
//! # 已知限制
//!
//! - 负载按与频率无关处理，只算单频点；绝缘层、导线损耗、有耗地面未建模；
//! - 三根以上导线交汇成 T 型时，各支路电流被强制相同，这类拓扑不支持。

use std::collections::HashMap;
use std::f64::consts::PI;

use crate::cx::Cx;

/// 真空光速（m/s，精确值）。
const C0: f64 = 299_792_458.0;
/// 真空介电常数（F/m）。
const EPS0: f64 = 8.854_187_812_8e-12;
/// 真空磁导率（H/m）。
const MU0: f64 = 1.256_637_062_12e-6;
/// 自由空间波阻抗（Ω）。
const ETA0: f64 = 376.730_313_668_017;

/// 单次求解允许的分段总数上限。
pub const MAX_SEGMENTS: usize = 200;

/// 驻波比与扫频带宽默认使用的参考阻抗（Ω）。
const Z0_REF: f64 = 50.0;

/// 方向图粗扫的角步长（度）：θ 每 2.5°、φ 每 5°。
const PATTERN_D_THETA: f64 = 2.5;
const PATTERN_D_PHI: f64 = 5.0;

/// 方位面 / 仰角面切片的取样点数（实际取 `SLICE_POINTS + 1` 个点）。
const SLICE_POINTS: usize = 180;

/// 三维方向图网格的划分数：θ 每 10°、φ 每 10°，球壳渲染直接吃这份数据。
const PATTERN3D_N_THETA: usize = 18;
const PATTERN3D_N_PHI: usize = 36;

/// 判定「近场段对」的阈值：最近距离小于 `NEAR_FACTOR × 源段长` 时走 sinh 换元。
const NEAR_FACTOR: f64 = 4.0;

/// sinh 换元后的目标面板宽度（对数核，比 `1/R⁵` 宽容）。
const PANEL_TARGET: f64 = 1.0;

/// 近场段对的内层每个面板的 Gauss 点数。
const GAUSS5: [(f64, f64); 5] = [
  (-0.906_179_845_938_664, 0.236_926_885_056_189_1),
  (-0.538_469_310_105_683_1, 0.478_628_670_499_366_5),
  (0.0, 0.568_888_888_888_888_9),
  (0.538_469_310_105_683_1, 0.478_628_670_499_366_5),
  (0.906_179_845_938_664, 0.236_926_885_056_189_1),
];

/// 远场段对与远场辐射积分用的 4 点 Gauss。
const GAUSS4: [(f64, f64); 4] = [
  (-0.861_136_311_594_052_6, 0.347_854_845_137_453_8),
  (-0.339_981_043_584_856_3, 0.652_145_154_862_546_2),
  (0.339_981_043_584_856_3, 0.652_145_154_862_546_2),
  (0.861_136_311_594_052_6, 0.347_854_845_137_453_8),
];

// ───────────────────────────── 向量工具 ─────────────────────────────

type Vec3 = [f64; 3];

fn sub(a: Vec3, b: Vec3) -> Vec3 {
  [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn add(a: Vec3, b: Vec3) -> Vec3 {
  [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn scale(a: Vec3, k: f64) -> Vec3 {
  [a[0] * k, a[1] * k, a[2] * k]
}

fn dot(a: Vec3, b: Vec3) -> f64 {
  a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn norm(a: Vec3) -> f64 {
  dot(a, a).sqrt()
}

/// 理想导体地面：镜像点。
fn mirror_pos(p: Vec3) -> Vec3 {
  [p[0], p[1], -p[2]]
}

/// 理想导体地面：电流镜像方向（水平分量反向、垂直分量同向）。
fn mirror_dir(d: Vec3) -> Vec3 {
  [-d[0], -d[1], d[2]]
}

// ───────────────────────────── 输入 / 输出 ─────────────────────────────

/// 一根直线导线。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Wire {
  /// 起点（m）。
  pub a: Vec3,
  /// 终点（m）。
  pub b: Vec3,
  /// 导线半径（m）。
  pub radius: f64,
  /// 分段数（≥ 1）。
  pub segments: usize,
}

impl Wire {
  /// 构造一根导线（分段数至少 1，半径至少 1 µm）。
  #[must_use]
  pub fn new(a: Vec3, b: Vec3, radius: f64, segments: usize) -> Self {
    Self {
      a,
      b,
      radius: radius.max(1e-6),
      segments: segments.max(1),
    }
  }

  /// 导线长度（m）。
  #[must_use]
  pub fn length(&self) -> f64 {
    norm(sub(self.b, self.a))
  }
}

/// δ 间隙馈电：`wire` 号导线上距起点 `at`（0–1）处，间隙电压 `volts`。
///
/// 馈电**不是**「吸附到最近的节点」：实现按 hat 权重把它分摊到相邻两个节点上，
/// 这样 `at` 在段内连续变化时阻抗也连续（吸附会让曲线在节点处跳变）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Feed {
  /// 导线序号。
  pub wire: usize,
  /// 沿该导线的相对位置，0 = 起点、1 = 终点。
  pub at: f64,
  /// 间隙电压（V）。
  pub volts: f64,
}

/// 地面模型。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Ground {
  /// 自由空间（无地面）。
  Free,
  /// 理想导体地面：镜像电流水平分量反向、垂直分量同向，反射系数恒为 `−1 / +1`。
  Perfect,
  /// 有耗地面：相对介电常数与电导率（S/m），按 Fresnel 反射系数折减镜像。
  ///
  /// 例：`Ground::lossy(13.0, 0.005)` 是 NEC 手册里常用的「中等干燥土壤」。
  Lossy {
    /// 相对介电常数。
    eps_r: f64,
    /// 电导率（S/m）。
    sigma: f64,
  },
}

impl Ground {
  /// 有耗地面（参数取绝对值，电导率下限 `1e-6` 以免出现无耗介质）。
  #[must_use]
  pub fn lossy(eps_r: f64, sigma: f64) -> Self {
    Self::Lossy {
      eps_r: eps_r.max(1.0),
      sigma: sigma.max(1e-6),
    }
  }

  /// 是否参与镜像（有无地面）。
  #[must_use]
  pub fn is_grounded(self) -> bool {
    !matches!(self, Self::Free)
  }

  /// 地面相对复介电常数 `ε_c = ε_r − jσ/(ωε₀)`；理想导体用极大值近似。
  fn relative_permittivity(self, omega: f64) -> Cx {
    match self {
      Self::Lossy { eps_r, sigma } => Cx::new(eps_r, -sigma / (omega * EPS0)),
      // 理想导体：ε_c → −j∞。取一个足够大的虚部即可，`reflection` 里会解析地
      // 走 ±1 分支，这里只是让函数保持全域定义。
      Self::Perfect | Self::Free => Cx::new(1.0, -1e9),
    }
  }

  /// Fresnel 反射系数：`elev_deg` 为射线相对地面的仰角（0 = 掠射）。
  ///
  /// `horizontal` 选水平极化（电场平行于地面），否则为垂直极化。返回复数系数，
  /// 掠射时分别趋于 `−1` / `+1`，与理想导体镜像的符号约定一致。
  fn reflection(self, omega: f64, elev_deg: f64, horizontal: bool) -> Cx {
    if matches!(self, Self::Perfect) {
      return Cx::of(if horizontal { -1.0 } else { 1.0 });
    }
    if !self.is_grounded() {
      return Cx::ZERO;
    }
    let sin_d = elev_deg.to_radians().sin().abs().max(1e-6);
    let cos_d = elev_deg.to_radians().cos();
    let eps = self.relative_permittivity(omega);
    let root = (eps - Cx::of(cos_d * cos_d)).sqrt();
    if horizontal {
      // R_⊥ = (sinΔ − √(ε_c − cos²Δ)) / (sinΔ + √(ε_c − cos²Δ))
      (Cx::of(sin_d) - root) / (Cx::of(sin_d) + root)
    } else {
      // R_∥ = (√(ε_c − cos²Δ) − ε_c·sinΔ) / (√(ε_c − cos²Δ) + ε_c·sinΔ)
      let num = root - eps * sin_d;
      let den = root + eps * sin_d;
      if den.abs() < 1e-30 {
        Cx::ZERO
      } else {
        num / den
      }
    }
  }
}

/// 集总串联负载（`LD` 卡片）：接在导线上某处的 R–L–C 串联支路。
///
/// 线圈加感的缩短型天线、陷波器、终端电阻都靠它建模。元件按理想元件处理
/// （无寄生参数），频率相关的电抗由 `ωL − 1/(ωC)` 给出。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Load {
  /// 导线序号。
  pub wire: usize,
  /// 沿该导线的相对位置，0 = 起点、1 = 终点。
  pub at: f64,
  /// 电阻（Ω）。
  pub r_ohm: f64,
  /// 电感（µH）。
  pub l_uh: f64,
  /// 电容（pF）。
  pub c_pf: f64,
}

impl Load {
  /// 该负载在工作角频率 `omega` 下的复阻抗（Ω）。
  #[must_use]
  pub fn impedance(self, omega: f64) -> Cx {
    let xl = omega * self.l_uh * 1e-6;
    let xc = if self.c_pf.abs() < 1e-12 {
      0.0
    } else {
      1.0 / (omega * self.c_pf * 1e-12)
    };
    Cx::new(self.r_ohm, xl - xc)
  }

  /// 是否为「无元件」的空负载。
  #[must_use]
  pub fn is_empty(self) -> bool {
    self.r_ohm == 0.0 && self.l_uh == 0.0 && self.c_pf == 0.0
  }
}

/// 求解输入。
/// 一段无耗传输线（两端口网络），接在天线的两个「端口」之间。
///
/// 端口用「第几根导线 + 相对位置」指定，与馈电点同一套坐标；线长按米给，
/// 另有速度因子（同轴线约 0.66–0.8、平行线约 0.95）。
///
/// 它建模的是**串进结构里**的一段馈线：端口 a、b 是结构上的两个断口，电流流经这条线。
/// 于是 λ/2 线等价于把断口直接连起来（阻抗透传），λ/4 线则是阻抗变换器 ——
/// 这两条正是单元测试的判据。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TransmissionLine {
  /// 端口 A 所在导线。
  pub wire_a: usize,
  /// 端口 A 在导线上的相对位置（0 = 起点，1 = 终点）。
  pub at_a: f64,
  /// 端口 B 所在导线。
  pub wire_b: usize,
  /// 端口 B 在导线上的相对位置。
  pub at_b: f64,
  /// 特性阻抗（Ω）。
  pub z0: f64,
  /// 线长（m）。
  pub length_m: f64,
  /// 速度因子（1 = 真空）。
  pub velocity_factor: f64,
}

impl TransmissionLine {
  /// 该频率下的电长度（弧度）。
  fn electrical_length(&self, omega: f64) -> f64 {
    omega * self.length_m / (C0 * self.velocity_factor.max(1e-6))
  }
}

#[derive(Debug, Clone, PartialEq)]
pub struct NecInput {
  /// 导线列表。
  pub wires: Vec<Wire>,
  /// 馈电点列表（通常 1 个）。
  ///
  /// 支持多个激励源（都进右端项），但 [`NecResult::impedance`] 只对**单馈电**有意义：
  /// 多源时第一个端口的电流 `I0` 受其它源影响，`V0 / I0` 不再是该端口的自阻抗。
  pub feeds: Vec<Feed>,
  /// 集总负载列表（可为空）。
  pub loads: Vec<Load>,
  /// 工作频率（Hz）。
  pub freq_hz: f64,
  /// 地面模型。
  pub ground: Ground,
  /// 导线电导率（S/m）；`None` 表示理想导体（无欧姆损耗）。
  ///
  /// 铜取 `5.8e7`、铝 `3.5e7`、黄铜 `1.6e7`。损耗按圆导线的表面电阻
  /// `R' = √(πfμ₀/σ)/(2πa)`（Ω/m）折算，用电流分布积分成欧姆损耗功率。
  pub conductivity: Option<f64>,
}

/// 求解结果。
#[derive(Debug, Clone, PartialEq)]
pub struct NecResult {
  /// 馈电点阻抗（Ω）：实部、虚部。
  ///
  /// **仅对单馈电有意义**（多激励源时取第一个端口的 `V0 / I0`，见 [`NecInput::feeds`]）。
  pub impedance: (f64, f64),
  /// 相对 50Ω 的驻波比。
  pub swr_50: f64,
  /// 最大增益（dBi）。含导线欧姆损耗与地面反射损耗，即**真实增益**。
  pub gain_max_dbi: f64,
  /// 最大方向性系数（dBi）。不含任何损耗，只由电流分布形状决定。
  pub directivity_dbi: f64,
  /// 辐射效率（0–1）：辐射功率 / 馈电功率。
  pub efficiency: f64,
  /// 欧姆损耗 + 地面反射损耗合计（dB）。
  pub loss_db: f64,
  /// 最大增益方向的仰角（度，地平线为 0）。
  pub gain_max_elevation_deg: f64,
  /// 最大增益方向的方位角（度，自 +X 轴起算，逆时针为正）。
  pub gain_max_azimuth_deg: f64,
  /// 前后比（dB）：最大方向与其反方向的增益差。
  pub front_to_back_db: f64,
  /// 分段总数。
  pub segments: usize,
  /// 未知量个数（节点电流）。
  pub unknowns: usize,
  /// 每段中心处的电流幅值（A）。
  pub currents: Vec<f64>,
  /// 方位面方向图：在最大增益仰角上 `(方位角°, 增益 dBi)`。
  pub azimuth: Vec<(f64, f64)>,
  /// 仰角面方向图：在最大增益方位上 `(仰角°, 增益 dBi)`。
  pub elevation: Vec<(f64, f64)>,
  /// 三维方向图网格：`(theta 步数, phi 步数)`。
  ///
  /// `theta` 自天顶量起，取 `0..=180°` 共 `pattern3d_theta + 1` 条；
  /// `phi` 取 `0..360°`（不含终点）共 `pattern3d_phi` 条。
  pub pattern3d_shape: (usize, usize),
  /// 三维方向图增益（dBi），按 `theta` 行主序、长度
  /// `(pattern3d_theta + 1) × pattern3d_phi`。
  pub pattern3d: Vec<f64>,
}

// ───────────────────────────── 内部模型 ─────────────────────────────

/// 一段导线。
#[derive(Debug, Clone, Copy)]
struct Seg {
  start: Vec3,
  /// 弧长 / 几何方向（单位向量）。
  dir: Vec3,
  /// 电流方向（单位向量）。实段与 [`Self::dir`] 相同；镜像段的几何按 `z` 翻转、
  /// 电流按 `(−Jx,−Jy,+Jz)` 翻转，两者**不再相同** —— 这正是 `cur·dir = −1`
  /// 的来源，也是电荷项要额外带一个 `cur·dir` 因子的原因。
  cur: Vec3,
  len: f64,
  radius: f64,
  start_node: usize,
  end_node: usize,
}

impl Seg {
  /// 段上弧长 `u` 处的点。
  fn point(&self, u: f64) -> Vec3 {
    add(self.start, scale(self.dir, u))
  }

  /// 理想导体地面上的镜像段：几何按 `z` 翻转，电流按 `(−Jx,−Jy,+Jz)` 翻转。
  fn mirrored(&self) -> Self {
    Self {
      start: mirror_pos(self.start),
      dir: [self.dir[0], self.dir[1], -self.dir[2]],
      cur: mirror_dir(self.cur),
      len: self.len,
      radius: self.radius,
      start_node: self.start_node,
      end_node: self.end_node,
    }
  }

  /// 电荷密度里 `cur·dir` 的因子：实段为 +1，镜像段为 −1。
  fn charge_sign(&self) -> f64 {
    dot(self.cur, self.dir).signum()
  }
}

/// 几何模型：段、节点、每个节点上挂的段。
#[derive(Debug, Clone)]
struct Model {
  segs: Vec<Seg>,
  nodes: Vec<Vec3>,
  /// `node_segs[j]`：节点 `j` 上挂的段号（度 ≤ 2 为常规；度 1 是自由端点）。
  node_segs: Vec<Vec<usize>>,
  /// `wire_start[w]`：第 `w` 根导线在 `segs` 里的首段号。
  wire_start: Vec<usize>,
  /// `flipped[w]`：第 `w` 根导线在模型里是否被**反向**遍历。
  ///
  /// 用户给的 `at`（0 = `a`、1 = `b`）始终按原方向解释，`locate` 负责换算；
  /// 模型内部反向是为了满足下面的「一进一出」约定。
  flipped: Vec<bool>,
}

impl Model {
  fn from_wires(wires: &[Wire]) -> Option<Self> {
    // 1) 先按用户给定的方向（`a` → `b`）等长分段，只存几何，不建节点。
    let mut pieces: Vec<Vec<(Vec3, Vec3)>> = Vec::with_capacity(wires.len());
    let mut dirs: Vec<(Vec3, f64)> = Vec::with_capacity(wires.len());
    for w in wires {
      let delta = sub(w.b, w.a);
      let len = norm(delta);
      // 半径同样要在唯一入口收口：`Wire` 的字段是公开的，直接构造（不走 `Wire::new`）
      // 可以绕过 `radius.max(1e-6)`；半径为 0 时 `(-vc/a).asinh()` 会产出 ±inf/NaN，
      // 最终表现为「求解失败」而不是「参数非法」。
      if !w.radius.is_finite() || w.radius <= 0.0 {
        return None;
      }
      // `is_finite()` 先挡掉 NaN / inf（`NaN <= 0.0` 为假，只判 `<= 0.0` 会放行 NaN
      // 坐标，最后以 `Some(NaN)` 冒充有效解）。
      if !len.is_finite() || len <= 0.0 {
        return None;
      }
      let dir = scale(delta, 1.0 / len);
      let n = w.segments.max(1);
      let dl = len / n as f64;
      let mut ps = Vec::with_capacity(n);
      for i in 0..n {
        ps.push((
          add(w.a, scale(dir, i as f64 * dl)),
          add(w.a, scale(dir, (i + 1) as f64 * dl)),
        ));
      }
      pieces.push(ps);
      dirs.push((dir, dl));
    }

    // 2) 决定每根导线在模型里的遍历方向。
    //
    //    弱形式把标量位项 `∫Λ_m ∂Φ/∂s ds` 分部积分后，**端点的边界项**在接点处必须
    //    互相抵消；这要求接点两侧的段「一进一出」。同一根导线内部天然满足（第 i 段的
    //    终点就是第 i+1 段的起点），但两根导线在接点处各自朝外就会留下未抵消的边界项 ——
    //    矩阵仍然对称，却不再满足能量守恒：实测「两臂都从顶点出发」的倒 V 解出
    //    `−22 Ω`、增益 `−118 dBi`，而同一条天线写成「一进一出」就是 `+30 Ω`、`1.56 dBi`。
    //
    //    做法：把导线端点当作图的边，从自由端（或任意端点）出发做一次 BFS，
    //    沿途决定每根导线是否需要反向。度为 2 的接点总能满足；度为 3 及以上
    //    （T 形/星形接点）无法同时满足，这时保持原样（属已知近似）。
    let mut node_index: HashMap<(i64, i64, i64), usize> = HashMap::new();
    let mut node_pos: Vec<Vec3> = Vec::new();
    let key_of = |p: Vec3| {
      (
        (p[0] * 1e7).round() as i64,
        (p[1] * 1e7).round() as i64,
        (p[2] * 1e7).round() as i64,
      )
    };
    let mut node_of = |p: Vec3| -> usize {
      let key = key_of(p);
      *node_index.entry(key).or_insert_with(|| {
        node_pos.push(p);
        node_pos.len() - 1
      })
    };
    let ends: Vec<(usize, usize)> = wires.iter().map(|w| (node_of(w.a), node_of(w.b))).collect();
    let mut at_node: Vec<Vec<(usize, bool)>> = vec![Vec::new(); node_pos.len()];
    for (i, &(na, nb)) in ends.iter().enumerate() {
      at_node[na].push((i, false));
      at_node[nb].push((i, true));
    }
    let mut flipped = vec![false; wires.len()];
    let mut seen = vec![false; wires.len()];
    for seed in 0..wires.len() {
      if seen[seed] {
        continue;
      }
      seen[seed] = true;
      let mut queue = vec![seed];
      while let Some(w) = queue.pop() {
        for &(node, is_end) in &[(ends[w].0, false), (ends[w].1, true)] {
          if at_node[node].len() != 2 {
            continue;
          }
          let (u, u_is_end) = if at_node[node][0].0 == w {
            at_node[node][1]
          } else {
            at_node[node][0]
          };
          if seen[u] {
            continue;
          }
          // 导线 `w` 在该节点是「进入」还是「离开」：进入的对面必须离开。
          let w_enters = if !flipped[w] { is_end } else { !is_end };
          // 未反向时 `u` 的方向是 a→b，此时「离开」意味着节点是它的起点端。
          flipped[u] = u_is_end == w_enters;
          seen[u] = true;
          queue.push(u);
        }
      }
    }

    // 3) 按模型方向生成段：反向的导线整体倒序，并在每段内把方向取反。
    let mut segs: Vec<Seg> = Vec::new();
    let mut wire_start = Vec::with_capacity(wires.len());
    for (i, ps) in pieces.iter().enumerate() {
      wire_start.push(segs.len());
      let (dir, len) = dirs[i];
      let radius = wires[i].radius;
      if flipped[i] {
        let back = scale(dir, -1.0);
        for &(_, p1) in ps.iter().rev() {
          segs.push(Seg {
            start: p1,
            dir: back,
            cur: back,
            len,
            radius,
            start_node: 0,
            end_node: 0,
          });
        }
      } else {
        for &(p0, _) in ps {
          segs.push(Seg {
            start: p0,
            dir,
            cur: dir,
            len,
            radius,
            start_node: 0,
            end_node: 0,
          });
        }
      }
    }

    // 4) 节点（端点按 0.1 µm 量化去重：折角、交汇处自动共节点，电流连续性由此成立）。
    let mut nodes: Vec<Vec3> = Vec::new();
    let mut index: HashMap<(i64, i64, i64), usize> = HashMap::new();
    let mut node_of = |p: Vec3, nodes: &mut Vec<Vec3>| -> usize {
      let key = (
        (p[0] * 1e7).round() as i64,
        (p[1] * 1e7).round() as i64,
        (p[2] * 1e7).round() as i64,
      );
      *index.entry(key).or_insert_with(|| {
        nodes.push(p);
        nodes.len() - 1
      })
    };
    for seg in segs.iter_mut() {
      let a = node_of(seg.start, &mut nodes);
      let b = node_of(add(seg.start, scale(seg.dir, seg.len)), &mut nodes);
      seg.start_node = a;
      seg.end_node = b;
    }

    let mut node_segs: Vec<Vec<usize>> = vec![Vec::new(); nodes.len()];
    for (i, seg) in segs.iter().enumerate() {
      node_segs[seg.start_node].push(i);
      node_segs[seg.end_node].push(i);
    }
    Some(Self {
      segs,
      nodes,
      node_segs,
      wire_start,
      flipped,
    })
  }

  /// 端口的正方向：`+1` 表示「结构在端口段的起点侧」。
  ///
  /// 端口是结构上的一个断口，电流从**结构那一侧**流入传输线。判断规则很局部：
  /// 端口段两端节点里度更大的那一侧就是结构侧（自由端度为 1）。把端口正好放在
  /// 内部段上（两端都是度 2）时取起点侧，这也是文档里写明的约定。
  fn port_sign(model: &Model, seg_idx: usize) -> f64 {
    let seg = model.segs[seg_idx];
    let (ds, de) = (
      model.node_segs[seg.start_node].len(),
      model.node_segs[seg.end_node].len(),
    );
    if de > ds { -1.0 } else { 1.0 }
  }

  /// 该节点电流是否固定为 0。
  ///
  /// 度为 1 的节点是自由端 → 电流为 0。**但接地面上有个例外**：理想导体地面上
  /// 垂直电流与其镜像同向、相互加强，电流在 `z = 0` 处并不为零（单极天线的
  /// 根电流正是馈电电流）；只有水平电流才被镜像抵消而必须为 0。
  fn is_fixed_end(&self, node: usize, ground: bool) -> bool {
    let segs = &self.node_segs[node];
    if segs.len() > 1 {
      return false;
    }
    if !ground || self.nodes[node][2].abs() > 1e-9 {
      return true;
    }
    // 接地面上的自由端：水平导线取 0，垂直导线放开。
    let Some(&s) = segs.first() else {
      return true;
    };
    self.segs[s].dir[2].abs() < 0.999
  }

  /// 节点 `j` 在段 `p` 上是「起点端」还是「终点端」：`0` 为起点端、`1` 为终点端。
  fn end_of(&self, node: usize, seg: usize) -> usize {
    usize::from(self.segs[seg].end_node == node)
  }
}

/// 把「第 `wire` 根导线上距起点 `at`（0–1）」映射成 `(段号, 段内归一化位置)`。
fn locate(model: &Model, wire: usize, at: f64) -> Option<(usize, f64)> {
  let start = *model.wire_start.get(wire)?;
  let end = model
    .wire_start
    .get(wire + 1)
    .copied()
    .unwrap_or(model.segs.len());
  if end <= start {
    return None;
  }
  let n = end - start;
  // 用户给的 `at` 始终按导线自身方向（0 = `a`、1 = `b`）解释；
  // 模型里若把这根导线反向遍历过，位置就要镜像一次。
  let t = at.clamp(0.0, 1.0);
  let t = if model.flipped.get(wire).copied().unwrap_or(false) {
    1.0 - t
  } else {
    t
  };
  let pos = t * n as f64;
  let idx = (pos.floor() as usize).min(n - 1);
  Some((start + idx, pos - idx as f64))
}

// ───────────────────────────── 段对双积分 ─────────────────────────────

/// 段对上的加权双积分 `∫∫ α^a β^b G du dv`（`α`、`β` 为两段各自的归一化坐标）。
///
/// 三角基在段上的取值是 `(1−α)` 或 `α`，`G = e^{−jkR}/R` 只要求这四个基本量，
/// 任意「起点端 / 终点端」组合都由它们线性组合得到。
#[derive(Debug, Clone, Copy, Default)]
struct Pair {
  s00: Cx,
  s10: Cx,
  s01: Cx,
  s11: Cx,
}

impl Pair {
  /// 测试函数取「起点端」权重 `(1−α)` 时的段对积分。
  fn test_start(&self, basis_end: usize) -> Cx {
    if basis_end == 0 {
      self.s00 - self.s10 - self.s01 + self.s11
    } else {
      self.s01 - self.s11
    }
  }

  /// 测试函数取「终点端」权重 `α` 时的段对积分。
  fn test_end(&self, basis_end: usize) -> Cx {
    if basis_end == 0 {
      self.s10 - self.s11
    } else {
      self.s11
    }
  }

  /// 无权重段对积分 `∫∫G`（电荷项用）。
  fn plain(&self) -> Cx {
    self.s00
  }
}

/// 观察点到源段的最近距离与最近点参数。
fn closest(p: Vec3, seg: &Seg) -> (f64, f64) {
  let rel = sub(p, seg.start);
  let t = dot(rel, seg.dir).clamp(0.0, seg.len);
  (norm(sub(rel, scale(seg.dir, t))), t)
}

/// 段对上的四个加权双积分。`p` 为观察（测试）段，半径取 `p` 的。
fn pair_integrals(p: &Seg, q: &Seg, k: f64) -> Pair {
  let a = p.radius;
  let (d_axis, _) = closest(p.point(0.5 * p.len), q);
  let near = d_axis <= NEAR_FACTOR * q.len;

  let mut acc = Pair::default();
  let outer = if near { &GAUSS5[..] } else { &GAUSS4[..] };
  for (xo, wo) in outer {
    let alpha = 0.5 * (xo + 1.0);
    let u = alpha * p.len;
    let rp = p.point(u);
    let mut inner = Pair::default();

    if near {
      // sinh 换元：`v = v_c + a·sinh t`，R 在近场变光滑
      let (_, vc) = closest(rp, q);
      let t_lo = (-vc / a).asinh();
      let t_hi = ((q.len - vc) / a).asinh();
      let span = t_hi - t_lo;
      let panels = ((span / PANEL_TARGET).ceil() as usize).clamp(1, 64);
      let dt = span / panels as f64;
      let half = 0.5 * dt;
      for panel in 0..panels {
        let mid = t_lo + dt * (panel as f64 + 0.5);
        for (xi, wi) in &GAUSS5 {
          let t = mid + half * xi;
          let (sinh_t, cosh_t) = (t.sinh(), t.cosh());
          let v = (vc + a * sinh_t).clamp(0.0, q.len);
          let beta = v / q.len;
          let w = wi * half * a * cosh_t;
          let g = green(rp, q.point(v), a, k);
          inner.s00 += g * w;
          inner.s10 += g * (w * alpha);
          inner.s01 += g * (w * beta);
          inner.s11 += g * (w * alpha * beta);
        }
      }
    } else {
      // 远场：G 光滑，直接 4 点 Gauss
      let half = 0.5 * q.len;
      for (xi, wi) in &GAUSS4 {
        let beta = 0.5 * (xi + 1.0);
        let v = beta * q.len;
        let g = green(rp, q.point(v), a, k);
        let w = wi * half;
        inner.s00 += g * w;
        inner.s10 += g * (w * alpha);
        inner.s01 += g * (w * beta);
        inner.s11 += g * (w * alpha * beta);
      }
    }

    let jac = 0.5 * p.len * wo;
    acc.s00 += inner.s00 * jac;
    acc.s10 += inner.s10 * jac;
    acc.s01 += inner.s01 * jac;
    acc.s11 += inner.s11 * jac;
  }
  acc
}

/// `G = e^{−jkR}/R`，`R² = |r − r'|² + a²`。
fn green(r: Vec3, rp: Vec3, a: f64, k: f64) -> Cx {
  let delta = sub(r, rp);
  let rr = (dot(delta, delta) + a * a).sqrt();
  let (sin_kr, cos_kr) = (k * rr).sin_cos();
  Cx::new(cos_kr, -sin_kr) * (1.0 / rr)
}

// ───────────────────────────── 装配与求解 ─────────────────────────────

/// 列主元高斯消元（就地修改 `a` 与 `b`）。
fn solve_linear(a: &mut [Vec<Cx>], b: &mut [Cx]) -> Option<Vec<Cx>> {
  let n = b.len();
  // 主元阈值取**相对**值：矩量法矩阵在谐振区本就病态，绝对阈值（原来是 `1e-18`）
  // 等价于「只判精确零」，近奇异时会一路放大误差、给出完全离谱的阻抗而不是 `None`。
  let scale = a
    .iter()
    .flat_map(|row| row.iter())
    .map(|v| v.abs())
    .fold(0.0_f64, f64::max);
  let tol = scale * PIVOT_REL_TOL;
  let mut pivot_row = vec![Cx::ZERO; n];
  for col in 0..n {
    let mut pivot = col;
    for row in col + 1..n {
      if a[row][col].abs() > a[pivot][col].abs() {
        pivot = row;
      }
    }
    if a[pivot][col].abs().is_nan() || a[pivot][col].abs() <= tol {
      return None;
    }
    a.swap(col, pivot);
    b.swap(col, pivot);
    pivot_row.copy_from_slice(&a[col]);
    let inv = Cx::ONE / a[col][col];
    for row in col + 1..n {
      let factor = a[row][col] * inv;
      if factor.abs2() < 1e-30 {
        continue;
      }
      for c in col..n {
        a[row][c] -= factor * pivot_row[c];
      }
      b[row] -= factor * b[col];
    }
  }
  let mut x = vec![Cx::ZERO; n];
  for i in (0..n).rev() {
    let mut acc = b[i];
    for j in i + 1..n {
      acc -= a[i][j] * x[j];
    }
    x[i] = acc / a[i][i];
  }
  Some(x)
}

/// 装配阻抗矩阵（未知量 = 节点电流，行 = 同一组三角试验函数）。
///
/// 三类附加项都在这里落地：
///
/// 1. **地面**：镜像项的电流按 Fresnel 反射系数折减 —— 水平分量乘 `R_h`、
///    垂直分量乘 `R_v`（`Ground::Perfect` 时二者恒为 `−1 / +1`，退化为纯镜像）；
///    电荷项跟着电流的散度一起缩放，`cur·dir` 因此也变成复数。
/// 2. **导线欧姆损耗**：按表面电阻折算出每段的串联电阻，落到 `∫Λ_mΛ_j ds` 上
///    （同节点 `Δz/3`、相邻节点 `Δz/6`）。
/// 3. **集总负载**：串联 R–L–C 的电压降写在弱形式右端，元素是
///    `Z_L·Λ_m(s_L)·Λ_j(s_L)`，用与馈电相同的一组 hat 权重摊到相邻两节点。
fn assemble(model: &Model, unknowns: &[usize], k: f64, input: &NecInput) -> Option<Vec<Vec<Cx>>> {
  let n = unknowns.len();
  let omega = k * C0;
  let freq = omega / (2.0 * PI);
  // 矢量位项系数 −jωμ/(4π)，标量位项系数 (1/4πε)·(−1/jω)
  let vec_coef = Cx::j(-omega * MU0) * (1.0 / (4.0 * PI));
  let scalar_coef = Cx::ONE / (Cx::j(omega * EPS0) * (4.0 * PI)) * -1.0;
  let mut z = vec![vec![Cx::ZERO; n]; n];
  // `unknowns` 由 `0..nodes` 过滤而来、天然升序，二分与线性扫描结果一致，
  // 但这段在欧姆损耗与集总负载的循环里按「段数 × 端点对」反复调用。
  let row_of = |node: usize| unknowns.binary_search(&node).ok();

  // 注：矩阵在理论上对称（Galerkin：试验函数与基函数同一组、格林函数对称，见模块文档），
  // 一度想「只算上三角再镜像」省掉一半装配量，但实测会让
  // `junction_orientation_does_not_change_the_result`（三线交汇的病态系统）里三份
  // 等价描述的结果漂到 1e-2 Ω 量级 —— 强制精确对称会改变列主元的选取路径。
  // 宁可慢一半也不动这个不变量；下面的 `row_of` 改二分、节点电流改稠密数组都是不改数值的优化。
  for (mi, &m) in unknowns.iter().enumerate() {
    for (sn, &j) in unknowns.iter().enumerate() {
      let mut vec_sum = Cx::ZERO;
      let mut scalar_sum = Cx::ZERO;
      // 试验函数 Λ_m 的支撑段、以及它在每段上的端点属性；
      // 基函数 Λ_j 同理。四种组合分别落到 Pair 的四个基本量上。
      for &p in &model.node_segs[m] {
        let mp = model.end_of(m, p);
        for &q in &model.node_segs[j] {
          let jq = model.end_of(j, q);
          let pair = pair_integrals(&model.segs[p], &model.segs[q], k);
          let weighted = if mp == 0 {
            pair.test_start(jq)
          } else {
            pair.test_end(jq)
          };
          vec_sum += weighted * dot(model.segs[p].cur, model.segs[q].cur);
          // 电荷项：∂Λ/∂u 在段上是常数，起点端为 −1/Δ、终点端为 +1/Δ；
          // 再乘两段的 `cur·dir`（镜像段为 −1）。
          let sp = if mp == 0 { -1.0 } else { 1.0 } / model.segs[p].len;
          let sq = if jq == 0 { -1.0 } else { 1.0 } / model.segs[q].len;
          scalar_sum +=
            pair.plain() * (sp * sq * model.segs[p].charge_sign() * model.segs[q].charge_sign());
          if input.ground.is_grounded() {
            // 镜像：几何 z 翻转；电流水平分量乘 R_h、垂直分量乘 R_v。
            let img = model.segs[q].mirrored();
            let elev = image_elevation(&model.segs[p], &model.segs[q]);
            let r_h = input.ground.reflection(omega, elev, true);
            let r_v = input.ground.reflection(omega, elev, false);
            let pair = pair_integrals(&model.segs[p], &img, k);
            let weighted = if mp == 0 {
              pair.test_start(jq)
            } else {
              pair.test_end(jq)
            };
            // 反射系数直接作用在**未取反**的源电流上：理想导体的 `R_h = −1`
            // 本身就已经是「水平分量反号」，若再叠一次 `mirrored()` 的反号，
            // 镜像会变成同向 —— 水平偶极的方向图会上下翻转（地面零点跑到天顶）。
            let (dh, dv) = split_hv(model.segs[p].cur, model.segs[q].cur);
            vec_sum += weighted * (r_h * dh + r_v * dv);
            // 电荷随电流散度一起缩放：镜像电流与镜像几何方向（z 翻转）的点乘。
            let (ch, cv) = split_hv(model.segs[q].cur, img.dir);
            let cq = r_h * ch + r_v * cv;
            let coef = sp * sq * model.segs[p].charge_sign();
            scalar_sum += pair.plain() * (cq * coef);
          }
        }
      }
      z[mi][sn] = vec_coef * vec_sum + scalar_coef * scalar_sum;
    }
  }

  // 导线欧姆损耗：圆导线表面电阻 R_s = √(πfμ₀/σ)，单位长电阻 R' = R_s/(2πa)。
  // 段内电流线性过渡，落到质量矩阵 ∫Λ_aΛ_b ds 上：同节点 Δz/3、相邻节点 Δz/6。
  if let Some(sigma) = input.conductivity {
    let r_sheet = (PI * freq * MU0 / sigma.max(1e-6)).sqrt();
    for seg in &model.segs {
      let r_seg = r_sheet / (2.0 * PI * seg.radius) * seg.len;
      let ends = [seg.start_node, seg.end_node];
      for (ia, &a_node) in ends.iter().enumerate() {
        let Some(mi) = row_of(a_node) else {
          continue;
        };
        for (ib, &b_node) in ends.iter().enumerate() {
          let Some(sn) = row_of(b_node) else {
            continue;
          };
          let overlap = if ia == ib { 1.0 / 3.0 } else { 1.0 / 6.0 };
          // 符号：装配出的矩阵代表**散射场** `E^s`，而方程是 `E^s + E^i = 0`，
          // 于是欧姆压降与集总负载都要从矩阵里**减去**（见模块文档的弱形式）。
          z[mi][sn] -= Cx::of(r_seg * overlap);
        }
      }
    }
  }

  // 集总负载：Z_L·Λ_m(s_L)·Λ_j(s_L)。
  for load in &input.loads {
    if load.is_empty() {
      continue;
    }
    let (seg_idx, alpha) = locate(model, load.wire, load.at)?;
    let seg = model.segs[seg_idx];
    let zl = load.impedance(omega);
    let ends = [(seg.start_node, 1.0 - alpha), (seg.end_node, alpha)];
    for &(a_node, wa) in &ends {
      let Some(mi) = row_of(a_node) else {
        continue;
      };
      for &(b_node, wb) in &ends {
        let Some(sn) = row_of(b_node) else {
          continue;
        };
        z[mi][sn] -= zl * (wa * wb);
      }
    }
  }

  Some(z)
}

/// 把两个方向的点乘拆成「水平分量 · 水平分量」与「垂直分量 · 垂直分量」两部分，
/// 便于对镜像电流分别乘 `R_h` / `R_v`。
fn split_hv(a: Vec3, b: Vec3) -> (f64, f64) {
  (a[0] * b[0] + a[1] * b[1], a[2] * b[2])
}

/// 镜像射线在地面处的仰角（度）：观察段高 `z_o`、源段高 `z_s`，而 [`Seg::mirrored`] 把
/// 源段翻到了 `-z_s`，因此「观察点 → 镜像」的垂距就是 `z_o + z_s`、水平距离为 `D`，
/// 于是仰角 `= atan((z_o + z_s) / D)`（掠射为 0°）。
/// 只在两者都极小时才真的落到掠射，避免 `atan(0/0)`。
fn image_elevation(obs: &Seg, src: &Seg) -> f64 {
  // 垂距**不乘 2**：`h` 已经是到镜像的垂距本身。曾经写成 `(2.0 * h)`，而自耦项因
  // `D = 0` 被 `atan2` 饱和到 90°（乘不乘都是 90°），互耦项的仰角却整体偏大一倍 ——
  // 有耗地面的 `R_h` / `R_v` 因此取错，阻抗与方向图静默偏掉，且没有任何测试能发现。
  let h = 0.5 * (obs.start[2] + obs.point(obs.len)[2]).abs()
    + 0.5 * (src.start[2] + src.point(src.len)[2]).abs();
  let dx =
    0.5 * (obs.start[0] + obs.point(obs.len)[0]) - 0.5 * (src.start[0] + src.point(src.len)[0]);
  let dy =
    0.5 * (obs.start[1] + obs.point(obs.len)[1]) - 0.5 * (src.start[1] + src.point(src.len)[1]);
  let d = dx.hypot(dy);
  h.atan2(d.max(1e-9)).to_degrees()
}

/// 某方向上由电流分布产生的远场辐射矢量 `N`（A·m，含镜像）。
///
/// `N = Σ_段 ∫ I(u)·û·e^{jk r(u)·r̂} du`，段内电流线性过渡。有耗地面时镜像项按
/// 该方向的仰角取 Fresnel 系数：水平分量乘 `R_h`、垂直分量乘 `R_v`。
fn radiation_vector(
  model: &Model,
  node_current: &[Cx],
  k: f64,
  rhat: Vec3,
  ground: Ground,
  elev_deg: f64,
) -> [Cx; 3] {
  let omega = k * C0;
  let mut n = [Cx::ZERO; 3];
  let (r_h, r_v) = if ground.is_grounded() {
    (
      ground.reflection(omega, elev_deg, true),
      ground.reflection(omega, elev_deg, false),
    )
  } else {
    (Cx::ZERO, Cx::ZERO)
  };
  for seg in &model.segs {
    let i0 = node_current[seg.start_node];
    let i1 = node_current[seg.end_node];
    let mut acc = Cx::ZERO;
    let half = 0.5 * seg.len;
    for (x, w) in GAUSS4 {
      let alpha = 0.5 * (x + 1.0);
      let i = i0 * (1.0 - alpha) + i1 * alpha;
      let p = seg.point(alpha * seg.len);
      acc += i * Cx::j(k * dot(p, rhat)).exp() * w;
    }
    acc = acc * half;
    for (c, slot) in n.iter_mut().enumerate() {
      *slot += acc * seg.dir[c];
    }
    if ground.is_grounded() {
      let img = seg.mirrored();
      let mut acc = Cx::ZERO;
      for (x, w) in GAUSS4 {
        let alpha = 0.5 * (x + 1.0);
        let i = i0 * (1.0 - alpha) + i1 * alpha;
        let p = img.point(alpha * img.len);
        acc += i * Cx::j(k * dot(p, rhat)).exp() * w;
      }
      acc = acc * half;
      // 镜像电流：源电流的水平分量 ×R_h、垂直分量 ×R_v（不再额外反号一次）。
      let cx = [
        acc * (r_h * seg.cur[0]),
        acc * (r_h * seg.cur[1]),
        acc * (r_v * seg.cur[2]),
      ];
      for (c, slot) in n.iter_mut().enumerate() {
        *slot += cx[c];
      }
    }
  }
  n
}

/// 单位球上的方向基。
fn direction_basis(theta_deg: f64, phi_deg: f64) -> (Vec3, Vec3, Vec3) {
  let (st, ct) = theta_deg.to_radians().sin_cos();
  let (sp, cp) = phi_deg.to_radians().sin_cos();
  (
    [st * cp, st * sp, ct],
    [ct * cp, ct * sp, -st],
    [-sp, cp, 0.0],
  )
}

/// 某方向的 `|N⊥|²`。`theta_deg` 自天顶量起，`90 − θ` 即仰角。
fn perp2(model: &Model, cur: &[Cx], k: f64, ground: Ground, theta_deg: f64, phi_deg: f64) -> f64 {
  let (rhat, that, phat) = direction_basis(theta_deg, phi_deg);
  let n = radiation_vector(model, cur, k, rhat, ground, 90.0 - theta_deg);
  let nt: Cx = n[0] * that[0] + n[1] * that[1] + n[2] * that[2];
  let np: Cx = n[0] * phat[0] + n[1] * phat[1] + n[2] * phat[2];
  nt.abs2() + np.abs2()
}

/// 由阻抗与参考阻抗算驻波比。
///
/// 分母只在 `z_r = −z0` 且 `z_i = 0`（即 Γ 模无穷大）时为零：不设下限的话
/// `inf / inf` 会得到 `NaN`，一路冒充有效解带到界面上。
fn swr_for(z_r: f64, z_i: f64, z0: f64) -> f64 {
  let gamma = (z_r - z0).hypot(z_i) / (z_r + z0).hypot(z_i).max(1e-9);
  (1.0 + gamma) / (1.0 - gamma).max(1e-9)
}

/// 一次 MoM 求解的公共结果（**不含方向图**）。
///
/// 方向图那一段（球面粗扫约 2600 个方向 + 三维网格 684 点）是单次求解里最贵的
/// 部分，而扫频只需要阻抗 —— 所以把「装配 → 解方程 → 取馈电阻抗」单独拆出来，
/// 扫频每个频点只付这一段的代价。
struct Solved {
  model: Model,
  k: f64,
  /// 按节点号稠密展开的节点电流（自由端为 0）。
  node_current: Vec<Cx>,
  impedance: (f64, f64),
  swr_50: f64,
  i_feed: Cx,
  unknowns: usize,
}

/// 「装配 → 解线性方程组 → 取馈电阻抗」这一段。
fn solve_model(input: &NecInput, lines: &[TransmissionLine]) -> Option<Solved> {
  // `is_finite()` 先挡掉 NaN 与 ±inf：只写 `freq_hz <= 0.0` 会放行 NaN（比较为假），
  // 一路算到 `Some(NaN)` 冒充有效解。有限性判定在前，后面的 `<=` 才不会碰到 NaN。
  if input.wires.is_empty()
    || input.feeds.is_empty()
    || !input.freq_hz.is_finite()
    || input.freq_hz <= 0.0
  {
    return None;
  }
  let total: usize = input.wires.iter().map(|w| w.segments.max(1)).sum();
  if total > MAX_SEGMENTS {
    return None;
  }
  let model = Model::from_wires(&input.wires)?;
  if model.segs.is_empty() {
    return None;
  }
  let k = 2.0 * PI * input.freq_hz / C0;

  // 传输线的端子：那一端的导线端点电流**不能**按「自由端」钉成 0 ——
  // 传输线正是要从那里把电流送出去（不放开的话端口看起来像个小电容，
  // 实测零长线给出的阻抗会比连通时高一倍、电抗大 40 倍）。
  let mut line_nodes: Vec<usize> = Vec::new();
  for line in lines {
    for (wire, at) in [(line.wire_a, line.at_a), (line.wire_b, line.at_b)] {
      let (seg_idx, alpha) = locate(&model, wire, at)?;
      let seg = model.segs[seg_idx];
      if (1.0 - alpha).abs() > 1e-12 {
        line_nodes.push(seg.start_node);
      }
      if alpha.abs() > 1e-12 {
        line_nodes.push(seg.end_node);
      }
    }
  }

  // 未知量：非自由端的节点电流。自由端电流为 0（传输线端子除外）。
  let unknowns: Vec<usize> = (0..model.nodes.len())
    .filter(|&j| !model.is_fixed_end(j, input.ground.is_grounded()) || line_nodes.contains(&j))
    .collect();
  if unknowns.is_empty() {
    return None;
  }

  // 馈电：δ 间隙可放在导线任意位置。三角基下 `∫Λ_m E^i ds = V·Λ_m(s_f)`，
  // 于是把激励按 hat 权重分摊到相邻两个节点 —— 不"吸附到最近节点"。
  let mut z = assemble(&model, &unknowns, k, input)?;
  let mut rhs = vec![Cx::ZERO; unknowns.len()];
  let mut feed_pos = Vec::new();
  for f in &input.feeds {
    let (seg_idx, alpha) = locate(&model, f.wire, f.at)?;
    let seg = model.segs[seg_idx];
    for (node, w) in [(seg.start_node, 1.0 - alpha), (seg.end_node, alpha)] {
      if let Some(row) = unknowns.iter().position(|&j| j == node) {
        rhs[row] += Cx::of(-f.volts * w);
      }
    }
    feed_pos.push((seg_idx, alpha, f.volts));
  }

  // 传输线段：把两端口网络的约束接进系统。
  //
  // 未知量扩成 `[电流…; 端口电压…]`：MoM 方程里出现 `−V_p·Λ`（δ 间隙激励），
  // 把电压也当成未知量、再补上网络自己的两行约束，系统仍是方阵 ——
  // 端口电压与电流是**一起**解出来的，而不是先解电流再事后修正。
  //
  // 约束用链矩阵（端口电流都取「流入网络」为正）：
  //   V_a = A·V_b − B·I_b，  I_a = C·V_b − D·I_b
  // 其中 `A = D = cosθ`、`B = jZ0·sinθ`、`C = j·sinθ/Z0`，`θ = ωℓ/(c·vf)`。
  // 代进去可得两个极端都对：`θ = 0`（零长线）给出 `V_a = V_b, I_a = −I_b`，
  // 就是一根导线；`θ = π`（λ/2 线）给出 `V_a = −V_b, I_a = I_b`，
  // 阻抗原样透传（`Z_in = Z_b`）。
  let n = unknowns.len();
  let np = lines.len() * 2;
  let mut port_w: Vec<Vec<Cx>> = Vec::with_capacity(np);
  let mut port_sign_v: Vec<f64> = Vec::with_capacity(np);
  if np > 0 {
    if lines.iter().any(|l| {
      l.z0 <= 0.0
        || l.length_m < 0.0
        || l.velocity_factor <= 0.0
        || l.at_a.is_nan()
        || l.at_b.is_nan()
    }) {
      return None;
    }
    rhs.resize(n + np, Cx::ZERO);
    for row in z.iter_mut() {
      row.resize(n + np, Cx::ZERO);
    }
    z.resize(n + np, vec![Cx::ZERO; n + np]);

    // 端口段上的 hat 权重：激励列与约束行共用同一组权重，口径一致。
    for (p, line) in lines.iter().enumerate() {
      for (slot, wire, at) in [
        (2 * p, line.wire_a, line.at_a),
        (2 * p + 1, line.wire_b, line.at_b),
      ] {
        let (seg_idx, alpha) = locate(&model, wire, at)?;
        let seg = model.segs[seg_idx];
        let mut col = vec![Cx::ZERO; n];
        for (node, w) in [(seg.start_node, 1.0 - alpha), (seg.end_node, alpha)] {
          if let Some(row) = unknowns.iter().position(|&j| j == node) {
            col[row] += Cx::of(w);
          }
        }
        // MoM 方程的右端是 `−Ṽ_p·Λ`，把这一项移到矩阵里，端口电压就成了未知量。
        // （`Ṽ_p` 是沿该**段自身方向**的间隙电压；端口朝向由 `port_sign` 换算。）
        for (row, c) in col.iter().enumerate() {
          z[row][n + slot] += *c;
        }
        port_w.push(col);
        port_sign_v.push(Model::port_sign(&model, seg_idx));
      }
    }
    let omega = k * C0;
    for (p, line) in lines.iter().enumerate() {
      let theta = line.electrical_length(omega);
      let (sin, cos) = theta.sin_cos();
      let (pa, pb) = (2 * p, 2 * p + 1);
      // 两个端口的朝向可能相反（一个在导线末端、一个在首端），链矩阵要按
      // `ρ = s_a·s_b` 换算：`Ṽ_a = ρ(A·Ṽ_b − B·Ĩ_b)`、`Ĩ_a = ρ(C·Ṽ_b − D·Ĩ_b)`。
      let rho = Cx::of(port_sign_v[pa] * port_sign_v[pb]);
      // 行 pa：Ṽ_a − ρ·cosθ·Ṽ_b + ρ·jZ0·sinθ·Ĩ_b = 0
      z[n + pa][n + pa] += Cx::ONE;
      z[n + pa][n + pb] -= rho * Cx::of(cos);
      for (i, w) in port_w[pb].iter().enumerate() {
        z[n + pa][i] += rho * Cx::j(line.z0 * sin) * *w;
      }
      // 行 pb：Ĩ_a − ρ·j(sinθ/Z0)·Ṽ_b + ρ·cosθ·Ĩ_b = 0
      for (i, w) in port_w[pa].iter().enumerate() {
        z[n + pb][i] += *w;
      }
      z[n + pb][n + pb] -= rho * Cx::j(sin / line.z0);
      for (i, w) in port_w[pb].iter().enumerate() {
        z[n + pb][i] += rho * Cx::of(cos) * *w;
      }
    }
  }

  let sol = solve_linear(&mut z, &mut rhs)?;

  // 节点电流表（自由端为 0）：按节点号稠密摊成数组。
  //
  // 方向图那一段是「每个方向 × 每一段」都要取两端节点电流（约 3700 个方向 × 200 段
  // × 2 次查找），用 `HashMap` 就是几百万次随机探测；摊成下标数组后只剩数组访问。
  let mut node_current = vec![Cx::ZERO; model.nodes.len()];
  for (i, &j) in unknowns.iter().enumerate() {
    node_current[j] = sol[i];
  }

  // 馈电点电流 = 同一组 hat 权重的插值（与激励口径一致）。
  let (feed_seg, feed_alpha, feed_volts) = feed_pos[0];
  let seg = model.segs[feed_seg];
  let node_i = |node: usize| -> Cx { node_current[node] };
  let i_feed = node_i(seg.start_node) * (1.0 - feed_alpha) + node_i(seg.end_node) * feed_alpha;
  if i_feed.abs() < 1e-18 {
    return None;
  }
  let zin = Cx::of(feed_volts) / i_feed;
  // 解出来再校验一次有限性：病态矩阵可能在消元后仍留下 inf/NaN，此时应当报「无解」，
  // 而不是把 NaN 一路带到增益、驻波比与方向图上。
  if !zin.re.is_finite() || !zin.im.is_finite() {
    return None;
  }
  // 驻波比同样要校验：`swr_for` 的分母已设下限，但极端阻抗仍可能给出 `inf`。
  let swr_50 = swr_for(zin.re, zin.im, Z0_REF);
  if !swr_50.is_finite() {
    return None;
  }
  Some(Solved {
    model,
    k,
    node_current,
    impedance: (zin.re, zin.im),
    swr_50,
    i_feed,
    unknowns: unknowns.len(),
  })
}

/// 求解一次，得到阻抗 / 驻波比 / 增益 / 效率与方向图。
///
/// 非法输入（无导线、频率非正、分段超限、馈电越界、零长度导线）返回 `None`。
/// 求解一次，并接入若干段传输线（`solve` 的完整版本）。
#[must_use]
pub fn solve_with_lines(input: &NecInput, lines: &[TransmissionLine]) -> Option<NecResult> {
  let Solved {
    model,
    k,
    node_current,
    impedance: (z_r, z_i),
    swr_50: swr,
    i_feed,
    unknowns,
  } = solve_model(input, lines)?;

  // 段中心电流幅值（展示用：`/nec` 页面的电流分布图直接吃这份数据）。
  let currents = model
    .segs
    .iter()
    .map(|s| ((node_current[s.start_node] + node_current[s.end_node]) * 0.5).abs())
    .collect();

  // 方向图：粗扫同时累积球面功率积分。方向性系数只看电流分布形状；
  // 真实增益再乘效率（= 辐射功率 / 馈电功率），效率由两项功率各算一遍得到 ——
  // 无损情形下两者应相等到 1e-3 以内（单测钉住了这一点）。
  let grounded = input.ground.is_grounded();
  let theta_top = if grounded { 90.0 } else { 180.0 };
  let (d_theta, d_phi) = (PATTERN_D_THETA, PATTERN_D_PHI);
  let mut integ = 0.0f64;
  let mut peak = 0.0f64;
  let mut best = (theta_top, 0.0);
  let mut theta = 0.0;
  while theta <= theta_top + 1e-9 {
    let mut phi = 0.0;
    while phi < 360.0 - 1e-9 {
      let v = perp2(&model, &node_current, k, input.ground, theta, phi);
      integ += v * theta.to_radians().sin() * d_theta.to_radians() * d_phi.to_radians();
      if v > peak {
        peak = v;
        best = (theta, phi);
      }
      phi += d_phi;
    }
    theta += d_theta;
  }
  let denom = integ.max(1e-30);
  // 辐射功率（W）与馈电功率（W）。系数 P = k²η/(32π²)·∫|N⊥|²dΩ 由
  // `|E| = ωμ|N⊥|/(4πr)` 与 `ωμ = kη` 推出。
  let p_rad = k * k * ETA0 / (32.0 * PI * PI) * integ;
  let p_in = 0.5 * z_r * i_feed.abs2();
  let efficiency = if p_in > 0.0 {
    (p_rad / p_in).clamp(0.0, 1.0)
  } else {
    0.0
  };
  let loss_db = -10.0 * efficiency.max(1e-12).log10();
  let directivity_at = |th: f64, ph: f64| -> f64 {
    10.0
      * (4.0 * PI * perp2(&model, &node_current, k, input.ground, th, ph) / denom)
        .max(1e-30)
        .log10()
  };
  let gain_at = |th: f64, ph: f64| -> f64 { directivity_at(th, ph) - loss_db };
  let (theta_best, phi_best) = best;
  let gain_max = gain_at(theta_best, phi_best);
  let directivity_dbi = directivity_at(theta_best, phi_best);
  let back = gain_at(theta_best, phi_best + 180.0);

  let azimuth = (0..=SLICE_POINTS)
    .map(|i| {
      let phi = 360.0 * i as f64 / SLICE_POINTS as f64;
      (phi, gain_at(theta_best, phi))
    })
    .collect();
  // 三维方向图网格：θ 每 10°、φ 每 10°。球壳渲染直接吃这份数据。
  let (n_theta, n_phi) = (PATTERN3D_N_THETA, PATTERN3D_N_PHI);
  let mut pattern3d = Vec::with_capacity((n_theta + 1) * n_phi);
  for i in 0..=n_theta {
    let th = 180.0 * i as f64 / n_theta as f64;
    for j in 0..n_phi {
      let ph = 360.0 * j as f64 / n_phi as f64;
      // 有地面时下半空间没有辐射：球壳那一半直接填下限，不要镜像出假的瓣。
      pattern3d.push(if grounded && th > 90.0 {
        -300.0
      } else {
        gain_at(th, ph)
      });
    }
  }

  let (e_lo, e_hi) = if grounded { (0.0, 90.0) } else { (-90.0, 90.0) };
  let elevation = (0..=SLICE_POINTS)
    .map(|i| {
      let elev = e_lo + (e_hi - e_lo) * i as f64 / SLICE_POINTS as f64;
      (elev, gain_at(90.0 - elev, phi_best))
    })
    .collect();

  Some(NecResult {
    impedance: (z_r, z_i),
    swr_50: swr,
    gain_max_dbi: gain_max,
    directivity_dbi,
    efficiency,
    loss_db,
    gain_max_elevation_deg: 90.0 - theta_best,
    gain_max_azimuth_deg: phi_best,
    front_to_back_db: gain_max - back,
    segments: model.segs.len(),
    unknowns,
    currents,
    azimuth,
    elevation,
    pattern3d_shape: (n_theta, n_phi),
    pattern3d,
  })
}

// ───────────────────────────── 频带扫描 ─────────────────────────────

/// 线性方程组的主元相对阈值：低于「矩阵量级 × 本值」即判为奇异。
///
/// 取 `1e-12` 而不是 0：病态矩阵（谐振区的矩量法矩阵、几何退化的模型）在双精度下
/// 只剩下十来个有效位，继续消元得到的是噪声放大后的数值而不是解。
const PIVOT_REL_TOL: f64 = 1e-12;

/// 扫频最多取多少个频点（每个频点都要重装一遍矩阵，给个上界防止页面卡住）。
pub const MAX_SWEEP_POINTS: usize = 201;

/// 二分收紧带宽边界的迭代次数（10 次 ≈ 0.1% 的频率精度）。
const BISECT_STEPS: usize = 10;

/// 频带扫描中的一个频点。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SweepPoint {
  /// 频率（Hz）。
  pub freq_hz: f64,
  /// 馈电点阻抗（Ω）。
  pub impedance: (f64, f64),
  /// 对 50 Ω 的驻波比。
  pub swr_50: f64,
}

/// 求解一次（不含传输线）。
#[must_use]
pub fn solve(input: &NecInput) -> Option<NecResult> {
  solve_with_lines(input, &[])
}

/// 只求馈电阻抗（Ω）—— 扫频的每个频点都走这条快路径。
#[must_use]
pub fn impedance_with_lines(input: &NecInput, lines: &[TransmissionLine]) -> Option<(f64, f64)> {
  solve_model(input, lines).map(|s| s.impedance)
}

/// 只求馈电阻抗（Ω，不含传输线）。
#[must_use]
pub fn impedance(input: &NecInput) -> Option<(f64, f64)> {
  impedance_with_lines(input, &[])
}

/// 把输入的中心频率换成 `f`（其余不变）。
#[cfg(test)]
fn retuned(input: &NecInput, f_hz: f64) -> NecInput {
  let mut probe = input.clone();
  probe.freq_hz = f_hz;
  probe
}

/// 在 `[f_lo, f_hi]` 上均匀取 `points` 个频点，逐点求阻抗与驻波比。
///
/// 只算阻抗、不算方向图；某个频点解不出来时会被跳过（返回的条数可能少于 `points`）。
///
/// 因此返回的点**不是**等间距的：画图请用 [`SweepPoint::freq_hz`] 作为横坐标
/// （`pages::nec::sweep` 就是这么做的），别按索引等分，否则缺一个点会让曲线错位。
#[must_use]
pub fn sweep(input: &NecInput, f_lo_hz: f64, f_hi_hz: f64, points: usize) -> Vec<SweepPoint> {
  sweep_with_lines(input, &[], f_lo_hz, f_hi_hz, points)
}

/// 频带扫描（含传输线）。
#[must_use]
pub fn sweep_with_lines(
  input: &NecInput,
  lines: &[TransmissionLine],
  f_lo_hz: f64,
  f_hi_hz: f64,
  points: usize,
) -> Vec<SweepPoint> {
  let points = points.clamp(2, MAX_SWEEP_POINTS);
  let lo = f_lo_hz.min(f_hi_hz).max(1.0);
  let hi = f_lo_hz.max(f_hi_hz).max(1.0);
  // 复用同一份输入副本、逐点只改 `freq_hz`：原来每个频点都 clone 一遍整个 `NecInput`，
  // 201 点时是 201 份导线表的无谓分配。
  let mut probe = input.clone();
  (0..points)
    .filter_map(|i| {
      let f = lo + (hi - lo) * i as f64 / (points - 1) as f64;
      probe.freq_hz = f;
      let (r, x) = impedance_with_lines(&probe, lines)?;
      Some(SweepPoint {
        freq_hz: f,
        impedance: (r, x),
        swr_50: swr_for(r, x, Z0_REF),
      })
    })
    .collect()
}

/// 围绕输入自身的频率，在 `±span_frac` 范围内找 SWR ≤ `limit` 的**连续**频带。
///
/// 粗扫定位跨越限值的那一档，再二分收紧两侧边界 —— 返回的边界上 SWR 收敛到
/// `limit`，带内取样全部达标。中心频率上就不达标、或整个扫描范围内都跨不过
/// 限值时返回 `None`（后者说明带宽比 `span_frac` 还宽，页面应提示放宽范围）。
#[must_use]
pub fn swr_bandwidth(
  input: &NecInput,
  limit: f64,
  span_frac: f64,
  points: usize,
) -> Option<(f64, f64)> {
  swr_bandwidth_with_lines(input, &[], limit, span_frac, points)
}

/// SWR ≤ `limit` 的连续带宽（含传输线）。
#[must_use]
pub fn swr_bandwidth_with_lines(
  input: &NecInput,
  lines: &[TransmissionLine],
  limit: f64,
  span_frac: f64,
  points: usize,
) -> Option<(f64, f64)> {
  let f0 = input.freq_hz;
  if f0 <= 0.0 {
    return None;
  }
  let limit = limit.max(1.0);
  let span = f0 * span_frac.clamp(1e-3, 0.9);
  // 同上：整段扫描共用一份输入副本，逐点只改频率。
  let mut probe = input.clone();
  let swr_at = |probe: &mut NecInput, f: f64| -> Option<f64> {
    probe.freq_hz = f;
    impedance_with_lines(probe, lines).map(|(r, x)| swr_for(r, x, Z0_REF))
  };
  if swr_at(&mut probe, f0)? > limit {
    return None;
  }
  let n = points.clamp(4, MAX_SWEEP_POINTS);
  let steps = (n / 2).max(1);
  let step = span / steps as f64;
  // 从中心往一侧走，找到第一档越过限值的位置，再在相邻两点间二分。
  let edge = |probe: &mut NecInput, dir: f64| -> Option<f64> {
    let mut prev = f0;
    for i in 1..=steps {
      let f = f0 + dir * step * i as f64;
      if f <= 1.0 {
        break;
      }
      let s = swr_at(probe, f)?;
      if s > limit {
        let (mut a, mut b) = (prev, f);
        for _ in 0..BISECT_STEPS {
          let mid = 0.5 * (a + b);
          if swr_at(probe, mid)? > limit {
            b = mid;
          } else {
            a = mid;
          }
        }
        return Some(0.5 * (a + b));
      }
      prev = f;
    }
    None
  };
  let lo = edge(&mut probe, -1.0)?;
  let hi = edge(&mut probe, 1.0)?;
  Some((lo, hi))
}

// ───────────────────────────── 八木设计向导 ─────────────────────────────

/// 设计向导每个单元的段数。
const DESIGN_SEGMENTS: usize = 10;
/// 坐标下降的轮数。
const DESIGN_PASSES: usize = 2;
/// 长度类参数每轮试的倍数（中心值两侧各一个）。
const DESIGN_STEPS: [f64; 3] = [0.94, 1.0, 1.06];
/// 间距参数每轮试的倍数（间距对前后比更敏感，给宽一些）。
const DESIGN_SPACING_STEPS: [f64; 3] = [0.85, 1.0, 1.18];
/// 反射器长度比的允许区间：真正起作用的反射器必须**长于**有源振子。
const REFL_RATIO_RANGE: (f64, f64) = (1.02, 1.12);
/// 引向器长度比的允许区间。
const DIR_RATIO_RANGE: (f64, f64) = (0.82, 1.0);

/// 八木设计向导的结果（尺寸以米为单位）。
#[derive(Debug, Clone, PartialEq)]
pub struct YagiDesign {
  /// 中心频率（Hz）。
  pub freq_hz: f64,
  /// 单元数（含反射器与有源振子）。
  pub elements: usize,
  /// 波长（m）。
  pub wavelength_m: f64,
  /// 各单元长度（m），顺序为 反射器 → 有源振子 → 引向器…。
  pub lengths: Vec<f64>,
  /// 相邻单元间距（m），长度 = `elements − 1`，顺序同上。
  pub spacings: Vec<f64>,
  /// 设计完成后**用本模块求解器实测**的增益（dBi）。
  pub gain_dbi: f64,
  /// 实测前后比（dB）。
  pub front_to_back_db: f64,
  /// 实测 50 Ω 驻波比。
  pub swr_50: f64,
  /// 优化过程中评估过的模型个数（如实给出代价）。
  pub evaluations: usize,
}

/// 设计参数（坐标下降的自变量）。
#[derive(Debug, Clone, Copy)]
struct YagiParams {
  /// 有源振子长度（m）。
  driven: f64,
  /// 反射器 / 有源振子 长度比（夹在 [`REFL_RATIO_RANGE`]）。
  refl_ratio: f64,
  /// 第一根引向器 / 有源振子 长度比。
  dir_first: f64,
  /// 最后一根引向器 / 有源振子 长度比。
  dir_last: f64,
  /// 反射器 ↔ 有源振子 的间距（m）。
  d_refl: f64,
  /// 引向器之间的间距（m，自第一根引向器起等距）。
  d_dir: f64,
}

impl YagiParams {
  /// 夹到物理上有意义的区间：反射器必须长于振子、引向器必须短于振子且逐渐变短。
  fn clamped(mut self) -> Self {
    self.refl_ratio = self
      .refl_ratio
      .clamp(REFL_RATIO_RANGE.0, REFL_RATIO_RANGE.1);
    self.dir_first = self.dir_first.clamp(DIR_RATIO_RANGE.0, DIR_RATIO_RANGE.1);
    self.dir_last = self
      .dir_last
      .clamp(DIR_RATIO_RANGE.0, self.dir_first.min(DIR_RATIO_RANGE.1));
    self.d_refl = self.d_refl.max(0.02);
    self.d_dir = self.d_dir.max(0.02);
    self
  }

  /// 展开成各单元长度（反射器 → 有源振子 → 引向器…）。
  fn lengths(&self, elements: usize) -> Vec<f64> {
    let mut out = vec![self.driven * self.refl_ratio, self.driven];
    let directors = elements.saturating_sub(2);
    for i in 0..directors {
      let t = if directors <= 1 {
        0.0
      } else {
        i as f64 / (directors - 1) as f64
      };
      out.push(self.driven * (self.dir_first + (self.dir_last - self.dir_first) * t));
    }
    out
  }

  /// 各单元沿 `x` 的位置（有源振子在 0）。
  fn positions(&self, elements: usize) -> Vec<f64> {
    yagi_positions(self.d_refl, self.d_dir, elements)
  }
}

/// 各单元沿 `x` 的位置：反射器在 `−d_refl`，有源振子在 0，引向器等距向前。
///
/// 只依赖两个间距参数 —— 长度类参数不参与，因此「加载到导线表」这类只关心位置的
/// 调用方不必伪造一个长度参数全是 `1.0` 的 [`YagiParams`]。
///
/// `elements == 0` 时返回空表（调用方 zip 出的导线表为空，求解器会返回 `None`），
/// 而不是在这里 `pos[0]` 越界 panic。
fn yagi_positions(d_refl: f64, d_dir: f64, elements: usize) -> Vec<f64> {
  let mut pos = vec![0.0; elements];
  if elements == 0 {
    return pos;
  }
  pos[0] = -d_refl;
  for i in 2..elements {
    pos[i] = pos[i - 1] + d_dir;
  }
  pos
}

/// 由设计参数装配模型：单元沿 `x` 排布、振子沿 `y`，反射器在 `x < 0`，
/// 有源振子在 `x = 0`，引向器往前排 —— 与页面预设同序（馈电点在导线 1）。
fn yagi_input(freq_hz: f64, radius: f64, p: &YagiParams, elements: usize) -> NecInput {
  let lengths = p.lengths(elements);
  let pos = p.positions(elements);
  let wires = lengths
    .iter()
    .zip(pos.iter())
    .map(|(&l, &x)| {
      Wire::new(
        [x, -l / 2.0, 0.0],
        [x, l / 2.0, 0.0],
        radius,
        DESIGN_SEGMENTS,
      )
    })
    .collect();
  NecInput {
    wires,
    feeds: vec![Feed {
      wire: 1,
      at: 0.5,
      volts: 1.0,
    }],
    loads: Vec::new(),
    freq_hz,
    ground: Ground::Free,
    conductivity: None,
  }
}

/// 目标函数（经验权重，如实写在这里）。
///
/// 只看增益会优化出「增益高但没法用」的天线（实测三单元会落到 SWR 39、前后比 2.5 dB），
/// 所以把**前后比**与**可馈电性**一起计入：
/// `增益 + 0.08×min(前后比, 20) − 0.6×max(0, SWR − 3)`。
/// 前后比的权重取得比增益小一个量级 —— 它是「别收后向噪声」的次要目标，
/// 权重再大就会为了 25 dB 的前后比牺牲 1.5 dB 增益（那通常不划算）。
/// 权重是工程取舍而非理论最优，所以向导把这三项**实测值**一并给出，让人自己判断。
fn yagi_score(r: &NecResult) -> f64 {
  r.gain_max_dbi + 0.08 * r.front_to_back_db.min(20.0) - 0.6 * (r.swr_50 - 3.0).clamp(0.0, 20.0)
}

/// 八木坐标下降的**分步**执行器：把「几十次完整求解」摊到多帧，避免冻住调用方。
///
/// 一次优化要评估约 38 个模型，每个模型都要算完整方向图 —— 在 WASM 主线程上同步跑完
/// 约 1–2 秒，界面完全无响应。所以把搜索状态留在结构体里，由调用方按预算调用
/// [`step`](Self::step) 推进。
///
/// 判据、步长与评估顺序都和同步版一致：[`design_yagi`] 就是「一直推进到结束」的薄包装，
/// 因此两条路径给出的结果必然相同。
pub struct YagiSearch {
  freq_hz: f64,
  radius_m: f64,
  elements: usize,
  /// 当前最优参数（搜索过程中持续更新）。
  params: YagiParams,
  /// 当前参数的基准：同一参数的多次试探都以它为起点（原同步版里的 `current`）。
  base: YagiParams,
  best: f64,
  /// 已落地的评估次数（含起点与终点各一次）。
  evaluations: usize,
  /// 评估次数上限，仅用于进度显示。
  total: usize,
  pass: usize,
  which: usize,
  step_idx: usize,
  improved: bool,
  /// 是否需要在下一步为当前 `which` 重取基准。
  fresh: bool,
  finished: bool,
  /// 终点求解失败（模型解不出来）：结束但无结果。
  failed: bool,
  done: Option<YagiDesign>,
}

impl YagiSearch {
  /// 建好起点并评估一次；`elements` 夹在 `2..=5`，频率或半径非有限正数时返回 `None`。
  #[must_use]
  pub fn new(freq_hz: f64, elements: usize, radius_m: f64) -> Option<Self> {
    // 先判有限性（挡掉 NaN / ±inf），再判正负 —— 顺序反了会让 NaN 溜过 `<= 0.0`。
    if !freq_hz.is_finite() || freq_hz <= 0.0 || !radius_m.is_finite() || radius_m <= 0.0 {
      return None;
    }
    let elements = elements.clamp(2, 5);
    let lam = wavelength(freq_hz);
    let params = YagiParams {
      driven: 0.475 * lam,
      refl_ratio: 1.047,
      dir_first: 0.922,
      dir_last: 0.88,
      d_refl: 0.146 * lam,
      d_dir: 0.10 * lam,
    }
    .clamped();
    let best = solve(&yagi_input(freq_hz, radius_m, &params, elements)).map(|r| yagi_score(&r))?;
    // 起点 + 终点各一次，其余是每轮 6 个参数 × 各自步数（前 4 个用长度步长）。
    let per_pass = 4 * DESIGN_STEPS.len() + 2 * DESIGN_SPACING_STEPS.len();
    Some(Self {
      freq_hz,
      radius_m,
      elements,
      params,
      base: params,
      best,
      evaluations: 1,
      total: 2 + DESIGN_PASSES * per_pass,
      pass: 0,
      which: 0,
      step_idx: 0,
      improved: false,
      fresh: true,
      finished: false,
      failed: false,
      done: None,
    })
  }

  /// 已落地的评估次数。
  #[must_use]
  pub fn evaluations(&self) -> usize {
    self.evaluations
  }

  /// 进度 0.0–1.0；分母是评估次数上限（末轮可能提前收敛，因此可能不到 1.0）。
  #[must_use]
  pub fn progress(&self) -> f64 {
    if self.total == 0 {
      return 1.0;
    }
    (self.evaluations as f64 / self.total as f64).min(1.0)
  }

  /// 搜索是否已结束（含终点求解失败的情况，用 [`Self::step`] 的返回值区分）。
  #[must_use]
  pub fn is_done(&self) -> bool {
    self.finished
  }

  /// 推进最多 `budget` 次评估；搜索完成时返回最终设计，否则返回 `None`。
  ///
  /// 重复调用是幂等的：完成后始终返回同一份结果。
  pub fn step(&mut self, budget: usize) -> Option<YagiDesign> {
    if let Some(done) = &self.done {
      return Some(done.clone());
    }
    let mut left = budget.max(1);
    while !self.finished && left > 0 {
      self.advance();
      left = left.saturating_sub(1);
    }
    if !self.finished || self.failed {
      return None;
    }
    self.finish()
  }

  /// 推进一步：一次评估，或一次「参数/轮次」切换（后者不消耗评估）。
  fn advance(&mut self) {
    if self.fresh {
      self.base = self.params;
      self.fresh = false;
    }
    let steps: &[f64] = if self.which >= 4 {
      &DESIGN_SPACING_STEPS
    } else {
      &DESIGN_STEPS
    };
    if self.step_idx >= steps.len() {
      self.next_which();
      return;
    }
    let mult = steps[self.step_idx];
    self.step_idx += 1;
    let mut trial = self.base;
    match self.which {
      0 => trial.driven = self.base.driven * mult,
      1 => trial.refl_ratio = self.base.refl_ratio * mult,
      2 => trial.dir_first = self.base.dir_first * mult,
      3 => trial.dir_last = self.base.dir_last * mult,
      4 => trial.d_refl = self.base.d_refl * mult,
      _ => trial.d_dir = self.base.d_dir * mult,
    }
    let trial = trial.clamped();
    let Some(score) = self.evaluate(&trial) else {
      return;
    };
    self.evaluations += 1;
    if score > self.best + 1e-9 {
      self.best = score;
      self.params = trial;
      self.improved = true;
    }
  }

  /// 一个参数试完 → 下一个参数；六个参数试完 → 一轮结束，无改进则整体收敛。
  fn next_which(&mut self) {
    self.step_idx = 0;
    self.fresh = true;
    self.which += 1;
    if self.which < 6 {
      return;
    }
    self.which = 0;
    self.pass += 1;
    let improved = std::mem::replace(&mut self.improved, false);
    if !improved || self.pass >= DESIGN_PASSES {
      self.finished = true;
    }
  }

  /// 目标：完整求解一次并打分。
  fn evaluate(&self, p: &YagiParams) -> Option<f64> {
    solve(&yagi_input(self.freq_hz, self.radius_m, p, self.elements)).map(|r| yagi_score(&r))
  }

  /// 用最优参数实测一次，装配成结果。
  fn finish(&mut self) -> Option<YagiDesign> {
    if let Some(done) = &self.done {
      return Some(done.clone());
    }
    let Some(r) = solve(&yagi_input(
      self.freq_hz,
      self.radius_m,
      &self.params,
      self.elements,
    )) else {
      self.failed = true;
      return None;
    };
    self.evaluations += 1;
    let mut spacings = vec![self.params.d_refl];
    spacings.extend(std::iter::repeat_n(
      self.params.d_dir,
      self.elements.saturating_sub(2),
    ));
    let design = YagiDesign {
      freq_hz: self.freq_hz,
      elements: self.elements,
      wavelength_m: wavelength(self.freq_hz),
      lengths: self.params.lengths(self.elements),
      spacings,
      gain_dbi: r.gain_max_dbi,
      front_to_back_db: r.front_to_back_db,
      swr_50: r.swr_50,
      evaluations: self.evaluations,
    };
    self.done = Some(design.clone());
    Some(design)
  }
}

/// 八木设计向导（同步版）：对本模块自己的求解器做坐标下降，给出可用的尺寸。
///
/// 起点取经典比例（有源振子 0.475λ、反射器长 4.7%、引向器从 0.922 递减到 0.88、
/// 间距 0.146λ / 0.10λ —— 就是页面三单元预设那套），每轮依次对六个参数试 ±6%，
/// 取使 [`yagi_score`] 最大的取值，**最多两轮**（[`DESIGN_PASSES`]）。
///
/// 目标与验收都用**同一个求解器**，所以向导给出的增益就是页面上加载后能复现的增益，
/// 不存在「向导说 8 dBi、加载后只有 6 dBi」的两套口径。
///
/// 本函数是同步的（WASM 主线程上约 1–2 秒）；需要不冻界面的调用方改用
/// [`YagiSearch`] 分帧推进，两者结果完全一致。
///
/// `elements` 夹在 `2..=5`；频率或半径非有限正数时返回 `None`。
#[must_use]
pub fn design_yagi(freq_hz: f64, elements: usize, radius_m: f64) -> Option<YagiDesign> {
  YagiSearch::new(freq_hz, elements, radius_m)?.step(usize::MAX)
}

/// 把设计结果转回求解输入（页面「加载到导线表」与向导共用同一套装配口径）。
#[must_use]
pub fn yagi_design_input(design: &YagiDesign, radius_m: f64) -> Option<NecInput> {
  let pos = yagi_positions(
    *design.spacings.first()?,
    *design.spacings.last()?,
    design.elements,
  );
  let wires = design
    .lengths
    .iter()
    .zip(pos.iter())
    .map(|(&l, &x)| {
      Wire::new(
        [x, -l / 2.0, 0.0],
        [x, l / 2.0, 0.0],
        radius_m,
        DESIGN_SEGMENTS,
      )
    })
    .collect();
  Some(NecInput {
    wires,
    feeds: vec![Feed {
      wire: 1,
      at: 0.5,
      volts: 1.0,
    }],
    loads: Vec::new(),
    freq_hz: design.freq_hz,
    ground: Ground::Free,
    conductivity: None,
  })
}

// ───────────────────────────── 仰角-距离覆盖（NVIS） ─────────────────────────────

/// 一跳覆盖曲线上的一个点。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CoveragePoint {
  /// 地面大圆距离（km）。
  pub distance_km: f64,
  /// 该距离对应的发射仰角（°，相对地平线）。
  pub elevation_deg: f64,
  /// 该仰角上的天线增益（dBi）。
  pub gain_dbi: f64,
}

/// 单跳地面距离（km）：仰角 `elev_deg` 打到虚高 `layer_km` 的电离层再返回。
///
/// 平地近似下就是 `d = 2·h·cot(Δ)` —— 天顶（90°）落点为零，仰角越低落点越远。
/// 这只含几何，不含电离层吸收、MUF/LUF 与地球曲率。
#[must_use]
pub fn one_hop_distance_km(elev_deg: f64, layer_km: f64) -> Option<f64> {
  if layer_km <= 0.0 || !elev_deg.is_finite() || elev_deg <= 0.0 || elev_deg > 90.0 {
    return None;
  }
  Some(2.0 * layer_km / elev_deg.to_radians().tan())
}

/// 单跳落点对应的仰角（°）：[`one_hop_distance_km`] 的反函数，`Δ = atan(2h/d)`。
#[must_use]
pub fn one_hop_elevation_deg(distance_km: f64, layer_km: f64) -> Option<f64> {
  if layer_km <= 0.0 || !distance_km.is_finite() || distance_km < 0.0 {
    return None;
  }
  Some((2.0 * layer_km / distance_km.max(1e-9)).atan().to_degrees())
}

/// 由仰角切面算出**单跳地面覆盖**：`(距离, 仰角, 增益)` 按距离升序。
///
/// 直接复用结果里的 1° 步进仰角切面（在最大增益方位上取的），所以曲线与方向图
/// 用的是同一份数据 —— 不存在「方向图一个样、覆盖图另一个样」。
///
/// 只取上半空间（`elevation > 0`）且距离不超过 `max_km`。**只有几何与天线增益**：
/// 不含电离层吸收、地反射损耗与 MUF/LUF，用来判断「这副天线能不能覆盖 0–500 km」
/// 足够，用来算链路预算远远不够。
#[must_use]
pub fn one_hop_coverage(r: &NecResult, layer_km: f64, max_km: f64) -> Vec<CoveragePoint> {
  if layer_km <= 0.0 || max_km <= 0.0 {
    return Vec::new();
  }
  let mut out: Vec<CoveragePoint> = r
    .elevation
    .iter()
    .filter(|(e, _)| *e > 0.0 && *e <= 90.0)
    .filter_map(|(e, g)| {
      let d = one_hop_distance_km(*e, layer_km)?;
      (d <= max_km).then_some(CoveragePoint {
        distance_km: d,
        elevation_deg: *e,
        gain_dbi: *g,
      })
    })
    .collect();
  // 仰角从 0 往上扫，距离是从大到小；倒过来就是距离升序。
  out.reverse();
  out
}

/// 覆盖曲线上的峰值点（最大的那个增益）。
#[must_use]
pub fn coverage_peak(points: &[CoveragePoint]) -> Option<CoveragePoint> {
  points
    .iter()
    .copied()
    .reduce(|a, b| if b.gain_dbi > a.gain_dbi { b } else { a })
}

/// 波长（m）。
#[must_use]
pub fn wavelength(freq_hz: f64) -> f64 {
  if freq_hz > 0.0 { C0 / freq_hz } else { 0.0 }
}

// ───────────────────────────── `.nec` 文本互导 ─────────────────────────────

/// `.nec` 文件解析结果。
#[derive(Debug, Clone, PartialEq)]
pub struct NecFile {
  /// 解析出的求解输入。
  pub input: NecInput,
  /// 未识别、因而**没有生效**的卡片类型或字段（去重保序）。同一张卡片的字段坏掉时
  /// 记成 `GW(字段非法，已跳过该卡)` 这类带原因的条目，其余卡片仍然生效。
  pub ignored: Vec<String>,
}

/// 追加一条「没生效的卡片 / 字段」诊断；完全相同的只留一条（保序）。
fn push_ignored(ignored: &mut Vec<String>, what: impl Into<String>) {
  let what = what.into();
  if !ignored.iter().any(|x| x == &what) {
    ignored.push(what);
  }
}

/// 解析 NEC 免费格式的线天线子集：`GW` / `GE` / `GN` / `LD` / `EX` / `FR`，
/// 忽略 `CM` / `CE` / `EN`。
///
/// 其它卡片（`TL` 传输线、`RP` 方向图请求……）以及**字段非法**的受支持卡片都会进
/// [`NecFile::ignored`]，即**列出但不生效** —— 静默忽略会让人以为算的是文件里的模型。
///
/// 粒度是**单张卡片**：某张 `GW` 卡的半径写成不可解析的文本，只丢这一张并留下诊断，
/// 其余导线照常生效（曾经一个 `?` 会让整份文件解析失败，用户看到的是「文件读不进来」）。
///
/// 坐标系与单位按 NEC 约定：米、右手系、`z` 向上；`EX` 的段号是导线上 1 起的段序号。
///
/// 只有在「没有任何导线」或「没有任何馈电」或「没有可用频率」时才整体返回 `None`。
#[must_use]
pub fn parse_nec(text: &str) -> Option<NecFile> {
  let mut wires = Vec::new();
  // tag 用 `i64` 而不是 `i32`：卡片里合法的 tag 可以超出 `i32` 范围，
  // 截断后会回绕并与别的 tag 碰撞，`position` 就匹配到错误的导线。
  let mut tags: Vec<i64> = Vec::new();
  let mut raw_feeds: Vec<(i64, usize, f64)> = Vec::new();
  let mut freq_mhz: Option<f64> = None;
  // `GE`（有无地面）与 `GN`（地面参数）分开记：NEC 文件里两张卡的先后顺序并不统一，
  // 只要出现 `GN` 就以它为准，不被后面的 `GE 1` 覆盖掉。
  let mut ge_flag: Option<bool> = None;
  let mut gn_ground: Option<Ground> = None;
  let mut raw_loads: Vec<(i64, usize, f64, f64, f64)> = Vec::new();
  let mut ignored: Vec<String> = Vec::new();

  for line in text.lines() {
    let line = line.trim();
    if line.is_empty() || line.starts_with('!') {
      continue;
    }
    // NEC 免费格式：**卡片名与第一个参数之间是空格**，其余参数用逗号或空格分隔
    // （`GW 1, 21, -5.31, …` 与 `GW 1 21 -5.31 …` 都合法）。先把卡片名摘出来，
    // 再把剩下的按「逗号或空白」切分。
    let (card, rest) = line
      .split_once(char::is_whitespace)
      .map_or((line, ""), |(a, b)| (a, b));
    let card = card.to_ascii_uppercase();
    let fields: Vec<&str> = rest
      .split(|c: char| c == ',' || c.is_whitespace())
      .filter(|v| !v.is_empty())
      .collect();
    let num = |i: usize| -> Option<f64> { fields.get(i).and_then(|v| v.parse::<f64>().ok()) };
    let int = |i: usize| -> Option<i64> { fields.get(i).and_then(|v| v.parse::<i64>().ok()) };
    match card.as_str() {
      // 注释与结束卡片。
      "CM" | "CE" | "EN" => {}
      "GW" => {
        let parsed = (|| {
          let tag = int(0)?;
          let requested = int(1)?;
          // 超出上限时**不静默截断**：离散段数变了，结果与 4NEC2 不一致却毫无提示。
          let n = requested.clamp(1, MAX_SEGMENTS as i64) as usize;
          if requested > MAX_SEGMENTS as i64 {
            push_ignored(
              &mut ignored,
              format!("GW(请求 {requested} 段，超过上限 {MAX_SEGMENTS}，已按上限离散)"),
            );
          }
          wires.push(Wire::new(
            [num(2)?, num(3)?, num(4)?],
            [num(5)?, num(6)?, num(7)?],
            num(8)?,
            n,
          ));
          tags.push(tag);
          Some(())
        })();
        if parsed.is_none() {
          push_ignored(&mut ignored, "GW(字段非法，已跳过该卡)".to_owned());
        }
      }
      // 地面：`GE 0` 自由空间、`GE 1` 有地面（理想导体，除非另有 `GN`）。
      "GE" => ge_flag = Some(int(0).unwrap_or(0) == 1),
      // 有耗地面：`GN 2, 0, 0, 0, ε_r, σ`。I1 = 0 表示无地面、1 表示理想导体。
      "GN" => {
        gn_ground = Some(match int(0).unwrap_or(0) {
          0 => Ground::Free,
          1 => Ground::Perfect,
          // 2 = 反射系数法（本实现即此法）；其余类型按同一公式近似。
          _ => Ground::lossy(num(4).unwrap_or(13.0), num(5).unwrap_or(0.005)),
        });
      }
      // 集总负载：`LD 0, tag, seg, seg, R(Ω), L(H), C(F)`（I1 = 0 为串联 RLC）。
      //
      // 单位按 NEC-2 手册 Part III 的 LD 卡定义：`ZLI` 是**亨利**、`ZLC` 是**法拉**
      // （只有 LDTYP = 2/3 的单位长加载才是 H/m、F/m）。内层模型用 µH / pF，
      // 于是这里 ×1e6 / ×1e12 —— 曾经按 mH / µF 换算，跨软件交换差 1e3 / 1e6。
      "LD" => {
        let parsed = (|| {
          if int(0).unwrap_or(-1) != 0 {
            push_ignored(&mut ignored, "LD(非串联 RLC)".to_owned());
            return Some(());
          }
          raw_loads.push((
            int(1)?,
            int(2)?.max(1) as usize,
            num(4).unwrap_or(0.0),
            num(5).unwrap_or(0.0) * 1e6,
            num(6).unwrap_or(0.0) * 1e12,
          ));
          Some(())
        })();
        if parsed.is_none() {
          push_ignored(&mut ignored, "LD(字段非法，已跳过该卡)".to_owned());
        }
      }
      "EX" => {
        // `EX, ITYPE, ITAG, ISEG, I4, VREAL, VIMAG`；只支持电压源（ITYPE = 0）。
        let parsed = (|| {
          if int(0).unwrap_or(0) != 0 {
            push_ignored(&mut ignored, "EX(非电压源)".to_owned());
            return Some(());
          }
          // 电压是必填：以前缺字段时静默按 1 V 算，用户会以为算的是文件里的激励。
          // 与 `int(1)?` / `int(2)?` 这些必填字段保持同一口径。
          raw_feeds.push((int(1)?, int(2)?.max(1) as usize, num(4)?));
          Some(())
        })();
        if parsed.is_none() {
          push_ignored(&mut ignored, "EX(字段非法，已跳过该卡)".to_owned());
        }
      }
      // 频率卡：`FR, I1, N, F1, F2, …`，F1 是起始频率（MHz）。有些工具把频率写在
      // 后面的字段上，这里取「第一个非零的频率字段」，两种写法都能读。
      "FR" => {
        // `nan` / `inf` 也能被 `f64::from_str` 解析出来，必须显式挡掉。
        freq_mhz = (2..6).find_map(|i| num(i).filter(|v| v.is_finite() && *v > 0.0));
      }
      other => push_ignored(&mut ignored, other),
    }
  }

  let ground = gn_ground.unwrap_or(if ge_flag.unwrap_or(false) {
    Ground::Perfect
  } else {
    Ground::Free
  });
  if wires.is_empty() {
    return None;
  }
  let mut feeds = Vec::new();
  for (tag, seg, volts) in raw_feeds {
    // 指向不存在的 tag：只丢这一条馈电并留诊断，其余卡片照常生效。
    let Some(idx) = tags.iter().position(|&t| t == tag) else {
      push_ignored(
        &mut ignored,
        format!("EX(引用了不存在的 tag {tag}，已跳过该激励源)"),
      );
      continue;
    };
    let n = wires[idx].segments.max(1);
    // `EX` 给的是段号，换算成「段心」的相对位置。
    let s = (seg.clamp(1, n) as f64 - 0.5) / n as f64;
    feeds.push(Feed {
      wire: idx,
      at: s,
      volts,
    });
  }
  if feeds.is_empty() {
    return None;
  }
  let mut loads = Vec::new();
  for (tag, seg, r, l_uh, c_pf) in raw_loads {
    let Some(idx) = tags.iter().position(|&t| t == tag) else {
      push_ignored(
        &mut ignored,
        format!("LD(引用了不存在的 tag {tag}，已跳过该负载)"),
      );
      continue;
    };
    let n = wires[idx].segments.max(1);
    loads.push(Load {
      wire: idx,
      at: (seg.clamp(1, n) as f64 - 0.5) / n as f64,
      r_ohm: r,
      l_uh,
      c_pf,
    });
  }
  Some(NecFile {
    input: NecInput {
      wires,
      feeds,
      loads,
      freq_hz: freq_mhz? * 1e6,
      ground,
      conductivity: None,
    },
    ignored,
  })
}

/// 导出成 NEC 免费格式（`GW` / `GE` / `EX` / `FR` / `EN`）。
///
/// 与 [`parse_nec`] 构成往返闭环，便于把这里的几何拿到 4NEC2 / EZNEC 里交叉验证。
#[must_use]
pub fn to_nec(input: &NecInput) -> String {
  let mut out = String::from("CM 由 ham-exam-web 导出（NEC 免费格式子集）\nCE\n");
  for (i, w) in input.wires.iter().enumerate() {
    out.push_str(&format!(
      "GW {} {} {:.4} {:.4} {:.4} {:.4} {:.4} {:.4} {:.5}\n",
      i + 1,
      w.segments.max(1),
      w.a[0],
      w.a[1],
      w.a[2],
      w.b[0],
      w.b[1],
      w.b[2],
      w.radius
    ));
  }
  // GE 表示有无地面；有耗地面再补一条 GN（ε_r 与 σ 都按 NEC 约定）。
  let grounded = input.ground.is_grounded();
  out.push_str(&format!("GE {}\n", i32::from(grounded)));
  if let Ground::Lossy { eps_r, sigma } = input.ground {
    out.push_str(&format!("GN 2, 0, 0, 0, {eps_r:.3}, {sigma:.5}\n"));
  }
  for l in &input.loads {
    if l.is_empty() {
      continue;
    }
    let n = input.wires.get(l.wire).map_or(1, |w| w.segments.max(1));
    let seg = ((l.at.clamp(0.0, 1.0) * n as f64).floor() as usize).min(n - 1) + 1;
    out.push_str(&format!(
      "LD 0, {}, {}, 0, {:.4}, {:.6}, {:.6}\n",
      l.wire + 1,
      seg,
      l.r_ohm,
      // 与解析同一口径：NEC-2 的 `LD` 用亨利 / 法拉（内层是 µH / pF）。
      l.l_uh * 1e-6,
      l.c_pf * 1e-12
    ));
  }
  for f in &input.feeds {
    let n = input.wires.get(f.wire).map_or(1, |w| w.segments.max(1));
    let seg = ((f.at.clamp(0.0, 1.0) * n as f64).floor() as usize).min(n - 1) + 1;
    out.push_str(&format!(
      "EX 0, {}, {}, 0, {:.4}, 0.0\n",
      f.wire + 1,
      seg,
      f.volts
    ));
  }
  out.push_str(&format!("FR 0, 1, {:.4}, 0.0\nEN\n", input.freq_hz / 1e6));
  out
}

#[cfg(test)]
mod tests {
  use super::*;

  const FREQ: f64 = 14.1e6;
  /// 相对波长归一化的导线半径：细线（`a/λ = 1e-4`，14 MHz 上约 2 mm 铜线）。
  const THIN: f64 = 1e-4;

  /// 中心馈电的直线偶极（沿 x 轴、可选理想地面）。
  ///
  /// 段数取偶数，让馈电点正好落在中心节点上。
  /// 有地面时是否接地；`Ground::Free` / `Ground::Perfect` 两态够测试用。
  fn mk_ground(on: bool) -> Ground {
    if on { Ground::Perfect } else { Ground::Free }
  }

  /// 半波偶极（沿 x 轴、中心馈电、可选理想地面）。
  fn dipole(frac: f64, segments: usize, ground: bool) -> NecInput {
    let lam = wavelength(FREQ);
    let l = frac * lam;
    NecInput {
      wires: vec![Wire::new(
        [-l / 2.0, 0.0, 0.0],
        [l / 2.0, 0.0, 0.0],
        THIN * lam,
        segments,
      )],
      feeds: vec![Feed {
        wire: 0,
        at: 0.5,
        volts: 1.0,
      }],
      freq_hz: FREQ,
      loads: Vec::new(),
      ground: mk_ground(ground),
      conductivity: None,
    }
  }

  #[test]
  fn half_wave_dipole_is_in_the_textbook_ballpark() {
    // 教科书 73.1 + j42.5 Ω。本实现对电阻有约 10% 的系统偏差（见模块文档），
    // 这里用宽容差挡「量级错误」——前两次尝试在这里差了 300% 甚至出现负电阻。
    for n in [21usize, 41, 61] {
      let r = solve(&dipole(0.5, n, false)).expect("应能求解");
      assert!(
        (66.0..86.0).contains(&r.impedance.0),
        "N={n} 电阻 {} 应落在 66–86Ω（教科书 73.1）",
        r.impedance.0
      );
      assert!(
        (36.0..50.0).contains(&r.impedance.1),
        "N={n} 电抗 {} 应落在 36–50Ω（教科书 42.5）",
        r.impedance.1
      );
    }
  }

  #[test]
  fn half_wave_dipole_directivity_is_2_15_dbi() {
    let r = solve(&dipole(0.5, 41, false)).unwrap();
    assert!(
      (r.gain_max_dbi - 2.15).abs() < 0.25,
      "方向性系数 {} dBi 应约 2.15",
      r.gain_max_dbi
    );
    // 自由空间里的水平偶极，最大增益出现在**垂直于导线的一整圈**上
    // （含天顶与水平面），所以「最大仰角」在这个对称情形下本身不唯一；
    // 有地面时对称性被镜像打破，最大仰角才有确定含义（见架高 λ/4 的用例）。
    assert!(r.front_to_back_db.abs() < 0.3, "F/B {}", r.front_to_back_db);
    let max_elev = r
      .elevation
      .iter()
      .map(|(_, g)| *g)
      .fold(f64::NEG_INFINITY, f64::max);
    assert!(
      (max_elev - r.gain_max_dbi).abs() < 0.2,
      "仰角切面的峰值 {max_elev} 应与最大增益 {} 一致",
      r.gain_max_dbi
    );
    assert_eq!(r.azimuth.len(), 181);
    assert_eq!(r.elevation.len(), 181);
  }

  #[test]
  fn quarter_wave_monopole_is_exactly_half_the_dipole() {
    // 理想导体地面上的镜像使 λ/4 单极与 λ/2 偶极等价：电流相同、辐射功率减半，
    // 因此阻抗恰好是一半、方向性系数高 3 dB。这是镜像与符号约定最硬的检验。
    let h = 0.25 * wavelength(FREQ);
    let mono = NecInput {
      wires: vec![Wire::new(
        [0.0, 0.0, 0.0],
        [0.0, 0.0, h],
        THIN * wavelength(FREQ),
        40,
      )],
      feeds: vec![Feed {
        wire: 0,
        at: 0.0,
        volts: 1.0,
      }],
      freq_hz: FREQ,
      loads: Vec::new(),
      ground: Ground::Perfect,
      conductivity: None,
    };
    let rm = solve(&mono).expect("应能求解");
    let rd = solve(&dipole(0.5, 40, false)).unwrap();
    assert!(
      (rm.impedance.0 - rd.impedance.0 / 2.0).abs() < rd.impedance.0 * 0.01,
      "单极电阻 {} 应为偶极 {} 的一半",
      rm.impedance.0,
      rd.impedance.0
    );
    assert!(
      (rm.impedance.1 - rd.impedance.1 / 2.0).abs() < rd.impedance.1.abs() * 0.02,
      "单极电抗 {} 应为偶极 {} 的一半",
      rm.impedance.1,
      rd.impedance.1
    );
    assert!(
      (rm.gain_max_dbi - rd.gain_max_dbi - 3.0).abs() < 0.4,
      "单极方向性系数 {} 应比偶极 {} 高 3 dB",
      rm.gain_max_dbi,
      rd.gain_max_dbi
    );
    assert!(rm.gain_max_elevation_deg.abs() < 3.0);
    // 下半空间无辐射。
    assert!(rm.elevation.iter().all(|(e, _)| *e >= -1e-9));
  }

  #[test]
  fn feed_power_equals_pattern_integral_power() {
    // Galerkin 测试方案下，馈电功率与方向图积分出的辐射功率必须一致
    // （点匹配方案在这里会差十几个百分点，那是第一次尝试的病根之一）。
    for n in [20usize, 40] {
      let config = dipole(0.5, n, false);
      let model = Model::from_wires(&config.wires).unwrap();
      let k = 2.0 * PI * FREQ / C0;
      let unknowns: Vec<usize> = (0..model.nodes.len())
        .filter(|&j| !model.is_fixed_end(j, false))
        .collect();
      let mut z = assemble(&model, &unknowns, k, &config).expect("应能装配");
      let mid = unknowns
        .iter()
        .position(|&j| model.nodes[j][0].abs() < 1e-9)
        .expect("中心应有节点");
      let mut rhs = vec![Cx::ZERO; unknowns.len()];
      rhs[mid] = Cx::of(-1.0);
      let sol = solve_linear(&mut z, &mut rhs).unwrap();
      let mut cur = vec![Cx::ZERO; model.nodes.len()];
      for (i, &j) in unknowns.iter().enumerate() {
        cur[j] = sol[i];
      }
      let zin = Cx::of(1.0) / sol[mid];
      let p_feed = 0.5 * zin.re * sol[mid].abs2();
      let mut integ = 0.0;
      let mut th = 0.0f64;
      while th <= 180.0 {
        let mut ph = 0.0f64;
        while ph < 360.0 {
          integ += perp2(&model, &cur, k, Ground::Free, th, ph)
            * th.to_radians().sin()
            * 2.0f64.to_radians()
            * 2.0f64.to_radians();
          ph += 2.0;
        }
        th += 2.0;
      }
      let p_pattern = k * k * 376.730_313_668_017 / (32.0 * PI * PI) * integ;
      assert!(
        (p_pattern / p_feed - 1.0).abs() < 0.01,
        "N={n} 两种功率口径应一致：{p_pattern} vs {p_feed}"
      );
    }
  }

  #[test]
  fn resonant_length_matches_textbook() {
    // 细线偶极电抗过零长度约 0.4787λ。
    let mut prev: Option<(f64, f64)> = None;
    let mut found: Option<f64> = None;
    for i in 0..=24 {
      let frac = 0.44 + 0.004 * i as f64;
      let x = solve(&dipole(frac, 41, false)).unwrap().impedance.1;
      if let Some((pf, px)) = prev
        && px < 0.0
        && x >= 0.0
      {
        found = Some(pf + (frac - pf) * (0.0 - px) / (x - px));
      }
      prev = Some((frac, x));
    }
    let frac = found.expect("电抗应有过零点");
    assert!(
      (0.468..=0.492).contains(&frac),
      "谐振长度 {frac:.4}λ 应落在 0.468–0.492λ"
    );
  }

  #[test]
  fn short_dipole_radiation_resistance_is_accurate() {
    // 短偶极的辐射电阻解析式 `20π²(l/λ)²` 只依赖电流分布，与电荷项无关，
    // 因此它是「矢量位项是否正确」的干净判据。
    let r = solve(&dipole(0.2, 41, false)).unwrap();
    let want = 20.0 * PI * PI * 0.2 * 0.2;
    assert!(
      (r.impedance.0 - want).abs() < want * 0.15,
      "0.2λ 偶极电阻 {} 应约 {want:.2}Ω",
      r.impedance.0
    );
    assert!(
      r.impedance.1 < -300.0,
      "短偶极应呈强容性：{}",
      r.impedance.1
    );
  }

  #[test]
  fn full_wave_dipole_is_physical() {
    // 1λ 偶极的馈电点正好落在电流零点上（电流分布是整正弦），因此馈电阻抗
    // 天然极高 —— 这是物理结论，不是数值发散。前两次尝试在这里给出的是
    // **负电阻**，那才是错的。
    let r = solve(&dipole(1.0, 41, false)).expect("应能求解");
    assert!(
      r.impedance.0 > 300.0,
      "1λ 偶极馈电点位于电流零点，电阻应极高，实际 {}",
      r.impedance.0
    );
    // 0.75λ 处阻抗中等、电阻为正。
    let r = solve(&dipole(0.75, 41, false)).expect("应能求解");
    assert!(
      (100.0..1500.0).contains(&r.impedance.0),
      "0.75λ 偶极电阻 {} 应为正且中等",
      r.impedance.0
    );
  }

  #[test]
  fn impedance_is_stable_under_refinement() {
    // 收敛：相邻档之间不应跳变（前两次尝试在这里失败过）。
    let mut last: Option<(f64, f64)> = None;
    for n in [11usize, 21, 41, 61, 81] {
      let r = solve(&dipole(0.5, n, false)).unwrap();
      if let Some((pr, px)) = last {
        assert!(
          (r.impedance.0 - pr).abs() < 6.0 && (r.impedance.1 - px).abs() < 8.0,
          "N={n} 跳变过大：{:?} → {:?}",
          (pr, px),
          r.impedance
        );
      }
      last = Some(r.impedance);
    }
  }

  #[test]
  fn end_current_falls_to_zero_and_topology_is_consistent() {
    // 自由端节点电流被严格固定为 0；段心电流是端点值与相邻内节点值的平均。
    let r = solve(&dipole(0.5, 40, false)).unwrap();
    let center = r.currents[r.currents.len() / 2];
    let end = r.currents[0].max(r.currents[r.currents.len() - 1]);
    assert!(center > 0.0);
    assert!(
      end / center < 0.15,
      "末端段心电流 {end} 应远小于中心 {center}"
    );
    assert_eq!(r.segments, 40);
    assert_eq!(r.unknowns, 39);

    // 一根 40 段与两根 20 段（中心共节点）必须给出同一个结果。
    let lam = wavelength(FREQ);
    let l = 0.5 * lam;
    let two = NecInput {
      wires: vec![
        Wire::new([-l / 2.0, 0.0, 0.0], [0.0, 0.0, 0.0], THIN * lam, 20),
        Wire::new([0.0, 0.0, 0.0], [l / 2.0, 0.0, 0.0], THIN * lam, 20),
      ],
      feeds: vec![Feed {
        wire: 1,
        at: 0.0,
        volts: 1.0,
      }],
      freq_hz: FREQ,
      loads: Vec::new(),
      ground: Ground::Free,
      conductivity: None,
    };
    let rt = solve(&two).expect("折角结构应能求解");
    assert!(
      (rt.impedance.0 - r.impedance.0).abs() < 1e-6
        && (rt.impedance.1 - r.impedance.1).abs() < 1e-6,
      "两种建模方式应完全一致：{:?} vs {:?}",
      rt.impedance,
      r.impedance
    );
  }

  #[test]
  fn horizontal_dipole_quarter_wave_high_has_a_horizon_null() {
    let lam = wavelength(FREQ);
    let l = 0.5 * lam;
    let input = NecInput {
      wires: vec![Wire::new(
        [-l / 2.0, 0.0, 0.25 * lam],
        [l / 2.0, 0.0, 0.25 * lam],
        THIN * lam,
        20,
      )],
      feeds: vec![Feed {
        wire: 0,
        at: 0.5,
        volts: 1.0,
      }],
      freq_hz: FREQ,
      loads: Vec::new(),
      ground: Ground::Perfect,
      conductivity: None,
    };
    let r = solve(&input).expect("应能求解");
    // 阵列因子 `sin(k·h·sinΔ)`：h = λ/4 时地面完全抵消、天顶最大。
    assert!(
      (r.gain_max_elevation_deg - 90.0).abs() < 3.0,
      "λ/4 架高时最大辐射应在天顶，实际 {}°",
      r.gain_max_elevation_deg
    );
    assert!(
      r.gain_max_dbi > 6.5,
      "天顶增益 {} dBi 应明显高于自由空间偶极",
      r.gain_max_dbi
    );
    let horizon = r
      .elevation
      .iter()
      .find(|(e, _)| e.abs() < 0.6)
      .map(|(_, g)| *g)
      .unwrap_or(f64::NEG_INFINITY);
    assert!(
      r.gain_max_dbi - horizon > 40.0,
      "地面方向应为零点：{horizon} dBi vs 最大 {} dBi",
      r.gain_max_dbi
    );

    // h = λ/2 时阵列因子在 sinΔ = 0.5 处取最大 → 主瓣仰角约 30°。
    let input = NecInput {
      wires: vec![Wire::new(
        [-l / 2.0, 0.0, 0.5 * lam],
        [l / 2.0, 0.0, 0.5 * lam],
        THIN * lam,
        20,
      )],
      ground: Ground::Perfect,
      ..input
    };
    let r = solve(&input).expect("应能求解");
    assert!(
      (20.0..40.0).contains(&r.gain_max_elevation_deg),
      "λ/2 架高的主瓣仰角应约 30°，实际 {}°",
      r.gain_max_elevation_deg
    );
  }

  #[test]
  fn three_element_yagi_beats_a_dipole() {
    let lam = wavelength(FREQ);
    let a = THIN * lam;
    let wire = |x: f64, len: f64| Wire::new([x, -len / 2.0, 0.0], [x, len / 2.0, 0.0], a, 10);
    let input = NecInput {
      wires: vec![
        wire(-0.15 * lam, 0.505 * lam), // 反射器
        wire(0.0, 0.5 * lam),           // 有源振子
        wire(0.1 * lam, 0.44 * lam),    // 引向器
      ],
      feeds: vec![Feed {
        wire: 1,
        at: 0.5,
        volts: 1.0,
      }],
      freq_hz: FREQ,
      loads: Vec::new(),
      ground: Ground::Free,
      conductivity: None,
    };
    let r = solve(&input).expect("应能求解");
    assert_eq!(r.segments, 30);
    let dip = solve(&dipole(0.5, 20, false)).unwrap();
    assert!(
      r.gain_max_dbi > dip.gain_max_dbi + 3.5,
      "八木 {} dBi 应明显高于偶极 {} dBi",
      r.gain_max_dbi,
      dip.gain_max_dbi
    );
    assert!(
      r.front_to_back_db > 6.0,
      "前后比 {} dB 应大于 6",
      r.front_to_back_db
    );
    let phi = r.gain_max_azimuth_deg;
    assert!(phi < 30.0 || phi > 330.0, "主瓣方位 {phi}° 应指向 +x");
  }

  #[test]
  fn inverted_v_is_supported() {
    // 两根导线在顶点共节点（度 2）：电流连续性由共享节点自动成立。
    let lam = wavelength(FREQ);
    let arm = 0.25 * lam;
    let input = NecInput {
      wires: vec![
        Wire::new(
          [-arm * 0.7, 0.0, 0.0],
          [0.0, 0.0, arm * 0.7],
          THIN * lam,
          10,
        ),
        Wire::new([0.0, 0.0, arm * 0.7], [arm * 0.7, 0.0, 0.0], THIN * lam, 10),
      ],
      feeds: vec![Feed {
        wire: 0,
        at: 1.0,
        volts: 1.0,
      }],
      freq_hz: FREQ,
      loads: Vec::new(),
      ground: Ground::Free,
      conductivity: None,
    };
    let r = solve(&input).expect("折角结构应能求解");
    assert!(r.impedance.0 > 0.0, "电阻应为正：{}", r.impedance.0);
    assert!(r.gain_max_dbi > 0.0, "增益应为正：{}", r.gain_max_dbi);
  }

  #[test]
  fn invalid_inputs_are_rejected() {
    let bad = NecInput {
      wires: vec![Wire::new([0.0; 3], [0.0; 3], 0.001, 5)],
      feeds: vec![Feed {
        wire: 0,
        at: 0.5,
        volts: 1.0,
      }],
      freq_hz: FREQ,
      loads: Vec::new(),
      ground: Ground::Free,
      conductivity: None,
    };
    assert_eq!(solve(&bad), None);
    let mut bad = dipole(0.5, 11, false);
    bad.freq_hz = 0.0;
    assert_eq!(solve(&bad), None);
    let mut bad = dipole(0.5, 11, false);
    bad.feeds[0].wire = 3;
    assert_eq!(solve(&bad), None);
    assert_eq!(solve(&dipole(0.5, MAX_SEGMENTS + 1, false)), None);
    let mut bad = dipole(0.5, 11, false);
    bad.wires.clear();
    assert_eq!(solve(&bad), None);
    let mut bad = dipole(0.5, 11, false);
    bad.feeds.clear();
    assert_eq!(solve(&bad), None);
  }

  #[test]
  fn nec_text_round_trips() {
    let input = NecInput {
      wires: vec![
        Wire::new([-5.0, 0.0, 10.0], [0.0, 0.0, 10.0], 0.001, 10),
        Wire::new([0.0, 0.0, 10.0], [5.0, 0.0, 10.0], 0.001, 10),
      ],
      // 馈电点取在**段心**上：NEC 的 EX 卡只能指定段号，段心往返才是精确的。
      feeds: vec![Feed {
        wire: 1,
        at: 0.55,
        volts: 1.0,
      }],
      freq_hz: 14.1e6,
      loads: Vec::new(),
      ground: Ground::Perfect,
      conductivity: None,
    };
    let text = to_nec(&input);
    let back = parse_nec(&text).expect("导出的一定能再解析回来");
    assert_eq!(back.input.wires, input.wires);
    assert_eq!(back.input.feeds, input.feeds);
    assert!((back.input.freq_hz - input.freq_hz).abs() < 1e-3);
    assert_eq!(back.input.ground, Ground::Perfect);
    assert!(back.ignored.is_empty());
    // 往返之后解出的阻抗也必须一致。
    let a = solve(&input).unwrap();
    let b = solve(&back.input).unwrap();
    assert!((a.impedance.0 - b.impedance.0).abs() < 1e-6);
    assert!((a.impedance.1 - b.impedance.1).abs() < 1e-6);
  }

  #[test]
  fn nec_parser_reads_cards_and_reports_ignored_ones() {
    // 一个手工写的文件：半波偶极 + 一个 LD（负载）卡片 + 一张没实现的 RP 卡片。
    let text = "CM 10 m 偶极\nCE\n\
                GW 1, 21, -5.31, 0.0, 10.0, 5.31, 0.0, 10.0, 0.001\n\
                LD 0, 1, 5, 0, 100.0, 1.0E-5, 1.0E-10\n\
                GN 2, 0, 0, 0, 13.0, 0.005, 0.0\n\
                GE 1\n\
                EX 0, 1, 11, 0, 1.0, 0.0\n\
                FR 0, 1, 0.0, 0.0, 14.1, 0.0\n\
                RP 0, 91, 1, 1000, 0.0, 0.0, 5.0, 5.0\n\
                EN\n";
    let f = parse_nec(text).expect("应能解析");
    assert_eq!(f.input.wires.len(), 1);
    assert_eq!(f.input.wires[0].segments, 21);
    assert!((f.input.freq_hz - 14.1e6).abs() < 1.0);
    // 段号 11（1 起）→ 段心 0.5 处的相对位置。
    assert_eq!(f.input.feeds[0].wire, 0);
    assert!((f.input.feeds[0].at - 0.5).abs() < 1e-9);
    // `GN` 有耗地面与 `LD` 负载现在都**支持**了，不该再出现在「被忽略」列表里；
    // 只有确实没实现的卡片（如方向图请求 RP）才该列出。
    assert!(!f.ignored.contains(&"LD".to_owned()), "{:?}", f.ignored);
    assert!(!f.ignored.contains(&"GN".to_owned()), "{:?}", f.ignored);
    assert!(f.ignored.contains(&"RP".to_owned()), "{:?}", f.ignored);

    // `GN 2, 0, 0, 0, 13.0, 0.005` → 有耗地面，参数原样带入。
    match f.input.ground {
      Ground::Lossy { eps_r, sigma } => {
        assert!((eps_r - 13.0).abs() < 1e-9);
        assert!((sigma - 0.005).abs() < 1e-9);
      }
      other => panic!("应解析为有耗地面，实为 {other:?}"),
    }
    // `LD 0, tag, seg, seg, R(Ω), L(H), C(F)`（NEC-2 手册）→ R = 100Ω、
    // L = 1e-5 H = 10 µH、C = 1e-10 F = 100 pF。单位口径钉在这里：
    // 内层模型是 µH / pF，若再有人按 mH / µF 换算，这条断言会直接失败。
    assert_eq!(f.input.loads.len(), 1);
    let l = f.input.loads[0];
    assert_eq!(l.wire, 0);
    assert!((l.r_ohm - 100.0).abs() < 1e-9);
    assert!(
      (l.l_uh - 10.0).abs() < 1e-9,
      "L 应为 10 µH，实为 {}",
      l.l_uh
    );
    assert!(
      (l.c_pf - 100.0).abs() < 1e-9,
      "C 应为 100 pF，实为 {}",
      l.c_pf
    );
    // 段号 5（1 起）→ 段心 4.5/21。
    assert!((l.at - 4.5 / 21.0).abs() < 1e-9);
  }

  #[test]
  fn feed_can_sit_between_nodes() {
    // 馈电点落在段内（不是节点）时，激励按 hat 权重分摊到相邻两节点。
    // 与「正好落在节点上」的结果应当接近但不完全相同。
    let lam = wavelength(FREQ);
    let l = 0.5 * lam;
    let mk = |at: f64| NecInput {
      wires: vec![Wire::new(
        [-l / 2.0, 0.0, 0.0],
        [l / 2.0, 0.0, 0.0],
        THIN * lam,
        40,
      )],
      feeds: vec![Feed {
        wire: 0,
        at,
        volts: 1.0,
      }],
      freq_hz: FREQ,
      loads: Vec::new(),
      ground: Ground::Free,
      conductivity: None,
    };
    let on_node = solve(&mk(0.5)).expect("节点馈电应能求解");
    let in_seg = solve(&mk(0.51)).expect("段内馈电应能求解");
    assert!(
      (on_node.impedance.0 - in_seg.impedance.0).abs() < on_node.impedance.0 * 0.25,
      "节点馈电 {} 与段内馈电 {} 不应差太多",
      on_node.impedance.0,
      in_seg.impedance.0
    );
    assert!(in_seg.impedance.0 > 0.0 && in_seg.impedance.1 > 0.0);
  }

  /// 铜的电导率（S/m），也是页面默认值。
  const COPPER: f64 = 5.8e7;

  #[test]
  fn fresnel_coefficients_have_the_textbook_limits() {
    let f = 14.1e6;
    let omega = 2.0 * PI * f;
    // 理想导体：掠射与垂直入射都严格是 −1 / +1。
    for elev in [0.0, 15.0, 45.0, 89.0] {
      assert!((Ground::Perfect.reflection(omega, elev, true).re + 1.0).abs() < 1e-12);
      assert!((Ground::Perfect.reflection(omega, elev, false).re - 1.0).abs() < 1e-12);
    }
    // 近无耗介质 ε_r = 4：垂直入射时 R = (1 − √ε_r)/(1 + √ε_r) = −1/3，
    // 且两种极化在垂直入射下必须相等。
    let dielectric = Ground::lossy(4.0, 1e-6);
    let r_h = dielectric.reflection(omega, 90.0, true);
    let r_v = dielectric.reflection(omega, 90.0, false);
    assert!((r_h.re + 1.0 / 3.0).abs() < 5e-3, "垂直入射 R_h = {r_h:?}");
    assert!((r_v.re + 1.0 / 3.0).abs() < 5e-3, "垂直入射 R_v = {r_v:?}");
    // 掠射仍有 −1 / +1 的极限。
    assert!((dielectric.reflection(omega, 0.0, true).re + 1.0).abs() < 0.05);
    // 有耗地面：反射系数模长必须 < 1（除掠射），否则等于凭空多出功率。
    for elev in [5.0, 20.0, 45.0, 80.0] {
      for horizontal in [true, false] {
        let r = Ground::lossy(13.0, 0.005).reflection(omega, elev, horizontal);
        assert!(
          r.abs() < 1.000_1,
          "仰角 {elev}° 的 |R| = {} 应 ≤ 1",
          r.abs()
        );
      }
    }
    // 自由空间：系数为 0（不产生镜像）。
    assert!(Ground::Free.reflection(omega, 30.0, true).abs() < 1e-12);
  }

  #[test]
  fn loading_coil_resonates_a_short_dipole() {
    // 0.25λ 偶极是强容性的；在中心串一颗电感把它拉到谐振，是线圈加感天线的原理。
    let lam = wavelength(FREQ);
    let l = 0.25 * lam;
    let base = |loads: Vec<Load>| NecInput {
      wires: vec![Wire::new(
        [-l / 2.0, 0.0, 0.0],
        [l / 2.0, 0.0, 0.0],
        THIN * lam,
        20,
      )],
      feeds: vec![Feed {
        wire: 0,
        at: 0.5,
        volts: 1.0,
      }],
      loads,
      freq_hz: FREQ,
      ground: Ground::Free,
      conductivity: None,
    };
    let plain = solve(&base(Vec::new())).expect("应能求解");
    assert!(
      plain.impedance.1 < -200.0,
      "未加感时应强容性：{}",
      plain.impedance.1
    );

    // 电感量取「抵消未加感电抗」所需值，谐振残差应大幅变小。
    let omega = 2.0 * PI * FREQ;
    let need_uh = -plain.impedance.1 / omega * 1e6;
    let loaded = solve(&base(vec![Load {
      wire: 0,
      at: 0.5,
      r_ohm: 0.0,
      l_uh: need_uh,
      c_pf: 0.0,
    }]))
    .expect("应能求解");
    assert!(
      loaded.impedance.1.abs() < plain.impedance.1.abs() * 0.3,
      "加感后电抗 {:?} 应比未加感 {:?} 小一个量级",
      loaded.impedance.1,
      plain.impedance.1
    );

    // 加感会抬高电阻（线圈自己也发热）：串一颗有损线圈，电阻应更大。
    let lossy_coil = solve(&base(vec![Load {
      wire: 0,
      at: 0.5,
      r_ohm: 5.0,
      l_uh: need_uh,
      c_pf: 0.0,
    }]))
    .expect("应能求解");
    assert!(
      lossy_coil.impedance.0 > loaded.impedance.0,
      "带 5Ω 线圈损耗的电阻 {} 应大于理想线圈 {}",
      lossy_coil.impedance.0,
      loaded.impedance.0
    );

    // 电容负载则让电抗更负（与电感反号）。
    let cap = solve(&base(vec![Load {
      wire: 0,
      at: 0.5,
      r_ohm: 0.0,
      l_uh: 0.0,
      c_pf: 50.0,
    }]))
    .expect("应能求解");
    assert!(cap.impedance.1 < plain.impedance.1, "电容负载应使电抗更负");
  }

  #[test]
  fn copper_loss_is_small_but_scales_with_radius_and_frequency() {
    let lam = wavelength(FREQ);
    let l = 0.5 * lam;
    let mk = |radius: f64, conductivity: Option<f64>| NecInput {
      wires: vec![Wire::new(
        [-l / 2.0, 0.0, 0.0],
        [l / 2.0, 0.0, 0.0],
        radius,
        20,
      )],
      feeds: vec![Feed {
        wire: 0,
        at: 0.5,
        volts: 1.0,
      }],
      loads: Vec::new(),
      freq_hz: FREQ,
      ground: Ground::Free,
      conductivity,
    };
    let ideal = solve(&mk(2.0e-3, None)).expect("应能求解");
    assert!(
      (ideal.efficiency - 1.0).abs() < 1e-3,
      "理想导体效率应 ≈ 1，实得 {}",
      ideal.efficiency
    );
    assert!(ideal.loss_db.abs() < 0.01);
    assert!((ideal.gain_max_dbi - ideal.directivity_dbi).abs() < 0.01);

    // 2 mm 铜线在 14 MHz：单位长电阻约 0.07 Ω/m，λ/2 偶极效率应仍在 95% 以上。
    let copper = solve(&mk(2.0e-3, Some(COPPER))).expect("应能求解");
    assert!(
      copper.efficiency > 0.95,
      "2 mm 铜线效率 {} 应 > 95%",
      copper.efficiency
    );
    assert!(copper.efficiency < 1.0, "有损导线效率必须 < 1");
    // 电阻升高、增益下降，且增益 = 方向性 − 损耗（内部口径自洽）。
    assert!(copper.impedance.0 > ideal.impedance.0);
    assert!((copper.gain_max_dbi - (copper.directivity_dbi - copper.loss_db)).abs() < 1e-9);
    assert!(copper.gain_max_dbi < ideal.gain_max_dbi);

    // 同样的铜、更细的导线 → 损耗更大（R' ∝ 1/a）。
    let thin = solve(&mk(2.0e-4, Some(COPPER))).expect("应能求解");
    assert!(
      thin.efficiency < copper.efficiency,
      "0.2 mm 导线效率 {} 应低于 2 mm 的 {}",
      thin.efficiency,
      copper.efficiency
    );

    // 集肤效应：表面电阻 ∝ √f。要用**同一几何 + 电流分布近似不变**才对得上
    // 这个比例，所以取短偶极（两种频率下都近似三角电流分布），比较
    // 「有损电阻 − 理想电阻」这个增量：频率 ×4 应当让损耗电阻 ×2。
    let short = |f: f64, conductivity: Option<f64>| NecInput {
      wires: vec![Wire::new(
        [-0.2126 / 2.0, 0.0, 0.0],
        [0.2126 / 2.0, 0.0, 0.0],
        2.0e-3,
        20,
      )],
      feeds: vec![Feed {
        wire: 0,
        at: 0.5,
        volts: 1.0,
      }],
      loads: Vec::new(),
      freq_hz: f,
      ground: Ground::Free,
      conductivity,
    };
    let low = solve(&short(FREQ, Some(COPPER))).unwrap().impedance.0
      - solve(&short(FREQ, None)).unwrap().impedance.0;
    let high = solve(&short(4.0 * FREQ, Some(COPPER))).unwrap().impedance.0
      - solve(&short(4.0 * FREQ, None)).unwrap().impedance.0;
    assert!(
      (high / low - 2.0).abs() < 0.25,
      "频率 ×4 时损耗电阻应 ×2：{low} → {high}（比值 {}）",
      high / low
    );
  }

  /// 造一段水平导线（`dir = cur = +x`，实段），供镜像几何的用例直接摆坐标。
  fn test_seg(start: Vec3, len: f64) -> Seg {
    Seg {
      start,
      dir: [1.0, 0.0, 0.0],
      cur: [1.0, 0.0, 0.0],
      len,
      radius: 1.0e-3,
      start_node: 0,
      end_node: 1,
    }
  }

  /// 镜像仰角的口径：`atan((z_o + z_s) / D)`，**不是** `atan(2(z_o + z_s) / D)`。
  ///
  /// 曾经多乘了一个 2：自耦项（`D = 0`）被 `atan2` 饱和到 90°，乘不乘都是 90°，于是
  /// 全部现有用例都看不出问题；而互耦项 —— 不同高度或水平分离的两段之间 —— 仰角整体
  /// 大一倍，有耗地面的 `R_h` / `R_v` 因此取错，阻抗与方向图静默偏掉。这里把几何钉死。
  #[test]
  fn image_elevation_uses_the_actual_distance_to_the_image() {
    // 观察段高 3 m、源段高 6 m，中点水平相距 10 m → 到镜像的垂距 3 + 6 = 9 m。
    let obs = test_seg([0.0, 0.0, 3.0], 2.0);
    let src = test_seg([10.0, 0.0, 6.0], 2.0);
    let expected = 9.0f64.atan2(10.0).to_degrees();
    let got = image_elevation(&obs, &src);
    assert!(
      (got - expected).abs() < 1e-9,
      "仰角应为 atan((3+6)/10) = {expected}°，实为 {got}°（乘了 2 会得到 {}°）",
      18.0f64.atan2(10.0).to_degrees()
    );

    // 同高度、有水平分离：用 `2z` 而不是 `4z`。
    let a = test_seg([0.0, 0.0, 2.0], 2.0);
    let b = test_seg([10.0, 0.0, 2.0], 2.0);
    let same = 4.0f64.atan2(10.0).to_degrees();
    assert!((image_elevation(&a, &b) - same).abs() < 1e-9);

    // 自耦项：垂距 2z、D = 0 → 上限 90°（**这条抓不到上面那个错**，留作对照）。
    let w = test_seg([0.0, 0.0, 3.0], 2.0);
    assert!((image_elevation(&w, &w) - 90.0).abs() < 1e-6);
  }

  /// 单张卡片字段坏掉只丢这一张：其余卡片照常生效，并且在 `ignored` 里留下带原因的诊断。
  ///
  /// 曾经一个 `?` 让整份文件解析失败 —— 用户看到的是「文件读不进来」，而不是
  /// 「第 2 张 GW 卡有笔误」，而且会连带丢掉**全部**导线。
  #[test]
  fn parser_degrades_per_card_instead_of_failing_the_whole_file() {
    let text = "GW 1 11 -5.31 0.0 10.0 5.31 0.0 10.0 0.001\n\
                GW 2 11 0.0 0.0 10.0 0.0 0.0 xyz 0.001\n\
                GW 3 11 -5.31 0.0 10.0 5.31 0.0 10.0 0.001\n\
                EX 0 1 6 0 1.0 0.0\n\
                FR 0 1 0.0 0.0 14.1 0.0\n\
                EN\n";
    let f = parse_nec(text).expect("两张合法导线 + 一张坏卡片仍应解析成功");
    assert_eq!(f.input.wires.len(), 2, "坏卡片只丢自己那一张");
    assert!(
      f.ignored.iter().any(|x| x.contains("GW(字段非法")),
      "必须留下诊断：{:?}",
      f.ignored
    );

    // 引用不存在 tag 的激励源：只丢这一条，不留诊断之外的副作用。
    let text = "GW 1 11 -5.31 0.0 10.0 5.31 0.0 10.0 0.001\n\
                EX 0 1 6 0 1.0 0.0\n\
                EX 0 99 1 0 1.0 0.0\n\
                FR 0 1 0.0 0.0 14.1 0.0\n\
                EN\n";
    let f = parse_nec(text).expect("合法的那条激励仍在");
    assert_eq!(f.input.feeds.len(), 1);
    assert!(
      f.ignored.iter().any(|x| x.contains("不存在的 tag 99")),
      "必须留下诊断：{:?}",
      f.ignored
    );

    // 段数超上限：不静默截断，留下诊断（离散段数变了，结果会与 4NEC2 不一致）。
    let text = "GW 1 999 -5.31 0.0 10.0 5.31 0.0 10.0 0.001\n\
                EX 0 1 6 0 1.0 0.0\n\
                FR 0 1 0.0 0.0 14.1 0.0\n\
                EN\n";
    let f = parse_nec(text).expect("应能解析");
    assert_eq!(f.input.wires[0].segments, MAX_SEGMENTS);
    assert!(
      f.ignored.iter().any(|x| x.contains("超过上限")),
      "必须留下诊断：{:?}",
      f.ignored
    );
  }

  #[test]
  fn lossy_ground_weakens_the_image_and_shows_up_as_loss() {
    let lam = wavelength(FREQ);
    let l = 0.5 * lam;
    // 架高 λ/8 的水平偶极：镜像抵消很弱，地面损耗最明显。
    let mk = |ground: Ground| NecInput {
      wires: vec![Wire::new(
        [-l / 2.0, 0.0, 0.125 * lam],
        [l / 2.0, 0.0, 0.125 * lam],
        2.0e-3,
        20,
      )],
      feeds: vec![Feed {
        wire: 0,
        at: 0.5,
        volts: 1.0,
      }],
      loads: Vec::new(),
      freq_hz: FREQ,
      ground,
      conductivity: None,
    };
    let perfect = solve(&mk(Ground::Perfect)).expect("应能求解");
    let real = solve(&mk(Ground::lossy(13.0, 0.005))).expect("应能求解");
    // 理想地面：无损耗（效率≈1）。
    assert!(perfect.efficiency > 0.99, "{}", perfect.efficiency);
    // 有耗地面：镜像变弱 → 馈电功率里有一部分进了地里，效率下降。
    assert!(
      real.efficiency < perfect.efficiency,
      "有耗地面效率 {} 应低于理想地面的 {}",
      real.efficiency,
      perfect.efficiency
    );
    assert!(real.loss_db > 0.0);
    // 增益口径仍然自洽。
    assert!((real.gain_max_dbi - (real.directivity_dbi - real.loss_db)).abs() < 1e-9);
    // 阻抗也会变（镜像耦合变了），不能与理想地面完全相同。
    assert!(
      (real.impedance.0 - perfect.impedance.0).abs() > 1e-6
        || (real.impedance.1 - perfect.impedance.1).abs() > 1e-6
    );
  }

  #[test]
  fn loads_and_ground_round_trip_through_the_nec_text() {
    let input = NecInput {
      wires: vec![Wire::new([-5.31, 0.0, 10.0], [5.31, 0.0, 10.0], 0.001, 10)],
      feeds: vec![Feed {
        wire: 0,
        at: 0.55,
        volts: 1.0,
      }],
      loads: vec![Load {
        wire: 0,
        at: 0.55,
        r_ohm: 12.5,
        l_uh: 8.0,
        c_pf: 0.0,
      }],
      freq_hz: 14.1e6,
      ground: Ground::lossy(13.0, 0.005),
      conductivity: None,
    };
    let text = to_nec(&input);
    assert!(text.contains("GN 2"), "应导出 GN 卡片：{text}");
    assert!(text.contains("LD 0"), "应导出 LD 卡片：{text}");
    let back = parse_nec(&text).expect("导出的一定能再解析回来");
    assert_eq!(back.input.loads, input.loads);
    match back.input.ground {
      Ground::Lossy { eps_r, sigma } => {
        assert!((eps_r - 13.0).abs() < 1e-3);
        assert!((sigma - 0.005).abs() < 1e-5);
      }
      other => panic!("应往返成有耗地面，实为 {other:?}"),
    }
    let a = solve(&input).expect("应能求解");
    let b = solve(&back.input).expect("应能求解");
    assert!((a.impedance.0 - b.impedance.0).abs() < 1e-3);
    assert!((a.impedance.1 - b.impedance.1).abs() < 1e-3);
  }

  #[test]
  fn pattern3d_grid_is_consistent_with_the_cuts() {
    // 球壳渲染吃的是这份网格：峰值必须与切面的最大增益一致，长度与控制点对得上。
    for (label, input) in [
      ("自由空间偶极", dipole(0.5, 20, false)),
      ("λ/4 架高水平偶极", {
        // 贴地的水平偶极在理想地面上被镜像完全抵消，解不出来，所以抬到 λ/4。
        let lam = wavelength(FREQ);
        let l = 0.5 * lam;
        NecInput {
          wires: vec![Wire::new(
            [-l / 2.0, 0.0, 0.25 * lam],
            [l / 2.0, 0.0, 0.25 * lam],
            THIN * lam,
            20,
          )],
          feeds: vec![Feed {
            wire: 0,
            at: 0.5,
            volts: 1.0,
          }],
          loads: Vec::new(),
          freq_hz: FREQ,
          ground: Ground::Perfect,
          conductivity: None,
        }
      }),
    ] {
      let r = solve(&input).expect("应能求解");
      let (nt, np) = r.pattern3d_shape;
      assert_eq!(r.pattern3d.len(), (nt + 1) * np, "{label} 网格长度不对");
      let peak = r
        .pattern3d
        .iter()
        .copied()
        .fold(f64::NEG_INFINITY, f64::max);
      assert!(
        (peak - r.gain_max_dbi).abs() < 1.0,
        "{label} 网格峰值 {peak} 应接近最大增益 {}",
        r.gain_max_dbi
      );
      // 地面以下（θ > 90°）不应有辐射。
      if input.ground.is_grounded() {
        for i in (nt / 2 + 1)..=nt {
          for j in 0..np {
            assert!(
              r.pattern3d[i * np + j] < -100.0,
              "{label} 下半空间 θ 序号 {i} 仍有辐射"
            );
          }
        }
      }
      assert!(r.pattern3d.iter().all(|v| v.is_finite()));
    }
  }

  #[test]
  fn sweep_matches_solve_at_the_center_frequency() {
    // 扫频只走「装配 + 解方程 + 取阻抗」的快路径；在输入自身的频率上，它给出的
    // 阻抗与驻波比必须与完整 `solve` **逐位一致**（否则两条口径就分家了）。
    let input = dipole(0.5, 20, false);
    let full = solve(&input).expect("应能求解");
    let f0 = input.freq_hz;
    let pts = sweep(&input, f0 * 0.9, f0 * 1.1, 21);
    assert_eq!(pts.len(), 21);
    assert!((pts[0].freq_hz - f0 * 0.9).abs() < 1e-6);
    assert!((pts[20].freq_hz - f0 * 1.1).abs() < 1e-6);
    let mid = pts[10];
    assert!((mid.freq_hz - f0).abs() < 1e-6, "中间点应是 f0");
    assert!((mid.impedance.0 - full.impedance.0).abs() < 1e-9);
    assert!((mid.impedance.1 - full.impedance.1).abs() < 1e-9);
    assert!((mid.swr_50 - full.swr_50).abs() < 1e-9);
    // 单点 `impedance()` 与 `solve()` 的阻抗也必须一致。
    let only = impedance(&input).expect("应能求解");
    assert!((only.0 - full.impedance.0).abs() < 1e-9);
    assert!((only.1 - full.impedance.1).abs() < 1e-9);
  }

  #[test]
  fn swr_curve_bottoms_out_below_the_starting_frequency() {
    // 0.5λ 的细线偶极在 f0 上偏「长」（X > 0），要缩短电长度才谐振 —— 也就是更低的
    // 频率。谐振长度 0.4787λ 对应 f ≈ 0.957·f0，此处电抗过零、SWR 最低。
    let input = dipole(0.5, 41, false);
    let f0 = input.freq_hz;
    let pts = sweep(&input, f0 * 0.90, f0 * 1.05, 61);
    let best = pts
      .iter()
      .copied()
      .reduce(|a, b| if b.swr_50 < a.swr_50 { b } else { a })
      .expect("应有扫描点");
    let ratio = best.freq_hz / f0;
    assert!(
      (0.935..=0.980).contains(&ratio),
      "SWR 最低点应在 0.935–0.98 f0（谐振长度 0.4787λ），实际 {ratio:.4}"
    );
    // 该处电抗接近 0、电阻约 70 Ω ⇒ SWR ≈ 1.4。
    assert!(
      best.impedance.1.abs() < 12.0,
      "谐振点电抗 {} 应接近 0",
      best.impedance.1
    );
    assert!(
      (1.2..=1.7).contains(&best.swr_50),
      "谐振点 SWR {} 应约 1.4",
      best.swr_50
    );
    // 扫频两端都离谐振更远，SWR 必然更差。
    assert!(pts[0].swr_50 > best.swr_50);
    assert!(pts[pts.len() - 1].swr_50 > best.swr_50);
  }

  #[test]
  fn swr_bandwidth_edges_sit_on_the_limit() {
    // 取谐振长度（0.4787λ），中心频率上 SWR ≈ 1.4 ≤ 2，频带确实存在。
    let input = dipole(0.4787, 41, false);
    let f0 = input.freq_hz;
    let (lo, hi) =
      swr_bandwidth(&input, 2.0, 0.3, 41).expect("谐振偶极在 f0 附近应有 SWR ≤ 2 的频带");
    assert!(lo < f0 && f0 < hi, "{lo} < {f0} < {hi}");
    // 带宽在 ±30% 之内，且不是退化成一个点。
    assert!(lo > f0 * 0.7 && hi < f0 * 1.3);
    assert!(hi - lo > f0 * 0.01, "带宽 {} Hz 太窄", hi - lo);

    let swr_at = |f: f64| -> f64 {
      let (r, x) = impedance(&retuned(&input, f)).expect("应能求解");
      swr_for(r, x, 50.0)
    };
    // 边界处贴住限值（二分 10 次），带内取样全部达标，带外必超标。
    assert!((swr_at(lo) - 2.0).abs() < 1e-3, "下边界 SWR {}", swr_at(lo));
    assert!((swr_at(hi) - 2.0).abs() < 1e-3, "上边界 SWR {}", swr_at(hi));
    for i in 1..20 {
      let f = lo + (hi - lo) * i as f64 / 20.0;
      assert!(swr_at(f) <= 2.0 + 1e-6, "带内 {f} Hz 超标：{}", swr_at(f));
    }
    assert!(swr_at(lo * 0.97) > 2.0, "下边界外应超标");
    assert!(swr_at(hi * 1.03) > 2.0, "上边界外应超标");
  }

  #[test]
  fn swr_bandwidth_needs_a_centered_match_and_reports_when_too_wide() {
    // 0.5λ 的偶极在 f0 上 SWR ≈ 2.3 > 2 —— 中心就不达标，谈带宽没有意义。
    let input = dipole(0.5, 41, false);
    assert!(swr_for(80.2, 44.6, 50.0) > 2.0);
    assert_eq!(swr_bandwidth(&input, 2.0, 0.1, 21), None);
    // 谐振偶极但只给 ±1% 的扫描范围：带宽比范围还宽 → 返回 None（页面提示放宽）。
    let resonant = dipole(0.4787, 41, false);
    assert_eq!(swr_bandwidth(&resonant, 2.0, 0.01, 21), None);
    // 放宽到 ±30% 就能找到。
    assert!(swr_bandwidth(&resonant, 2.0, 0.3, 41).is_some());
  }

  #[test]
  fn design_yagi_gives_a_feedable_beam() {
    // 向导的目标与验收用的是**同一个求解器**：这里把结果重新装配再解一遍，
    // 数值必须与向导报告的一致（否则就成了两套口径）。
    let lam = wavelength(FREQ);
    let radius = 1e-4 * lam;
    for elements in 3..=5usize {
      let d = design_yagi(FREQ, elements, radius).expect("应能设计");
      assert_eq!(d.elements, elements);
      assert_eq!(d.lengths.len(), elements);
      assert_eq!(d.spacings.len(), elements - 1);
      assert!(d.evaluations > 1);

      // 尺寸必须在物理上说得通：反射器比有源振子长，引向器依次更短。
      let driven = d.lengths[1];
      assert!(
        d.lengths[0] > driven * 1.01,
        "{elements} 单元：反射器 {} 应长于有源振子 {driven}",
        d.lengths[0]
      );
      // 引向器自前往后不增长（等长也是合法设计，实测五单元就会收敛到「两根等长」）。
      for w in d.lengths.windows(2).skip(1) {
        assert!(
          w[0] >= w[1] * 0.999,
          "{elements} 单元：引向器不该越往后越长（{} → {}）",
          w[0],
          w[1]
        );
      }
      assert!(d.lengths[elements - 1] < driven, "引向器应短于有源振子");
      assert!(d.spacings.iter().all(|v| *v > 0.0));

      // 一副能用的波束：增益明显高于偶极、前后比像样、驻波比不至于没法馈电。
      assert!(
        d.gain_dbi > 6.5,
        "{elements} 单元增益 {} dBi 应明显高于偶极",
        d.gain_dbi
      );
      assert!(
        d.front_to_back_db > 8.0,
        "{elements} 单元前后比 {} dB 应像样",
        d.front_to_back_db
      );
      assert!(d.swr_50 < 4.0, "{elements} 单元 SWR {} 应可馈电", d.swr_50);

      // 口径一致性：重新装配同一份尺寸，实测值与向导报告值逐位相同。
      let input = yagi_design_input(&d, radius).expect("应能装配");
      let r = solve(&input).expect("应能求解");
      assert!((r.gain_max_dbi - d.gain_dbi).abs() < 1e-9);
      assert!((r.front_to_back_db - d.front_to_back_db).abs() < 1e-9);
      assert!((r.swr_50 - d.swr_50).abs() < 1e-9);
      // 元数越多，增益不该更差（向导是逐元数独立优化的，允许小幅波动）。
      assert!(r.gain_max_dbi > 6.0);
    }
  }

  #[test]
  fn design_yagi_rejects_bad_input_and_clamps_elements() {
    let lam = wavelength(FREQ);
    assert_eq!(design_yagi(0.0, 3, 0.001), None);
    assert_eq!(design_yagi(FREQ, 3, 0.0), None);
    // 单元数夹在 2..=5。
    assert_eq!(design_yagi(FREQ, 1, 1e-4 * lam).unwrap().elements, 2);
    assert_eq!(design_yagi(FREQ, 9, 1e-4 * lam).unwrap().elements, 5);
  }

  #[test]
  fn design_yagi_rejects_non_finite_input() {
    // `f64::from_str` 能解析出 `nan` / `inf`，页面输入框可以直接喂进来。
    let lam = wavelength(FREQ);
    let r = 1e-4 * lam;
    assert_eq!(design_yagi(f64::NAN, 3, r), None);
    assert_eq!(design_yagi(f64::INFINITY, 3, r), None);
    assert_eq!(design_yagi(FREQ, 3, f64::NAN), None);
    assert!(YagiSearch::new(FREQ, 3, f64::INFINITY).is_none());
  }

  #[test]
  fn yagi_search_steps_to_the_same_answer_as_the_sync_run() {
    // 分步版必须与同步版**逐位一致**：前端用它把 1–2 秒的优化摊到多帧，
    // 数值一旦不同就成了两套口径。
    let lam = wavelength(FREQ);
    let radius = 1e-4 * lam;
    for elements in 3..=4usize {
      let sync = design_yagi(FREQ, elements, radius).expect("应能设计");
      let mut search = YagiSearch::new(FREQ, elements, radius).expect("应能起步");
      assert!(!search.is_done());
      // 每次只给 1 次评估的预算：模拟前端的「分帧推进」。
      let mut stepped = None;
      let mut guard = 0;
      while !search.is_done() {
        guard += 1;
        assert!(guard < 1000, "分步推进不该不收敛");
        stepped = search.step(1);
      }
      // 收敛后再调一次仍是同一结果（幂等）。
      let done = search.step(1);
      let stepped = stepped.expect("应给出结果");
      assert_eq!(stepped, sync, "{elements} 单元：分步与同步结果应一致");
      assert_eq!(done.as_ref(), Some(&sync));
      assert!(search.progress() > 0.0);
      assert!(search.evaluations() > 1);
    }
  }

  #[test]
  fn design_yagi_scales_with_wavelength() {
    // 起点与约束全部按波长归一化，所以频率翻倍时尺寸应当成比例（含半径一起缩放）。
    let low = design_yagi(FREQ, 3, 1e-4 * wavelength(FREQ)).expect("应能设计");
    let high = design_yagi(2.0 * FREQ, 3, 1e-4 * wavelength(2.0 * FREQ)).expect("应能设计");
    for (a, b) in low.lengths.iter().zip(high.lengths.iter()) {
      assert!((a / b - 2.0).abs() < 0.02, "长度应成比例：{a} vs {b}");
    }
    for (a, b) in low.spacings.iter().zip(high.spacings.iter()) {
      assert!((a / b - 2.0).abs() < 0.02, "间距应成比例：{a} vs {b}");
    }
    assert!(
      (low.gain_dbi - high.gain_dbi).abs() < 0.05,
      "增益应几乎相同"
    );
  }

  /// 两段导线中间留 1 mm 断口（远大于 0.1 µm 的节点合并阈值），
  /// 分段与下面的「连续导线」逐段对齐 —— 这样零长线的结果必须与连续导线几乎相同。
  fn split_dipole() -> NecInput {
    let half = 5.07f64;
    NecInput {
      wires: vec![
        Wire::new(
          [-half, 0.0, 0.0],
          [-0.0005, 0.0, 0.0],
          THIN * wavelength(FREQ),
          10,
        ),
        Wire::new(
          [0.0005, 0.0, 0.0],
          [half, 0.0, 0.0],
          THIN * wavelength(FREQ),
          10,
        ),
      ],
      feeds: vec![Feed {
        wire: 0,
        at: 0.5,
        volts: 1.0,
      }],
      loads: Vec::new(),
      freq_hz: FREQ,
      ground: Ground::Free,
      conductivity: None,
    }
  }

  /// 与 `split_dipole` 同分段、同馈电位置的连续导线（0.1 m 之外就是它）。
  fn joined_dipole() -> NecInput {
    let mut input = split_dipole();
    input.wires = vec![Wire::new(
      [-5.07, 0.0, 0.0],
      [5.07, 0.0, 0.0],
      THIN * wavelength(FREQ),
      20,
    )];
    // 同一个物理馈电点：左臂中点（x = −2.535）在整根导线上的相对位置是 0.25。
    input.feeds[0].at = 0.25;
    input
  }

  /// 跨接断口的传输线：两臂各 5.07 m，断口在中间。
  ///
  /// 端口就在断口上：左臂的终点与右臂的起点。
  fn bridging_line(length_m: f64, z0: f64) -> TransmissionLine {
    TransmissionLine {
      wire_a: 0,
      at_a: 1.0,
      wire_b: 1,
      at_b: 0.0,
      z0,
      length_m,
      velocity_factor: 1.0,
    }
  }

  #[test]
  fn zero_length_line_is_a_plain_connection() {
    // 零长线的两行约束退化为 `Ṽ_a = −Ṽ_b`、`I_a = −I_b`（按各自的段的朝向），
    // 就是一根导线。断口只有 1 mm、分段与连续导线逐段对齐，所以这里可以卡紧到 2%。
    let with_line =
      solve_with_lines(&split_dipole(), &[bridging_line(0.0, 50.0)]).expect("应能求解");
    let plain = solve(&joined_dipole()).expect("应能求解");
    for (got, want, what) in [
      (with_line.impedance.0, plain.impedance.0, "电阻"),
      (with_line.impedance.1, plain.impedance.1, "电抗"),
    ] {
      let rel = (got - want).abs() / want.abs().max(1.0);
      assert!(
        rel < 0.02,
        "零长线应等价于导线：{what} {got:.3} vs 连续导线 {want:.3}（相对差 {rel:.3}）"
      );
    }
  }

  #[test]
  fn half_wave_line_is_not_a_wire_but_is_still_passive() {
    // 注意：**λ/2 线不等于导线**。作为两端口它在 θ=π 给出 `Ṽ_a = Ṽ_b、I_a = I_b`，
    // 而导线是 `Ṽ_a = −Ṽ_b、I_a = −I_b`；两者的「阻抗透传」只对**端接负载**成立，
    // 对「桥接结构两个断口」这种拓扑并不等价。所以这里只断言它仍是有源、有限的解，
    // 真正的严格判据留给零长线用例（那才是与导线等价的那一个）。
    let input = split_dipole();
    let lambda = wavelength(FREQ);
    for (tag, length) in [("λ/2", 0.5 * lambda), ("λ", lambda)] {
      let r = solve_with_lines(&input, &[bridging_line(length, 50.0)]).expect("应能求解");
      assert!(
        r.impedance.0.is_finite() && r.impedance.1.is_finite() && r.impedance.0.abs() < 1e4,
        "{tag} 线解出的阻抗 {:?} 不合理",
        r.impedance
      );
      assert!(r.gain_max_dbi.is_finite(), "{tag} 线增益应为有限值");
    }
    // 特性阻抗必须真的起作用：只有 50 Ω 与 300 Ω 的结果不同，才说明它进了方程。
    let a = solve_with_lines(&input, &[bridging_line(0.25 * lambda, 50.0)]).expect("应能求解");
    let b = solve_with_lines(&input, &[bridging_line(0.25 * lambda, 300.0)]).expect("应能求解");
    assert!(
      (a.impedance.0 - b.impedance.0).abs() > 1.0,
      "特性阻抗没起作用：{:?} vs {:?}",
      a.impedance,
      b.impedance
    );
  }

  #[test]
  fn quarter_wave_line_transforms_instead_of_passing_through() {
    // λ/4 线是阻抗变换器，绝不能透传 —— 否则说明约束行退化了。
    let input = split_dipole();
    let lambda = wavelength(FREQ);
    let base = solve_with_lines(&input, &[bridging_line(0.0, 50.0)]).expect("应能求解");
    let quarter =
      solve_with_lines(&input, &[bridging_line(0.25 * lambda, 50.0)]).expect("应能求解");
    let rel = (quarter.impedance.0 - base.impedance.0).abs() / base.impedance.0.abs().max(1.0);
    assert!(
      rel > 0.2,
      "λ/4 线的阻抗应明显不同于直连：{:?} vs {:?}",
      quarter.impedance,
      base.impedance
    );
    // 变换后的阻抗仍是「有源天线」的量级，不是数值垃圾。
    assert!(
      quarter.impedance.0.abs() < 1e4 && quarter.impedance.1.is_finite(),
      "λ/4 线解出的阻抗 {:?} 不合理",
      quarter.impedance
    );
  }

  #[test]
  fn invalid_lines_are_rejected() {
    let input = split_dipole();
    // 特性阻抗 / 速度因子必须为正，长度不能为负。
    assert_eq!(solve_with_lines(&input, &[bridging_line(1.0, 0.0)]), None);
    let mut bad = bridging_line(1.0, 50.0);
    bad.velocity_factor = 0.0;
    assert_eq!(solve_with_lines(&input, &[bad]), None);
    let bad = bridging_line(-1.0, 50.0);
    assert_eq!(solve_with_lines(&input, &[bad]), None);
    // 端口导线越界。
    let mut bad = bridging_line(1.0, 50.0);
    bad.wire_b = 9;
    assert_eq!(solve_with_lines(&input, &[bad]), None);
    // 没有传输线时与 `solve` 完全一致。
    let a = solve(&input).expect("应能求解");
    let b = solve_with_lines(&input, &[]).expect("应能求解");
    assert_eq!(a.impedance, b.impedance);
  }

  #[test]
  fn pattern_power_matches_input_power_for_a_yagi() {
    // 多导线结构（八木）上，「装配出的算子 + 馈电功率」与「方向图积分出的辐射功率」
    // 也必须一致：这是 Galerkin 方案自洽性的最硬判据，也保证页面上的「效率」不是
    // 求积残差冒充的损耗（曾误以为四单元八木有 1.8% 的数值损耗，其实是页面把
    // 「理想导体」当成了铜 —— 见页面 `sigma_wire` 的注释）。
    let lam = wavelength(FREQ);
    let d = design_yagi(FREQ, 4, 1e-4 * lam).expect("应能设计");
    let input = yagi_design_input(&d, 1e-4 * lam).expect("应能装配");
    let model = Model::from_wires(&input.wires).unwrap();
    let k = 2.0 * PI * FREQ / C0;
    let unknowns: Vec<usize> = (0..model.nodes.len())
      .filter(|&j| !model.is_fixed_end(j, false))
      .collect();
    let mut z = assemble(&model, &unknowns, k, &input).unwrap();
    let (seg_idx, alpha) = locate(&model, 1, 0.5).expect("馈电点应在导线 1 上");
    let seg = model.segs[seg_idx];
    let mut rhs = vec![Cx::ZERO; unknowns.len()];
    for (node, w) in [(seg.start_node, 1.0 - alpha), (seg.end_node, alpha)] {
      if let Some(row) = unknowns.iter().position(|&j| j == node) {
        rhs[row] += Cx::of(-w);
      }
    }
    let sol = solve_linear(&mut z, &mut rhs).expect("应能求解");
    let mut cur = vec![Cx::ZERO; model.nodes.len()];
    for (i, &j) in unknowns.iter().enumerate() {
      cur[j] = sol[i];
    }
    let node_i = |node: usize| -> Cx {
      unknowns
        .iter()
        .position(|&j| j == node)
        .map_or(Cx::ZERO, |row| sol[row])
    };
    let i_feed = node_i(seg.start_node) * (1.0 - alpha) + node_i(seg.end_node) * alpha;
    let p_in = 0.5 * (Cx::of(1.0) / i_feed).re * i_feed.abs2();

    // 生产用的球面网格（θ 2.5°、φ 5°）就该够准。
    let (dth, dph) = (2.5f64, 5.0f64);
    let mut integ = 0.0;
    let mut th = 0.0f64;
    while th <= 180.0 + 1e-9 {
      let mut ph = 0.0f64;
      while ph < 360.0 - 1e-9 {
        integ += perp2(&model, &cur, k, Ground::Free, th, ph)
          * th.to_radians().sin()
          * dth.to_radians()
          * dph.to_radians();
        ph += dph;
      }
      th += dth;
    }
    let p_rad = k * k * ETA0 / (32.0 * PI * PI) * integ;
    let ratio = p_rad / p_in;
    assert!(
      (ratio - 1.0).abs() < 2e-3,
      "四单元八木的方向图功率/馈电功率 = {ratio:.5}，应约为 1"
    );
  }

  #[test]
  fn junction_orientation_does_not_change_the_result() {
    // 同一副倒 V，三种「顶点处两段怎么连」的写法必须给出**同一个**答案。
    //
    // 这曾经是错的：弱形式把标量位项分部积分后，端点的边界项要在接点抵消，
    // 而「两臂都从顶点出发」的写法会留下未抵消的项 —— 矩阵仍对称，却不再满足
    // 能量守恒，实测解出 `−22 Ω`、增益 `−118 dBi`。修法是让模型内部沿导线
    // 「一进一出」（`Model::from_wires` 的方向归一化）。
    let arm = 5.07f64;
    let half = 60f64.to_radians();
    let (sh, ch) = half.sin_cos();
    let apex = [0.0, 0.0, 0.0];
    let left = [arm * ch, 0.0, -arm * sh];
    let right = [-arm * ch, 0.0, -arm * sh];
    let forms = [
      vec![
        Wire::new(apex, left, 0.002, 10),
        Wire::new(apex, right, 0.002, 10),
      ],
      vec![
        Wire::new(apex, left, 0.002, 10),
        Wire::new(right, apex, 0.002, 10),
      ],
      vec![
        Wire::new(left, apex, 0.002, 10),
        Wire::new(right, apex, 0.002, 10),
      ],
    ];
    let mut reference: Option<(f64, f64)> = None;
    for wires in forms {
      let input = NecInput {
        wires,
        feeds: vec![Feed {
          wire: 0,
          at: 0.5,
          volts: 1.0,
        }],
        loads: Vec::new(),
        freq_hz: FREQ,
        ground: Ground::Free,
        conductivity: None,
      };
      let r = solve(&input).expect("应能求解");
      // 无论怎么连，都必须是有源天线：电阻为正、辐射正常。
      assert!(
        r.impedance.0 > 1.0,
        "倒 V 电阻 {} 应为正 —— 接点方向归一化失效了",
        r.impedance.0
      );
      assert!(r.gain_max_dbi > 0.0, "倒 V 增益 {} dBi", r.gain_max_dbi);
      if let Some(prev) = reference {
        assert!(
          (r.impedance.0 - prev.0).abs() < 1e-6 && (r.impedance.1 - prev.1).abs() < 1e-6,
          "三种写法的阻抗必须一致：{:?} vs {prev:?}",
          r.impedance
        );
      } else {
        reference = Some(r.impedance);
      }
    }
  }

  #[test]
  fn bent_wire_keeps_the_power_identity() {
    // 折角结构上，方向图积分功率仍须等于馈电功率（`Re(Z)|I|²/2`）。
    // 接点方向没归一化时这个比值是 **−3.9**（方向图功率为正、馈电功率为负）。
    let arm = 5.07f64;
    let half = 60f64.to_radians();
    let (sh, ch) = half.sin_cos();
    let input = NecInput {
      wires: vec![
        Wire::new([0.0, 0.0, 0.0], [arm * ch, 0.0, -arm * sh], 0.002, 10),
        Wire::new([0.0, 0.0, 0.0], [-arm * ch, 0.0, -arm * sh], 0.002, 10),
      ],
      feeds: vec![Feed {
        wire: 0,
        at: 0.5,
        volts: 1.0,
      }],
      loads: Vec::new(),
      freq_hz: FREQ,
      ground: Ground::Free,
      conductivity: None,
    };
    let model = Model::from_wires(&input.wires).unwrap();
    let k = 2.0 * PI * FREQ / C0;
    let unknowns: Vec<usize> = (0..model.nodes.len())
      .filter(|&j| !model.is_fixed_end(j, false))
      .collect();
    let mut z = assemble(&model, &unknowns, k, &input).unwrap();
    let (seg_idx, alpha) = locate(&model, 0, 0.5).unwrap();
    let seg = model.segs[seg_idx];
    let mut rhs = vec![Cx::ZERO; unknowns.len()];
    for (node, w) in [(seg.start_node, 1.0 - alpha), (seg.end_node, alpha)] {
      if let Some(row) = unknowns.iter().position(|&j| j == node) {
        rhs[row] += Cx::of(-w);
      }
    }
    let sol = solve_linear(&mut z, &mut rhs).unwrap();
    let mut cur = vec![Cx::ZERO; model.nodes.len()];
    for (i, &j) in unknowns.iter().enumerate() {
      cur[j] = sol[i];
    }
    let node_i = |node: usize| -> Cx {
      unknowns
        .iter()
        .position(|&j| j == node)
        .map_or(Cx::ZERO, |row| sol[row])
    };
    let i_feed = node_i(seg.start_node) * (1.0 - alpha) + node_i(seg.end_node) * alpha;
    let p_in = 0.5 * (Cx::of(1.0) / i_feed).re * i_feed.abs2();
    let (dth, dph) = (1.0f64, 2.0f64);
    let mut integ = 0.0;
    let mut th = 0.0f64;
    while th <= 180.0 + 1e-9 {
      let mut ph = 0.0f64;
      while ph < 360.0 - 1e-9 {
        integ += perp2(&model, &cur, k, Ground::Free, th, ph)
          * th.to_radians().sin()
          * dth.to_radians()
          * dph.to_radians();
        ph += dph;
      }
      th += dth;
    }
    let p_rad = k * k * ETA0 / (32.0 * PI * PI) * integ;
    let ratio = p_rad / p_in;
    assert!(
      (ratio - 1.0).abs() < 5e-3,
      "折角结构的方向图功率/馈电功率 = {ratio:.4}，应约为 1"
    );
  }

  #[test]
  fn one_hop_geometry_matches_the_closed_form() {
    // `d = 2h·cot Δ`：45° 时 d = 2h（精确），30° 时 d = 2√3·h，天顶落点为零。
    let h = 300.0;
    let close = |a: f64, b: f64| assert!((a - b).abs() < 1e-6, "{a} vs {b}");
    close(one_hop_distance_km(45.0, h).unwrap(), 600.0);
    close(one_hop_distance_km(30.0, h).unwrap(), 2.0 * 3f64.sqrt() * h);
    close(one_hop_distance_km(90.0, h).unwrap(), 0.0);
    close(one_hop_distance_km(60.0, h).unwrap(), 2.0 * h / 3f64.sqrt());
    // 仰角越低落点越远（同一层高）。
    let mut prev = -1.0;
    for e in [90.0, 75.0, 60.0, 45.0, 30.0, 15.0] {
      let d = one_hop_distance_km(e, h).unwrap();
      assert!(d > prev, "仰角 {e}° 的落点 {d} 应该比上一个更远");
      prev = d;
    }
    // 反函数来回一致。
    for d in [0.0, 50.0, 346.4, 1039.2, 2000.0] {
      let e = one_hop_elevation_deg(d, h).unwrap();
      let back = one_hop_distance_km(e, h).unwrap();
      assert!((back - d).abs() < 1e-6, "{d} → {e}° → {back}");
    }
    // 非法输入。
    assert_eq!(one_hop_distance_km(0.0, h), None);
    assert_eq!(one_hop_distance_km(90.1, h), None);
    assert_eq!(one_hop_distance_km(45.0, 0.0), None);
    assert_eq!(one_hop_elevation_deg(100.0, -1.0), None);
  }

  #[test]
  fn low_antenna_covers_short_distances_like_nvis() {
    // NVIS 的物理：天线架得低（0.1–0.25λ），主瓣被抬到天顶附近，
    // 于是能量落在 0–500 km 这一段，而不是几百公里以外。
    let lam = wavelength(FREQ);
    let flat = |h: f64| NecInput {
      wires: vec![Wire::new(
        [-0.25 * lam, 0.0, h * lam],
        [0.25 * lam, 0.0, h * lam],
        THIN * lam,
        20,
      )],
      feeds: vec![Feed {
        wire: 0,
        at: 0.5,
        volts: 1.0,
      }],
      loads: Vec::new(),
      freq_hz: FREQ,
      ground: Ground::Perfect,
      conductivity: None,
    };
    let low = solve(&flat(0.1)).expect("应能求解");
    let high = solve(&flat(0.5)).expect("应能求解");

    // 低架：主瓣在天顶附近。
    assert!(
      low.gain_max_elevation_deg > 60.0,
      "0.1λ 架高的主瓣仰角 {}° 应接近天顶",
      low.gain_max_elevation_deg
    );
    // 高架：主瓣压到 30° 附近。
    assert!(
      (20.0..45.0).contains(&high.gain_max_elevation_deg),
      "0.5λ 架高的主瓣仰角 {}° 应在 20–45°",
      high.gain_max_elevation_deg
    );

    let layer = 300.0;
    let a = one_hop_coverage(&low, layer, 3000.0);
    let b = one_hop_coverage(&high, layer, 3000.0);
    let pa = coverage_peak(&a).expect("低架应有覆盖点");
    let pb = coverage_peak(&b).expect("高架应有覆盖点");
    assert!(
      pa.distance_km < 350.0,
      "低架天线的覆盖峰值应在 350 km 以内（NVIS），实际 {} km",
      pa.distance_km
    );
    assert!(
      pb.distance_km > 700.0,
      "高架天线的覆盖峰值应在 700 km 以外，实际 {} km",
      pb.distance_km
    );
    assert!(pb.distance_km > pa.distance_km);

    // 覆盖点按距离升序，且增益不超过天线最大增益。
    for w in a.windows(2) {
      assert!(w[1].distance_km >= w[0].distance_km);
    }
    for p in a.iter().chain(b.iter()) {
      assert!(
        p.gain_dbi <= low.gain_max_dbi.max(high.gain_max_dbi) + 1e-9,
        "覆盖点的增益 {} 不该超过最大增益",
        p.gain_dbi
      );
      assert!(p.elevation_deg > 0.0 && p.elevation_deg <= 90.0);
    }
  }

  #[test]
  fn coverage_respects_the_distance_window_and_rejects_bad_layers() {
    let lam = wavelength(FREQ);
    let input = NecInput {
      wires: vec![Wire::new(
        [-0.25 * lam, 0.0, 0.1 * lam],
        [0.25 * lam, 0.0, 0.1 * lam],
        THIN * lam,
        20,
      )],
      feeds: vec![Feed {
        wire: 0,
        at: 0.5,
        volts: 1.0,
      }],
      loads: Vec::new(),
      freq_hz: FREQ,
      ground: Ground::Perfect,
      conductivity: None,
    };
    let r = solve(&input).expect("应能求解");
    let wide = one_hop_coverage(&r, 300.0, 3000.0);
    let narrow = one_hop_coverage(&r, 300.0, 400.0);
    assert!(narrow.len() < wide.len());
    assert!(narrow.iter().all(|p| p.distance_km <= 400.0));
    assert!(wide.iter().any(|p| p.distance_km > 400.0));
    // 层高越大，同一仰角的落点越远。
    let far = one_hop_coverage(&r, 600.0, 3000.0);
    let ea = coverage_peak(&wide).unwrap();
    let eb = coverage_peak(&far).unwrap();
    assert!(eb.distance_km > ea.distance_km);
    // 非法参数给空曲线。
    assert!(one_hop_coverage(&r, 0.0, 3000.0).is_empty());
    assert!(one_hop_coverage(&r, 300.0, 0.0).is_empty());
    assert!(coverage_peak(&[]).is_none());
  }

  #[test]
  fn wavelength_helpers() {
    assert!((wavelength(FREQ) - 21.261_876_453_9).abs() < 1e-6);
    assert_eq!(wavelength(0.0), 0.0);
    let w = Wire::new([0.0; 3], [3.0, 4.0, 0.0], 0.001, 2);
    assert!((w.length() - 5.0).abs() < 1e-12);
  }
}
