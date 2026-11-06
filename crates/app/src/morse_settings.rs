//! 摩尔斯页全局播放设置：音调与音量，持久化到 `localStorage["morse-settings"]`。

pub use ham_web_core::morse_store::MorseSettings;
use leptos::prelude::*;

use crate::util::storage;

const KEY: &str = "morse-settings";

fn load() -> MorseSettings {
  storage::get_json(KEY).unwrap_or_default()
}

fn save(s: &MorseSettings) {
  storage::set_json(KEY, s);
}

/// 摩尔斯播放设置上下文（由 `MorsePage` 提供，供各训练器与卡片读取）。
#[derive(Clone, Copy)]
pub struct MorseSettingsCtx {
  pub settings: RwSignal<MorseSettings>,
}

impl MorseSettingsCtx {
  /// 当前音调（Hz）。
  pub fn tone_hz(self) -> f32 {
    self.settings.get().tone_hz as f32
  }

  /// 当前音量（0–1）。
  pub fn volume(self) -> f32 {
    self.settings.get().volume.clamp(0.0, 1.0)
  }

  pub fn set_tone(self, hz: u32) {
    self.settings.update(|s| {
      s.tone_hz = hz;
      save(s);
    });
  }

  pub fn set_volume(self, v: f32) {
    self.settings.update(|s| {
      s.volume = v.clamp(0.0, 1.0);
      save(s);
    });
  }
}

/// 在页面根组件提供设置上下文。
pub fn provide_morse_settings() {
  provide_context(MorseSettingsCtx {
    settings: RwSignal::new(load()),
  });
}

/// 读取设置上下文。
pub fn use_morse_settings() -> MorseSettingsCtx {
  expect_context::<MorseSettingsCtx>()
}
