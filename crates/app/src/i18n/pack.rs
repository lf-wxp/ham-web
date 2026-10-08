//! 运行时语言包：非内嵌域的译文不编进 wasm，按语言拉一次。
//!
//! [`crate::i18n::t`] 是同步查表，而包是异步到达的 —— 因此两处入口都**先载包再切换**：
//! 挂载之前（[`crate::i18n::load_stored`]）与切语言之前（[`crate::i18n::set_locale`]）。
//! 这样界面不会「先渲染中文再翻成译文」，调用点也一字不用改。
//!
//! 拉取失败不致命：内嵌域（`common` / `shell`）的译文仍在 wasm 里，其余回退中文 ——
//! 因此失败后的界面是**混排**（导航/按钮内嵌域用译文、其它域中文），不是整站中文。
//! 失败后按 [`RETRY_DELAY_MS`] 自动重试最多 [`MAX_RETRIES`] 次，成功即 bump `READY` 自愈。
//!
//! 包由 `ham-web-tools i18n-pack` 从 `data/i18n/{lang}/` 生成到 `public/data/i18n/{lang}.json`，
//! Trunk 在 `trunk serve`（开发）与 `trunk build`（发布）都会把 `public/` 拷进产物目录，
//! 因此开发与发布用的是同一份文件。版本信号不需要额外的 `?v=`：`sw.js` 的预缓存清单
//! 带内容哈希，包一变，`__CACHE_VERSION__` 就变，离线缓存随之刷新。
//!
//! 包里分两部分：扁平词条 `flat` 与复数词条 `plural`（`key → { 类别 → 文本 }`）。
//! 复数词条与扁平词条互斥，查表时按 [`Category`] 取变体，缺则回退 `other`。

use std::cell::RefCell;
use std::collections::HashMap;

use ham_web_core::plural::Category;
use leptos::prelude::{ArcRwSignal, Track, Update};

use crate::data::{fetch_text_with_timeout, retry_allowed};
use crate::util::now_ms;

/// 拉取超时（毫秒）：挂载前同步等待，卡太久会一直停在启动页，因此设上限。
///
/// 取 3 秒而不是更长的原因：非中文用户的首屏**完全**压在这次网络请求上（内嵌域之外的
/// 译文都在包里），而慢网下等到 6 秒才放人进去、用户看到的是「打开就卡住」。
/// 超时后不是永久退化：`schedule_retry` 会在几秒内补拉一次，包一到 `READY` 就 bump，
/// 已渲染的视图自动重算成正确语言。
///
/// `crate::i18n::PACK_TIMEOUT_MS` 会把它透出去，供首屏挂载兜底取「包超时 + 余量」。
pub(super) const TIMEOUT_MS: i32 = 3000;

/// 失败后的重试间隔（毫秒）与最大次数。
///
/// 首屏那次拉取失败若不再试，页面会一直停在「内嵌域是译文、其余是中文」的混排状态，
/// 直到用户手动切一次语言。重试次数有界：离线时不该无限打必然失败的请求。
const RETRY_DELAY_MS: u32 = 3_000;
const MAX_RETRIES: u32 = 3;

/// 一种语言的运行时语言包。
struct Pack {
  /// 普通词条：`key → 文本`。
  flat: HashMap<String, String>,
  /// 复数词条：`key → (类别 → 文本)`。
  plural: HashMap<String, HashMap<String, String>>,
}

/// 包的 JSON 结构（`i18n-pack` 的产物）；两个字段都允许缺失，便于向前兼容。
#[derive(serde::Deserialize)]
struct PackPayload {
  #[serde(default)]
  flat: HashMap<String, String>,
  #[serde(default)]
  plural: HashMap<String, HashMap<String, String>>,
}

thread_local! {
  /// 已载入的语言包：`语言 → 包`。
  ///
  /// 用 `RefCell` 而非 `OnceLock`：切语言要整体替换，而 `OnceLock` 只在首次写入生效。
  /// 所有读写都在主线程（wasm 单线程），不存在并发竞争。
  static PACKS: RefCell<HashMap<String, Pack>> = RefCell::new(HashMap::new());
  /// 每种语言最近一次拉取失败的时间戳（毫秒）；没有记录表示从未失败。
  static FAILED_AT: RefCell<HashMap<String, i64>> = RefCell::new(HashMap::new());
  /// 每种语言已经排队过的重试次数（见 `schedule_retry`）。
  static RETRY_QUEUED: RefCell<HashMap<String, u32>> = RefCell::new(HashMap::new());
  /// 语言包载入的版本号。
  ///
  /// 包是**异步**到达的，而 [`crate::i18n::t`] 是同步查表：不通知响应式系统的话，
  /// 在包到达之前渲染过的视图不会重算，非内嵌域会一直停在中文。首屏兜底挂载
  /// （`main.rs` 的 `BOOT_TIMEOUT_MS`）与「拉取失败退避后重试成功」都会走到这个场景。
  ///
  /// 用 `ArcRwSignal` 而非 `RwSignal`：`RwSignal` 是 arena 分配的，会随创建时所在的
  /// 响应式 `Owner` 一起被释放，而这里的订阅发生在任意页面组件内部。
  static READY: ArcRwSignal<u32> = ArcRwSignal::new(0);
}

/// 订阅语言包的载入状态；包到达后调用方会重新求值（在非响应式上下文里是空操作）。
pub(super) fn track() {
  READY.with(|s| s.track());
}

/// `lang` 的语言包是否已载入（中文没有包，视为已就绪）。
#[must_use]
pub(super) fn ready(lang: &str) -> bool {
  PACKS.with(|c| c.borrow().contains_key(lang))
}

/// 查运行时语言包的普通词条；未载入或没有该 key 时 `None`（调用方回退内嵌表与中文）。
///
/// 扁平表里查不到时继续查复数表的 `other` 变体 —— 这样「词典里是复数对象、源码里仍
/// 写 `t(key)`」的调用点不会漏出中文。
#[must_use]
pub(super) fn get(lang: &str, key: &str) -> Option<String> {
  PACKS.with(|c| {
    c.borrow().get(lang).and_then(|p| {
      p.flat
        .get(key)
        .cloned()
        .or_else(|| variant(&p.plural, key, Category::Other.as_str()))
    })
  })
}

/// 查运行时语言包里 `key` 在 `category` 下的复数变体；没有该类别时回退 `other`。
#[must_use]
pub(super) fn plural(lang: &str, key: &str, category: Category) -> Option<String> {
  PACKS.with(|c| {
    c.borrow()
      .get(lang)
      .and_then(|p| variant(&p.plural, key, category.as_str()))
  })
}

/// 变体查表：`category` 缺失时回退 `other`（兜底类别）。
fn variant(
  plural: &HashMap<String, HashMap<String, String>>,
  key: &str,
  category: &str,
) -> Option<String> {
  let variants = plural.get(key)?;
  variants
    .get(category)
    .cloned()
    .or_else(|| variants.get("other").cloned())
}

/// 测试用：直接塞入一份包，免去网络拉取。
///
/// 非内嵌域（`exam` / `learning` / `log`…）的译文只在运行时包里，单测没有网络，
/// 只能这样验证它们的复数变体确实被 [`super::tp`] 选中。
#[cfg(test)]
pub(super) fn seed_for_test(lang: &str, payload: &str) {
  // 解析失败要直接炸：静默 return 会被当成「该语言没有这个 key」，把 fixture 写坏
  // 伪装成「复数变体没被选中」。
  let d =
    serde_json::from_str::<PackPayload>(payload).expect("测试 fixture 必须是合法的语言包 JSON");
  PACKS.with(|c| {
    c.borrow_mut().insert(
      lang.to_owned(),
      Pack {
        flat: d.flat,
        plural: d.plural,
      },
    )
  });
}

/// 载入 `lang` 的语言包；已载入、或处在失败退避窗口内时直接返回。
///
/// 失败一律**说出来**（`console.error`）：静默回退中文时，线上只会表现为「某几个域的
/// 文案没翻」，没有任何信号。空词典同样按失败处理（不落常驻缓存、写退避时间戳）：
/// 「文件在、但一条词条都没有」只可能是该语言还没导出或产物损坏，让后续请求还能重试。
pub(super) async fn load(lang: &str) {
  if ready(lang) || !can_retry(lang) {
    return;
  }
  // 包里含全部域（连内嵌域一起）：多出的几十 KB 换来「不必在构建脚本与生成脚本两处
  // 维护同一份内嵌域清单」，查表时内嵌表优先，结果一致。
  let url = format!("/data/i18n/{lang}.json");
  let dict = match fetch_text_with_timeout(&url, TIMEOUT_MS).await {
    Ok(text) => match serde_json::from_str::<PackPayload>(&text) {
      Ok(d) => Some(d),
      Err(e) => {
        web_sys::console::error_1(&format!("[i18n] 语言包 {lang} 解析失败：{e}").into());
        None
      }
    },
    Err(e) => {
      web_sys::console::error_1(&format!("[i18n] 语言包 {lang} 拉取失败：{e}").into());
      None
    }
  };
  match dict.filter(|d| !d.flat.is_empty() || !d.plural.is_empty()) {
    Some(d) => {
      let pack = Pack {
        flat: d.flat,
        plural: d.plural,
      };
      PACKS.with(|c| c.borrow_mut().insert(lang.to_owned(), pack));
      FAILED_AT.with(|c| c.borrow_mut().remove(lang));
      // 通知已渲染的视图：非内嵌域的译文现在查得到了。
      READY.with(|s| s.update(|n| *n = n.wrapping_add(1)));
    }
    None => {
      FAILED_AT.with(|c| c.borrow_mut().insert(lang.to_owned(), now_ms()));
      schedule_retry(lang);
    }
  }
}

/// 拉取失败后安排一次重试（间隔 [`RETRY_DELAY_MS`]，最多 [`MAX_RETRIES`] 次）。
///
/// 没有它，首屏那次失败就再也没有第二次机会：`load` 只有「挂载前」与「用户切语言」两个
/// 调用点，页面会一直停在混排语言上（内嵌域是译文、其余是中文），且没有任何提示。
/// 重试成功会 bump `READY`，已渲染的视图随之重算 —— 与「包晚于挂载到达」是同一套自愈机制。
fn schedule_retry(lang: &str) {
  let attempt = RETRY_QUEUED.with(|c| {
    let mut queued = c.borrow_mut();
    let n = queued.entry(lang.to_owned()).or_insert(0);
    *n += 1;
    *n
  });
  if attempt > MAX_RETRIES {
    return;
  }
  let lang = lang.to_owned();
  // `set_timeout` + `wasm_bindgen_futures::spawn_local` 而不是 `leptos::task::spawn_local`：
  // 首次拉取发生在挂载之前，那时 Leptos 的执行器还没注册（同 `main.rs` 的理由）。
  leptos::prelude::set_timeout(
    move || {
      // 先清掉失败时间戳：`can_retry` 用的是与题库等资源共用的 30 秒退避窗口，
      // 而首屏语言包需要更短的节奏；重复排队已由上面的次数上限挡住。
      FAILED_AT.with(|c| c.borrow_mut().remove(&lang));
      let lang = lang.clone();
      wasm_bindgen_futures::spawn_local(async move {
        load(&lang).await;
      });
    },
    std::time::Duration::from_millis(u64::from(RETRY_DELAY_MS)),
  );
}

/// 距上次失败是否已超过退避窗口（可以再试一次）；退避判定本身在 `data` 里，与题库等
/// 其它按需加载的资源共用一套语义。
fn can_retry(lang: &str) -> bool {
  retry_allowed(FAILED_AT.with(|c| c.borrow().get(lang).copied().unwrap_or(0)))
}
