//! 轻量国际化：中文（默认）/ 英文 / 西班牙文界面文案。
//!
//! 词条以语义 key（`<domain>.<slug>`）索引：源码里写 `t("shell.home")`，中文与其它语言
//! 一样是 `data/i18n/` 里的一份译文。改中文文案不会再让其它语言的译文静默失效。
//!
//! 词条本体维护在 `data/i18n/{lang}/{domain}.json`（按语言 / 域拆分），由 `build.rs` 在
//! 构建期编成静态表；代码里不含任何词条，改文案不用碰 Rust 源文件。架构见
//! `docs/i18n-refactor.md`。
//!
//! 只有中文与内嵌域（见 `build.rs` 的 `BUNDLED_DOMAINS`）编进 wasm，其余域的译文由
//! [`pack`] 在运行时按语言拉一次 —— 拉包发生在挂载之前与切语言之前，因此 [`t`] 仍是
//! 同步的，也不会有「先中文后译文」的闪烁。
//!
//! 涉及数量的文案用 [`tp`]（如 `tp("common.days", n, &[&n.to_string()])`）：按 CLDR
//! 复数类别选词形，缺变体时回退 [`tf`] 的行为。中文没有复数变化，[`tp`] 在中文下
//! 直接走 [`tf`]。

mod catalog;
mod pack;

use std::cell::Cell;

use leptos::prelude::*;

use ham_web_core::plural;
use ham_web_core::saved_state::keys;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Locale {
  Zh,
  En,
  Es,
}

impl Locale {
  /// 全部可选语言（切换器按此顺序展示）。
  pub const ALL: &'static [Locale] = &[Locale::Zh, Locale::En, Locale::Es];

  #[must_use]
  pub const fn code(self) -> &'static str {
    match self {
      Self::Zh => "zh",
      Self::En => "en",
      Self::Es => "es",
    }
  }

  /// 语言自身名称（各语言下均显示本族名称，不翻译）。
  #[must_use]
  pub const fn label(self) -> &'static str {
    match self {
      Self::Zh => "中文",
      Self::En => "English",
      Self::Es => "Español",
    }
  }

  /// BCP-47 语言标签：`<html lang>` 与浏览器 `Intl` / `toLocaleDateString` 使用。
  ///
  /// 中文取 `zh-CN` 而非 `zh` —— `Intl` 默认会按更宽泛的 `zh` 给出繁体格式。
  #[must_use]
  pub const fn intl_code(self) -> &'static str {
    match self {
      Self::Zh => "zh-CN",
      Self::En => "en",
      Self::Es => "es",
    }
  }

  #[must_use]
  pub fn from_code(s: &str) -> Self {
    if s.eq_ignore_ascii_case("en") {
      Self::En
    } else if s.eq_ignore_ascii_case("es") {
      Self::Es
    } else {
      Self::Zh
    }
  }
}

thread_local! {
  /// 当前界面语言（全局，供 `t` / `set_title` 等非上下文场景读取）。
  ///
  /// 用 `ArcRwSignal` 而非 `RwSignal`：`RwSignal` 是 arena 分配的，会挂到创建时所在的
  /// 响应式 `Owner` 上并随其清理而释放；而这个信号是全站共享的，绝不能随某个页面
  /// `Owner` 一起失效。引用计数信号只要还有引用就一直有效。
  static LOCALE: ArcRwSignal<Locale> = ArcRwSignal::new(Locale::Zh);
  /// 最近一次「切语言」请求的序号。
  ///
  /// 语言包是异步到达的：点 es（开始拉包）→ 再点 en（内嵌域已就绪，立即生效）→
  /// es 包到达后 `set(Es)`，界面又变回西语，而 `localStorage` 里写的是 en ——
  /// 刷新前后两种语言。序号用来丢弃过期的加载结果。
  static SWITCH_SEQ: Cell<u64> = const { Cell::new(0) };
}

/// 运行时语言包的拉取超时（毫秒）。
///
/// 首屏挂载兜底（`main.rs` 的 `BOOT_TIMEOUT_MS`）取「本值 + 余量」：兜底若早于包超时，
/// 非内嵌域会先渲染成中文。
pub const PACK_TIMEOUT_MS: i32 = pack::TIMEOUT_MS;

/// 当前界面语言信号。
pub fn locale() -> ArcRwSignal<Locale> {
  LOCALE.with(|l| l.clone())
}

/// `localStorage` 里保存的语言（没有记录时是中文）。
fn stored_locale() -> Locale {
  crate::util::storage::get(keys::LOCALE).map_or(Locale::Zh, |s| Locale::from_code(&s))
}

/// 载入 `l` 的运行时语言包；中文没有包（全量内嵌），直接返回。
///
/// 挂载前与切语言前都要先跑一次，否则非内嵌域会先漏出中文。
pub async fn load_for(l: Locale) {
  if l != Locale::Zh {
    pack::load(l.code()).await;
  }
}

/// 挂载前调用：按 `localStorage` 里的设置把语言包拉好。
pub async fn load_stored() {
  load_for(stored_locale()).await;
}

/// 初始化语言：从 `localStorage` 读取并提供上下文，同时同步 `<html lang>`。
pub fn provide_locale() {
  let signal = locale();
  signal.set(stored_locale());
  Effect::new(move |_| {
    let l = signal.get();
    apply_lang(l);
    // 页面标题在进入页面时一次性写入，不在响应式上下文里 —— 切语言后要按新语言重设，
    // 否则浏览器标签会一直停留在旧语言。
    crate::util::refresh_title();
  });
}

fn apply_lang(l: Locale) {
  if let Some(root) = crate::util::document().document_element() {
    let _ = root.set_attribute("lang", l.intl_code());
  }
}

/// 切换语言并持久化。
///
/// 目标语言的运行时语言包若尚未载入，先载入再切：[`t`] 是同步查表，先切后载会让界面
/// 先翻成中文、包到达后再翻成译文。加载失败也照切 —— 内嵌域仍有译文，其余回退中文，
/// 总好过点了没反应。
pub fn set_locale(l: Locale) {
  crate::util::storage::set(keys::LOCALE, l.code());
  let seq = SWITCH_SEQ.with(|s| {
    let n = s.get().wrapping_add(1);
    s.set(n);
    n
  });
  if l == Locale::Zh || pack::ready(l.code()) {
    locale().set(l);
    return;
  }
  leptos::task::spawn_local(async move {
    pack::load(l.code()).await;
    // 等待期间用户又点过别的语言：这次的结果已经过期，直接丢弃，
    // 否则界面显示的语言会与 `localStorage` 里记的不一致。
    if SWITCH_SEQ.with(Cell::get) == seq {
      locale().set(l);
    }
  });
}

/// 翻译：按语义 key 查当前语言的词典。
///
/// 检索顺序：当前语言的内嵌域 → 当前语言的运行时语言包 → 中文 → 原样返回。中间会通过
/// [`catalog::resolve`] 兼容「中文原文」入参 —— 导航分组名、页面标题等来自 `crates/core`
/// registry 的文案是运行时传进来的，拿不到编译期 key，反向索引让它们继续按中文原文命中。
pub fn t(key: &str) -> String {
  // 订阅语言包版本：包异步到达后，已渲染的视图要重算（否则非内嵌域停在中文）。
  pack::track();
  // 渲染时调用极其频繁，直接读 thread_local，省掉 `locale()` 返回时的一次 `Arc` clone。
  match LOCALE.with(|l| l.get()) {
    Locale::Zh => zh(key),
    Locale::En => translate("en", catalog::en_map(), key),
    Locale::Es => translate("es", catalog::es_map(), key),
  }
}

/// 中文侧：命中则返回中文原文；未命中按语义 key 原样返回（由 `check-i18n` 在 CI 里兜住）。
fn zh(key: &str) -> String {
  zh_resolved(catalog::resolve(key))
}

/// 已还原成 key 的中文查表（中文没有复数变化，不查复数表）。
fn zh_resolved(k: &str) -> String {
  catalog::zh_map()
    .get(k)
    .map_or_else(|| k.to_owned(), |v| (*v).to_owned())
}

/// 非中文侧：内嵌表 → 内嵌复数表的 `other` → 运行时语言包 → 中文。
fn translate(
  lang: &str,
  bundled: &'static std::collections::HashMap<&'static str, &'static str>,
  key: &str,
) -> String {
  let k = catalog::resolve(key);
  lookup(lang, bundled, k).unwrap_or_else(|| zh_resolved(k))
}

/// 三级查表：扁平表 → 复数表的 `other` 变体 → 运行时语言包（含同样的复数回退）。
fn lookup(
  lang: &str,
  bundled: &'static std::collections::HashMap<&'static str, &'static str>,
  k: &str,
) -> Option<String> {
  if let Some(v) = bundled.get(k) {
    return Some((*v).to_owned());
  }
  if let Some(v) = catalog::plural_other(lang, k) {
    return Some(v.to_owned());
  }
  pack::get(lang, k)
}

/// 带占位符的翻译：把词条中的 `{}` 按顺序替换为 `args`（数量不足时保留剩余占位符）。
pub fn tf(key: &str, args: &[&str]) -> String {
  substitute(t(key), args)
}

/// 带数量的翻译：按 CLDR 复数类别选词形变体，再替换占位符。
///
/// `count` **单独传、不占 `args`**：决定复数的量未必是第一个占位符
/// （如 `common.questions-left-unseen-in` = `{} 类还有 {} 题没做过`，决定复数的是
/// 第 2 个占位符）。
///
/// 检索顺序：内嵌复数表（按类别）→ 运行时语言包的复数表 → [`t`]（`other` 变体 /
/// 扁平词条 / 中文）。**没有复数变体时与 [`tf`] 完全等价**，因此迁移可以一条一条做，
/// 不必一次改完。
///
/// 中文没有复数变化，直接走 [`tf`] —— zh 全量扁平内嵌，零额外开销。
pub fn tp(key: &str, count: impl Into<plural::Count>, args: &[&str]) -> String {
  // 复数变体可能整条来自运行时语言包（不经 [`t`]），所以这里也要订阅包版本。
  pack::track();
  let n = count.into();
  match LOCALE.with(|l| l.get()) {
    Locale::Zh => tf(key, args),
    Locale::En => tp_in("en", catalog::en_map(), key, n, args),
    Locale::Es => tp_in("es", catalog::es_map(), key, n, args),
  }
}

/// 非中文侧的 [`tp`]：复数表未命中时回退 [`t`] 的整条链路。
fn tp_in(
  lang: &str,
  bundled: &'static std::collections::HashMap<&'static str, &'static str>,
  key: &str,
  n: plural::Count,
  args: &[&str],
) -> String {
  let k = catalog::resolve(key);
  let category = plural::category(lang, n);
  let text = catalog::plural(lang, k, category)
    .map(str::to_owned)
    .or_else(|| pack::plural(lang, k, category))
    .or_else(|| lookup(lang, bundled, k))
    .unwrap_or_else(|| zh_resolved(k));
  substitute(text, args)
}

/// 把词条中的 `{}` 按顺序替换为 `args`（数量不足时保留剩余占位符）。
///
/// 每轮从**上次替换结束的位置**往后找：某段实参文本自身带 `{}` 时（呼号、备注、
/// 搜索词都可能带），从头找会把那个 `{}` 当成下一个占位符、用后面的实参覆盖它，
/// 参数就整体错位了。
fn substitute(mut s: String, args: &[&str]) -> String {
  let mut from = 0;
  // 「实参多于占位符」的开发期提示只在 wasm 下编译（`web_sys` 原生下会 panic），
  // 计数器也跟着只在 wasm 下声明，原生目标不留下「只写不读」的死变量。
  #[cfg(target_arch = "wasm32")]
  let mut used = 0;
  for a in args {
    let Some(rel) = s[from..].find("{}") else {
      break;
    };
    let pos = from + rel;
    s.replace_range(pos..pos + 2, a);
    from = pos + a.len();
    #[cfg(target_arch = "wasm32")]
    {
      used += 1;
    }
  }
  // 实参多于 `{}`：调用点写错了，多传的参数会被静默丢掉（页面上只表现为「少半句话」）。
  // 开发期给一条信号，生产环境不打扰用户。整段只在 wasm 下编译：`web_sys` 在原生
  // 目标（单测）下调用即 panic。
  #[cfg(target_arch = "wasm32")]
  if cfg!(debug_assertions) && used < args.len() {
    web_sys::console::warn_1(
      &format!(
        "i18n: 实参多于占位符（{} 个实参 / {used} 个占位符）：{s}",
        args.len()
      )
      .into(),
    );
  }
  s
}

/// 题库类别标签（按语言调整词序）：中文「B 类」、英文「Class B」、西班牙文「Clase B」。
pub fn bank_class(bank: &str) -> String {
  match locale().get() {
    Locale::Zh => format!("{bank} 类"),
    Locale::En => format!("Class {bank}"),
    Locale::Es => format!("Clase {bank}"),
  }
}

#[cfg(test)]
mod tests {
  use crate::i18n::{catalog, pack, tp_in};
  use leptos::prelude::{Get, Set};

  use super::{Locale, locale, t, tf, tp};

  /// 切换语言后断言，再切回中文（测试之间的顺序不影响，其余测试都期待默认中文）。
  fn with_locale<F: FnOnce()>(l: Locale, f: F) {
    let signal = locale();
    let prev = signal.get();
    signal.set(l);
    f();
    signal.set(prev);
  }

  /// `common.days` 是复数词条：英文 1 天是 `1 day`，2 天才是 `2 days`。
  #[test]
  fn tp_picks_the_plural_variant_by_count() {
    with_locale(Locale::En, || {
      assert_eq!(tp("common.days", 1, &["1"]), "1 day");
      assert_eq!(tp("common.days", 2, &["2"]), "2 days");
      assert_eq!(tp("common.days", 0, &["0"]), "0 days");
    });
    with_locale(Locale::Es, || {
      assert_eq!(tp("common.days", 1, &["1"]), "1 día");
      assert_eq!(tp("common.days", 2, &["2"]), "2 días");
    });
    // 中文没有复数变化，`tp` 直接走 `tf`
    assert_eq!(tp("common.days", 1, &["1"]), "1 天");
    assert_eq!(tp("common.days", 2, &["2"]), "2 天");
  }

  /// count 与 args 解耦：决定复数的量未必是第一个占位符
  ///（`common.questions-left-unseen-in` 的 count 是第 2 个 `{}`）。
  #[test]
  fn tp_counts_the_placeholder_that_decides_the_form() {
    with_locale(Locale::En, || {
      assert_eq!(
        tp("common.questions-left-unseen-in", 1, &["A", "1"]),
        "Class A: 1 question left unseen"
      );
      assert_eq!(
        tp("common.questions-left-unseen-in", 3, &["A", "3"]),
        "Class A: 3 questions left unseen"
      );
      assert_eq!(tp("common.questions-3", 1, &["1"]), "1 question");
    });
    with_locale(Locale::Es, || {
      assert_eq!(
        tp("common.questions-left-unseen-in", 1, &["A", "1"]),
        "Clase A: queda 1 pregunta sin hacer"
      );
    });
  }

  /// 从**源词典**（`data/i18n/{lang}/{domain}.json`，作者层的入库文件）摘出指定 key，
  /// 塞进运行时包缓存。
  ///
  /// 刻意不读 `public/data/i18n/{lang}.json`：那是 `i18n-pack` 的产物，干净检出后若还没
  /// 跑过 `cargo make i18n-pack` 就不存在，读它会让 `cargo test` 以 panic 收场
  /// （而不是给出一条有意义的断言失败）。源词典是手写并入库的，永远是唯一事实源。
  fn seed_from_sources(lang: &str, domains: &[&str], keys: &[&str]) {
    let root = format!("{}/../../data/i18n/{lang}", env!("CARGO_MANIFEST_DIR"));
    let mut flat = serde_json::Map::new();
    let mut plural = serde_json::Map::new();
    for domain in domains {
      let path = format!("{root}/{domain}.json");
      let text =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("读不到源词典 {path}：{e}"));
      let value: serde_json::Value = serde_json::from_str(&text).expect("源词典必须是合法 JSON");
      let map = value
        .as_object()
        .unwrap_or_else(|| panic!("{path} 顶层应为对象"));
      for key in keys {
        let Some(v) = map.get(*key) else { continue };
        if v.is_string() {
          flat.insert((*key).to_owned(), v.clone());
        } else {
          plural.insert((*key).to_owned(), v.clone());
        }
      }
    }
    let payload = serde_json::json!({ "flat": flat, "plural": plural }).to_string();
    pack::seed_for_test(lang, &payload);
  }

  /// 第二批（exam / learning / log）：count 在第 2 个占位符，以及西语动词随数变化。
  ///
  /// 这三个域不是内嵌域，译文只在运行时包里：从源词典摘出这几条塞进包缓存，再走
  /// [`tp_in`] —— 顺便钉住「包里的复数变体与源码里的 count 对得上」。
  /// 不切全局 locale：locale 是全局信号，并发测试会互相踩。
  #[test]
  fn tp_counts_the_second_placeholder_and_agrees_in_spanish() {
    const DOMAINS: [&str; 3] = ["exam", "log", "learning"];
    const KEYS: [&str; 4] = [
      "exam.questions-2",
      "log.grid-qsos",
      "log.records",
      "learning.days-to-exam",
    ];
    seed_from_sources("en", &DOMAINS, &KEYS);
    seed_from_sources("es", &DOMAINS, &KEYS);
    let en = |key, n: u64, args: &[&str]| tp_in("en", catalog::en_map(), key, n.into(), args);
    let es = |key, n: u64, args: &[&str]| tp_in("es", catalog::es_map(), key, n.into(), args);
    assert_eq!(en("exam.questions-2", 1, &["A", "1"]), "A: 1 question");
    assert_eq!(en("exam.questions-2", 3, &["A", "3"]), "A: 3 questions");
    assert_eq!(en("log.grid-qsos", 1, &["PM95", "1"]), "Grid PM95 · 1 QSO");
    assert_eq!(en("log.grid-qsos", 4, &["PM95", "4"]), "Grid PM95 · 4 QSOs");
    assert_eq!(
      es("learning.days-to-exam", 1, &["1"]),
      "Falta 1 día para el examen"
    );
    assert_eq!(
      es("learning.days-to-exam", 3, &["3"]),
      "Faltan 3 días para el examen"
    );
    assert_eq!(es("log.records", 1, &["1"]), "1 registro");
    assert_eq!(es("log.records", 9, &["9"]), "9 registros");
  }

  /// 没有复数变体的词条：`tp` 与 `tf` 完全等价 —— 迁移因此能一条一条做。
  #[test]
  fn tp_equals_tf_for_plain_entries() {
    with_locale(Locale::En, || {
      // 逐字比对（而不是「两边都非空」这种恒真的比较）：无占位符与带占位符各一条。
      assert_eq!(tp("shell.home", 1, &[]), tf("shell.home", &[]));
      assert_eq!(tp("shell.home", 1, &[]), t("shell.home"));
      assert_eq!(
        tp("common.zone-2", 1, &["1", "2"]),
        tf("common.zone-2", &["1", "2"])
      );
    });
  }

  /// `substitute` 的契约（三者都是「静默」的边界行为，必须钉住，别哪天变成 panic）。
  #[test]
  fn substitute_contract_is_pinned() {
    use super::substitute;
    // 实参多于占位符：多余的丢掉（开发期另有一条 console 告警）。
    assert_eq!(substitute("a {} b {}".into(), &["1", "2", "3"]), "a 1 b 2");
    // 实参不足：剩余 `{}` 原样保留。
    assert_eq!(substitute("a {} b {}".into(), &["1"]), "a 1 b {}");
    // 实参自带 `{}`：不能当作下一个占位符（否则参数整体错位）。
    assert_eq!(substitute("{}“{}”".into(), &["x{}y", "z"]), "x{}y“z”");
  }

  /// 复数词条被老调用点 `t(key)` 查到时返回 `other` 变体，不会漏出中文。
  #[test]
  fn t_falls_back_to_the_other_variant() {
    with_locale(Locale::En, || {
      assert_eq!(t("common.days"), "{} days");
    });
  }
}
