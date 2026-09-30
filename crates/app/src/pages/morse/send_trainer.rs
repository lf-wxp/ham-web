use ham_web_core::morse::{DIGITS, LETTERS, MorseChar, code_of};
use leptos::prelude::*;

use super::{morse_display, random_index};

/// 发报练习：按住按钮拍发，短按为点、长按为划，松开后判定。
#[component]
pub(super) fn SendTrainer() -> impl IntoView {
  let target = RwSignal::new("A".to_owned());
  let marks = RwSignal::new(String::new());
  let press_time = RwSignal::new(None::<f64>);
  let feedback = RwSignal::new(None::<bool>);
  let correct = RwSignal::new(0usize);
  let wrong = RwSignal::new(0usize);

  let next_target = move || {
    let pool: Vec<&'static MorseChar> = LETTERS.iter().chain(DIGITS).collect();
    let c = pool[random_index(pool.len())];
    target.set(c.ch.to_owned());
    marks.set(String::new());
    feedback.set(None);
  };

  let on_down = move |_| {
    press_time.set(Some(js_sys::Date::now()));
  };
  let on_up = move |_| {
    if let Some(t0) = press_time.get() {
      let dt = js_sys::Date::now() - t0;
      press_time.set(None);
      let mark = if dt < 200.0 { "." } else { "-" };
      marks.update(|m| m.push_str(mark));
    }
  };

  let judge = move || {
    let m = marks.get();
    let expected = target.get().chars().next().and_then(code_of).unwrap_or("");
    let ok = !m.is_empty() && m == expected;
    if ok {
      correct.update(|v| *v += 1);
    } else {
      wrong.update(|v| *v += 1);
    }
    feedback.set(Some(ok));
  };

  let clear_marks = move || {
    marks.set(String::new());
    feedback.set(None);
  };

  view! {
    <section class="rounded-xl border bg-card">
      <h2 class="border-b px-4 py-3 text-sm font-semibold">"发报练习"</h2>
      <div class="space-y-4 p-4">
        <div class="flex flex-col items-center gap-2 rounded-xl border bg-muted/30 px-4 py-6">
          <div class="text-xs text-muted-foreground">"目标字符"</div>
          <div class="text-4xl font-semibold tabular-nums">{move || target.get()}</div>
          <div class="font-mono text-2xl font-semibold tracking-[0.3em] text-primary">
            {move || {
              let m = marks.get();
              if m.is_empty() { "…".to_owned() } else { morse_display(&m) }
            }}
          </div>
          <div class="text-xs text-muted-foreground">"短按（<0.2s）为点，长按为划"</div>
        </div>

        <div class="flex flex-col items-center gap-3">
          <button
            type="button"
            on:pointerdown=on_down
            on:pointerup=on_up
            on:pointerleave=on_up
            on:pointercancel=on_up
            style="touch-action: none;"
            class="flex h-28 w-28 select-none items-center justify-center rounded-full bg-primary text-primary-foreground shadow transition-transform active:scale-95"
          >
            "按住发报"
          </button>
          <div class="flex flex-wrap items-center justify-center gap-2">
            <button
              type="button"
              class="h-10 shrink-0 whitespace-nowrap rounded-lg bg-primary px-4 text-sm font-medium text-primary-foreground transition-opacity hover:opacity-90"
              on:click=move |_| judge()
            >
              "判定"
            </button>
            <button
              type="button"
              class="h-10 shrink-0 whitespace-nowrap rounded-lg border px-3 text-sm transition-colors hover:bg-accent"
              on:click=move |_| clear_marks()
            >
              "重拍"
            </button>
            <button
              type="button"
              class="h-10 shrink-0 whitespace-nowrap rounded-lg border px-3 text-sm transition-colors hover:bg-accent"
              on:click=move |_| next_target()
            >
              "下一题"
            </button>
          </div>

          <div class="min-h-5 text-sm">
            {move || {
              feedback.get().map(|ok| {
                if ok {
                  view! {
                    <span class="font-medium text-emerald-600 dark:text-emerald-400">"正确！"</span>
                  }
                  .into_any()
                } else {
                  let expected = target.get().chars().next().and_then(code_of).unwrap_or("");
                  view! {
                    <span class="font-medium text-red-600 dark:text-red-400">
                      "答案是 " <span class="font-mono">{morse_display(expected)}</span>
                    </span>
                  }
                  .into_any()
                }
              })
            }}
          </div>

          <div class="text-xs text-muted-foreground">
            "正确 " <span class="font-semibold tabular-nums text-foreground">{move || correct.get()}</span>
            "　错误 " <span class="font-semibold tabular-nums text-foreground">{move || wrong.get()}</span>
          </div>
        </div>
      </div>
    </section>
  }
}
