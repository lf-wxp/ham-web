//! 题目卡片：题干、附图、选项（单选/多选），可选即时显示答案。

use ham_web_core::QuestionItem;
use ham_web_core::question::AnswerGate;
use leptos::prelude::*;

use crate::components::common::{OptionMark, OptionVerdict, PreviewableImage};
use crate::i18n::{t, tf};
use crate::icons::{Icon, IconKind};
use crate::speech;
use crate::ui::{
  BadgeVariant, Button, CARD_HEADER, Checkbox, Label, RadioGroup, RadioGroupItem, Size, Tooltip,
  Variant, badge_class, card_class, card_content_class, card_title_class,
};
use crate::util::unique_id;

/// 选项行的选中态与判分态：**形状不动**（单选是切角八边形、多选是方形，改了会破坏辨识），
/// 用「整行浅底 + 左 2px 色条」表达状态。
///
/// - 未判分：选中 = 主色条 + 浅主色底；
/// - 判分后：正确选项 = 通关绿，用户选错的 = 危险红（同时行尾有 ✓ / ✗，不只靠颜色）。
///   判分配色是另一套语义，不与「选中」混用（`docs/ui/pixel.md` §2 硬规则 11）。
///
/// 只向左出血 8px（`-ml-2 pl-2`）：色条与浅底比文字更靠左，文字位置一动不动。
/// 不能向右出血——负的右外边距会让选项行溢出容器（`scrollWidth > clientWidth`），
/// 被布局审计判成「文本裁剪」。
fn option_row_class(selected: bool, verdict: OptionVerdict, modern: bool) -> &'static str {
  if modern {
    // 现代风：整行卡片式选项，选中 / 判分只换描边与浅底（`style/modern/kit.css` 的 `.md-opt*`）。
    return match (verdict, selected) {
      (OptionVerdict::Right, _) => "md-opt md-opt-right",
      (OptionVerdict::Wrong, _) => "md-opt md-opt-wrong",
      (OptionVerdict::None, true) => "md-opt md-opt-sel",
      (OptionVerdict::None, false) => "md-opt",
    };
  }
  match (verdict, selected) {
    (OptionVerdict::Right, _) => {
      "flex items-start gap-2 -ml-2 border-l-2 border-win bg-win/10 pl-2"
    }
    (OptionVerdict::Wrong, _) => {
      "flex items-start gap-2 -ml-2 border-l-2 border-destructive bg-destructive/10 pl-2"
    }
    (OptionVerdict::None, true) => {
      "flex items-start gap-2 -ml-2 border-l-2 border-primary bg-primary/5 pl-2"
    }
    (OptionVerdict::None, false) => {
      "flex items-start gap-2 -ml-2 border-l-2 border-transparent pl-2"
    }
  }
}

#[component]
pub fn QuestionCard(
  index: usize,
  total: usize,
  question: QuestionItem,
  #[prop(into)] selected: Signal<Vec<String>>,
  on_change: Callback<Vec<String>>,
  #[prop(into)] show_answer: Signal<bool>,
  #[prop(optional, into)] read_only: Signal<bool>,
  /// 未作答也直接显示正确答案（交卷后的回顾用）。
  ///
  /// 默认 `false`：练习页「允许显示答案」不等于「提前给答案」，没作答就画答案行会让练习
  /// 变成开卷。但考试交卷后已经结束作答，空白题的答案正是回顾最需要的信息，不该再让人
  /// 逐题点按钮。
  #[prop(optional)]
  reveal_unanswered: bool,
  /// 多选题要先点「检查答案」才判分（练习页开启）。
  ///
  /// 默认 `false`：对战 / 每日挑战这类由页面自己控制判分时机的场景不受影响。练习页开启，
  /// 因为多选题勾上第一个框就算「已作答」，答案行会当场亮出整套正确答案，后面的框照抄即可。
  #[prop(optional)]
  confirm_multiple: bool,
  /// 这道题此前是否已经「偷看」过答案（卡片按题重建，由调用方把记录带回来）。
  #[prop(optional)]
  peeked: bool,
  /// 用户主动点了「显示正确答案」时通知调用方。
  ///
  /// 为什么要通知：看过答案再点出来的「答对」不是自测结果，调用方需要据此不把它计为
  /// 掌握 / 连对 —— 只在卡片里藏一个本地布尔，统计层就无从知道。
  ///
  /// 注意：只有**主动点按钮**才算偷看。「先选 → 看到答案行 → 改成正确项」目前仍按最终作答计入
  /// （`e2e/tests/study.spec.ts`「自己答对的才计」把它当契约），这个口径是否合理见
  /// `docs/ui/pixel.md` §2 规则 11 的待决项，不在界面层擅自改统计语义。
  #[prop(optional, into)]
  on_reveal: Option<Callback<()>>,
) -> impl IntoView {
  let base_id = unique_id("q");
  // 风格在组件挂载时定一次即可：练习页没有切风格的入口，切风格要走导航（页面会重建）。
  let modern = crate::kit::is_modern_at_mount();
  let input_class = if modern { "md-opt-input" } else { "mt-1" };
  let is_multiple = question.is_multiple();
  let answer_keys = question.answer_keys.clone();
  let answer_keys_sv = StoredValue::new(answer_keys.clone());
  let question_text = question.question.clone();
  // 两个分支各要一个闭包，克隆一份给现代风的朗读按钮。
  let question_text_modern = question_text.clone();
  let j_code = question.j_code().map(ToOwned::to_owned);
  let kind_label = question.kind.label();
  let pages = question.pages.and_then(|p| {
    let start = p.start?;
    Some(match p.end {
      Some(end) if end != start => format!("P.{start}-{end}"),
      _ => format!("P.{start}"),
    })
  });

  let image = question.image().map(|src| {
    let (alt, title) = match &j_code {
      Some(j) => (
        tf("exam.no-image", &[(j.as_str())]),
        tf("exam.question-figure", &[(j.as_str())]),
      ),
      None => (t("exam.question-image"), t("exam.question-image")),
    };
    view! {
      <div class="mt-2">
        <PreviewableImage src=src.to_owned() alt=alt title=title />
      </div>
    }
  });

  // 手动「看答案」：不想作答、只想对一眼答案时用。它只展开答案行，不写作答，
  // 因此不会污染错题本与统计。
  let revealed = RwSignal::new(peeked);
  // 多选题「检查答案」：回到已作答的题（卡片按题重建）时视为已检查 —— 离开题目时那次作答
  // 已经计入统计，这里不该再让人重点一遍。
  //
  // 已知取舍：此时答案保持可见，改选项时答案会跟着变，等价于对这道题「开卷」。它与
  // 「先选 → 看到答案行 → 改成正确项」是同一件事，属于统计语义（`docs/ui/pixel.md` §2 规则 11 的
  // B1 / H15 待决），要改就同时改这里与 `study.spec.ts` 的契约，别在界面层单独动。
  let needs_check = confirm_multiple && is_multiple;
  let checked = RwSignal::new(needs_check && !selected.with_untracked(Vec::is_empty));

  // `show_answer` 是「允许显示答案」，不是「提前给答案」：没作答（也没主动展开）就不画
  // 这一行，否则练习页从第 1 题起就是开卷（错题本记下的也是「照着答案答」的行为）。
  // 判定本身在 core（`AnswerGate`，带单测），这里只把信号喂进去。
  let gate = move || AnswerGate {
    allowed: show_answer.get(),
    reveal_unanswered,
    revealed: revealed.get(),
    answered: selected.with(|s| !s.is_empty()),
    needs_check,
    checked: checked.get(),
  };
  // 某个选项行在判分后的结论：正确项 / 用户选错的 / 无关。
  let verdict_of = move |key: &str, picked: bool| {
    if !gate().visible() {
      OptionVerdict::None
    } else if answer_keys_sv.with_value(|ks| ks.iter().any(|k| k == key)) {
      OptionVerdict::Right
    } else if picked {
      OptionVerdict::Wrong
    } else {
      OptionVerdict::None
    }
  };

  let options = if is_multiple {
    let items = question
      .options
      .clone()
      .into_iter()
      .map(|opt| {
        let key = StoredValue::new(opt.key.clone());
        let checked_sel =
          Signal::derive(move || selected.with(|s| key.with_value(|k| s.contains(k))));
        let verdict = Signal::derive(move || key.with_value(|k| verdict_of(k, checked_sel.get())));
        let toggle = Callback::new(move |on: bool| {
          if read_only.get_untracked() {
            return;
          }
          let k = key.get_value();
          let mut next = selected.get_untracked();
          next.retain(|x| *x != k);
          if on {
            next.push(k);
          }
          on_change.run(next);
        });
        view! {
          <label class=move || option_row_class(checked_sel.get(), verdict.get(), modern)>
            <Checkbox class=input_class checked=checked_sel on_change=toggle disabled=read_only />
            <span class="whitespace-pre-line leading-6 text-base">
              <strong class="mr-2">{opt.key} "."</strong>
              {opt.text}
            </span>
            <OptionMark verdict=verdict />
          </label>
        }
      })
      .collect_view();
    view! { <div class="grid gap-2">{items}</div> }.into_any()
  } else {
    let options = question.options.clone();
    let base_id = base_id.clone();
    // 单选值要在构建选项行之前就有：行的选中态（整行浅底 + 左主色条）由它驱动。
    let value = Signal::derive(move || selected.with(|s| s.first().cloned().unwrap_or_default()));
    // 选项必须在 RadioGroup 内部（其 context 提供之后）再构建
    let items = move || {
      options
        .clone()
        .into_iter()
        .map(|opt| {
          let id = format!("{base_id}-{index}-{}", opt.key);
          // block 覆盖 Label 默认的 flex（cn 的 display 冲突组），
          // 使 "A." 与文本内联排布，长文本换行时始终对齐第一行。
          // `text-base` 覆盖 `Label` 默认的 `text-sm`：多选分支的文字是裸 `<span>`、继承选项行字号，
          // 不显式对齐会让两种题型的选项字号差 4~6px（像素风 12px vs 18px）。
          let label_class = Signal::derive(move || {
            if read_only.get() {
              "block whitespace-pre-line leading-6 text-base cursor-default"
            } else {
              "block whitespace-pre-line leading-6 text-base cursor-pointer"
            }
            .to_owned()
          });
          let key = opt.key.clone();
          let verdict = {
            let key = key.clone();
            Signal::derive(move || verdict_of(&key, value.get() == key))
          };
          let row_class = {
            let key = key.clone();
            move || option_row_class(value.get() == key, verdict.get(), modern)
          };
          view! {
            <div class=row_class>
              <RadioGroupItem class=input_class value=opt.key.clone() id=id.clone() disabled=read_only />
              <Label r#for=id class=label_class>
                <strong class="mr-2">{opt.key} "."</strong>
                {opt.text}
              </Label>
              <OptionMark verdict=verdict />
            </div>
          }
        })
        .collect_view()
    };
    let on_value = Callback::new(move |v: String| {
      if !read_only.get_untracked() {
        on_change.run(if v.is_empty() { vec![] } else { vec![v] });
      }
    });
    // `RadioGroup` 默认 `gap-3`：表单里选项之间留 12px 好点。题卡的两种题型是同一张
    // 卡片里的并列选项，多选那边是 `grid gap-2`（8px），这里必须跟着收，否则同一道题
    // 换个题型、选项之间的空隙就变一次。
    view! {
      <RadioGroup value=value on_change=on_value disabled=read_only class="gap-2">
        {items()}
      </RadioGroup>
    }
    .into_any()
  };

  // 答案行：结论 = 图标 + 文字 + 动效三个通道（不只靠红绿），并住在常驻的 live region 里
  // —— 读屏用户作答后能听到「已答对 / 答错了」；节点必须先于内容存在，否则部分读屏器
  // 不播报「插入时就带内容」的区域。`empty:hidden`：没有内容时不占 `space-y-3` 的间距。
  let answer_line = move || {
    gate().visible().then(|| {
      let sel = selected.get();
      let correct = !sel.is_empty() && ham_web_core::question::same_set(&sel, &answer_keys);
      let verdict = (!sel.is_empty()).then(|| {
        if correct {
          if modern {
            view! {
              <span class="md-verdict md-verdict-right">
                "✓ " {move || t("exam.answered-correct")}
              </span>
            }
            .into_any()
          } else {
            view! {
              <span class="motion-pop ml-2 inline-flex items-center gap-1 align-middle text-base text-win-text">
                <Icon kind=IconKind::CheckCircle class="size-6" />
                {move || t("exam.answered-correct")}
              </span>
            }
            .into_any()
          }
        } else {
          let err = if is_multiple {
            ham_web_core::question::multi_error_kind(&sel, &answer_keys)
              .map(|e| format!(" · {}", e.label()))
              .unwrap_or_default()
          } else {
            String::new()
          };
          if modern {
            view! {
              <span class="md-verdict md-verdict-wrong">
                "✗ " {move || t("exam.answered-wrong")}
                <span class="md-sm">
                  {" · "}
                  {tf("common.your-answer", &[&(sel.join(", ")).to_string(), &err])}
                </span>
              </span>
            }
            .into_any()
          } else {
            view! {
              <span class="pxl-shake ml-2 inline-flex flex-wrap items-center gap-x-1 align-middle text-base text-hp-text">
                <Icon kind=IconKind::XCircle class="size-6" />
                {move || t("exam.answered-wrong")}
                <span class="text-sm">
                  {" · "}
                  {tf("common.your-answer", &[&(sel.join(", ")).to_string(), &err])}
                </span>
              </span>
            }
            .into_any()
          }
        }
      });
      view! {
        {move || t("exam.correct-answer-prefix")} {answer_keys.join(", ")} {verdict}
      }
    })
  };

  // 多选题「检查答案」：选好之后再点，才判分。没选任何项时不出现（没东西可检查）。
  let check_button = move || {
    (needs_check
      && show_answer.get()
      && !read_only.get()
      && !checked.get()
      && !selected.with(Vec::is_empty))
    .then(|| {
      view! {
        {if modern {
          view! {
            <button
              type="button"
              class="md-btn md-btn-accent md-btn-sm"
              on:click=move |_| checked.set(true)
            >
              <crate::kit_modern::ModernIcon kind=crate::kit_modern::MIcon::Check />
              {move || t("exam.check-answer")}
            </button>
          }
            .into_any()
        } else {
          view! {
            <Button
              variant=Variant::Default
              size=Size::Sm
              on_click=Callback::new(move |_| checked.set(true))
            >
              <Icon kind=IconKind::Check class="size-6" />
              {move || t("exam.check-answer")}
            </Button>
          }
            .into_any()
        }}
      }
    })
  };

  // 未作答时给一个「显示正确答案」入口（作答后答案自动出现，按钮随之隐藏）。
  let reveal_button = move || {
    (show_answer.get() && !reveal_unanswered && selected.with(Vec::is_empty) && !revealed.get())
      .then(|| {
        view! {
          {if modern {
            view! {
              <button
                type="button"
                class="md-btn md-btn-ghost md-btn-sm"
                on:click=move |_| {
                  revealed.set(true);
                  if let Some(cb) = on_reveal {
                    cb.run(());
                  }
                }
              >
                {move || t("exam.show-correct-answer")}
              </button>
            }
              .into_any()
          } else {
            view! {
              <Button
                variant=Variant::Text
                size=Size::Xs
                on_click=Callback::new(move |_| {
                  revealed.set(true);
                  if let Some(cb) = on_reveal {
                    cb.run(());
                  }
                })
              >
                {move || t("exam.show-correct-answer")}
              </Button>
            }
              .into_any()
          }}
        }
      })
  };

  view! {
    <div data-slot="card" class=card_class(if modern { "md-q" } else { "" })>
      <div data-slot="card-header" class=CARD_HEADER>
        // `flex-wrap`：「第 N / M 题」+ 题型 + 题号三个徽标都是不换行的定宽内容，
        // 320px 下（英文 / 西语最明显）会把标题行撑出视口，整页横向滚动。
        <div data-slot="card-title" class=card_title_class("flex flex-wrap items-center gap-2")>
          <span>{move || tf("exam.question", &[&(index + 1).to_string(), &total.to_string()])}</span>
          <span data-slot="badge" class=badge_class(BadgeVariant::Secondary, "")>
            {move || t(kind_label)}
          </span>
          {pages.map(|p| view! { <span data-slot="badge" class=badge_class(BadgeVariant::Outline, "")>{p}</span> })}
          {j_code
            .clone()
            .map(|j| {
              view! {
                <Tooltip label=Signal::derive(move || t("exam.question-number-label"))>
                  <span data-slot="badge" class=badge_class(BadgeVariant::Outline, "")>
                    {j}
                  </span>
                </Tooltip>
              }
            })}
          {if modern {
            // 刻意的例外：现代风的图标按钮不是 `ui::Button`（那套只产出像素类名），
            // `kit_modern` 的 `md-icon-btn` 也没有单控件组件 —— 这里手写原生 `<button>`，
            // 三个要点与 `Button` 对齐：`type="button"`、可见的 `aria-label`、不吞键盘事件。
            view! {
              <button
                type="button"
                class="md-icon-btn ml-auto shrink-0"
                aria-label=move || t("exam.read-question-aloud")
                on:click=move |_| speech::speak_zh(&question_text_modern)
              >
                <crate::kit_modern::ModernIcon kind=crate::kit_modern::MIcon::Volume />
              </button>
            }
              .into_any()
          } else {
            view! {
              <Button
                variant=Variant::Ghost
                size=Size::IconSm
                class="ml-auto shrink-0"
                title=Signal::derive(move || t("exam.read-question-aloud"))
                aria_label=Signal::derive(move || t("exam.read-question-aloud"))
                on_click=Callback::new(move |_| speech::speak_zh(&question_text))
              >
                <Icon kind=IconKind::Volume2 class="size-6" />
              </Button>
            }
              .into_any()
          }}
        </div>
      </div>
      <div data-slot="card-content" class=card_content_class("space-y-3")>
        <div class="whitespace-pre-line">{question.question.clone()}</div>
        {image}
        <div class="space-y-2">{options}</div>
        {check_button}
        {reveal_button}
        <div role="status" aria-live="polite" class="text-sm text-muted-foreground empty:hidden">
          {answer_line}
        </div>
      </div>
    </div>
  }
}
