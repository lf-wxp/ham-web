//! 语音合成朗读（Web Speech API）。
//!
//! 朗读语言分两类，用错会念得很难听，调用前先分清：
//!
//! - **内容语言**：被朗读的文本本身是什么语言。字母解释法的代表单词（Alpha / Bravo）
//!   与呼号拼读恒为英文；题库题干与解析目前只有中文。分别用 [`speak_en`] / [`speak_zh`]。
//! - **界面语言**：随用户在导航栏切换的语言，只用于朗读**已随界面翻译**的文案。
//!
//! 标签统一由 [`speak_with`] 设置，语言只是一等参数，不再散落各处硬编码。

use web_sys::SpeechSynthesisUtterance;

/// 题库与知识库正文的语言标签（BCP-47）。
///
/// 目前题库内容只有中文，故恒为 `zh-CN`。单独导出（而不是在听题模式等处各写一份）
/// 是为了在题库内容完成国际化后只有**一个**地方需要改。
#[must_use]
pub const fn content_lang() -> &'static str {
  "zh-CN"
}

/// 字母解释法与呼号拼读的语言标签：ITU 规定的英文单词，与界面语言无关。
const EN_LANG: &str = "en-US";

/// 按指定 BCP-47 语言标签朗读一段文本。
///
/// 浏览器没有对应语音包时会退回到默认语音，不会报错。
pub fn speak_with(text: &str, lang: &str) {
  let Ok(synth) = crate::util::window().speech_synthesis() else {
    return;
  };
  // 停止当前朗读，避免连续点击叠加
  synth.cancel();
  let Ok(utterance) = SpeechSynthesisUtterance::new() else {
    return;
  };
  utterance.set_text(text);
  utterance.set_lang(lang);
  synth.speak(&utterance);
}

/// 朗读一段英文文本（如字母解释法的代表单词、呼号拼读）。
///
/// 字母解释法是 ITU 规定的英文单词，**不随界面语言变化**：西语界面下也应当用
/// 英语读出 "Alpha"，否则就失去了字母解释法的意义。
pub fn speak_en(text: &str) {
  speak_with(text, EN_LANG);
}

/// 朗读一段中文文本（如题库题干、解析）。
///
/// 题库内容目前只有中文，故与界面语言无关；待题库翻译落地后改走 [`content_lang`]。
pub fn speak_zh(text: &str) {
  speak_with(text, content_lang());
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn language_tags_are_bcp47() {
    assert_eq!(content_lang(), "zh-CN");
    assert_eq!(EN_LANG, "en-US");
    // 语言标签必须是合法 BCP-47：主标签 2–3 个 ASCII 字母，可选地区子标签。
    for tag in [content_lang(), EN_LANG] {
      let parts: Vec<&str> = tag.split('-').collect();
      assert!(!parts.is_empty() && parts.len() <= 2, "tag {tag}");
      assert!(
        matches!(parts[0].len(), 2 | 3) && parts[0].chars().all(|c| c.is_ascii_lowercase()),
        "tag {tag}"
      );
    }
  }
}
