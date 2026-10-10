//! 设备评测与选购：精选机型库 + 按用途加权的六轴雷达图与参数对比。
//!
//! 三类数据各带各的口径，界面上分得清：
//!
//! - 器材库（[`ham_web_core::gear`]）是「公开且稳定的高层次信息」，不写会随批次与渠道
//!   变化的精确参数，价格只给相对档位；
//! - 接收机实测指标（[`ham_web_core::gear_rx`]）来自 Rob Sherwood 的实测表，带出处与
//!   核对日期，**只在他测过的机型上有值**，其余留空；
//! - 加权得分与雷达图（[`ham_web_core::gear_score`]）是把上面两者按用途折算成 0–100，
//!   缺数据的轴跳过，依据不足时只说明、不给分。
//!
//! 用户自己录的实测值（[`ham_web_core::gear_user`]）只存本地，输入期间由草稿信号驱动回显
//! —— 细节见 `drafts` 的注释，那里同时交代了为什么不能让存储回写输入框。

mod radar;
mod score_row;
mod shared;

use std::collections::BTreeMap;

use ham_web_core::gear::{GEAR, GEAR_CATEGORIES, GEAR_PICKS, GEAR_TIPS, Gear, in_category};
use ham_web_core::gear_rx;
use ham_web_core::gear_score::{self, Axis, Preset};
use ham_web_core::gear_user::{UserField, UserMeasurements};
use leptos::prelude::*;

use crate::components::common::{BulletSection, KnowledgePage, TableSection};
use crate::i18n::{t, tf};
use crate::pages::log::utc_today;
use crate::ui::{
  Button, Chip, ChipGroup, ChipToggle, Field, NumberField, NumberKind, Size, Variant,
};
use crate::util::{alert, set_title, unique_id};

use radar::{GearRadar, RadarSeries};
use score_row::ScoreRow;

/// 用户实测值的存储 key（只存本地，不进任何导出）。
const MEASURE_KEY: &str = "gear-measurements";

/// 读用户实测值。
fn load_measurements() -> UserMeasurements {
  crate::util::storage::get_json(MEASURE_KEY).unwrap_or_default()
}

/// 录入项草稿的键：一台机器的一个字段一条。
fn draft_key(gear_id: &str, field: UserField) -> String {
  format!("{gear_id}|{}", field.key)
}

/// 录入项的界面名（字面量 key 写在调用点，静态扫描靠它认条目）。
fn field_label(field: UserField) -> String {
  match field.key {
    "current" => t("knowledge.measure-current"),
    // 与对比表那一行共用一条文案（中文释义全库唯一）。
    "noise-floor" => t("knowledge.rx-noise-floor"),
    "rmdr" => t("knowledge.measure-rmdr"),
    _ => t("knowledge.measure-phase-noise"),
  }
}

/// 这一项「怎么量」（写在输入框下面，省得用户猜测量条件）。
fn field_hint(field: UserField) -> String {
  match field.key {
    "current" => t("knowledge.measure-current-how"),
    "noise-floor" => t("knowledge.measure-noise-floor-how"),
    "rmdr" => t("knowledge.measure-rmdr-how"),
    _ => t("knowledge.measure-phase-noise-how"),
  }
}

/// 器材类别名（界面文案；数据里只留 key，名称归词典）。
fn category_label(key: &str) -> String {
  match key {
    "hf" => t("knowledge.gear-category-hf"),
    "portable" => t("knowledge.gear-category-portable"),
    "handheld" => t("knowledge.gear-category-handheld"),
    "mobile" => t("knowledge.gear-category-mobile"),
    "sdr" => t("knowledge.gear-category-sdr"),
    _ => t("knowledge.gear-category-accessory"),
  }
}

/// 推荐表表头。
const PICK_HEADERS: &[&str] = &[
  "common.uses",
  "knowledge.recommended-radios",
  "knowledge.why",
];

/// 对比表最多同时对比的机型数（也是雷达图上色板的长度）。
const MAX_COMPARE: usize = 4;

/// 某类别默认选中的机型（前 3 台）。
fn default_selection(category: &str) -> Vec<String> {
  in_category(category)
    .iter()
    .take(3)
    .map(|g| g.id.to_owned())
    .collect()
}

/// 预设的界面名（字面量 key 写在调用点，静态扫描靠它认条目）。
fn preset_label(preset: &Preset) -> String {
  match preset.key {
    "dx" => t("knowledge.preset-dx"),
    "contest" => t("knowledge.preset-contest"),
    "field" => t("knowledge.preset-field"),
    "satellite" => t("knowledge.preset-satellite"),
    _ => preset.key.to_owned(),
  }
}

/// 对比表的一行。
#[derive(Debug, Clone, PartialEq)]
struct Row {
  /// 行标题（**已经翻译好**：`t("…")` 的字面量必须写在调用点，静态扫描靠它认条目）。
  label: String,
  /// 每台机型对应的取值（与传入的顺序一致）。
  values: Vec<String>,
  /// 该行最好的是哪几台（并列最优时全列出）；空 = 没有「好坏」或不足两台有数据。
  best: Vec<usize>,
}

/// 缺数据时的占位文案。
fn not_listed() -> String {
  t("knowledge.not-listed")
}

/// 一台机型在某一轴上的文字取值（用有效值：用户测过的项盖过原表）。
fn axis_text(axis: Axis, gear: &Gear, ms: &UserMeasurements) -> String {
  let rx = gear_score::rx_values(gear.id, ms.get(gear.id));
  let Some(raw) = gear_score::value(axis, gear, &rx) else {
    return not_listed();
  };
  match axis {
    Axis::ImdNarrow => {
      // 窄间隔的间隔本身是数据的一部分（2 或 3 kHz）：原表值写它实测的间隔，
      // 用户自己测的那条按字段定义固定是 2 kHz。
      let khz = if ms.get(gear.id).and_then(|m| m.rmdr_db).is_some() {
        2.0
      } else {
        gear_rx::for_gear(gear.id).map_or(0.0, |r| r.narrow_khz)
      };
      format!("{raw:.0} dB（{khz:.0} kHz）")
    }
    Axis::ImdWide => format!("{raw:.0} dB"),
    Axis::NoiseFloor => format!("{raw:.0} dBm"),
    Axis::Power => format!("{raw:.0} W"),
    Axis::TopBand => format!("{raw:.0} MHz"),
    Axis::Modes => format!("{raw:.0}"),
  }
}

/// 某一轴上最好的是哪几台（并列最优时全列出；不足两台有数据时为空）。
fn best_on(axis: Axis, models: &[&Gear], ms: &UserMeasurements) -> Vec<usize> {
  let values: Vec<(usize, f64)> = models
    .iter()
    .enumerate()
    .filter_map(|(i, g)| {
      let rx = gear_score::rx_values(g.id, ms.get(g.id));
      gear_score::value(axis, g, &rx).map(|v| (i, gear_score::normalize(axis, v)))
    })
    .collect();
  if values.len() < 2 {
    return Vec::new();
  }
  // 归一化后再比，方向（越大越好 / 越小越好）就交给核心定义，界面不重复判；
  // 取到最优值后把并列的都标出来（`best` 就是其中某个值，`==` 是精确命中而非算出来的阈值）。
  let best = values
    .iter()
    .map(|(_, v)| *v)
    .fold(f64::NEG_INFINITY, f64::max);
  values
    .into_iter()
    .filter(|(_, v)| *v == best)
    .map(|(i, _)| i)
    .collect()
}

/// 对比表的行：行标题 + 各机型的取值 + 该行最好的是哪台。
///
/// 前五行是器材库里的高层次信息（不判好坏），后五行是实测指标与得分（标出该行最好）。
fn comparison_rows(models: &[&Gear], preset: &Preset, ms: &UserMeasurements) -> Vec<Row> {
  if models.is_empty() {
    return Vec::new();
  }
  let text = |f: fn(&Gear) -> String| models.iter().map(|g| f(g)).collect::<Vec<_>>();
  // 行标题由调用点传入（`t("…")` 的字面量必须在调用点，静态扫描才认得出条目）。
  let axis_row = |axis: Axis, label: String| Row {
    label,
    values: models.iter().map(|g| axis_text(axis, g, ms)).collect(),
    best: best_on(axis, models, ms),
  };
  let scores: Vec<gear_score::Score> = models
    .iter()
    .map(|g| gear_score::score(g, &gear_score::rx_values(g.id, ms.get(g.id)), preset))
    .collect();
  let best_score = {
    let with_points: Vec<(usize, f64)> = scores
      .iter()
      .enumerate()
      .filter_map(|(i, s)| s.points.map(|p| (i, p)))
      .collect();
    if with_points.len() < 2 {
      Vec::new()
    } else {
      let best = with_points
        .iter()
        .map(|(_, p)| *p)
        .fold(f64::NEG_INFINITY, f64::max);
      with_points
        .into_iter()
        .filter(|(_, p)| *p == best)
        .map(|(i, _)| i)
        .collect()
    }
  };
  vec![
    Row {
      label: t("knowledge.brand"),
      values: text(|g| g.brand.to_owned()),
      best: Vec::new(),
    },
    Row {
      label: t("knowledge.price-tier"),
      values: text(|g| g.tier.to_owned()),
      best: Vec::new(),
    },
    Row {
      label: t("knowledge.band"),
      values: text(|g| g.bands.to_owned()),
      best: Vec::new(),
    },
    Row {
      label: t("contest.power"),
      values: text(|g| g.power.to_owned()),
      best: Vec::new(),
    },
    Row {
      label: t("log.mode"),
      values: text(|g| g.modes.to_owned()),
      best: Vec::new(),
    },
    axis_row(Axis::ImdNarrow, t("knowledge.rx-imd-narrow")),
    axis_row(Axis::ImdWide, t("knowledge.rx-imd-wide")),
    axis_row(Axis::NoiseFloor, t("knowledge.rx-noise-floor")),
    Row {
      label: t("knowledge.rx-measured"),
      values: models
        .iter()
        .map(|g| gear_rx::for_gear(g.id).map_or_else(not_listed, |r| r.measured.to_owned()))
        .collect(),
      best: Vec::new(),
    },
    Row {
      label: t("knowledge.weighted-score"),
      values: scores
        .iter()
        .map(|s| {
          s.points
            .map_or_else(|| t("knowledge.score-insufficient"), |p| format!("{p:.0}"))
        })
        .collect(),
      best: best_score,
    },
    Row {
      label: t("knowledge.features"),
      values: text(|g| g.highlight.to_owned()),
      best: Vec::new(),
    },
    Row {
      label: t("knowledge.comments"),
      values: text(|g| g.note.to_owned()),
      best: Vec::new(),
    },
  ]
}

/// 雷达图上的序列：每台一条实线（原表/库里的值）；自己测过的机器再多一条同色虚线。
fn radar_series(models: &[&Gear], preset: &Preset, ms: &UserMeasurements) -> Vec<RadarSeries> {
  let axes = preset.axes();
  let series_values = |g: &Gear, rx: &gear_score::RxValues| -> Vec<f64> {
    axes
      .iter()
      .map(|&a| gear_score::value(a, g, rx).map_or(0.0, |raw| gear_score::normalize(a, raw)))
      .collect()
  };
  let mut out = Vec::new();
  for (i, g) in models.iter().enumerate() {
    let label = format!("{} {}", g.brand, g.model);
    let table = gear_score::rx_values(g.id, None);
    out.push(RadarSeries {
      label: label.clone(),
      color: i,
      values: series_values(g, &table),
      dashed: false,
    });
    let user = ms.get(g.id);
    // 只有「有基准」的实测值才画：电流与相位噪声在本库没有可比刻度（见 `gear_user`）。
    if user.is_some_and(|m| m.plottable_count() > 0) {
      out.push(RadarSeries {
        label: tf("knowledge.your-measurement", &[&label]),
        color: i,
        values: series_values(g, &table.with_user(user)),
        dashed: true,
      });
    }
  }
  out
}

#[component]
pub fn GearPage() -> impl IntoView {
  set_title("knowledge.gear-reviews-buying-guide");

  let category = RwSignal::new(GEAR_CATEGORIES[0].to_owned());
  let selected = RwSignal::new(default_selection(GEAR_CATEGORIES[0]));
  // 用途预设：决定雷达图看哪几条轴、加权得分怎么算。
  let preset_key = RwSignal::new(gear_score::DEFAULT_PRESET.to_owned());
  // 用户实测值：只存 localStorage（与器材库、原表值都分开）。
  let measurements = RwSignal::new(load_measurements());
  // 每个 (机型, 字段) 一个**独立**的草稿信号，承载「用户正在敲的原文」。
  //
  // 为什么必须有这一层：`Input` 的 `on_change` 挂在 `on:input` 上（每次按键都回调），
  // 落库又会通知订阅了 `measurements` 的录入区重新渲染 —— 若输入框的值直接由存储派生，
  // 每敲一个字符都会被「存储里的格式化文本」覆盖：敲 `2` 立刻变成 `2.0`，接着敲 `2`
  // 就成了 `2.02`（实测过：想输 22 会存下 2.02）。草稿只由用户输入改动，存储不再回写。
  //
  // 每个字段一个信号而不是共用一个 map：共用一个 map 时，任意字段的输入都会让**所有**
  // 输入框重算 `value`，光标会被顶到末尾。
  //
  // **信号必须建在组件这一层**：录入区每次重建都会清理掉自己的 owner，在渲染闭包里建的
  // 信号会跟着被 dispose，下一次读就是「已释放的响应式值」panic（实测踩过）。`GEAR` ×
  // `UserField::ALL` 是有限的静态组合（几十个），一次建完最省事，初值取存储里的值。
  let drafts = {
    let stored = measurements.get_untracked();
    let map = GEAR
      .iter()
      .flat_map(|g| {
        let record = stored.get(g.id).cloned().unwrap_or_default();
        UserField::ALL.map(|f| (draft_key(g.id, f), RwSignal::new(f.text(&record))))
      })
      .collect::<BTreeMap<String, RwSignal<String>>>();
    StoredValue::new(map)
  };
  // 取不到草稿时用这个（`GEAR` × `UserField::ALL` 已全覆盖，走不到；真走到了也只是
  // 显示空串，不该把页面炸掉）。
  let no_draft = RwSignal::new(String::new());
  // 落库：只有「能采信的值」或「清空」才写存储。半截输入（`-`、`1.`）与越界的值只留在
  // 草稿里 —— 这时候写进去，受控回显会把用户正在敲的文本抹掉（见 `drafts`）。
  let commit_measure = move |gear_id: &str, field: UserField, raw: String| {
    let text = raw.trim();
    let value = if text.is_empty() {
      None
    } else {
      match text.parse::<f64>() {
        Ok(v) if field.accepts(v) => Some(v),
        _ => return,
      }
    };
    measurements.update(|ms| {
      ms.set_field(gear_id, field, value);
      ms.stamp(gear_id, &utc_today());
    });
    crate::util::storage::set_json(MEASURE_KEY, &measurements.get_untracked());
  };
  let clear_measure = move |gear_id: &str| {
    measurements.update(|ms| {
      for field in UserField::ALL {
        ms.set_field(gear_id, field, None);
      }
    });
    crate::util::storage::set_json(MEASURE_KEY, &measurements.get_untracked());
    // 草稿要一起清，否则输入框还显示着刚被清掉的值。
    drafts.update_value(|m| {
      for field in UserField::ALL {
        if let Some(sig) = m.get(&draft_key(gear_id, field)) {
          sig.set(String::new());
        }
      }
    });
  };
  // 当前选中的机型（选择顺序 + 类别）与派生的行 / 雷达序列都放 Memo：
  // `t()` 只在 Memo 里才会订阅语言包版本，切语言时表头与行标题才会跟着变。
  let models = Memo::new(move |_| {
    let sel = selected.get();
    in_category(&category.get())
      .into_iter()
      .filter(|g| sel.iter().any(|id| id == g.id))
      .collect::<Vec<&'static Gear>>()
  });
  let preset =
    Memo::new(move |_| gear_score::preset(&preset_key.get()).unwrap_or(&gear_score::PRESETS[0]));

  let rows = Memo::new(move |_| comparison_rows(&models.get(), preset.get(), &measurements.get()));
  let series = Memo::new(move |_| radar_series(&models.get(), preset.get(), &measurements.get()));

  let switch_category = move |key: &str| {
    selected.set(default_selection(key));
    category.set(key.to_owned());
  };

  let toggle = move |id: &str| {
    let mut too_many = false;
    selected.update(|v| {
      if let Some(pos) = v.iter().position(|x| x == id) {
        v.remove(pos);
      } else if v.len() >= MAX_COMPARE {
        too_many = true;
      } else {
        v.push(id.to_owned());
      }
    });
    if too_many {
      let limit = MAX_COMPARE.to_string();
      alert(&tf("knowledge.you-can-compare-at", &[&limit]));
    }
  };

  view! {
    <KnowledgePage
      title=t("knowledge.gear-reviews-buying-guide")
      subtitle=t("knowledge.featured-radios-spec-comparison")
    >
      <TableSection
        title="knowledge.recommendations-by-use"
        headers=PICK_HEADERS
        rows=GEAR_PICKS
        min_width=720
      />

      <section class="rounded-xl border bg-card">
        <h2 class="border-b px-4 py-3 text-sm font-semibold">
          {move || t("knowledge.spec-comparison")}
        </h2>
        <p class="px-4 pt-3 text-xs text-muted-foreground">
          {move || t("knowledge.pick-a-category-and")}
        </p>

        // ── 器材类别（互斥，密度如工具条 → ChipGroup）──────────────────
        <ChipGroup
          value=Signal::derive(move || category.get())
          on_change=Callback::new(move |key: String| switch_category(&key))
          aria_label=Signal::derive(move || t("knowledge.equipment-type"))
          class="p-4 pb-2"
        >
          {GEAR_CATEGORIES
            .iter()
            .map(|&key| {
              // 文案先包成一层 `view!` 再交给 `Chip`：直接写 `{move || …}` 会被
              // `Children`（`FnOnce`）当即求值，语言包异步到达后就不会再重算了
              // （照 `nec/preset_chip.rs` 的写法）。
              let label = view! { {move || category_label(key)} }.into_any();
              view! { <Chip value=key.to_owned()>{label}</Chip> }
            })
            .collect_view()}
        </ChipGroup>

        // ── 机型（可多选、可一个都不选 → 独立的开关，不是单选组）─────────
        <div class="flex flex-wrap gap-1.5 px-4 pb-4">
          {move || {
            let cat = category.get();
            in_category(&cat)
              .into_iter()
              .map(|g| {
                let id = g.id;
                view! {
                  <ChipToggle
                    active=Signal::derive(move || selected.get().iter().any(|x| x == id))
                    on_change=Callback::new(move |_: bool| toggle(id))
                  >
                    {g.model}
                  </ChipToggle>
                }
              })
              .collect_view()
          }}
        </div>

        // ── 按用途加权：决定雷达图的轴与得分口径 ────────────────────────
        <div class="flex flex-wrap items-center gap-2 border-t px-4 py-3">
          <span class="text-xs font-medium text-muted-foreground">
            {move || t("knowledge.weigh-by-use")}
          </span>
          <ChipGroup
            value=Signal::derive(move || preset_key.get())
            on_change=Callback::new(move |key: String| preset_key.set(key))
            aria_label=Signal::derive(move || t("knowledge.weigh-by-use"))
          >
            {gear_score::PRESETS
              .iter()
              .map(|p| {
                let key = p.key;
                let label = view! { {move || preset_label(p)} }.into_any();
                view! { <Chip value=key.to_owned()>{label}</Chip> }
              })
              .collect_view()}
          </ChipGroup>
        </div>

        // 外层只负责「一台机型都没选」的判空；雷达图 / 得分 / 录入区 / 对比表各自成闭包，
        // 各自的订阅范围最小 —— 尤其是录入区：它**不能**订阅 `measurements`，否则每敲一个
        // 被采信的字符都会把输入框整块重建，快速连打时后面的字符会丢（见那里的注释）。
        {move || {
          if models.get().is_empty() {
            return view! {
              <p class="border-t px-4 py-4 text-sm text-muted-foreground">
                {move || t("knowledge.select-at-least-one")}
              </p>
            }
              .into_any();
          }
          view! {
            // ── 雷达图 ────────────────────────────────────────────
            <div class="space-y-3 border-t p-4">
              <div class="flex flex-wrap items-baseline justify-between gap-2">
                <h3 class="text-sm font-semibold">{move || t("knowledge.radar-title")}</h3>
                <span class="text-xs text-muted-foreground">
                  {move || t("knowledge.radar-hint")}
                </span>
              </div>
              {move || view! { <GearRadar axes=preset.get().axes() series=series.get() /> }}
            </div>

            // ── 加权得分 ──────────────────────────────────────────
            <div class="border-t px-4 py-3">
              <h3 class="mb-1 text-sm font-semibold">
                {move || t("knowledge.weighted-score")}
              </h3>
              <div class="divide-y">
                {move || {
                  let preset = preset.get();
                  let ms = measurements.get();
                  models
                    .get()
                    .iter()
                    .map(|g| {
                      let score =
                        gear_score::score(g, &gear_score::rx_values(g.id, ms.get(g.id)), preset);
                      let user_based = ms.get(g.id).is_some_and(|m| m.plottable_count() > 0);
                      view! {
                        <ScoreRow
                          label=format!("{} {}", g.brand, g.model)
                          score=score
                          user_based=user_based
                        />
                      }
                    })
                    .collect_view()
                }}
              </div>
            </div>

            // ── 我的实测值（只存本地）────────────────────────────
            // 这一块**刻意不订阅 `measurements`**：落库会通知订阅者，整块重建会连输入框与
            // 焦点一起换掉 —— 快速连打时后面的字符就丢了（e2e 的 `pressSequentially` 钉的
            // 就是这条）。输入框的值由草稿信号驱动（见 `drafts`），只有日期那一行自己订阅。
            <div class="border-t px-4 py-3">
              <h3 class="text-sm font-semibold">
                {move || t("knowledge.user-measurements")}
              </h3>
              <p class="mt-1 mb-3 text-xs text-muted-foreground">
                {move || t("knowledge.user-measure-hint")}
              </p>
              <div class="grid gap-3 sm:grid-cols-2">
                {move || {
                  models
                    .get()
                    .iter()
                    .map(|g| {
                      let gear_id = g.id;
                      view! {
                      <div class="rounded-lg border p-3">
                        <div class="mb-2 flex flex-wrap items-baseline justify-between gap-2">
                          <span class="text-xs font-medium">
                            {format!("{} {}", g.brand, g.model)}
                          </span>
                          // 日期这一行自己订阅存储：录入区不重建，它也得跟着更新。
                          <span class="text-[10px] text-muted-foreground">
                            {move || {
                              let on = measurements
                                .get()
                                .get(gear_id)
                                .map(|m| m.measured_on.clone())
                                .unwrap_or_default();
                              (!on.is_empty()).then(|| tf("knowledge.measured-on", &[&on]))
                            }}
                          </span>
                        </div>
                        <div class="grid gap-2 sm:grid-cols-2">
                          {UserField::ALL
                            .iter()
                            .map(|f| {
                              let field = *f;
                              // 机型名先拷出来：`Signal::derive` 的闭包要求 `'static`，
                              // 直接捕获 `g`（借用自 `models`）活不过闭包。
                              let model = g.model;
                              let input_id = unique_id("measure");
                              // 草稿（用户敲的原文）驱动回显，不由存储回写（见 `drafts`）。
                              let value = drafts
                                .with_value(|m| m.get(&draft_key(gear_id, field)).copied())
                                .unwrap_or(no_draft);
                              // 负数上界不能逐键钳上限（见 `NumberField::clamp_typed`）。
                              // 先算成变量：`view!` 的属性值里出现 `>` 会被当成标签结束。
                              let clamp_typed = field.max > 0.0;
                              // 越界 / 半截输入：留在输入框里（草稿），同时明确说「没保存」，
                              // 而不是让值悄悄弹回去。
                              let error = Signal::derive(move || {
                                let text = value.get();
                                let text = text.trim();
                                if text.is_empty()
                                  || text.parse::<f64>().is_ok_and(|v| field.accepts(v))
                                {
                                  return String::new();
                                }
                                tf(
                                  "knowledge.measure-out-of-range",
                                  &[&format!("{}", field.min), &format!("{}", field.max)],
                                )
                              });
                              view! {
                                <Field
                                  label=Signal::derive(move || field_label(field))
                                  r#for=input_id.clone()
                                  hint=Signal::derive(move || field_hint(field))
                                  error=error
                                >
                                  <div class="flex items-center gap-1.5">
                                    <NumberField
                                      id=input_id
                                      // 十进制文本框：`-126.5` 的每一步（`-`、`-126.`）都是
                                      // 用户敲的原样，才会被当成「还没敲完」而不是「清空」。
                                      kind=NumberKind::Decimal
                                      value=value
                                      on_change=Callback::new(move |v: String| {
                                        value.set(v.clone());
                                        commit_measure(gear_id, field, v)
                                      })
                                      step=field.step
                                      min=field.min
                                      max=field.max
                                      clamp_typed=clamp_typed
                                      invalid=Signal::derive(move || !error.get().is_empty())
                                      aria_label=Signal::derive(move || {
                                        // 同一页上三台机器都有这个字段：无障碍名里带上机型，
                                        // 屏幕阅读器与 e2e 才分得清是给哪台录的。
                                        format!("{} {}", model, field_label(field))
                                      })
                                    />
                                    <span class="text-xs text-muted-foreground">{field.unit}</span>
                                  </div>
                                </Field>
                              }
                            })
                            .collect_view()}
                        </div>
                        <div class="mt-2 flex items-center justify-between gap-2">
                          <p class="text-[10px] text-muted-foreground">
                            {move || t("knowledge.measure-no-baseline")}
                          </p>
                          <Button
                            variant=Variant::Ghost
                            size=Size::Sm
                            aria_label=format!("{} {}", g.model, t("exam.clear"))
                            on_click=Callback::new(move |_| clear_measure(gear_id))
                          >
                            {move || t("exam.clear")}
                          </Button>
                        </div>
                      </div>
                    }
                    })
                    .collect_view()
                }}
              </div>
            </div>

            // ── 参数对比（跟着 `rows` 与选中的机型变）───────────────
            <div class="overflow-x-auto border-t">
              {move || {
                let models = models.get();
                let rows = rows.get();
                view! {
                  <table class="w-full border-collapse text-sm" style="min-width: 640px">
                    <thead class="bg-muted/60 text-xs">
                      <tr>
                        <th class="border px-3 py-2 text-left">
                          {move || t("knowledge.specs")}
                        </th>
                        {models
                          .iter()
                          .map(|g| {
                            view! {
                              <th class="border px-3 py-2 text-left">
                                <div class="font-semibold">{g.brand}</div>
                                <div class="font-normal text-muted-foreground">{g.model}</div>
                              </th>
                            }
                          })
                          .collect_view()}
                      </tr>
                    </thead>
                    <tbody>
                      {rows
                        .into_iter()
                        .map(|row| {
                          let best = row.best;
                          view! {
                            <tr class="border-t transition-colors hover:bg-muted/40">
                              <th
                                scope="row"
                                class="border px-3 py-2 text-left align-top font-medium whitespace-nowrap"
                              >
                                {row.label}
                              </th>
                              {row
                                .values
                                .into_iter()
                                .enumerate()
                                .map(|(i, value)| {
                                  // 差异高亮：该行最好的一格加重（表下注释里说明「加粗 = 最好」）。
                                  let class = if best.contains(&i) {
                                    "border px-3 py-2 text-left align-top font-semibold text-foreground"
                                  } else {
                                    "border px-3 py-2 text-left align-top text-muted-foreground"
                                  };
                                  view! { <td class=class>{value}</td> }
                                })
                                .collect_view()}
                            </tr>
                          }
                        })
                        .collect_view()}
                    </tbody>
                  </table>
                }
              }}
            </div>
          }
            .into_any()
        }}

        <p class="border-t px-4 py-3 text-xs text-muted-foreground">
          {move || t("knowledge.this-table-compiles-objective")}
        </p>
        <p class="border-t px-4 py-3 text-xs text-muted-foreground">
          {move || t("knowledge.rig-reviews-link")}
          " "
          <a
            href="/rig-reviews"
            class="underline underline-offset-2 hover:text-foreground"
          >
            {move || t("knowledge.rig-reviews")}
          </a>
        </p>
        <p class="border-t px-4 py-3 text-xs text-muted-foreground">
          {move || t("knowledge.best-marked-bold")}
          " "
          {move || {
            tf(
              "knowledge.rx-metrics-from",
              &[&format!("{}({})", gear_rx::SOURCE, gear_rx::CHECKED_ON)],
            )
          }}
          " "
          <a
            href=gear_rx::SOURCE_URL
            target="_blank"
            rel="noopener noreferrer"
            class="underline underline-offset-2 hover:text-foreground"
          >
            {gear_rx::SOURCE_URL}
          </a>
        </p>
      </section>

      <BulletSection title="common.buying-advice" items=GEAR_TIPS />
    </KnowledgePage>
  }
}
