use ham_web_core::radio_law::{find_refs, law};
use leptos::prelude::*;

use crate::cn::cn;
use crate::i18n::{t, tf};
use crate::icons::{Icon, IconKind};
use crate::speech;
use crate::ui::{Button, Size, Tooltip, Variant, card_class};

/// 解析正文切出的一段：普通文字，或带法规链接（`(href, 悬停文案)`）的引用。
struct Segment {
  text: String,
  link: Option<(String, String)>,
}

/// 把解析按法规引用切成「普通文字 + 链接」交替的片段；没有引用时就是整段一个片段。
fn segments(text: &str) -> Vec<Segment> {
  let mut out = Vec::new();
  let mut cursor = 0usize;
  for r in find_refs(text) {
    // `find_refs` 保证按起点升序且不重叠；这里仍防一手，游标回退会让文字重复。
    if r.start < cursor {
      continue;
    }
    if r.start > cursor {
      out.push(Segment {
        text: text[cursor..r.start].to_owned(),
        link: None,
      });
    }
    let number = r.number;
    let number_text = number.to_string();
    let href = format!("/radio-law#{}-{number}", r.law);
    // 悬停文案里给全称（含书名号），不给内部 key。
    let law_title = law(r.law).map_or(r.law, |l| l.title);
    let title = tf("knowledge.view-article-text", &[law_title, &number_text]);
    out.push(Segment {
      text: text[r.start..r.end].to_owned(),
      link: Some((href, title)),
    });
    cursor = r.end;
  }
  if cursor < text.len() {
    out.push(Segment {
      text: text[cursor..].to_owned(),
      link: None,
    });
  }
  out
}

/// 可折叠的「答案解析」卡片；题目切换时由调用方重建，自动恢复折叠状态。
/// 题目没有解析时显示轻提示，并引导到仓库补充。
///
/// 入参是解析文本而不是整道题：错题、考试复盘这类场景手里只有快照里的文本，
/// 没有 `QuestionItem`。空串与纯空白都按「没有解析」处理。
///
/// 入参是 `Signal`：错题 / 复盘的解析要等题库索引到了才换成最新版本，换文字时折叠 / 展开
/// 状态留在卡片外层，用户已经展开的卡片不会被重建而收起。静态文本直接传 `Option<String>`。
///
/// **正文首次展开前不构建**：错题列表最多 1000 条、复盘页一次铺开几十题，每张卡都先
/// 切法规引用、挂 `Tooltip` 的话，折叠着也要为用户不一定会看的文字付出渲染成本。
#[component]
pub fn ExplanationCard(#[prop(into)] explanation: Signal<Option<String>>) -> impl IntoView {
  let open = RwSignal::new(false);
  // 只增不减：展开过一次后正文留着，收起时高度过渡才有内容可裹。
  let opened_once = RwSignal::new(false);
  let text = Memo::new(move |_| {
    explanation
      .get()
      .map(|s| s.trim().to_owned())
      .filter(|s| !s.is_empty())
  });

  // 折叠面板：外层 grid 行高 0fr → 1fr 做高度过渡（`.ui-collapse` 负责裁剪与收起后隐藏），
  // 内层再叠一层淡入 + 轻微上移，避免只有高度变化显得生硬。
  let panel = Signal::derive(move || {
    cn(&[
      "ui-collapse",
      if open.get() {
        "grid-rows-[1fr]"
      } else {
        "grid-rows-[0fr]"
      },
    ])
  });
  let inner = Signal::derive(move || {
    cn(&[
      // Tailwind v4 的 `translate-y-*` 用的是 `translate` 属性而不是 `transform`，
      // 过渡属性要写成 `translate` 才动得了。
      "px-6 pb-5 transition-[opacity,translate] duration-[var(--motion-normal)] ease-[var(--ease-out-expo)]",
      if open.get() {
        "translate-y-0 opacity-100"
      } else {
        "-translate-y-1 opacity-0"
      },
    ])
  });

  move || {
    let Some(body) = text.get() else {
      return view! {
        <div data-slot="card" class=card_class("gap-0 py-0")>
          <div class="flex flex-wrap items-center gap-x-2 gap-y-1 px-6 py-4 text-sm text-muted-foreground">
            <Icon kind=IconKind::BookOpen class="size-6 w-4 shrink-0" />
            <span>{move || t("knowledge.no-explanation-yet-for")}</span>
            <a
              href="https://github.com/lf-wxp/ham-web"
              target="_blank"
              rel="noopener noreferrer"
              class="inline-flex items-center gap-1 text-primary underline-offset-2 hover:underline"
            >
              {move || t("knowledge.contribute")}
              <Icon kind=IconKind::ExternalLink class="size-6 w-3" />
            </a>
          </div>
        </div>
      }
      .into_any();
    };
    let speak_text = body.clone();

    view! {
      <div data-slot="card" class=card_class("gap-0 py-0")>
        <div class="flex items-center">
          <Button
            variant=Variant::Text
            // 文字色必须显式给：`Variant::Text` 的 `--btn-fg` 是 `muted-foreground`，而这里是卡片标题。
            class="h-auto flex-1 justify-between px-6 py-4 text-left text-foreground hover:bg-accent/50"
            aria_expanded=Signal::derive(move || open.get())
            on_click=Callback::new(move |_| {
              open.update(|v| *v = !*v);
              opened_once.set(true);
            })
          >
            <span class="pxl-section-head">{move || t("learning.explanation")}</span>
            <Icon
              kind=IconKind::ChevronDown
              class=Signal::derive(move || {
                cn(&[
                  "h-4 w-4 shrink-0 transition-transform duration-[var(--motion-normal)] ease-[var(--ease-out-expo)]",
                  if open.get() { "rotate-180" } else { "" },
                ])
              })
            />
          </Button>
          <Button
            variant=Variant::Ghost
            size=Size::IconSm
            class="mr-4 shrink-0"
            title=Signal::derive(move || t("learning.read-explanation-aloud"))
            aria_label=Signal::derive(move || t("learning.read-explanation-aloud"))
            on_click=Callback::new(move |_| speech::speak_zh(&speak_text))
          >
            <Icon kind=IconKind::Volume2 class="size-6 w-4" />
          </Button>
        </div>
        <div class=panel data-open=move || open.get().to_string()>
          <div>
            <div class=inner>
              {move || {
                opened_once.get().then(|| {
                  view! {
                    <div class="whitespace-pre-line border-t pt-4 text-sm leading-6 text-muted-foreground">
                      {segments(&body).into_iter().map(|Segment { text, link }| {
                        if let Some((href, title)) = link {
                          view! {
                            <Tooltip label=title>
                              <a
                                href=href
                                class="text-primary underline underline-offset-2 hover:opacity-80"
                              >
                                {text}
                              </a>
                            </Tooltip>
                          }
                            .into_any()
                        } else {
                          view! { <span>{text}</span> }.into_any()
                        }
                      }).collect_view()}
                    </div>
                  }
                })
              }}
            </div>
          </div>
        </div>
      </div>
    }
    .into_any()
  }
}
