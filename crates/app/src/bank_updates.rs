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
  let digest = Digest::of(questions);
  let old = storage::get_json::<Stored>(&key(bank));
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
