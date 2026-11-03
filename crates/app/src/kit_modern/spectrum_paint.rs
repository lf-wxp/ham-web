//! 频谱的绘制层：只按一帧模型数据作画，不持有状态。
//!
//! 层次从下到上：底噪柱 → 波形折线 → 峰值保持刻度 → 扫描线。
//! 每帧先用 `destination-out` 把上一帧擦淡一档 —— 画布是透明的，不能像普通画布那样
//! 铺一层半透明底色当拖尾，否则整条带子会变成一块灰底；擦出来的余辉正好是荧光屏的读法。

use ham_web_core::spectrum_scope::ScopeModel;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};

/// 柱色阶梯的档数：颜色只在换主题时预算一次，逐帧只做索引，不再拼 CSS 串。
const LADDER_STEPS: usize = 24;
/// 每帧擦掉的强度：越大余辉越短。0.42 约等于 4 帧（≈0.13 s）的拖尾。
const PERSISTENCE: f32 = 0.42;
/// 柱子占满幅高度的比例：顶上留一条缝，峰值刻度才不会被裁掉。
const SPAN: f64 = 0.92;
/// 显示增益：模型留了约 40% 的顶空（真机也不会一直满格），显示时抬一档再截顶。
/// 抬的是「整条」而不是加底噪，所以强信号撑得满、底噪仍贴着基线。
const DISPLAY_GAIN: f32 = 1.35;

/// 模型幅度 → 显示幅度。
fn display(v: f32) -> f32 {
  (v * DISPLAY_GAIN).min(1.0)
}

/// 一个颜色（0–255 分量）。Canvas 的填充只收 CSS 串，所以保留浮点分量自己拼。
#[derive(Clone, Copy)]
struct Rgb {
  r: f32,
  g: f32,
  b: f32,
}

impl Rgb {
  const fn new(r: f32, g: f32, b: f32) -> Self {
    Self { r, g, b }
  }

  fn css(self, alpha: f32) -> String {
    let (r, g, b) = (self.r.round(), self.g.round(), self.b.round());
    format!("rgba({r},{g},{b},{alpha})")
  }

  fn mix(self, other: Self, t: f32) -> Self {
    Self {
      r: self.r + (other.r - self.r) * t,
      g: self.g + (other.g - self.g) * t,
      b: self.b + (other.b - self.b) * t,
    }
  }

  /// 解析 `#rrggbb` / `#rgb`（规则与单测在 `ham_web_core::color::Rgb::parse_css`）。
  ///
  /// CSS 自定义属性取出来是**未展开的声明值**：`--md-rule` 这类 `color-mix(...)` 解析不了，
  /// 由调用点的兜底色接住；这里只认字面色值。
  fn from_hex(raw: &str) -> Option<Self> {
    let c = ham_web_core::color::Rgb::parse_css(raw)?;
    Some(Self::new(f32::from(c.0), f32::from(c.1), f32::from(c.2)))
  }
}

/// 一屏用到的颜色。**全部预算成字符串**：`Canvas` 的填充只收 CSS 串，逐帧 `format!`
/// 等于每帧十几次堆分配，而透明度都是固定的。
pub struct Palette {
  /// 余辉擦除用的填充。由 [`PERSISTENCE`] 在建调色板时算一次（`read` 每 90 帧才跑一回，
  /// 这点格式化开销可以忽略，好处是透明度只有一个出处）。
  fade: String,
  grid_soft: String,
  grid_strong: String,
  trace: String,
  peak_mark: String,
  /// 扫描线尾焰：从远到近 4 段。
  scan_tail: [String; 4],
  scan_head: String,
  /// 预乘好的柱色阶梯（弱信号 → 强信号）。
  ladder: Vec<String>,
}

impl Palette {
  /// 从画布所在的 DOM 上下文取色：切的明暗、换的配色都在 CSS 变量里，组件不必自己判主题。
  pub fn read(canvas: &HtmlCanvasElement) -> Self {
    let style = web_sys::window().and_then(|w| w.get_computed_style(canvas).ok().flatten());
    let var = |name: &str, fallback: Rgb| {
      style
        .as_ref()
        .and_then(|s| s.get_property_value(name).ok())
        .and_then(|v| Rgb::from_hex(v.trim()))
        .unwrap_or(fallback)
    };

    let quiet = var("--md-blue", Rgb::new(37.0, 64.0, 255.0));
    let hot = var("--md-orange", Rgb::new(255.0, 77.0, 31.0));
    let peak = var("--md-violet", Rgb::new(169.0, 123.0, 255.0));
    let grid = var("--md-border-strong", Rgb::new(195.0, 193.0, 182.0));

    let ladder = (0..LADDER_STEPS)
      .map(|i| {
        let t = i as f32 / (LADDER_STEPS - 1) as f32;
        // 平方一下：底噪那一段留在蓝里，只有真正抬起来的峰才转橙，强弱一眼分得开。
        quiet.mix(hot, t * t).css(0.45 + 0.5 * t)
      })
      .collect();

    Self {
      fade: format!("rgba(0,0,0,{PERSISTENCE})"),
      grid_soft: grid.css(0.16),
      grid_strong: grid.css(0.32),
      trace: hot.css(0.45),
      peak_mark: peak.css(0.7),
      // 与 `sweep` 的 4 段尾焰一一对应：越靠后越淡。
      scan_tail: std::array::from_fn(|k| quiet.css(0.03 * (4 - k) as f32)),
      scan_head: hot.css(0.7),
      ladder,
    }
  }
}

/// 画一帧。
pub fn paint(
  ctx: &CanvasRenderingContext2d,
  model: &ScopeModel,
  palette: &Palette,
  width: f64,
  height: f64,
) {
  fade(ctx, palette, width, height);
  grid(ctx, palette, width, height);
  bars(ctx, model, palette, width, height);
  trace(ctx, model, palette, width, height);
  peaks(ctx, model, palette, width, height);
  sweep(ctx, model, palette, width, height);
}

/// 余辉：整幅擦淡一档，上一帧的峰会留成一截渐隐的尾巴。
fn fade(ctx: &CanvasRenderingContext2d, palette: &Palette, width: f64, height: f64) {
  let _ = ctx.set_global_composite_operation("destination-out");
  ctx.set_fill_style_str(&palette.fade);
  ctx.fill_rect(0.0, 0.0, width, height);
  let _ = ctx.set_global_composite_operation("source-over");
}

/// 栅格：三条水平参考线 + 底边 12 等分刻度。不写数字 —— 64px 高的带子上写字只会糊。
fn grid(ctx: &CanvasRenderingContext2d, palette: &Palette, width: f64, height: f64) {
  ctx.set_fill_style_str(&palette.grid_soft);
  for i in 1..4 {
    let y = (height * f64::from(i) / 4.0).round();
    ctx.fill_rect(0.0, y, width, 1.0);
  }

  ctx.set_fill_style_str(&palette.grid_strong);
  let step = width / 12.0;
  for i in 0..=12 {
    let x = (f64::from(i) * step).min(width - 1.0).round();
    ctx.fill_rect(x, height - 3.0, 1.0, 3.0);
  }
}

/// 柱状：每个频点一根，颜色按幅度取阶梯。
fn bars(
  ctx: &CanvasRenderingContext2d,
  model: &ScopeModel,
  palette: &Palette,
  width: f64,
  height: f64,
) {
  let bins = model.bins();
  let n = bins.len();
  if n == 0 {
    return;
  }
  let pitch = width / n as f64;
  let gap = (pitch * 0.3).clamp(0.6, 2.4);
  let bar = (pitch - gap).max(0.8);
  let last = palette.ladder.len() - 1;

  for (i, &raw) in bins.iter().enumerate() {
    let v = display(raw);
    let h = f64::from(v) * height * SPAN;
    // 低于半像素的碎屑不画：省下的 `fill_rect` 是静默窗口里的大头。
    if h < 0.75 {
      continue;
    }
    let idx = (v * last as f32).round() as usize;
    ctx.set_fill_style_str(&palette.ladder[idx.min(last)]);
    ctx.fill_rect(i as f64 * pitch + gap * 0.5, height - h, bar, h);
  }
}

/// 波形折线：压在柱顶，把「一根根柱」连成一条可读的信号轮廓。
fn trace(
  ctx: &CanvasRenderingContext2d,
  model: &ScopeModel,
  palette: &Palette,
  width: f64,
  height: f64,
) {
  let bins = model.bins();
  let n = bins.len();
  if n < 2 {
    return;
  }
  let pitch = width / n as f64;
  ctx.set_stroke_style_str(&palette.trace);
  ctx.set_line_width(1.2);
  ctx.begin_path();
  for (i, &raw) in bins.iter().enumerate() {
    let x = i as f64 * pitch + pitch * 0.5;
    let y = height - f64::from(display(raw)) * height * SPAN;
    if i == 0 {
      ctx.move_to(x, y);
    } else {
      ctx.line_to(x, y);
    }
  }
  ctx.stroke();
}

/// 峰值保持：柱子顶上那排慢回落的小刻度，看出「刚刚到过多高」。
fn peaks(
  ctx: &CanvasRenderingContext2d,
  model: &ScopeModel,
  palette: &Palette,
  width: f64,
  height: f64,
) {
  let bins = model.bins();
  let n = bins.len();
  if n == 0 {
    return;
  }
  let pitch = width / n as f64;
  let gap = (pitch * 0.3).clamp(0.6, 2.4);
  let bar = (pitch - gap).max(0.8);

  ctx.set_fill_style_str(&palette.peak_mark);
  for (i, (&v, &h)) in bins.iter().zip(model.hold()).enumerate() {
    if h <= v + 0.02 {
      continue;
    }
    let y = (height - f64::from(display(h)) * height * SPAN - 2.0).max(0.0);
    ctx.fill_rect(i as f64 * pitch + gap * 0.5, y, bar, 1.5);
  }
}

/// 扫描线：一条竖线 + 向后拖的渐隐尾焰，读出「正在扫频」的方向感。
fn sweep(
  ctx: &CanvasRenderingContext2d,
  model: &ScopeModel,
  palette: &Palette,
  width: f64,
  height: f64,
) {
  let x = f64::from(model.scan()) * width;

  // 没开 `CanvasGradient` feature，用几条透明度递减的竖条叠出渐变 —— 4 次 `fill_rect`
  // 比每帧新建一个渐变对象更省。
  for (k, fill) in palette.scan_tail.iter().enumerate() {
    let band = 5.0 + k as f64 * 5.0;
    ctx.set_fill_style_str(fill);
    ctx.fill_rect(x - band, 0.0, band, height);
  }

  // 指针用行动色（橙）而不是同一条蓝：它要和柱子区分开，才读得出「现在扫到哪」。
  ctx.set_fill_style_str(&palette.scan_head);
  ctx.fill_rect(x, 0.0, 1.5, height);
}
