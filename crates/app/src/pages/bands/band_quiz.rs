use ham_web_core::exam::shuffle_in_place;
use leptos::prelude::*;

use crate::i18n::t;
use crate::util::random;

/// 常见业余波段的参考频率（MHz）。
const BAND_POOL: &[(&str, f64)] = &[
  ("160m", 1.8),
  ("80m", 3.6),
  ("40m", 7.1),
  ("30m", 10.1),
  ("20m", 14.1),
  ("17m", 18.1),
  ("15m", 21.1),
  ("12m", 24.9),
  ("10m", 28.4),
  ("6m", 50.1),
  ("2m", 144.1),
  ("70cm", 432.0),
];

/// 频率 → 波段换算训练：给一个参考频率，判断它属于哪个波段。
#[component]
pub(super) fn BandQuiz() -> impl IntoView {
  let question = RwSignal::new(String::new());
  let answer = StoredValue::new(String::new());
  let options = RwSignal::new(Vec::<String>::new());
  let feedback = RwSignal::new(None::<bool>);
  let correct = RwSignal::new(0usize);
  let wrong = RwSignal::new(0usize);

  let load_next = move || {
    let ans_idx = (random() * BAND_POOL.len() as f64) as usize;
    let (band, freq) = BAND_POOL[ans_idx];
    answer.set_value(band.to_owned());
    question.set(format!("{freq} MHz"));
    let mut distractors: Vec<String> = Vec::new();
    let mut guard = 0;
    while distractors.len() < 3 && guard < 200 {
      guard += 1;
      let di = (random() * BAND_POOL.len() as f64) as usize;
      let d = BAND_POOL[di];
      if di != ans_idx && !distractors.contains(&d.0.to_owned()) {
        distractors.push(d.0.to_owned());
      }
    }
    let mut opts = vec![band.to_owned()];
    opts.extend(distractors);
    let mut rng = random;
    shuffle_in_place(&mut opts, &mut rng);
    options.set(opts);
    feedback.set(None);
  };
  load_next();

  let judge = move |chosen: String| {
    let ok = chosen == answer.get_value();
    if ok {
      correct.update(|v| *v += 1);
    } else {
      wrong.update(|v| *v += 1);
    }
    feedback.set(Some(ok));
  };

  view! {
    <section class="rounded-xl border bg-card">
      <h2 class="border-b px-4 py-3 text-sm font-semibold">
        {move || t("频率 ↔ 波段换算")}
        <span class="ml-2 text-xs font-normal text-muted-foreground">{move || t("判断这个频率属于哪个波段")}</span>
      </h2>
      <div class="space-y-4 p-4">
        <div class="flex flex-col items-center gap-2 rounded-xl border bg-muted/30 px-4 py-6">
          <div class="text-xs text-muted-foreground">{move || t("这个频率属于哪个波段？")}</div>
          <div class="font-mono text-3xl font-semibold text-primary">{move || question.get()}</div>
        </div>
        <div class="grid grid-cols-2 gap-2 sm:grid-cols-4">
          {move || {
            options
              .get()
              .into_iter()
              .map(|opt| {
                let chosen = opt.clone();
                view! {
                  <button
                    type="button"
                    on:click=move |_| judge(chosen.clone())
                    class="rounded-lg border px-3 py-2.5 text-center font-mono text-sm font-semibold transition-colors hover:bg-accent"
                  >
                    {opt}
                  </button>
                }
              })
              .collect_view()
          }}
        </div>
        <div class="flex flex-col items-center gap-3">
          <div class="min-h-5 text-sm">
            {move || {
              feedback.get().map(|ok| {
                if ok {
                  view! {
                    <span class="font-medium text-emerald-600 dark:text-emerald-400">{move || t("正确！")}</span>
                  }
                  .into_any()
                } else {
                  view! {
                    <span class="font-medium text-red-600 dark:text-red-400">
                      {move || t("正确答案：")} <span class="font-mono font-semibold">{answer.get_value()}</span>
                    </span>
                  }
                  .into_any()
                }
              })
            }}
          </div>
          <button
            type="button"
            on:click=move |_| load_next()
            class="rounded-lg border px-4 py-2 text-sm font-medium transition-colors hover:bg-accent"
          >
            {move || t("下一题")}
          </button>
          <div class="text-xs text-muted-foreground">
            "正确 " <span class="font-semibold tabular-nums text-foreground">{move || correct.get()}</span>
            "　错误 " <span class="font-semibold tabular-nums text-foreground">{move || wrong.get()}</span>
          </div>
        </div>
      </div>
    </section>
  }
}
