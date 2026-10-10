//! 三维方向图：WebGL2 球壳渲染（不支持时回退到 SVG 线框）。
//!
//! 与 SVG 线框相比，WebGL2 版的价值在于**深度缓冲**：球壳是实体，被遮住的部分
//! 真的看不见，于是「哪个瓣朝哪边」一眼可判；再叠一层 `0 dBi` 参考球线框，方向图
//! 凹下去的地方就会露出网格。着色用一盏跟随视角的平行光（半球环境光），
//! 配合顶点法线做 Gouraud 插值。
//!
//! # 生命周期与性能
//!
//! - 渲染器在宿主 `div` 挂载时创建一次（`NodeRef::on_load`），此后**只换数据**：
//!   方向图变化重传缓冲区（几百个顶点，微秒级），视角变化只改 uniform，主题变化只改颜色。
//!   页面改一个几何参数会重算方向图，如果每次都重建 WebGL 上下文，浏览器很快就会
//!   开始丢弃上下文（上限约 16 个）—— 这是本模块刻意规避的第一个坑。
//! - 因此宿主 `div` 必须**常驻**：即使当前几何无解（`result` 为 `None`）也不卸载，
//!   只由外层把整块隐藏。反向也成立：切回有解时不会重建上下文。
//! - 单帧只有一次清屏 + 两次绘制（参考球线框、曲面），不跑 `requestAnimationFrame`：
//!   视角只在拖拽时变化，静止时零 GPU 开销。
//!
//! # 无障碍与降级
//!
//! 拖拽无法用键盘操作，宿主 `div` 只给一个 `role="img"` 与说明，具体读数由页面上的
//! 方位 / 仰角方向图与文字给出（它们才是无障碍入口）。浏览器不支持 WebGL2、或着色器
//! 编译失败时，宿主内回退渲染与旧版一致的 SVG 线框，功能与观感不丢。

use std::cell::Cell;
use std::rc::Rc;

use ham_web_core::nec::NecResult;
use ham_web_core::nec_mesh::{PatternMesh, build_pattern_mesh, unit_sphere_wireframe};
use leptos::html;
use leptos::prelude::*;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{
  Element, HtmlCanvasElement, ResizeObserver, WebGl2RenderingContext, WebGlBuffer, WebGlProgram,
  WebGlShader, WebGlUniformLocation,
};

use crate::i18n::t;
use crate::theme::{resolve_theme_color, use_display_prefs, use_theme};
use crate::util::{document, window};

/// 顶点着色器：顶点位置与法线都做「先偏航、再俯仰」的旋转，然后正交投影到裁剪空间。
///
/// 深度取 `0.5 − 0.5·z`：视点在 `+z`，于是越近的点深度越小（配合 `LESS` 深度测试）。
/// 法线只旋转不平移，因此同一套旋转矩阵就能给光照用。
const VERTEX_SHADER: &str = r#"#version 300 es
in vec3 aPosition;
in vec3 aNormal;
uniform vec2 uScale;
uniform float uYaw;
uniform float uPitch;
out vec3 vNormal;
vec3 rotate(vec3 p) {
  float cz = cos(uYaw), sz = sin(uYaw);
  vec3 q = vec3(p.x * cz - p.y * sz, p.x * sz + p.y * cz, p.z);
  float cx = cos(uPitch), sx = sin(uPitch);
  return vec3(q.x, q.y * cx - q.z * sx, q.y * sx + q.z * cx);
}
void main() {
  vec3 p = rotate(aPosition);
  vNormal = rotate(aNormal);
  gl_Position = vec4(p.xy * uScale, 0.5 - 0.5 * p.z, 1.0);
}
"#;

/// 片元着色器：`uFlat` 为 1 时输出纯色（参考球线框），否则按半球环境光 + 平行光着色。
///
/// 输出**预乘 alpha**（`rgb * a`），配合 `blendFunc(ONE, ONE_MINUS_SRC_ALPHA)`；
/// 曲面 `uAlpha = 1` 完全不透明，线框用较低的 `uAlpha` 透出背景。
const FRAGMENT_SHADER: &str = r#"#version 300 es
precision mediump float;
in vec3 vNormal;
uniform vec3 uColor;
uniform float uAmbient;
uniform float uAlpha;
uniform float uFlat;
out vec4 fragColor;
void main() {
  if (uFlat > 0.5) {
    fragColor = vec4(uColor * uAlpha, uAlpha);
    return;
  }
  vec3 n = normalize(vNormal);
  vec3 light = normalize(vec3(-0.35, 0.5, 0.79));
  float diffuse = max(dot(n, light), 0.0);
  float rim = pow(1.0 - clamp(abs(n.z), 0.0, 1.0), 3.0) * 0.18;
  float shade = uAmbient + (1.0 - uAmbient) * diffuse + rim;
  fragColor = vec4(uColor * shade * uAlpha, uAlpha);
}
"#;

/// 参考球线框的取样密度（纬线数 / 经线数 / 每条线分段数）。
const REF_RINGS: usize = 3;
const REF_MERIDIANS: usize = 8;
const REF_SAMPLES: usize = 48;

/// 半径 1 落在半边的 `0.86` 处（与旧版 SVG 的留白观感一致）。
const VIEW_SCALE: f32 = 0.86;
/// 半球环境光比例：太低会让背光面全黑，太高会丢掉立体感。
const AMBIENT: f32 = 0.42;
/// 参考球线框的不透明度。
const GRID_ALPHA: f32 = 0.3;
/// 画布分辨率上限（逻辑像素 × DPR 后的最长边），超过则等比降采样。
const MAX_RENDER_DIM: f64 = 1024.0;

/// 配色（跟随明暗主题，从主题令牌实时解析）。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Palette {
  /// 曲面颜色。
  surface: (f32, f32, f32),
  /// 参考球线框颜色。
  grid: (f32, f32, f32),
}

impl Palette {
  /// 从当前主题令牌解析配色；令牌缺失时回退到静态色值。
  #[must_use]
  fn detect() -> Self {
    Self {
      surface: resolve_theme_color("--primary").unwrap_or((0.06, 0.42, 0.4)),
      grid: resolve_theme_color("--muted-foreground").unwrap_or((0.45, 0.47, 0.5)),
    }
  }
}

/// 着色器 uniform 位置缓存：只在初始化时查一次。
struct Uniforms {
  scale: Option<WebGlUniformLocation>,
  yaw: Option<WebGlUniformLocation>,
  pitch: Option<WebGlUniformLocation>,
  color: Option<WebGlUniformLocation>,
  ambient: Option<WebGlUniformLocation>,
  alpha: Option<WebGlUniformLocation>,
  flat: Option<WebGlUniformLocation>,
}

impl Uniforms {
  fn new(gl: &WebGl2RenderingContext, program: &WebGlProgram) -> Self {
    let at = |name: &str| gl.get_uniform_location(program, name);
    Self {
      scale: at("uScale"),
      yaw: at("uYaw"),
      pitch: at("uPitch"),
      color: at("uColor"),
      ambient: at("uAmbient"),
      alpha: at("uAlpha"),
      flat: at("uFlat"),
    }
  }
}

/// 共享的渲染状态：观察器回调与渲染器句柄都持有它。
///
/// 拆出 `Shared` 是为了让「容器尺寸变化 → 重设视口」的回调能拿到 GL 状态，
/// 而回调必须在构造 `PatternRenderer` 之前就注册（`ResizeObserver` 无法事后补传）。
struct Shared {
  gl: WebGl2RenderingContext,
  canvas: HtmlCanvasElement,
  /// 程序对象保活（被 GC 会导致 uniform 失效）。
  _program: WebGlProgram,
  uniforms: Uniforms,
  /// `aPosition` / `aNormal` 的属性下标。
  pos_loc: u32,
  nrm_loc: u32,
  /// 曲面（交错存储的位置 + 法线）与索引缓冲。
  surface_vbo: WebGlBuffer,
  surface_ibo: WebGlBuffer,
  index_count: Cell<i32>,
  /// 参考球线框（同样交错存储，法线就用位置本身）。
  wire_vbo: WebGlBuffer,
  wire_count: Cell<i32>,
  palette: Cell<Palette>,
  yaw: Cell<f32>,
  pitch: Cell<f32>,
  /// 裁剪空间缩放（x / y 分开，兼容非正方形画布）。
  clip_scale: Cell<(f32, f32)>,
}

impl Shared {
  /// 创建画布、上下文、着色器与缓冲区，并把画布挂到 `host` 上。
  fn create(host: &Element) -> Option<Self> {
    let canvas: HtmlCanvasElement = document().create_element("canvas").ok()?.dyn_into().ok()?;
    let gl = create_context(&canvas)?;
    // 显式取 `&HtmlElement`：`canvas.style()` 在引入 Leptos prelude 后会撞上 tachys 的
    // `style(...)` 扩展方法（签名不同，报错指向别处）。
    let style = AsRef::<web_sys::HtmlElement>::as_ref(&canvas).style();
    let _ = style.set_property("display", "block");
    let _ = style.set_property("width", "100%");
    let _ = style.set_property("height", "100%");
    host.append_child(&canvas).ok()?;

    let program = build_program(&gl)?;
    let uniforms = Uniforms::new(&gl, &program);
    let pos_loc = gl.get_attrib_location(&program, "aPosition");
    let nrm_loc = gl.get_attrib_location(&program, "aNormal");
    if pos_loc < 0 || nrm_loc < 0 {
      web_sys::console::error_1(&"[nec-sphere] 顶点属性 aPosition / aNormal 缺失".into());
      return None;
    }
    let pos_loc = pos_loc as u32;
    let nrm_loc = nrm_loc as u32;
    gl.enable_vertex_attrib_array(pos_loc);
    gl.enable_vertex_attrib_array(nrm_loc);

    // 深度缓冲给出「实体遮挡」；混合让参考球线框透出背景。
    gl.enable(WebGl2RenderingContext::DEPTH_TEST);
    gl.depth_func(WebGl2RenderingContext::LESS);
    gl.enable(WebGl2RenderingContext::BLEND);
    gl.blend_func(
      WebGl2RenderingContext::ONE,
      WebGl2RenderingContext::ONE_MINUS_SRC_ALPHA,
    );
    gl.disable(WebGl2RenderingContext::CULL_FACE);

    let surface_vbo = gl.create_buffer()?;
    let surface_ibo = gl.create_buffer()?;
    let wire_vbo = gl.create_buffer()?;
    Some(Self {
      gl,
      canvas,
      _program: program,
      uniforms,
      pos_loc,
      nrm_loc,
      surface_vbo,
      surface_ibo,
      index_count: Cell::new(0),
      wire_vbo,
      wire_count: Cell::new(0),
      palette: Cell::new(Palette::detect()),
      yaw: Cell::new(0.0),
      pitch: Cell::new(0.0),
      clip_scale: Cell::new((VIEW_SCALE, VIEW_SCALE)),
    })
  }

  /// 每次绘制前把两个属性指针指到「当前绑定的顶点缓冲」上。
  ///
  /// 交错布局：`[位置(3) | 法线(3)]`，步长 24 字节，法线偏移 12 字节。
  fn bind_attribs(&self) {
    let gl = &self.gl;
    gl.vertex_attrib_pointer_with_i32(self.pos_loc, 3, WebGl2RenderingContext::FLOAT, false, 24, 0);
    gl.vertex_attrib_pointer_with_i32(
      self.nrm_loc,
      3,
      WebGl2RenderingContext::FLOAT,
      false,
      24,
      12,
    );
  }

  /// 上传参考球线框（法线取位置本身：单位球上两者同向）。
  fn upload_wire(&self) {
    let verts = unit_sphere_wireframe(REF_RINGS, REF_MERIDIANS, REF_SAMPLES);
    let data = interleave(&verts, &verts);
    let gl = &self.gl;
    gl.bind_buffer(WebGl2RenderingContext::ARRAY_BUFFER, Some(&self.wire_vbo));
    let array = js_sys::Float32Array::from(data.as_slice());
    gl.buffer_data_with_array_buffer_view(
      WebGl2RenderingContext::ARRAY_BUFFER,
      &array,
      WebGl2RenderingContext::STATIC_DRAW,
    );
    self.wire_count.set(verts.len() as i32);
  }

  /// 换方向图网格；`None` 表示当前无解（只画参考球）。
  fn set_pattern(&self, mesh: Option<&PatternMesh>) {
    let empty = PatternMesh::default();
    let mesh = mesh.filter(|m| !m.is_empty()).unwrap_or(&empty);
    let data = interleave(&mesh.positions, &mesh.normals);
    let gl = &self.gl;
    gl.bind_buffer(
      WebGl2RenderingContext::ARRAY_BUFFER,
      Some(&self.surface_vbo),
    );
    let array = js_sys::Float32Array::from(data.as_slice());
    gl.buffer_data_with_array_buffer_view(
      WebGl2RenderingContext::ARRAY_BUFFER,
      &array,
      WebGl2RenderingContext::DYNAMIC_DRAW,
    );
    gl.bind_buffer(
      WebGl2RenderingContext::ELEMENT_ARRAY_BUFFER,
      Some(&self.surface_ibo),
    );
    let indices = js_sys::Uint32Array::from(mesh.indices.as_slice());
    gl.buffer_data_with_array_buffer_view(
      WebGl2RenderingContext::ELEMENT_ARRAY_BUFFER,
      &indices,
      WebGl2RenderingContext::DYNAMIC_DRAW,
    );
    self.index_count.set(mesh.indices.len() as i32);
    self.render();
  }

  /// 换视角（度）。
  fn set_view(&self, yaw: f32, pitch: f32) {
    self.yaw.set(yaw);
    self.pitch.set(pitch);
    self.render();
  }

  /// 换配色。
  fn set_palette(&self, palette: Palette) {
    self.palette.set(palette);
    self.render();
  }

  /// 按容器尺寸重设画布缓冲区与视口，并重绘。
  fn resize(&self) {
    let Some(parent) = self.canvas.parent_element() else {
      return;
    };
    let rect = parent.get_bounding_client_rect();
    let width = rect.width().max(1.0);
    let height = rect.height().max(1.0);
    let dpr = window().device_pixel_ratio().clamp(1.0, 2.0);
    let longest = width.max(height) * dpr;
    // 超出预算就等比降采样：方向图是柔和曲面，降采样损失肉眼几乎不可见。
    let dpr = if longest > MAX_RENDER_DIM {
      (dpr * MAX_RENDER_DIM / longest).max(0.5)
    } else {
      dpr
    };
    let buffer_w = (width * dpr).round().max(1.0);
    let buffer_h = (height * dpr).round().max(1.0);
    self.canvas.set_width(buffer_w as u32);
    self.canvas.set_height(buffer_h as u32);
    self.gl.viewport(0, 0, buffer_w as i32, buffer_h as i32);
    // 半径 1 落在较短边的 0.86 处，两个轴向分别换算成裁剪空间比例。
    let half = buffer_w.min(buffer_h) / 2.0;
    let radius = f64::from(VIEW_SCALE) * half;
    self.clip_scale.set((
      (radius / (buffer_w / 2.0)) as f32,
      (radius / (buffer_h / 2.0)) as f32,
    ));
    self.render();
  }

  /// 绘制一帧：清屏 → 参考球线框 → 曲面。
  fn render(&self) {
    let gl = &self.gl;
    let uniforms = &self.uniforms;
    let palette = self.palette.get();
    gl.clear_color(0.0, 0.0, 0.0, 0.0);
    gl.clear(WebGl2RenderingContext::COLOR_BUFFER_BIT | WebGl2RenderingContext::DEPTH_BUFFER_BIT);

    let (sx, sy) = self.clip_scale.get();
    gl.uniform2f(uniforms.scale.as_ref(), sx, sy);
    gl.uniform1f(uniforms.yaw.as_ref(), self.yaw.get().to_radians());
    gl.uniform1f(uniforms.pitch.as_ref(), self.pitch.get().to_radians());
    gl.uniform1f(uniforms.ambient.as_ref(), AMBIENT);

    // 参考球先画，且**不写深度**：方向图的幅度永远 ≤ 峰值（半径 1），参考球恒在曲面
    // 之外，若让它写深度就会整张网格压在曲面上、把着色搅乱。不写深度则由后画的曲面
    // 直接覆盖自己的投影范围，网格就只剩轮廓外的一圈「笼子」，与旧版 SVG 的观感一致。
    if self.wire_count.get() > 0 {
      gl.bind_buffer(WebGl2RenderingContext::ARRAY_BUFFER, Some(&self.wire_vbo));
      self.bind_attribs();
      let (r, g, b) = palette.grid;
      gl.uniform3f(uniforms.color.as_ref(), r, g, b);
      gl.uniform1f(uniforms.alpha.as_ref(), GRID_ALPHA);
      gl.uniform1f(uniforms.flat.as_ref(), 1.0);
      gl.depth_mask(false);
      gl.draw_arrays(WebGl2RenderingContext::LINES, 0, self.wire_count.get());
      gl.depth_mask(true);
    }

    if self.index_count.get() > 0 {
      gl.bind_buffer(
        WebGl2RenderingContext::ARRAY_BUFFER,
        Some(&self.surface_vbo),
      );
      self.bind_attribs();
      gl.bind_buffer(
        WebGl2RenderingContext::ELEMENT_ARRAY_BUFFER,
        Some(&self.surface_ibo),
      );
      let (r, g, b) = palette.surface;
      gl.uniform3f(uniforms.color.as_ref(), r, g, b);
      gl.uniform1f(uniforms.alpha.as_ref(), 1.0);
      gl.uniform1f(uniforms.flat.as_ref(), 0.0);
      gl.draw_elements_with_f64(
        WebGl2RenderingContext::TRIANGLES,
        self.index_count.get(),
        WebGl2RenderingContext::UNSIGNED_INT,
        0.0,
      );
    }
  }
}

/// 尺寸观察：优先 `ResizeObserver`，不支持时回退到 `window.resize`。
enum SizeWatcher {
  Observer {
    observer: ResizeObserver,
    _callback: Closure<dyn FnMut(js_sys::Array)>,
  },
  Window {
    _callback: Closure<dyn FnMut()>,
  },
}

fn watch_size(host: &Element, shared: &Rc<Shared>) -> SizeWatcher {
  let resize_callback = {
    let shared = Rc::clone(shared);
    Closure::<dyn FnMut(js_sys::Array)>::new(move |_entries: js_sys::Array| shared.resize())
  };
  if let Ok(observer) =
    ResizeObserver::new(resize_callback.as_ref().unchecked_ref::<js_sys::Function>())
  {
    observer.observe(host);
    return SizeWatcher::Observer {
      observer,
      _callback: resize_callback,
    };
  }
  let fallback = {
    let shared = Rc::clone(shared);
    Closure::<dyn FnMut()>::new(move || shared.resize())
  };
  let _ = window().add_event_listener_with_callback("resize", fallback.as_ref().unchecked_ref());
  SizeWatcher::Window {
    _callback: fallback,
  }
}

/// 三维方向图渲染器句柄；`Drop` 时完成全部清理。
pub struct PatternRenderer {
  shared: Rc<Shared>,
  size_watcher: SizeWatcher,
}

impl PatternRenderer {
  /// 在 `host` 内建画布并渲染 `mesh`；浏览器不支持 WebGL2 或着色器失败时返回 `None`。
  #[must_use]
  pub fn new(host: &Element, mesh: &PatternMesh, yaw_deg: f32, pitch_deg: f32) -> Option<Self> {
    let shared = Rc::new(Shared::create(host)?);
    shared.upload_wire();
    shared.set_view(yaw_deg, pitch_deg);
    shared.set_pattern(Some(mesh));
    shared.set_palette(Palette::detect());
    let size_watcher = watch_size(host, &shared);
    // 首帧必须有正确的视口：`ResizeObserver` 的首个回调不保证在首次绘制前到达。
    shared.resize();
    Some(Self {
      shared,
      size_watcher,
    })
  }

  /// 换方向图网格（`None` 表示当前无解）。
  pub fn set_pattern(&self, mesh: Option<&PatternMesh>) {
    self.shared.set_pattern(mesh);
  }

  /// 换视角（度）。
  pub fn set_view(&self, yaw_deg: f32, pitch_deg: f32) {
    self.shared.set_view(yaw_deg, pitch_deg);
  }

  /// 换配色。
  pub fn set_palette(&self, palette: Palette) {
    self.shared.set_palette(palette);
  }
}

impl Drop for PatternRenderer {
  fn drop(&mut self) {
    match &self.size_watcher {
      SizeWatcher::Observer { observer, .. } => observer.disconnect(),
      SizeWatcher::Window { _callback } => {
        let _ = window()
          .remove_event_listener_with_callback("resize", _callback.as_ref().unchecked_ref());
      }
    }
    if let Some(parent) = self.shared.canvas.parent_element() {
      let _ = parent.remove_child(&self.shared.canvas);
    }
    lose_context(&self.shared.gl);
  }
}

/// 把「位置 / 法线」交错成 `[px, py, pz, nx, ny, nz, …]`。
fn interleave(positions: &[[f32; 3]], normals: &[[f32; 3]]) -> Vec<f32> {
  let count = positions.len().min(normals.len());
  let mut out = Vec::with_capacity(count * 6);
  for k in 0..count {
    out.extend_from_slice(&positions[k]);
    out.extend_from_slice(&normals[k]);
  }
  out
}

/// 创建 WebGL2 上下文：透明画布 + 预乘 alpha + 抗锯齿，关闭模板与 `preserveDrawingBuffer`。
fn create_context(canvas: &HtmlCanvasElement) -> Option<WebGl2RenderingContext> {
  let options = js_sys::Object::new();
  let set = |key: &str, value: JsValue| {
    let _ = js_sys::Reflect::set(&options, &JsValue::from_str(key), &value);
  };
  set("alpha", JsValue::TRUE);
  set("premultipliedAlpha", JsValue::TRUE);
  set("antialias", JsValue::TRUE);
  set("depth", JsValue::TRUE);
  set("stencil", JsValue::FALSE);
  set("preserveDrawingBuffer", JsValue::FALSE);
  // 方向图只在拖拽时重绘，不需要独立显卡常驻，优先低功耗 GPU。
  set("powerPreference", JsValue::from_str("low-power"));
  let context = canvas
    .get_context_with_context_options("webgl2", &options)
    .ok()??;
  context.dyn_into::<WebGl2RenderingContext>().ok()
}

/// 编译并链接着色器程序；失败时把日志打到控制台。
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
        "[nec-sphere] 着色器链接失败：{}",
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
      "[nec-sphere] 着色器编译失败：{}",
      gl.get_shader_info_log(&shader).unwrap_or_default()
    )
    .into(),
  );
  None
}

/// 通过 `WEBGL_lose_context` 主动释放上下文。
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

/// 三维方向图：`div` 宿主 + WebGL2 画布（不支持时回退 SVG 线框）。
///
/// - `result`：当前求解结果；为 `None` 时只画参考球。
/// - `yaw` / `pitch`：视角（度），由页面上的拖拽手势驱动。
///
/// 宿主 `div` 带 `data-renderer`（`webgl2` / `svg`）与 `data-yaw` / `data-pitch`，
/// 让端到端用例能在**不依赖具体渲染后端**的前提下断言「拖拽改变了视角」。
#[component]
pub fn PatternSphere(
  /// 当前求解结果（`None` 表示参数无法求解）。
  #[prop(into)]
  result: Signal<Option<NecResult>>,
  /// 偏航角（度）。
  yaw: RwSignal<f64>,
  /// 俯仰角（度）。
  pitch: RwSignal<f64>,
) -> impl IntoView {
  const SVG_SIZE: f64 = 300.0;
  let host = NodeRef::<html::Div>::new();
  let renderer = StoredValue::new_local(None::<PatternRenderer>);
  let webgl = RwSignal::new(false);

  host.on_load(move |el| {
    let mesh = result
      .get_untracked()
      .map_or_else(PatternMesh::default, |r| build_pattern_mesh(&r));
    match PatternRenderer::new(
      &el,
      &mesh,
      yaw.get_untracked() as f32,
      pitch.get_untracked() as f32,
    ) {
      Some(created) => {
        webgl.set(true);
        renderer.set_value(Some(created));
      }
      None => renderer.set_value(None),
    }
  });

  // 方向图变化 → 只重传缓冲区，不重建上下文。
  Effect::new(move |_| {
    let mesh = result.get().map(|r| build_pattern_mesh(&r));
    renderer.update_value(|slot| {
      if let Some(renderer) = slot.as_ref() {
        renderer.set_pattern(mesh.as_ref());
      }
    });
  });

  // 视角变化 → 只改 uniform。
  Effect::new(move |_| {
    let (y, p) = (yaw.get() as f32, pitch.get() as f32);
    renderer.update_value(|slot| {
      if let Some(renderer) = slot.as_ref() {
        renderer.set_view(y, p);
      }
    });
  });

  // 主题 / 配色方案变化 → 重新解析令牌（`--primary` / `--muted-foreground` 在明暗与方案间都不同）。
  let theme = use_theme();
  let prefs = use_display_prefs();
  Effect::new(move |_| {
    let _ = (theme.is_dark(), prefs.scheme());
    let palette = Palette::detect();
    renderer.update_value(|slot| {
      if let Some(renderer) = slot.as_ref() {
        renderer.set_palette(palette);
      }
    });
  });

  on_cleanup(move || renderer.update_value(|slot| *slot = None));

  // 回退线框：与 WebGL2 版共用同一份方向图网格，只是投影到 SVG 折线。
  let fallback = move || {
    result.get().map_or_else(String::new, |r| {
      pattern3d_paths(&r, yaw.get(), pitch.get(), SVG_SIZE)
    })
  };

  view! {
    <div
      node_ref=host
      class="aspect-square w-full"
      role="img"
      aria-label=move || t("tools.nec-3d-pattern-alt")
      data-renderer=move || if webgl.get() { "webgl2" } else { "svg" }
      data-yaw=move || format!("{:.1}", yaw.get())
      data-pitch=move || format!("{:.1}", pitch.get())
    >
      {move || {
        if webgl.get() {
          ().into_any()
        } else {
          view! {
            <svg viewBox="0 0 300 300" class="h-full w-full" aria-hidden="true">
              <circle
                cx=150.0
                cy=150.0
                r=140.0
                class="fill-none stroke-muted-foreground/20"
                stroke-width="1"
              />
              <path
                d=fallback
                class="fill-none stroke-primary/70"
                stroke-width="0.8"
                vector-effect="non-scaling-stroke"
              />
            </svg>
          }
          .into_any()
        }
      }}
    </div>
  }
}

/// 三维点的正交投影：先绕 `z` 偏航、再绕 `x` 俯仰，返回屏幕坐标（y 向下）。
fn project(p: [f64; 3], rot_x: f64, rot_z: f64, scale: f64, center: f64) -> (f64, f64) {
  let (sz, cz) = rot_z.to_radians().sin_cos();
  let (sx, cx) = rot_x.to_radians().sin_cos();
  let x1 = p[0] * cz - p[1] * sz;
  let y1 = p[0] * sz + p[1] * cz;
  let z1 = p[2];
  let y2 = y1 * cx - z1 * sx;
  (center + x1 * scale, center - y2 * scale)
}

/// 把三维方向图网格投影成 SVG 折线（纬线 + 经线），WebGL2 不可用时使用。
///
/// 半径按**幅度**折算（`10^{dB/20}`），这样球壳的凹凸就是方向图的真实形状；
/// 深零点夹到 `−40 dB`，否则单个零点会把整个曲面压成一点。
fn pattern3d_paths(r: &NecResult, rot_x: f64, rot_z: f64, size: f64) -> String {
  let (nt, np) = r.pattern3d_shape;
  let peak = r.gain_max_dbi;
  let center = size / 2.0;
  let scale = (size / 2.0 - 10.0) * 0.92;
  let point = |i: usize, j: usize| -> (f64, f64) {
    let th = 180.0 * i as f64 / nt as f64;
    let ph = 360.0 * j as f64 / np as f64;
    let db = r.pattern3d[i * np + j % np];
    let amp = 10f64.powf(((db - peak).max(-40.0)) / 20.0);
    let (st, ct) = th.to_radians().sin_cos();
    let (sp, cp) = ph.to_radians().sin_cos();
    project(
      [amp * st * cp, amp * st * sp, amp * ct],
      rot_x,
      rot_z,
      scale,
      center,
    )
  };
  let mut out = String::new();
  for i in (0..=nt).step_by(2) {
    for j in 0..=np {
      let (x, y) = point(i, j);
      out.push_str(&if j == 0 {
        format!("M{x:.1} {y:.1}")
      } else {
        format!(" L{x:.1} {y:.1}")
      });
    }
    out.push(' ');
  }
  for j in (0..np).step_by(3) {
    for i in 0..=nt {
      let (x, y) = point(i, j);
      out.push_str(&if i == 0 {
        format!("M{x:.1} {y:.1}")
      } else {
        format!(" L{x:.1} {y:.1}")
      });
    }
    out.push(' ');
  }
  out
}
