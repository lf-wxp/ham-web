//! 执照类别 × 频率 → 合规结论：可用性、功率上限、是否需协调、法条依据。
//!
//! 判定逻辑在 `ham_web_core::license_compliance`，本页只做输入与呈现；结论里的
//! 法条引用链到 `/radio-law` 收录的《业余无线电台管理办法》原文（法规原文保持中文）。

use ham_web_core::license_compliance::{self, Class};
use leptos::prelude::*;

use crate::components::common::{PageContainer, PageHeader};
use crate::i18n::{t, tf};
use crate::ui::{Field, NumberField, NumberKind, RadioGroup, RadioGroupItem};
use crate::util::{set_title, unique_id};

/// 功率格式化：整数去掉小数点（1000 / 25 / 15 / 1），非整数保留一位。
fn fmt_w(v: f64) -> String {
  if v == v.trunc() {
    format!("{v:.0}")
  } else {
    format!("{v:.1}")
  }
}

/// 法条引用 → `(链接文字, 锚点 href)`。书名保持法规原文（不译），条号走 UI 词典。
fn law_link(r: license_compliance::LawRef) -> (String, String) {
  let title =
    ham_web_core::radio_law::law(r.law).map_or_else(|| r.law.to_owned(), |l| l.title.to_owned());
  let text = format!(
    "{title}{}",
    tf("knowledge.radio-law-article-n", &[&r.article.to_string()])
  );
  let href = format!("/radio-law#{}-{}", r.law, r.article);
  (text, href)
}

/// 频率 → 合规结论。
#[component]
pub fn LicenseCompliancePage() -> impl IntoView {
  set_title("shell.frequency-power-compliance");

  let class = RwSignal::new(Class::A);
  let freq_text = RwSignal::new("145.0".to_owned());

  let a_id = unique_id("compliance-class-a");
  let b_id = unique_id("compliance-class-b");
  let c_id = unique_id("compliance-class-c");
  let freq_id = unique_id("compliance-freq");

  let parsed = Memo::new(move |_| {
    freq_text
      .get()
      .trim()
      .parse::<f64>()
      .ok()
      .filter(|f| f.is_finite() && *f > 0.0)
  });

  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-(--motion-normal) ease-out">
      <PageHeader
        title=Signal::derive(move || t("shell.frequency-power-compliance"))
        subtitle=Signal::derive(move || t("radio.compliance-subtitle"))
      />

      <PageContainer>
        <section class="pxl-card border bg-card">
          <div class="grid gap-4 p-4 sm:grid-cols-2">
            // 单选组的可见标签：`Field` 的 `r#for` 是可选的，这里刻意不传 —— 组不是单一控件，
            // 没有可关联的 `id`；组的无障碍名走 `RadioGroup` 的 `aria_label`。
            <Field label=Signal::derive(move || t("radio.compliance-class-label"))>
              <RadioGroup
                value=Signal::derive(move || class.get().label().to_owned())
                on_change=Callback::new(move |v: String| {
                  if let Some(c) = Class::parse(&v) {
                    class.set(c);
                  }
                })
                aria_label=Signal::derive(move || t("radio.compliance-class-label"))
                class="flex flex-wrap items-center gap-4 pt-1"
              >
                <label class="flex cursor-pointer items-center gap-2 text-sm">
                  <RadioGroupItem value="A" id=a_id />
                  {move || tf("learning.class", &["A"])}
                </label>
                <label class="flex cursor-pointer items-center gap-2 text-sm">
                  <RadioGroupItem value="B" id=b_id />
                  {move || tf("learning.class", &["B"])}
                </label>
                <label class="flex cursor-pointer items-center gap-2 text-sm">
                  <RadioGroupItem value="C" id=c_id />
                  {move || tf("learning.class", &["C"])}
                </label>
              </RadioGroup>
            </Field>

            <Field
              label=Signal::derive(move || t("log.frequency-mhz"))
              hint=Signal::derive(move || t("radio.compliance-freq-hint"))
              r#for=freq_id.clone()
            >
              <NumberField
                id=freq_id
                value=Signal::derive(move || freq_text.get())
                on_change=Callback::new(move |v: String| freq_text.set(v))
                controls=false
                kind=NumberKind::Decimal
              />
            </Field>
          </div>
        </section>

        // `data-slot` 是 e2e 的定位契约：页面上还有全局的 `role="status"`（toast / PWA 通知 /
        // 备份提示），不限定作用域会在那些元素同时出现时命中多个。
        <section class="pxl-card border bg-card" data-slot="compliance-result">
          <div class="space-y-2 p-4">
            {move || match parsed.get() {
              None => {
                view! {
                  <p role="alert" class="text-sm text-hp-text">
                    {move || t("radio.compliance-invalid-freq")}
                  </p>
                }
                  .into_any()
              }
              Some(freq) => {
                let c = license_compliance::check(class.get(), freq);
                match c.allocation {
                  None => {
                    view! {
                      <p role="status" class="text-sm text-hp-text">
                        {move || t("radio.compliance-not-in-band")}
                      </p>
                    }
                      .into_any()
                  }
                  Some(hit) => {
                    // 波段名与使用状态都可能来自 core 的「非文案」值：未命中常用波段是 `None`，
                    // 使用状态给的是词条 key —— 两者都必须落到当前语言，不能直接显示。
                    let band = hit.common_band.map_or_else(|| t("tools.other"), str::to_owned);
                    let usage = t(hit.usage.key());
                    let band_line =
                      tf("radio.compliance-band-line", &[&band, hit.range, &usage]);
                    if c.class_permitted {
                      // 「允许发射 ⇒ 有功率上限」这条不变量由 core 的单测守；页面不 `expect`：
                      // wasm 里的 panic 是整页崩，宁可少显示一行功率。
                      let power_line = c.tx_power_limit_w.map(|power| {
                        let power_s = fmt_w(power);
                        view! {
                          <p class="text-sm tabular-nums">
                            {tf("radio.compliance-power", &[&power_s])}
                          </p>
                        }
                      });
                      view! {
                        <div class="space-y-1.5">
                          <p role="status" class="text-sm text-win-text">
                            {move || t("radio.compliance-usable")}
                          </p>
                          <p class="text-sm">{band_line}</p>
                          {power_line}
                          {c.eirp_cap.map(|(w, code)| {
                            let w_s = fmt_w(w);
                            view! {
                              <p class="text-sm tabular-nums">
                                {tf("radio.compliance-eirp", &[code, &w_s])}
                              </p>
                            }
                          })}
                          <p class="text-sm">
                            {if c.needs_coordination {
                              t("radio.compliance-coord-needed")
                            } else {
                              t("radio.compliance-coord-none")
                            }}
                          </p>
                          {c.refs.iter().map(|r| {
                            let (text, href) = law_link(*r);
                            view! {
                              <a
                                href=href
                                class="text-xs text-primary underline-offset-2 hover:underline"
                              >
                                {text}
                              </a>
                            }
                          }).collect_view()}
                        </div>
                      }
                        .into_any()
                    } else {
                      let class_label = tf("learning.class", &[class.get().label()]);
                      view! {
                        <div class="space-y-1.5">
                          <p role="status" class="text-sm text-hp-text">
                            {move || t("radio.unusable")}
                          </p>
                          <p class="text-sm">{band_line}</p>
                          <p class="text-sm">{tf("radio.compliance-denied", &[&class_label])}</p>
                          {c.refs.iter().map(|r| {
                            let (text, href) = law_link(*r);
                            view! {
                              <a
                                href=href
                                class="text-xs text-primary underline-offset-2 hover:underline"
                              >
                                {text}
                              </a>
                            }
                          }).collect_view()}
                        </div>
                      }
                        .into_any()
                    }
                  }
                }
              }
            }}
          </div>
        </section>
      </PageContainer>
    </div>
  }
}
