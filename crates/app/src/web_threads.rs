//! Web Threads 动态背景（WebGL2 实现）。
//!
//! 与 React Bits 的 `WebThreads` 使用**同一套技术方案**：
//!
//! - **WebGL2 + 全屏三角形**：一次 `drawArrays` 覆盖整个视口，没有几何体与接缝开销；
//! - **GLSL ES 3.00 片元着色器**：逐像素叠加 `MAX_THREADS`（10）条正弦丝线，由 `glow()`
//!   的幂次衰减生成柔光；`uLightMode` 分支负责浅色主题下的「墨线」映射；
//! - **每帧只改 uniform**：参数 / 主题变化不重建 WebGL 上下文（见 [`WebThreadsRenderer::apply`]）；
//! - **性能与省电**（移动端尤其重要，全屏片元着色器是持续的 GPU 填充率开销）：
//!   [`RenderMode::Static`]（移动端默认）只渲染一帧当背景图，GPU 占用归零；
//!   帧率上限 `MAX_FPS`（30fps：背景是缓慢的氛围动画，观感几乎无差，GPU 绘制次数减半）；
//!   DPR 上限 2、最长边超过 [`MAX_RENDER_DIM`] 时等比降采样，紧凑设备（手机 / 竖屏平板）
//!   进一步降到 1.5 与 1280；`powerPreference: "low-power"` 提示优先使用低功耗 GPU；
//!   `IntersectionObserver` 离屏暂停、`visibilitychange` 切标签页暂停；
//!   卸载时 `WEBGL_lose_context` 主动释放显存，断开 rAF 自引用避免 `Rc` 环泄漏；
//! - **无障碍**：开启「减少动态效果」时只渲染一帧静态画面，不启动 `requestAnimationFrame`。
//!
//! 组件接入见 [`crate::components::web_threads`]；本模块只依赖 web-sys，不依赖 Leptos。

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use wasm_bindgen::closure::Closure;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{
  Element, HtmlCanvasElement, IntersectionObserver, IntersectionObserverEntry, ResizeObserver,
  WebGl2RenderingContext, WebGlBuffer, WebGlProgram, WebGlShader, WebGlUniformLocation,
};

use crate::util::{document, window};

/// 渲染分辨率上限（逻辑像素 × DPR 后的最长边），超过则等比降采样。
///
/// 与 React Bits 的 `Threads` 一致：效果本身柔和，降采样的损失肉眼几乎不可见，
/// 却能把片元着色器的填充率开销锁在固定预算内。
pub const MAX_RENDER_DIM: f64 = 1920.0;

/// 帧率上限。背景是缓慢的氛围动画（`speed` 默认 0.2），30fps 与 60fps 观感几乎无差，
/// 但每秒的 GPU 绘制与 uniform 上传直接减半 —— 移动端省电最有效的一档。
///
/// 注：`requestAnimationFrame` 本身仍按屏幕刷新率回调，这里是在回调里跳过绘制；
/// 真正省下的是片元着色器的填充率（全屏 shader 的耗电大头）。
const MAX_FPS: f64 = 30.0;

/// 帧间隔容差（ms）。刷新率是 16.67ms 的整数倍：60Hz 下 `2 × 16.67 = 33.34`
/// 与 `1000 / 30 = 33.33` 只差 0.01ms，抖动会让本该绘制的那帧被判为「未到点」
/// 而白白丢帧（实测会从 30fps 掉到 ~25fps）。留 2ms 容差即可稳定落在每 2 帧 1 次。
const FRAME_TOLERANCE_MS: f64 = 2.0;

/// 「紧凑设备」判定阈值（视口最小边，CSS px）：命中即手机 / 竖屏平板。
const COMPACT_VIEWPORT: f64 = 820.0;

/// 紧凑设备的 DPR 上限：手机像素密度普遍 2~3，按桌面档渲染纯属浪费电。
const COMPACT_DPR_CAP: f64 = 1.5;

/// 紧凑设备的渲染分辨率上限（逻辑像素 × DPR 后的最长边）。
///
/// 屏幕物理尺寸小，降采样后肉眼几乎看不出；对省电却是实打实的一笔。
const COMPACT_MAX_RENDER_DIM: f64 = 1280.0;

/// 鼠标跟随的指数平滑系数（与上游一致），产生轻微的跟手延迟。
const MOUSE_SMOOTHING: f32 = 0.05;

/// 全屏三角形顶点：一个覆盖整个裁剪空间的超大三角形。
const TRIANGLE: [f32; 6] = [-1.0, -1.0, 3.0, -1.0, -1.0, 3.0];

/// 顶点着色器：直通，把全屏三角形铺满裁剪空间。
const VERTEX_SHADER: &str = r#"#version 300 es
in vec2 position;
void main() {
  gl_Position = vec4(position, 0.0, 1.0);
}
"#;

/// 片元着色器（与 React Bits WebThreads 逐字一致）。
///
/// 逐像素叠加 `MAX_THREADS` 条发光丝线：第 `i` 条线的振幅随 `uSpread` 与 `uTaper` 递增、
/// 相位按 `TAU / n` 错开，`glow()` 用幂次衰减把「到线的距离」变成柔光。浅色主题走
/// `uLightMode` 分支：把发光能量重新映射成覆盖度与颜料色，输出不透明画面。
const FRAGMENT_SHADER: &str = r#"#version 300 es
precision highp float;
uniform vec2 iResolution;
uniform float iTime;
uniform float uSpeed;
uniform float uThreadCount;
uniform float uFrequency;
uniform float uSpread;
uniform float uTaper;
uniform float uPosition;
uniform float uFanMode;
uniform float uGlow;
uniform float uFalloff;
uniform float uThickness;
uniform float uBrightness;
uniform float uOpacity;
uniform float uMirror;
uniform float uShimmer;
uniform float uGrain;
uniform float uGrainIntensity;
uniform vec3 uColor1;
uniform vec3 uColor2;
uniform vec3 uColor3;
uniform vec3 uBackgroundColor;
uniform bool uLightMode;
uniform vec2 uMouse;
uniform float uMouseStrength;
uniform float uEnableMouse;
uniform float uMouseActive;
out vec4 fragColor;

#define TAU 6.28318530718
#define MAX_THREADS 10

float glow(float x, float str, float dist) {
  return dist / pow(max(x, 1e-4), str);
}

void main() {
  vec2 uv = gl_FragCoord.xy / iResolution.xy;
  float n = max(uThreadCount, 1.0);

  float pinchX = uFanMode < 0.5 ? 0.5 : (uFanMode < 1.5 ? 0.0 : 1.0);
  if (uEnableMouse > 0.5) {
    pinchX = mix(pinchX, uMouse.x, clamp(uMouseStrength, 0.0, 1.0) * uMouseActive);
  }

  float spreadDx = uSpread * abs(uv.x - pinchX);
  float baseT = iTime * uSpeed;
  float tauOverN = TAU / n;
  float mirror = uMirror > 0.5 ? sign(pinchX - uv.x) : 1.0;
  bool doShimmer = uShimmer > 0.5;
  float shimmerT = iTime * 1.7;
  float invThickness = 1.0 / max(uThickness, 0.01);
  float xFreq = uv.x * uFrequency;
  float yOff = uv.y - uPosition;
  float ciScale = n > 1.0 ? 1.0 / (n - 1.0) : 0.0;

  vec3 col = vec3(0.0);
  float gsum = 0.0;

  for (int idx = 0; idx < MAX_THREADS; idx++) {
    float i = float(idx);
    if (i >= n) break;

    float amplitude = spreadDx * (1.0 + i * uTaper);
    float shimmer = doShimmer ? sin(shimmerT + i * 1.3) * 0.35 : 0.0;
    float phase = (baseT + i * tauOverN) * mirror + shimmer;

    float sdf = abs(yOff + sin(xFreq + phase) * amplitude) * invThickness;

    float g = glow(sdf, uFalloff, uGlow);
    float ci = i * ciScale;
    vec3 threadCol = mix(uColor1, uColor2, ci);

    col += g * threadCol;
    gsum += g;
  }

  float coreAmt = smoothstep(0.5, 2.2, gsum);
  col = mix(col, uColor3 * gsum, coreAmt * 0.5);

  float bright = uBrightness;
  if (uEnableMouse > 0.5) {
    vec2 md = uv - uMouse;
    float d2 = dot(md, md);
    bright += clamp(uMouseStrength, 0.0, 1.0) * uMouseActive * exp(-d2 * 6.0) * 0.6;
  }
  col *= bright;

  float alpha = clamp(gsum, 0.0, 1.0) * uOpacity;

  vec3 outRgb = col * alpha;

  if (uGrain > 0.5) {
    float gv = (fract(sin(dot(gl_FragCoord.xy, vec2(12.9898, 78.233)) + iTime) * 43758.5453) - 0.5) * uGrainIntensity;
    outRgb = clamp(outRgb + gv, 0.0, 1.0);
    alpha = clamp(alpha + gv, 0.0, 1.0);
  }

  if (uLightMode) {
    vec3 mapped = vec3(1.0) - exp(-max(col, vec3(0.0)) * 1.3);
    float rawEnergy = clamp(max(mapped.r, max(mapped.g, mapped.b)) * uOpacity, 0.0, 1.0);
    float coverage = smoothstep(0.18, 0.72, rawEnergy);
    coverage *= coverage;
    vec3 hue = mapped / max(max(mapped.r, max(mapped.g, mapped.b)), 1e-4);
    vec3 chroma = pow(clamp(hue, 0.0, 1.0), vec3(0.78));
    vec3 pigment = mix(chroma, vec3(0.08), 0.12);
    vec3 ink = mix(vec3(0.9), pigment, 0.82 + coverage * 0.18);
    fragColor = vec4(mix(uBackgroundColor, ink, coverage), 1.0);
  } else {
    fragColor = vec4(outRgb, alpha);
  }
}
"#;

/// 外观参数，字段与 React Bits `WebThreads` 的 props 一一对应。
///
/// [`Default`] 即上游默认值；颜色为 sRGB 归一化分量（`0.0..=1.0`），
/// 由 [`hex_to_rgb`] 从十六进制色值转换。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ThreadsConfig {
  /// 第 1 条丝线的颜色。
  pub color1: (f32, f32, f32),
  /// 最后一条丝线的颜色（中间丝线由两者混合）。
  pub color2: (f32, f32, f32),
  /// 丝线交汇处的核心高光色。
  pub color3: (f32, f32, f32),
  /// 浅色模式下的画布底色，应与页面背景一致。
  pub background: (f32, f32, f32),
  /// 动画速度。
  pub speed: f32,
  /// 丝线条数（着色器上限 10）。
  pub thread_count: f32,
  /// 横向正弦频率。
  pub frequency: f32,
  /// 丝线横向散布幅度。
  pub spread: f32,
  /// 逐条线的振幅递增量。
  pub taper: f32,
  /// 纵向基准位置（`0.0..=1.0`）。
  pub position: f32,
  /// 收束点对齐：`0.0` 居中、`1.0` 靠左、`2.0` 靠右（对应上游 `fanMode`）。
  pub fan_mode: f32,
  /// 柔光强度。
  pub glow: f32,
  /// 柔光衰减指数。
  pub falloff: f32,
  /// 丝线粗细系数。
  pub thickness: f32,
  /// 亮度。
  pub brightness: f32,
  /// 不透明度（浅色模式下参与覆盖度计算）。
  pub opacity: f32,
  /// 是否让收束点左右镜像。
  pub mirror: bool,
  /// 是否开启流动微闪。
  pub shimmer: bool,
  /// 是否叠加胶片颗粒。
  pub grain: bool,
  /// 颗粒强度。
  pub grain_intensity: f32,
  /// 是否允许鼠标扰动。
  pub mouse_interaction: bool,
  /// 鼠标扰动强度。
  pub mouse_strength: f32,
  /// 浅色模式：把发光重新映射为不透明的「墨线」画面。
  pub light_mode: bool,
}

impl Default for ThreadsConfig {
  fn default() -> Self {
    Self {
      color1: hex_to_rgb("#5227FF"),
      color2: hex_to_rgb("#FF9FFC"),
      color3: hex_to_rgb("#FFFFFF"),
      background: hex_to_rgb("#FFFFFF"),
      speed: 0.2,
      thread_count: 6.0,
      frequency: 5.0,
      spread: 0.18,
      taper: 1.0,
      position: 0.5,
      fan_mode: 0.0,
      glow: 0.02,
      falloff: 0.6,
      thickness: 1.1,
      brightness: 0.6,
      opacity: 1.0,
      mirror: true,
      shimmer: false,
      grain: true,
      grain_intensity: 0.05,
      mouse_interaction: true,
      mouse_strength: 0.3,
      light_mode: false,
    }
  }
}

/// 把 `#RRGGBB` 十六进制色值转成 sRGB 归一化分量；解析失败时回退为白色。
///
/// 长度不是 6、或含非十六进制字符（`from_str_radix` 解析失败）时，一律回退为白色。
#[must_use]
pub fn hex_to_rgb(hex: &str) -> (f32, f32, f32) {
  let digits = hex.trim().trim_start_matches('#');
  if digits.len() != 6 {
    return (1.0, 1.0, 1.0);
  }
  let channel = |range: std::ops::Range<usize>| {
    u8::from_str_radix(digits.get(range)?, 16)
      .ok()
      .map(|v| f32::from(v) / 255.0)
  };
  match (channel(0..2), channel(2..4), channel(4..6)) {
    (Some(r), Some(g), Some(b)) => (r, g, b),
    _ => (1.0, 1.0, 1.0),
  }
}

/// `bool` → GLSL `float` 开关（`true` → `1.0`，`false` → `0.0`）。
fn bool_f32(value: bool) -> f32 {
  if value { 1.0 } else { 0.0 }
}

/// 着色器 uniform 位置缓存：只在初始化时查一次，之后每帧直接用。
struct Uniforms {
  i_resolution: Option<WebGlUniformLocation>,
  i_time: Option<WebGlUniformLocation>,
  speed: Option<WebGlUniformLocation>,
  thread_count: Option<WebGlUniformLocation>,
  frequency: Option<WebGlUniformLocation>,
  spread: Option<WebGlUniformLocation>,
  taper: Option<WebGlUniformLocation>,
  position: Option<WebGlUniformLocation>,
  fan_mode: Option<WebGlUniformLocation>,
  glow: Option<WebGlUniformLocation>,
  falloff: Option<WebGlUniformLocation>,
  thickness: Option<WebGlUniformLocation>,
  brightness: Option<WebGlUniformLocation>,
  opacity: Option<WebGlUniformLocation>,
  mirror: Option<WebGlUniformLocation>,
  shimmer: Option<WebGlUniformLocation>,
  grain: Option<WebGlUniformLocation>,
  grain_intensity: Option<WebGlUniformLocation>,
  color1: Option<WebGlUniformLocation>,
  color2: Option<WebGlUniformLocation>,
  color3: Option<WebGlUniformLocation>,
  background: Option<WebGlUniformLocation>,
  light_mode: Option<WebGlUniformLocation>,
  mouse: Option<WebGlUniformLocation>,
  mouse_strength: Option<WebGlUniformLocation>,
  enable_mouse: Option<WebGlUniformLocation>,
  mouse_active: Option<WebGlUniformLocation>,
}

impl Uniforms {
  fn new(gl: &WebGl2RenderingContext, program: &WebGlProgram) -> Self {
    let at = |name: &str| gl.get_uniform_location(program, name);
    Self {
      i_resolution: at("iResolution"),
      i_time: at("iTime"),
      speed: at("uSpeed"),
      thread_count: at("uThreadCount"),
      frequency: at("uFrequency"),
      spread: at("uSpread"),
      taper: at("uTaper"),
      position: at("uPosition"),
      fan_mode: at("uFanMode"),
      glow: at("uGlow"),
      falloff: at("uFalloff"),
      thickness: at("uThickness"),
      brightness: at("uBrightness"),
      opacity: at("uOpacity"),
      mirror: at("uMirror"),
      shimmer: at("uShimmer"),
      grain: at("uGrain"),
      grain_intensity: at("uGrainIntensity"),
      color1: at("uColor1"),
      color2: at("uColor2"),
      color3: at("uColor3"),
      background: at("uBackgroundColor"),
      light_mode: at("uLightMode"),
      mouse: at("uMouse"),
      mouse_strength: at("uMouseStrength"),
      enable_mouse: at("uEnableMouse"),
      mouse_active: at("uMouseActive"),
    }
  }

  /// 把配置写入静态 uniform（颜色、几何、开关等）。
  fn apply(&self, gl: &WebGl2RenderingContext, config: &ThreadsConfig) {
    let (r, g, b) = config.color1;
    gl.uniform3f(self.color1.as_ref(), r, g, b);
    let (r, g, b) = config.color2;
    gl.uniform3f(self.color2.as_ref(), r, g, b);
    let (r, g, b) = config.color3;
    gl.uniform3f(self.color3.as_ref(), r, g, b);
    let (r, g, b) = config.background;
    gl.uniform3f(self.background.as_ref(), r, g, b);

    gl.uniform1f(self.speed.as_ref(), config.speed);
    gl.uniform1f(self.thread_count.as_ref(), config.thread_count.round());
    gl.uniform1f(self.frequency.as_ref(), config.frequency);
    gl.uniform1f(self.spread.as_ref(), config.spread);
    gl.uniform1f(self.taper.as_ref(), config.taper);
    gl.uniform1f(self.position.as_ref(), config.position);
    gl.uniform1f(self.fan_mode.as_ref(), config.fan_mode);
    gl.uniform1f(self.glow.as_ref(), config.glow);
    gl.uniform1f(self.falloff.as_ref(), config.falloff);
    gl.uniform1f(self.thickness.as_ref(), config.thickness);
    gl.uniform1f(self.brightness.as_ref(), config.brightness);
    gl.uniform1f(self.opacity.as_ref(), config.opacity);
    // 注意：着色器里除 `uLightMode`（`bool`）外，所有开关都是 `float`，
    // 必须用 `uniform1f` 写入，否则触发 `GL_INVALID_OPERATION`（与上游 OGL 一致）。
    gl.uniform1f(self.mirror.as_ref(), bool_f32(config.mirror));
    gl.uniform1f(self.shimmer.as_ref(), bool_f32(config.shimmer));
    gl.uniform1f(self.grain.as_ref(), bool_f32(config.grain));
    gl.uniform1f(self.grain_intensity.as_ref(), config.grain_intensity);
    gl.uniform1i(self.light_mode.as_ref(), i32::from(config.light_mode));
    gl.uniform1f(self.mouse_strength.as_ref(), config.mouse_strength);
    gl.uniform1f(
      self.enable_mouse.as_ref(),
      bool_f32(config.mouse_interaction),
    );
  }
}

/// rAF 自引用闭包：持有一份指向自身的闭包，用于在回调内继续排帧。
type Tick = Rc<RefCell<Option<Closure<dyn FnMut(f64)>>>>;

/// 观察器回调（`ResizeObserver` / `IntersectionObserver` 都只读第一个入参）。
type EntriesCallback = Closure<dyn FnMut(js_sys::Array)>;

/// 尺寸观察：优先 `ResizeObserver`，不支持时回退到 `window.resize`。
///
/// 回调必须与观察器同生命周期存活，否则 JS 侧的函数引用会悬垂，故显式持有。
enum SizeWatcher {
  Observer {
    observer: ResizeObserver,
    _callback: EntriesCallback,
  },
  Window {
    _callback: Closure<dyn FnMut()>,
  },
}

/// 视口可见性观察：观察器与回调一并持有（理由同 [`SizeWatcher`]）。
struct IntersectionWatcher {
  observer: IntersectionObserver,
  _callback: EntriesCallback,
}

/// 渲染循环的共享状态：闭包与观察器都持有它的 `Rc`。
struct Shared {
  gl: WebGl2RenderingContext,
  /// 程序对象只在初始化时 `use_program`，此处仅作保活（被 GC 会导致 uniform 失效）。
  _program: WebGlProgram,
  /// 顶点缓冲保活：`vertexAttribPointer` 之后仍需被 JS 引用，否则可能被回收。
  _buffer: WebGlBuffer,
  uniforms: Uniforms,
  /// 当前排队的 rAF 句柄，`0` 表示未排队。
  raf: Cell<i32>,
  /// 容器是否在视口内。
  visible: Cell<bool>,
  /// 页面是否可见（未被切到后台标签页）。
  page_visible: Cell<bool>,
  /// 是否允许动画（用户未开启「减少动态效果」）。
  animating: Cell<bool>,
  /// 当前 / 目标鼠标位置（GL 坐标，左下为原点）。
  current_mouse: Cell<(f32, f32)>,
  target_mouse: Cell<(f32, f32)>,
  /// 当前 / 目标鼠标「活跃度」（`0..=1`，离开后平滑归零）。
  current_active: Cell<f32>,
  target_active: Cell<f32>,
  /// 首帧时间戳（ms），`0` 表示尚未启动。
  started_at: Cell<f64>,
  /// 上一帧的 rAF 时间戳（ms），用于把绘制节流到 [`MAX_FPS`]。
  last_frame: Cell<f64>,
}

impl Shared {
  /// 排一帧；不满足条件（已排队 / 不可见 / 已暂停）时不做任何事。
  fn ensure_running(self: &Rc<Self>, tick: &Tick) {
    if self.raf.get() != 0
      || !self.visible.get()
      || !self.page_visible.get()
      || !self.animating.get()
    {
      return;
    }
    if self.started_at.get() <= 0.0 {
      self.started_at.set(js_sys::Date::now());
    }
    if let Some(callback) = tick.borrow().as_ref()
      && let Ok(id) = window().request_animation_frame(callback.as_ref().unchecked_ref())
    {
      self.raf.set(id);
    }
  }

  /// 取消已排队的帧。
  fn stop(&self) {
    let id = self.raf.replace(0);
    if id != 0 {
      let _ = window().cancel_animation_frame(id);
    }
  }

  /// 按容器尺寸重设画布缓冲区、视口与 `iResolution`，并立即渲染一帧。
  fn resize(&self, canvas: &HtmlCanvasElement) {
    let Some(parent) = canvas.parent_element() else {
      return;
    };
    let rect = parent.get_bounding_client_rect();
    let width = rect.width().max(1.0);
    let height = rect.height().max(1.0);
    let dpr = device_pixel_ratio(width, height);
    let buffer_w = (width * dpr).round().max(1.0);
    let buffer_h = (height * dpr).round().max(1.0);
    canvas.set_width(buffer_w as u32);
    canvas.set_height(buffer_h as u32);
    self.gl.viewport(0, 0, buffer_w as i32, buffer_h as i32);
    self.gl.uniform2f(
      self.uniforms.i_resolution.as_ref(),
      buffer_w as f32,
      buffer_h as f32,
    );
    self.render();
  }

  /// 渲染一帧：推进鼠标平滑、更新动态 uniform、绘制全屏三角形。
  fn render(&self) {
    let started = self.started_at.get();
    let time = if started > 0.0 {
      (js_sys::Date::now() - started) * 0.001
    } else {
      0.0
    };

    let (mut mx, mut my) = self.current_mouse.get();
    let (tx, ty) = self.target_mouse.get();
    mx += MOUSE_SMOOTHING * (tx - mx);
    my += MOUSE_SMOOTHING * (ty - my);
    self.current_mouse.set((mx, my));

    let mut active = self.current_active.get();
    active += MOUSE_SMOOTHING * (self.target_active.get() - active);
    self.current_active.set(active);

    let gl = &self.gl;
    let uniforms = &self.uniforms;
    gl.uniform1f(uniforms.i_time.as_ref(), time as f32);
    gl.uniform2f(uniforms.mouse.as_ref(), mx, my);
    gl.uniform1f(uniforms.mouse_active.as_ref(), active);
    gl.draw_arrays(WebGl2RenderingContext::TRIANGLES, 0, 3);
  }
}

/// 渲染模式。
///
/// 背景层不必总是动：移动端整机功耗预算紧、散热差，静态一帧既保留了视觉效果，
/// 又把 GPU 占用降到零，所以 [`crate::components::web_threads_background::WebThreadsBackground`]
/// 在触屏设备上固定使用 [`RenderMode::Static`]。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RenderMode {
  /// 持续动画：受 [`MAX_FPS`] 上限约束，离屏 / 切后台暂停。
  /// 用户开启「减少动态效果」时自动降级为 [`RenderMode::Static`]。
  #[default]
  Animated,
  /// 只渲染一帧静态画面（当作背景图用）：不启动 `requestAnimationFrame`，GPU 占用为零。
  Static,
}

/// Web Threads 渲染器：负责 WebGL 上下文的创建、每帧渲染与资源释放。
///
/// 由 [`crate::components::web_threads::WebThreads`] 创建并持有；`Drop` 时完成全部清理。
pub struct WebThreadsRenderer {
  shared: Rc<Shared>,
  tick: Tick,
  canvas: HtmlCanvasElement,
  size_watcher: SizeWatcher,
  intersection: Option<IntersectionWatcher>,
  visibility_cb: Closure<dyn FnMut()>,
  /// 静态模式不挂鼠标监听（见 [`WebThreadsRenderer::new`]），故为 `Option`。
  move_cb: Option<Closure<dyn FnMut(web_sys::MouseEvent)>>,
  out_cb: Option<Closure<dyn FnMut(web_sys::MouseEvent)>>,
}

impl WebThreadsRenderer {
  /// 在 `container` 内创建画布并按 `mode` 渲染。
  ///
  /// 浏览器不支持 WebGL2、或着色器编译失败时返回 `None`。
  #[must_use]
  pub fn new(container: &Element, config: &ThreadsConfig, mode: RenderMode) -> Option<Self> {
    let canvas: HtmlCanvasElement = document().create_element("canvas").ok()?.dyn_into().ok()?;
    let gl = create_context(&canvas)?;
    append_canvas(container, &canvas);

    let program = build_program(&gl)?;
    let buffer = build_triangle(&gl, &program)?;
    let uniforms = Uniforms::new(&gl, &program);

    // 显式要求静态、或用户开启「减少动态效果」，都只渲染一帧。
    let animating = mode == RenderMode::Animated && crate::motion::motion_allowed();
    let shared = Rc::new(Shared {
      gl,
      _program: program,
      _buffer: buffer,
      uniforms,
      raf: Cell::new(0),
      visible: Cell::new(true),
      page_visible: Cell::new(!document().hidden()),
      animating: Cell::new(animating),
      current_mouse: Cell::new((0.5, 0.5)),
      target_mouse: Cell::new((0.5, 0.5)),
      current_active: Cell::new(0.0),
      target_active: Cell::new(0.0),
      started_at: Cell::new(0.0),
      last_frame: Cell::new(0.0),
    });

    // rAF 自引用：闭包通过 Rc 拿到自己以便继续排帧；清理时置空以断开引用环。
    let tick: Tick = Rc::new(RefCell::new(None));
    {
      let shared = Rc::clone(&shared);
      let slot = Rc::clone(&tick);
      let min_frame_ms = 1000.0 / MAX_FPS - FRAME_TOLERANCE_MS;
      let callback = Closure::<dyn FnMut(f64)>::new(move |timestamp: f64| {
        shared.raf.set(0);
        // 未到下一帧就只排帧、不绘制：屏幕刷新率高于上限时省下 GPU 填充率。
        if timestamp - shared.last_frame.get() >= min_frame_ms {
          shared.last_frame.set(timestamp);
          shared.render();
        }
        shared.ensure_running(&slot);
      });
      *tick.borrow_mut() = Some(callback);
    }

    // 应用初始参数并完成首次尺寸同步（会渲染首帧）。
    shared.uniforms.apply(&shared.gl, config);
    shared.resize(&canvas);

    let size_watcher = watch_size(container, &shared, &canvas);

    let intersection = watch_intersection(container, &shared, &tick);

    let visibility_cb = {
      let shared = Rc::clone(&shared);
      let tick = Rc::clone(&tick);
      let callback = Closure::<dyn FnMut()>::new(move || {
        shared.page_visible.set(!document().hidden());
        if shared.page_visible.get() {
          shared.ensure_running(&tick);
        } else {
          shared.stop();
        }
      });
      let _ = document()
        .add_event_listener_with_callback("visibilitychange", callback.as_ref().unchecked_ref());
      callback
    };

    // 鼠标交互挂在 `window` 上（背景层 `pointer-events: none`，不接收指针事件，
    // 因此不会抢占前景的点击 / 悬停），位置相对画布矩形换算。
    //
    // 静态模式没有逐帧重绘，扰动无从体现，索性不挂监听：处理函数里有
    // `getBoundingClientRect()`，每次 `mousemove` 都会读一次布局，不该白挂。
    let move_cb = animating.then(|| {
      let shared = Rc::clone(&shared);
      let canvas = canvas.clone();
      let callback =
        Closure::<dyn FnMut(web_sys::MouseEvent)>::new(move |event: web_sys::MouseEvent| {
          let rect = canvas.get_bounding_client_rect();
          if rect.width() <= 0.0 || rect.height() <= 0.0 {
            return;
          }
          let x = ((f64::from(event.client_x()) - rect.left()) / rect.width()) as f32;
          let y = (1.0 - (f64::from(event.client_y()) - rect.top()) / rect.height()) as f32;
          shared.target_mouse.set((x, y));
          shared.target_active.set(1.0);
        });
      let _ =
        window().add_event_listener_with_callback("mousemove", callback.as_ref().unchecked_ref());
      callback
    });

    // 指针离开文档（relatedTarget 为空）时让扰动平滑归零。
    let out_cb = animating.then(|| {
      let shared = Rc::clone(&shared);
      let callback =
        Closure::<dyn FnMut(web_sys::MouseEvent)>::new(move |event: web_sys::MouseEvent| {
          if event.related_target().is_none() {
            shared.target_active.set(0.0);
          }
        });
      let _ =
        document().add_event_listener_with_callback("mouseout", callback.as_ref().unchecked_ref());
      callback
    });

    if animating {
      shared.ensure_running(&tick);
    }

    Some(Self {
      shared,
      tick,
      canvas,
      size_watcher,
      intersection,
      visibility_cb,
      move_cb,
      out_cb,
    })
  }

  /// 热更新外观参数：只改 uniform，不重建 WebGL 上下文（主题切换走这里）。
  pub fn apply(&self, config: &ThreadsConfig) {
    self.shared.uniforms.apply(&self.shared.gl, config);
    // 静态画面（减少动态效果）也要立即反映新配色。
    self.shared.render();
  }

  /// 停帧并标记为不再动画（`Drop` 的第一步；也可手动提前停止）。
  pub fn dispose(&self) {
    self.shared.animating.set(false);
    self.shared.stop();
  }
}

impl Drop for WebThreadsRenderer {
  fn drop(&mut self) {
    self.dispose();

    match &self.size_watcher {
      SizeWatcher::Observer { observer, .. } => observer.disconnect(),
      SizeWatcher::Window { _callback } => {
        let _ = window()
          .remove_event_listener_with_callback("resize", _callback.as_ref().unchecked_ref());
      }
    }
    if let Some(watcher) = &self.intersection {
      watcher.observer.disconnect();
    }
    let document = document();
    if let Some(callback) = &self.move_cb {
      let _ = window()
        .remove_event_listener_with_callback("mousemove", callback.as_ref().unchecked_ref());
    }
    if let Some(callback) = &self.out_cb {
      let _ =
        document.remove_event_listener_with_callback("mouseout", callback.as_ref().unchecked_ref());
    }
    let _ = document.remove_event_listener_with_callback(
      "visibilitychange",
      self.visibility_cb.as_ref().unchecked_ref(),
    );

    // 断开 rAF 闭包的自引用（tick 持有闭包、闭包又捕获 tick），避免 Rc 环泄漏。
    *self.tick.borrow_mut() = None;

    if let Some(parent) = self.canvas.parent_element() {
      let _ = parent.remove_child(&self.canvas);
    }
    lose_context(&self.shared.gl);
  }
}

/// 计算渲染用的设备像素比：按视口尺寸取预算（DPR 上限 + 最长边上限）后等比降采样。
///
/// 紧凑设备（手机 / 竖屏平板）用更保守的一档：它们像素密度高、散热差、靠电池供电，
/// 而效果本身柔和，降采样的损失肉眼几乎不可见。
fn device_pixel_ratio(width: f64, height: f64) -> f64 {
  let compact = width.min(height) <= COMPACT_VIEWPORT;
  let (cap, max_dim) = if compact {
    (COMPACT_DPR_CAP, COMPACT_MAX_RENDER_DIM)
  } else {
    (2.0, MAX_RENDER_DIM)
  };
  let base = window().device_pixel_ratio().clamp(1.0, cap);
  let longest = width.max(height) * base;
  if longest > max_dim {
    (base * max_dim / longest).max(0.5)
  } else {
    base
  }
}

/// 创建 WebGL2 上下文（预乘 alpha、关闭抗锯齿与深度，与上游一致）。
///
/// 额外声明 `powerPreference: "low-power"`：双显卡设备 / 部分移动端会因此优先选择省电
/// 的 GPU。它只是提示，浏览器可以忽略；对全屏背景来说「省电优先」比「性能优先」更合适。
fn create_context(canvas: &HtmlCanvasElement) -> Option<WebGl2RenderingContext> {
  let options = js_sys::Object::new();
  let set = |key: &str, value: JsValue| {
    let _ = js_sys::Reflect::set(&options, &JsValue::from_str(key), &value);
  };
  set("alpha", JsValue::TRUE);
  set("premultipliedAlpha", JsValue::TRUE);
  set("antialias", JsValue::FALSE);
  set("depth", JsValue::FALSE);
  set("stencil", JsValue::FALSE);
  set("powerPreference", JsValue::from_str("low-power"));
  let context = canvas
    .get_context_with_context_options("webgl2", &options)
    .ok()??;
  context.dyn_into::<WebGl2RenderingContext>().ok()
}

/// 把画布挂到容器上并设为铺满（圆角 / 溢出由容器的 CSS 负责）。
fn append_canvas(container: &Element, canvas: &HtmlCanvasElement) {
  let style = canvas.style();
  let _ = style.set_property("display", "block");
  let _ = style.set_property("width", "100%");
  let _ = style.set_property("height", "100%");
  let _ = container.append_child(canvas);
}

/// 编译并链接着色器程序。
fn build_program(gl: &WebGl2RenderingContext) -> Option<WebGlProgram> {
  let vertex = compile_shader(gl, WebGl2RenderingContext::VERTEX_SHADER, VERTEX_SHADER)?;
  let fragment = compile_shader(gl, WebGl2RenderingContext::FRAGMENT_SHADER, FRAGMENT_SHADER)?;
  let program = gl.create_program()?;
  gl.attach_shader(&program, &vertex);
  gl.attach_shader(&program, &fragment);
  gl.link_program(&program);
  let linked = gl
    .get_program_parameter(&program, WebGl2RenderingContext::LINK_STATUS)
    .as_bool()
    .unwrap_or(false);
  if !linked {
    web_sys::console::error_1(
      &format!(
        "[web-threads] 着色器链接失败：{}",
        gl.get_program_info_log(&program).unwrap_or_default()
      )
      .into(),
    );
    return None;
  }
  gl.use_program(Some(&program));
  Some(program)
}

/// 编译单个着色器，失败时把编译日志打到控制台。
fn compile_shader(gl: &WebGl2RenderingContext, kind: u32, source: &str) -> Option<WebGlShader> {
  let shader = gl.create_shader(kind)?;
  gl.shader_source(&shader, source);
  gl.compile_shader(&shader);
  let compiled = gl
    .get_shader_parameter(&shader, WebGl2RenderingContext::COMPILE_STATUS)
    .as_bool()
    .unwrap_or(false);
  if compiled {
    return Some(shader);
  }
  web_sys::console::error_1(
    &format!(
      "[web-threads] 着色器编译失败：{}",
      gl.get_shader_info_log(&shader).unwrap_or_default()
    )
    .into(),
  );
  None
}

/// 上传全屏三角形顶点并绑定 `position` 属性。
fn build_triangle(gl: &WebGl2RenderingContext, program: &WebGlProgram) -> Option<WebGlBuffer> {
  let buffer = gl.create_buffer()?;
  gl.bind_buffer(WebGl2RenderingContext::ARRAY_BUFFER, Some(&buffer));
  let vertices = js_sys::Float32Array::from(TRIANGLE.as_slice());
  gl.buffer_data_with_array_buffer_view(
    WebGl2RenderingContext::ARRAY_BUFFER,
    &vertices,
    WebGl2RenderingContext::STATIC_DRAW,
  );
  let location = gl.get_attrib_location(program, "position");
  if location < 0 {
    return None;
  }
  let location = location as u32;
  gl.enable_vertex_attrib_array(location);
  gl.vertex_attrib_pointer_with_i32(location, 2, WebGl2RenderingContext::FLOAT, false, 0, 0);
  Some(buffer)
}

/// 尺寸观察：优先 `ResizeObserver`，不支持时回退到 `window.resize`。
fn watch_size(container: &Element, shared: &Rc<Shared>, canvas: &HtmlCanvasElement) -> SizeWatcher {
  let resize_callback = {
    let shared = Rc::clone(shared);
    let canvas = canvas.clone();
    Closure::<dyn FnMut(js_sys::Array)>::new(move |_entries: js_sys::Array| {
      shared.resize(&canvas);
    })
  };
  if let Ok(observer) =
    ResizeObserver::new(resize_callback.as_ref().unchecked_ref::<js_sys::Function>())
  {
    observer.observe(container);
    return SizeWatcher::Observer {
      observer,
      _callback: resize_callback,
    };
  }
  let fallback = {
    let shared = Rc::clone(shared);
    let canvas = canvas.clone();
    Closure::<dyn FnMut()>::new(move || shared.resize(&canvas))
  };
  let _ = window().add_event_listener_with_callback("resize", fallback.as_ref().unchecked_ref());
  SizeWatcher::Window {
    _callback: fallback,
  }
}

/// 视口可见性观察：离屏时停帧，回到视口再续。
fn watch_intersection(
  container: &Element,
  shared: &Rc<Shared>,
  tick: &Tick,
) -> Option<IntersectionWatcher> {
  let shared = Rc::clone(shared);
  let tick = Rc::clone(tick);
  let callback = Closure::<dyn FnMut(js_sys::Array)>::new(move |entries: js_sys::Array| {
    let Some(entry) = entries.get(0).dyn_into::<IntersectionObserverEntry>().ok() else {
      return;
    };
    shared.visible.set(entry.is_intersecting());
    if shared.visible.get() {
      shared.ensure_running(&tick);
    } else {
      shared.stop();
    }
  });
  let observer =
    IntersectionObserver::new(callback.as_ref().unchecked_ref::<js_sys::Function>()).ok()?;
  observer.observe(container);
  Some(IntersectionWatcher {
    observer,
    _callback: callback,
  })
}

/// 通过 `WEBGL_lose_context` 主动释放上下文（等价上游清理逻辑）。
fn lose_context(gl: &WebGl2RenderingContext) {
  let Ok(Some(extension)) = gl.get_extension("WEBGL_lose_context") else {
    return;
  };
  if let Ok(lose) = js_sys::Reflect::get(&extension, &JsValue::from_str("loseContext"))
    && let Some(f) = lose.dyn_ref::<js_sys::Function>()
  {
    let _ = f.call0(&extension);
  }
}
