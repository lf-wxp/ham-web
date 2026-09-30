//! 语音合成朗读（Web Speech API）：英文（字母解释法）与中文（题干 / 解析）朗读。

use web_sys::SpeechSynthesisUtterance;

/// 朗读一段英文文本（如字母解释法的代表单词）。
pub fn speak_en(text: &str) {
  let Ok(synth) = crate::util::window().speech_synthesis() else {
    return;
  };
  // 停止当前朗读，避免连续点击叠加
  synth.cancel();
  let Ok(utterance) = SpeechSynthesisUtterance::new() else {
    return;
  };
  utterance.set_text(text);
  utterance.set_lang("en-US");
  synth.speak(&utterance);
}

/// 朗读一段中文文本（如题干、解析）。
pub fn speak_zh(text: &str) {
  let Ok(synth) = crate::util::window().speech_synthesis() else {
    return;
  };
  synth.cancel();
  let Ok(utterance) = SpeechSynthesisUtterance::new() else {
    return;
  };
  utterance.set_text(text);
  utterance.set_lang("zh-CN");
  synth.speak(&utterance);
}
