//! 题库更新检测：记住每个题库上次的内容摘要，内容变化后通过全局事件通知界面。

use ham_web_core::bank_digest::{Digest, diff};
use ham_web_core::{Bank, QuestionItem};
use serde::{Deserialize, Serialize};
use wasm_bindgen::JsValue;
use web_sys::{CustomEvent, CustomEventInit};

use crate::data;
use crate::i18n::tf;
use crate::util::{storage, window};

/// 题库内容变化时派发的事件，`detail` 为提示文字。
pub const EVENT: &str = "ham:bank-updated";

#[derive(Serialize, Deserialize)]
struct Stored {
  #[serde(default)]
  rev: String,
  digest: Digest,
}

fn key(bank: Bank) -> String {
  format!("bank-digest:{bank}")
}

/// 题库加载后调用：与上次记录的摘要比较，有变化时派发 [`EVENT`]。首次见到的题库只记录不提示。
pub fn observe(bank: Bank, rev: &str, questions: &[QuestionItem]) {
  let old = storage::get_json::<Stored>(&key(bank));
  // `rev` 是构建后处理写入的内容哈希：与上次相同就说明文件没变，摘要也不会变，
  // 不必逐题重算指纹（每个会话首次打开练习 / 题库浏览 / 术语表都要算一遍，手机上几十毫秒）。
  // 开发环境没有 `rev`（空串），照旧逐题比较。
  if !rev.is_empty() && old.as_ref().is_some_and(|old| old.rev == rev) {
    return;
  }
  let digest = Digest::of(questions);
  if let Some(old) = &old {
    if old.digest == digest {
      if old.rev != rev {
        storage::set_json(
          &key(bank),
          &Stored {
            rev: rev.to_owned(),
            digest,
          },
        );
      }
      return;
    }
    let d = diff(&old.digest, &digest);
    if !d.is_empty() {
      notify(&tf(
        "common.class",
        &[&(bank).to_string(), &(d.describe()).to_string()],
      ));
    }
  }
  storage::set_json(
    &key(bank),
    &Stored {
      rev: rev.to_owned(),
      digest,
    },
  );
}

fn notify(text: &str) {
  let init = CustomEventInit::new();
  init.set_detail(&JsValue::from_str(text));
  if let Ok(e) = CustomEvent::new_with_event_init_dict(EVENT, &init) {
    let _ = window().dispatch_event(&e);
  }
}

/// 启动时检查：之前用过的题库若修订号已变，立即重新加载一次以便提示更新内容。
pub async fn check_on_start() {
  let Ok(cfg) = data::load_config(false).await else {
    return;
  };
  for bank in Bank::ALL {
    let Some(old) = storage::get_json::<Stored>(&key(bank)) else {
      continue;
    };
    if cfg
      .rev_of(&bank.default_url())
      .is_some_and(|rev| rev != old.rev)
    {
      let _ = data::load_bank(None, bank, false).await;
    }
  }
}

/// 要预热的题库：A 类永远在内（底部栏的「练习」默认进 A 类），其余只预热以前用过的
/// （本地有摘要记录 = 加载过）。没用过 B / C 的人不为它们白下几百 KB。
#[must_use]
pub fn prewarm_banks() -> Vec<Bank> {
  Bank::ALL
    .into_iter()
    .filter(|&b| b == Bank::A || storage::get_json::<Stored>(&key(b)).is_some())
    .collect()
}

/// 空闲时把常用题库提前下载、解析进内存缓存（`data::load_bank` 的 `CACHE`）。
///
/// 首次进入练习 / 题库浏览 / 术语表 / 考试时，页面要先等题库下载 + 解析（解析在手机上
/// 几十到上百毫秒），这段时间新页面只能显示骨架。预热之后这些页面拿到的是内存里的现成数据。
/// 每个题库各等一次空闲：解析是一个整块的同步任务，几个题库连着解析会形成长任务。
/// 用户开了「省流量」（`navigator.connection.saveData`）或页面在后台时不预热。
pub async fn prewarm() {
  for bank in prewarm_banks() {
    if save_data() || crate::util::document().hidden() {
      return;
    }
    crate::after_paint::idle(std::time::Duration::from_secs(5)).await;
    let _ = data::load_bank(None, bank, false).await;
  }
}

/// 用户是否开了「省流量」。读不到（Firefox / Safari 没有这个接口）按「没开」算。
fn save_data() -> bool {
  let nav = window().navigator();
  js_sys::Reflect::get(&nav, &"connection".into())
    .ok()
    .filter(|c| c.is_object())
    .and_then(|c| js_sys::Reflect::get(&c, &"saveData".into()).ok())
    .and_then(|v| v.as_bool())
    .unwrap_or(false)
}
