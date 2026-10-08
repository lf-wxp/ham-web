//! 浏览器 `Intl` 的薄封装：按当前界面语言格式化日期。
//!
//! 日期选择器要显示「星期几」「年月」，这些名称随语言变化。项目约定界面文案只放在
//! `data/i18n/`，但星期 / 月份属于浏览器已内建的区域数据 —— 交给 `Intl` 生成，
//! 既不新增词条，也不会在新增语言时漏翻。
//!
//! 全部在**响应式上下文**里调用（选择器弹层内），因此切换语言会随 [`crate::i18n::locale`]
//! 一起重算。

use std::cell::RefCell;
use std::collections::HashMap;

use leptos::prelude::Get;
use wasm_bindgen::{JsCast, JsValue};

/// 当前界面语言的 BCP-47 标签（中文取 `zh-CN`）。
fn locale_code() -> &'static str {
  crate::i18n::locale().get().intl_code()
}

/// 把 `{ property: value }` 组成本地化选项对象（等价 JS 对象字面量）。
fn options(props: &[(&str, &str)]) -> JsValue {
  let obj: JsValue = js_sys::Object::new().unchecked_into();
  for (k, v) in props {
    let _ = js_sys::Reflect::set(&obj, &JsValue::from_str(k), &JsValue::from_str(v));
  }
  obj
}

/// JS `Date` 的年份陷阱：`0..=99` 会被 `new Date(y, …)` 映射成 `1900 + y`。
///
/// 加 400 年即可规避（400 年是完整的闰年周期，星期与月相完全一致），否则 `0024-01-01`
/// 会按 `1924-01-01` 取星期。
///
/// **只用于「只取星期」的场合**（`date_picker::leading_blanks`）：加 400 之后
/// **年份本身**也变了，任何会显示年份的路径（[`self::fmt`]）都必须改用 [`date_of`]，
/// 否则 `0024-01-01` 会显示成「424 年」。
pub(super) fn js_year(y: i32) -> u32 {
  if (0..100).contains(&y) {
    (y + 400) as u32
  } else {
    y as u32
  }
}

/// 构造「公元 `y` 年 `m` 月 `d` 日」的 `Date`（`m` / `d` 从 1 开始）。
///
/// 用 `set_full_year` 而不是 `new Date(y, …)`：后者的 `0..100` 映射陷阱会把 `0024`
/// 变成 `1924`，而 [`js_year`] 的「加 400」只保住星期、保不住**显示出来的年份**。
/// `set_full_year` 按字面值设置，因此这里对任意年份都正确。
fn date_of(y: i32, m: u32, d: u32) -> js_sys::Date {
  let date = js_sys::Date::new_0();
  // `set_full_year` 按**字面值**设置年份（`1900 + y` 的偏移只作用于 `new Date(y, …)`
  // 这个构造函数），所以 `0024` 就是公元 24 年。接口是 `u32`，负数按 0 处理。
  date.set_full_year(y.max(0) as u32);
  date.set_month(m.saturating_sub(1));
  date.set_date(d);
  date
}

/// 日期文案缓存的键：`(界面语言, 形式, 年, 月, 日)`。
type CacheKey = (String, &'static str, i32, u32, u32);

thread_local! {
  /// 已算过的日期文案：`(界面语言, 形式, y, m, d)` → 文案。
  ///
  /// 日历面板每渲染一次就要为 42 个单元格各取一次 [`day_label`]，而
  /// `Date::to_locale_date_string` **每次调用都要重新解析 locale 与 options**
  /// （微秒到毫秒级）。同一月份被反复渲染时（重新打开面板、选完某天、切主题、
  /// 任意父级重渲）这些结果完全一样，没必要每次重算。
  ///
  /// 文案只取决于语言与日期，缓存是安全的：切语言后 `locale_code()` 变了，
  /// 键自然不同。
  static CACHE: RefCell<HashMap<CacheKey, String>> = RefCell::new(HashMap::new());
}

/// 缓存条数上限：超过就整体清空（文案本身很小，但有界才不会有长期膨胀的嫌疑）。
const CACHE_LIMIT: usize = 4096;

/// 按当前语言格式化 `y-m-d`（`m` / `d` 从 1 开始）。
fn fmt(y: i32, m: u32, d: u32, props: &[(&str, &str)]) -> String {
  let date = date_of(y, m, d);
  String::from(date.to_locale_date_string(locale_code(), &options(props)))
}

/// 带缓存的 [`fmt`]：`form` 区分「月标题」与「单日」，避免两种文案互相覆盖。
fn fmt_cached(form: &'static str, y: i32, m: u32, d: u32, props: &[(&str, &str)]) -> String {
  let key = (locale_code().to_owned(), form, y, m, d);
  if let Some(hit) = CACHE.with(|c| c.borrow().get(&key).cloned()) {
    return hit;
  }
  let text = fmt(y, m, d, props);
  CACHE.with(|c| {
    let mut cache = c.borrow_mut();
    if cache.len() >= CACHE_LIMIT {
      cache.clear();
    }
    cache.insert(key, text.clone());
  });
  text
}

/// 月份标题，如 `2026 年 10 月` / `October 2026`。
pub(super) fn month_title(y: i32, m: u32) -> String {
  fmt_cached("month", y, m, 1, &[("year", "numeric"), ("month", "long")])
}

/// 单日无障碍名称，如 `2026 年 10 月 7 日`。
pub(super) fn day_label(y: i32, m: u32, d: u32) -> String {
  fmt_cached(
    "day",
    y,
    m,
    d,
    &[("year", "numeric"), ("month", "long"), ("day", "numeric")],
  )
}

/// 周一到周日的短名称（共 7 项）。
pub(super) fn weekday_labels() -> [String; 7] {
  // 2024-01-01 是星期一：借它取「周一…周日」的本地化短名称。
  std::array::from_fn(|i| fmt(2024, 1, i as u32 + 1, &[("weekday", "short")]))
}
