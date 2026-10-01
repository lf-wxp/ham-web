use leptos::prelude::*;

/// 播放中的信号波纹指示器（任一摩尔斯音频播放时显示）。
#[component]
pub(crate) fn SignalBars() -> impl IntoView {
  let playing = crate::morse_audio::playing_signal();
  view! {
    <div class="flex items-center" aria-hidden="true">
      {move || {
        playing.get().then(|| {
          view! {
            <div class="flex items-end gap-0.5">
              {(0..4)
                .map(|i| {
                  view! {
                    <span
                      class="w-0.5 animate-pulse rounded-full bg-primary"
                      style=format!("height: {}px; animation-delay: {}ms", 8 + i * 4, i * 90)
                    ></span>
                  }
                })
                .collect_view()}
            </div>
          }
        })
      }}
    </div>
  }
}
