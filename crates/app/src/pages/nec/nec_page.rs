//! NEC 求解页面：把散落的输入攒成一份求解输入，交给核心求解，再把结果组装起来。
//!
//! 页面本身只负责**状态与联动**：预设与参数重建几何、输入防抖后才求解、表格增删、
//! 导入导出与方案库交接。每一块界面都是独立组件（见 [`super`] 的模块文档），
//! 全部计算在 `ham_web_core::nec` 里完成（浏览器本地，离线可用）。

use ham_web_core::nec::{Feed, Ground, Load, NecInput, Wire, parse_nec, solve_with_lines, to_nec};
use ham_web_core::nec_templates::nec_template;
use leptos::prelude::*;

use crate::i18n::{t, tf};
use crate::util::{cancel_debounce, debounce, set_title, take_nec_template};

use super::canvas::WireCanvas;
use super::design::DesignSection;
use super::ground_material::GroundMaterialSection;
use super::import_export::ImportExportSection;
use super::lines::parsed as parsed_lines;
use super::lines::{LineRow, LinesSection};
use super::load_table::LoadTable;
use super::num_field::NumField;
use super::nvis::NvisSection;
use super::pattern_3d::Pattern3d;
use super::preset_chip::chip;
use super::result_section::ResultSection;
use super::sweep::SweepSection;
use super::wire_table::WireTable;
use super::{LoadRow, MATERIALS, NOTE, PRESETS, Preset, WireRow};
use crate::ui::ChipGroup;

#[component]
pub fn NecPage() -> impl IntoView {
  set_title("tools.nec-antenna-solver");
  let preset = RwSignal::new(Preset::Dipole);
  let freq_mhz = RwSignal::new(String::from("14.1"));
  let len_m = RwSignal::new(String::from("10.1"));
  let height_m = RwSignal::new(String::from("10"));
  let spacing_m = RwSignal::new(String::from("3.1"));
  let radius_mm = RwSignal::new(String::from("2"));

  // ── 可编辑几何 ────────────────────────────────────────────────
  // `StoredValue<usize>` 是 Copy，闭包按值捕获即可成为 `'static`，可以被多处复用。
  let next_id = StoredValue::new(100usize);
  let id_gen = move || {
    let v = next_id.get_value();
    next_id.set_value(v + 1);
    v
  };
  // `move` 让闭包**拥有** id_gen（它是 Copy），这样才能被其他 `'static` 回调复用。
  let initial = move |rows: &[Wire]| -> Vec<WireRow> {
    rows
      .iter()
      .map(|w| WireRow::from_wire(id_gen(), w))
      .collect()
  };
  let preset_wires = {
    let (w, _, _) = Preset::Dipole.build(10.1, 10.0, 3.1, 0.002);
    initial(&w)
  };
  let wires = RwSignal::new(preset_wires);
  let loads = RwSignal::new(Vec::<LoadRow>::new());
  let ground_kind = RwSignal::new(1usize);
  let eps_r = RwSignal::new(String::from("13"));
  let sigma_ground = RwSignal::new(String::from("0.005"));
  let material = RwSignal::new(1usize);

  // 馈电点 `(导线号, 相对位置)`：预设给默认值，`.nec` 导入时按 EX 卡片覆盖。
  let feed = RwSignal::new((0usize, 0.5f64));
  let notice = RwSignal::new(String::new());
  let nec_text = RwSignal::new(String::new());
  // 传输线（TL）表格：行状态用字符串，派生出核心类型交给求解器。
  let lines = RwSignal::new(Vec::<LineRow>::new());
  let core_lines = Signal::derive(move || parsed_lines(&lines.get()));
  // 由核心 `Wire` 造一行导线：借用统一的 id 生成器，保证 `<For>` 键稳定。
  let row_factory = Callback::new(move |w: Wire| WireRow::from_wire(id_gen(), &w));

  let freq_hz = move || {
    freq_mhz
      .get()
      .trim()
      .parse::<f64>()
      .unwrap_or(14.1)
      .clamp(0.1, 6000.0)
      * 1e6
  };
  let len = move || len_m.get().trim().parse::<f64>().unwrap_or(10.1).max(0.01);
  let height = move || height_m.get().trim().parse::<f64>().unwrap_or(0.0).max(0.0);
  let spacing = move || {
    spacing_m
      .get()
      .trim()
      .parse::<f64>()
      .unwrap_or(3.1)
      .max(0.01)
  };
  let radius = move || {
    radius_mm
      .get()
      .trim()
      .parse::<f64>()
      .unwrap_or(2.0)
      .max(0.05)
      / 1000.0
  };
  let ground = move || match ground_kind.get() {
    1 => Ground::Perfect,
    2 => Ground::lossy(
      eps_r.get().trim().parse::<f64>().unwrap_or(13.0),
      sigma_ground.get().trim().parse::<f64>().unwrap_or(0.005),
    ),
    _ => Ground::Free,
  };
  // 材质直接决定电导率：`None` 就是理想导体（无损耗）。
  // 这里**不能**再回退到自定义输入框 —— 之前那样写会让「理想导体」悄悄按铜算。
  // 材质表与「地面与材质」卡片共用同一份（下标必须一一对应）。
  let sigma_wire = move || MATERIALS.get(material.get()).and_then(|(_, s)| *s);

  // 按当前预设重建导线表。**只有**「点预设」或「改预设参数」会调用它 ——
  // 不用 `Effect` 盯参数：那样每次参数变化都会把 `.nec` 导入进来的几何冲掉。
  let rebuild = move || {
    let value = preset.get_untracked();
    let (built, _, grounded) = value.build(len(), height(), spacing(), radius());
    wires.set(initial(&built));
    feed.set(value.feed());
    // 预设自带「是否需要地面」；用户若已选有耗地面（2）就保留，不擅自改回理想导体。
    if ground_kind.get_untracked() != 2 {
      ground_kind.set(if grounded { 1 } else { 0 });
    }
  };
  let apply_preset = move |value: Preset| {
    preset.set(value);
    notice.set(String::new());
    rebuild();
  };

  let active_input = Memo::new(move |_| {
    let ws: Option<Vec<Wire>> = wires.get().iter().map(WireRow::to_wire).collect();
    let ws = ws?;
    if ws.is_empty() {
      return None;
    }
    let (wire, at) = feed.get();
    let wire = wire.min(ws.len().saturating_sub(1));
    let ls: Option<Vec<Load>> = loads.get().iter().map(LoadRow::to_load).collect();
    let ls = ls?;
    Some(NecInput {
      wires: ws,
      feeds: vec![Feed {
        wire,
        at,
        volts: 1.0,
      }],
      loads: ls,
      freq_hz: freq_hz(),
      ground: ground(),
      conductivity: sigma_wire(),
    })
  });
  // ── 求解（输入防抖） ───────────────────────────────────────────
  // 矩量法求解要重装阻抗矩阵并扫整个方向图，是页面上最贵的一步；几何/频率输入框每敲
  // 一个字符都会让 `active_input` 变化，逐键求解会卡住主线程。这里把输入镜像到
  // `settled_input`，只在停止输入 250 ms 后求解一次。
  //
  // 起手就填当前值：否则首屏会先空一下再出结果。
  let settled_input = RwSignal::new(active_input.get_untracked());
  Effect::new(move |_| {
    let input = active_input.get();
    if input == settled_input.get_untracked() {
      return;
    }
    debounce("nec-solve", 250, move || settled_input.set(input));
  });
  // 定时器回调会在离开页面后触发，那时信号已释放 —— 必须撤销。
  on_cleanup(|| cancel_debounce("nec-solve"));

  let result = Memo::new(move |_| {
    settled_input
      .get()
      .and_then(|i| solve_with_lines(&i, &core_lines.get()))
  });

  // ── 导入 / 导出 ───────────────────────────────────────────────
  // 把一段 `.nec` 文本加载进页面：导入按钮与「方案库交接」共用同一条路径，
  // 保证两条入口的口径完全一致。返回是否成功。
  let apply_nec = move |text: &str| -> bool {
    match parse_nec(text) {
      Some(f) => {
        wires.set(initial(&f.input.wires));
        loads.set(
          f.input
            .loads
            .iter()
            .map(|l| LoadRow {
              id: id_gen(),
              wire: (l.wire + 1).to_string(),
              at: format!("{:.4}", l.at),
              r_ohm: format!("{:.3}", l.r_ohm),
              l_uh: format!("{:.3}", l.l_uh),
              c_pf: format!("{:.3}", l.c_pf),
            })
            .collect(),
        );
        ground_kind.set(match f.input.ground {
          Ground::Free => 0,
          Ground::Perfect => 1,
          Ground::Lossy { .. } => 2,
        });
        if let Ground::Lossy { eps_r: e, sigma } = f.input.ground {
          eps_r.set(format!("{e:.3}"));
          sigma_ground.set(format!("{sigma:.5}"));
        }
        freq_mhz.set(format!("{:.4}", f.input.freq_hz / 1e6));
        // 馈电点必须一起搬过来。`feed` 是独立信号，只有 `rebuild()`（点预设 / 改参数）才会写它；
        // 漏掉这一句的话，导入端馈（`efhw-40m`）或馈电不在 0 号导线上的模板（`yagi3-2m`）之后，
        // 仍按旧的 `(0, 0.5)` 馈电 —— 阻抗、驻波比、增益全是错的，界面上还看不出来。
        let first = f.input.feeds.first().map_or((0, 0.5), |x| (x.wire, x.at));
        feed.set(first);
        preset.set(match first {
          (0, at) if at > 0.45 && at < 0.55 => Preset::Dipole,
          _ => Preset::Vertical,
        });
        notice.set(if f.ignored.is_empty() {
          t("tools.nec-imported")
        } else {
          tf("tools.nec-ignored-cards", &[&f.ignored.join(", ")])
        });
        true
      }
      None => {
        notice.set(t("tools.nec-parse-failed"));
        false
      }
    }
  };
  let do_import = Callback::new(move |()| {
    apply_nec(&nec_text.get());
  });

  // 方案库交接：从 `/antenna-diy`、`/practical-antennas` 点「在求解器中打开」时，
  // 那边把模板 id 写进 localStorage 再跳过来；这里挂载时取走并清除。
  // 只认 id，正文与名字都从 `nec_templates` 取 —— 单一来源，改名不会对不上。
  if let Some(id) = take_nec_template() {
    match nec_template(&id) {
      Some(found) => {
        if apply_nec(found.text) {
          notice.set(tf("tools.nec-template-loaded", &[found.name]));
        }
      }
      None => notice.set(t("tools.nec-template-invalid")),
    }
  }
  let do_export = Callback::new(move |()| match active_input.get() {
    Some(input) => {
      nec_text.set(to_nec(&input));
      notice.set(t("tools.nec-exported"));
    }
    None => notice.set(t("tools.nec-parse-failed")),
  });

  // ── 表格操作 ─────────────────────────────────────────────────
  // 新增导线：复制最后一根并沿「垂直于它自身」的方向平移 1 m。
  // 直接复制端点会得到零长度导线（求解直接失败），用户还要手工改三格才能用。
  let add_wire = Callback::new(move |()| {
    let row = match wires.get().last().and_then(WireRow::to_wire) {
      Some(w) => {
        let dx = w.b[0] - w.a[0];
        let dy = w.b[1] - w.a[1];
        let dz = w.b[2] - w.a[2];
        let n = (dx * dx + dy * dy + dz * dz).sqrt().max(1e-9);
        // 与导线夹角最大的坐标轴方向作为平移方向。
        let offset = if (dy / n).abs() < 0.9 {
          [0.0, 1.0, 0.0]
        } else {
          [1.0, 0.0, 0.0]
        };
        let shift = |p: [f64; 3]| [p[0] + offset[0], p[1] + offset[1], p[2] + offset[2]];
        let moved = Wire::new(shift(w.a), shift(w.b), w.radius, w.segments);
        WireRow::from_wire(id_gen(), &moved)
      }
      None => WireRow::from_wire(
        id_gen(),
        &Wire::new([-5.0, 0.0, 0.0], [5.0, 0.0, 0.0], 0.002, 20),
      ),
    };
    wires.update(|w| w.push(row));
  });
  let remove_wire = Callback::new(move |id: usize| wires.update(|w| w.retain(|r| r.id != id)));
  let add_load = Callback::new(move |()| {
    loads.update(|l| {
      l.push(LoadRow {
        id: id_gen(),
        wire: "1".into(),
        at: "0.500".into(),
        r_ohm: "0".into(),
        l_uh: "10".into(),
        c_pf: "0".into(),
      });
    });
  });
  let remove_load = Callback::new(move |id: usize| loads.update(|l| l.retain(|r| r.id != id)));

  view! {
    <section class="mx-auto max-w-6xl space-y-4 px-4 py-5">
      <h1 class="text-lg font-semibold">{move || t("tools.nec-antenna-solver")}</h1>
      <p class=NOTE>
        {move || t("tools.nec-solves-wire-antennas")}
      </p>

      <ChipGroup
        value=Signal::derive(move || preset.get().key().to_owned())
        on_change=Callback::new(move |v: String| {
          if let Some(p) = PRESETS.into_iter().find(|p| p.key() == v) {
            apply_preset(p);
          }
        })
      >
        {PRESETS.iter().map(|&p| chip(p)).collect_view()}
      </ChipGroup>

      <div class="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
        <NumField label=Signal::derive(move || t("log.frequency-mhz"))
          value=freq_mhz step=0.1 min=0.1 max=6000.0 />
        <NumField label=Signal::derive(move || t("tools.nec-main-dimension"))
          value=len_m step=0.1 min=0.01 max=200.0
          on_input=Some(Callback::new(move |_: String| rebuild())) />
        {move || {
          if preset.get().needs_spacing() {
            view! {
              <NumField label=Signal::derive(move || t("tools.nec-element-spacing"))
                value=spacing_m step=0.1 min=0.05 max=50.0
          on_input=Some(Callback::new(move |_: String| rebuild())) />
            }
            .into_any()
          } else {
            view! {
              <NumField label=Signal::derive(move || t("tools.nec-height"))
                value=height_m step=0.5 min=0.0 max=100.0
          on_input=Some(Callback::new(move |_: String| rebuild())) />
            }
            .into_any()
          }
        }}
        <NumField label=Signal::derive(move || t("tools.nec-wire-radius"))
          value=radius_mm step=0.1 min=0.05 max=20.0
          on_input=Some(Callback::new(move |_: String| rebuild())) />
      </div>
      <p class=NOTE>{move || t("tools.nec-preset-note")}</p>

      <DesignSection
        freq_mhz=freq_mhz
        wires=wires
        feed=feed
        row_factory=row_factory
        radius_m=Signal::derive(radius)
        ground_kind=ground_kind
      />

      <WireCanvas wires=wires new_id=Callback::new(move |_| id_gen()) feed=feed />

      <WireTable wires=wires on_add=add_wire on_remove=remove_wire />

      <LoadTable loads=loads on_add=add_load on_remove=remove_load />

      <GroundMaterialSection
        ground_kind=ground_kind
        eps_r=eps_r
        sigma_ground=sigma_ground
        material=material
      />

      <ImportExportSection nec_text=nec_text on_import=do_import on_export=do_export />

      {move || {
        let msg = notice.get();
        if msg.is_empty() {
          ().into_any()
        } else {
          view! { <p class=NOTE>{msg}</p> }.into_any()
        }
      }}

      <ResultSection
        result=result
        freq_hz=Signal::derive(freq_hz)
        len_m=Signal::derive(len)
        ground_kind=ground_kind
      />

      <Pattern3d result=result />

      // 扫频同样订阅「稳定后的输入」：否则每敲一个字符都会重跑一次整段频带的求解。
      <SweepSection input=settled_input lines=core_lines />

      <LinesSection lines=lines new_id=Callback::new(move |_| id_gen()) />

      <NvisSection result=result />
    </section>
  }
}
