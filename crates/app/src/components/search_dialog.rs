//! 全局搜索命令面板：`/` 快捷键或导航栏搜索按钮唤起，居中弹出、不打断当前流程。

use ham_web_core::search::search;
use leptos::prelude::*;

use crate::data;
use crate::i18n::t;
use crate::icons::{Icon, IconKind};
use crate::pages::SLANG_CATEGORY;
use crate::ui::{Dialog, input_class};

/// 搜索结果分组：`(页面名, 路由, [(标题, 内容)])`。
type SearchGroup = (String, String, Vec<(String, String)>);

/// HTML 转义。
fn escape_html(s: &str) -> String {
  s.replace('&', "&amp;")
    .replace('<', "&lt;")
    .replace('>', "&gt;")
    .replace('"', "&quot;")
}

/// 高亮文本中的关键词（忽略大小写），返回 HTML。
///
/// 在原始文本上定位关键词、再对非匹配片段做转义，避免关键词命中 `&amp;` 等
/// HTML 实体内部而破坏标记。
fn highlight(text: &str, query: &str) -> String {
  let q = query.trim();
  if q.is_empty() {
    return escape_html(text);
  }
  let lower = text.to_lowercase();
  let ql = q.to_lowercase();
  let mut result = String::new();
  let mut last = 0;
  while let Some(pos) = lower[last..].find(&ql) {
    let start = last + pos;
    result.push_str(&escape_html(&text[last..start]));
    result.push_str("<mark class=\"rounded-sm bg-primary/20 px-0.5 text-primary\">");
    result.push_str(&escape_html(&text[start..start + q.len()]));
    result.push_str("</mark>");
    last = start + q.len();
  }
  result.push_str(&escape_html(&text[last..]));
  result
}

/// 术语表 / 简语关键词搜索（运行时 JSON 数据），返回 `(页面名, 路由, 标题, 内容)`。
fn search_glossary(query: &str) -> Vec<(&'static str, &'static str, String, String)> {
  let q = query.trim().to_lowercase();
  if q.is_empty() {
    return Vec::new();
  }
  let Some(glossary) = data::glossary_loaded() else {
    return Vec::new();
  };
  let mut matched: Vec<(u8, &'static str, &'static str, String, String)> = glossary
    .entries()
    .iter()
    .filter_map(|e| {
      let rank = e.match_rank(&q)?;
      let is_slang = e.category_key() == SLANG_CATEGORY;
      let (page, href) = if is_slang {
        ("简语", "/q-code")
      } else {
        ("术语表", "/glossary")
      };
      Some((rank, page, href, e.term.clone(), e.desc.clone()))
    })
    .collect();
  matched.sort_by_key(|(r, _, _, _, _)| *r);
  matched.truncate(30);
  matched
    .into_iter()
    .map(|(_, p, h, t, d)| (p, h, t, d))
    .collect()
}

/// 截断文本（按字符数，超出加省略号）。
fn truncate(s: &str, max: usize) -> String {
  if s.chars().count() <= max {
    s.to_owned()
  } else {
    let head: String = s.chars().take(max).collect();
    format!("{head}…")
  }
}

/// 题库题目搜索（题干 + 解析），返回 `(页面, 路由, 题干, 解析片段)`。
fn search_questions(query: &str) -> Vec<(String, String, String, String)> {
  let q = query.trim().to_lowercase();
  if q.is_empty() {
    return Vec::new();
  }
  let Some(index) = data::question_index_loaded() else {
    return Vec::new();
  };
  let mut matched: Vec<(String, String, String, String)> = index
    .iter()
    .filter(|e| e.search_text().contains(&q))
    .take(20)
    .map(|e| {
      let title = truncate(&e.q, 42);
      let text = if e.exp.is_empty() {
        title.clone()
      } else {
        truncate(&e.exp, 90)
      };
      (
        "题库".to_owned(),
        format!("/browse?bank={}&q={}", e.bank, query.trim()),
        title,
        text,
      )
    })
    .collect();
  // 题干越短通常越精确，靠前展示。
  matched.sort_by_key(|a| a.2.chars().count());
  matched
}

/// 全局搜索命令面板。开闭状态由 App 层 `provide_context::<RwSignal<bool>>()` 提供。
#[component]
pub fn SearchDialog() -> impl IntoView {
  let open = expect_context::<RwSignal<bool>>();
  let query = RwSignal::new(String::new());

  // 术语表 / 题目搜索索引按需加载：首次打开面板时拉取，加载完成后结果自动刷新。
  let glossary_ready = RwSignal::new(data::glossary_loaded().is_some());
  let question_ready = RwSignal::new(data::question_index_loaded().is_some());

  // 打开时清空输入。
  Effect::new(move |_| {
    if open.get() {
      query.set(String::new());
      if !glossary_ready.get_untracked() {
        leptos::task::spawn_local(async move {
          data::load_glossary().await;
          glossary_ready.set(true);
        });
      }
      if !question_ready.get_untracked() {
        leptos::task::spawn_local(async move {
          data::load_question_index().await;
          question_ready.set(true);
        });
      }
    }
  });

  view! {
    <Dialog open=open class="sm:max-w-2xl" show_close=false label=t("全站搜索")>
      <div class="flex flex-col gap-3">
        // 搜索框
        <div class="relative">
          <Icon
            kind=IconKind::Search
            class="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground"
          />
          <input
            type="search"
            aria-label=move || t("搜索关键词")
            placeholder=move || t("搜索术语、频率、呼号、天线、元件……")
            class=input_class("h-11 pl-9")
            prop:value=move || query.get()
            on:input=move |e| query.set(event_target_value(&e))
          />
        </div>

        // 结果列表
        <div class="max-h-[60vh] overflow-y-auto">
          {move || {
            glossary_ready.track();
            question_ready.track();
            let q = query.get();
            let trimmed = q.trim();
            if trimmed.is_empty() {
              return view! {
                <div class="px-2 py-8 text-center text-sm text-muted-foreground">
                  {move || t("输入关键词检索全站知识，如「驻波比」「FT8」「三极管」「DXCC」「APRS」。")}
                </div>
              }
              .into_any();
            }

            let mut raw: Vec<(String, String, String, String)> = search_glossary(&q)
              .into_iter()
              .map(|(p, h, t, d)| (p.to_owned(), h.to_owned(), t, d))
              .collect();
            for e in search(&q) {
              raw.push((e.page.to_owned(), e.href.to_owned(), e.title.clone(), e.text.clone()));
            }
            for e in search_questions(&q) {
              raw.push((e.0, e.1, e.2, e.3));
            }
            if raw.is_empty() {
              return view! {
                <div class="px-2 py-8 text-center text-sm text-muted-foreground">
                  {move || t("未找到与「")} <span class="font-medium text-foreground">{q.clone()}</span> {move || t("」相关的内容，换个关键词试试。")}
                </div>
              }
              .into_any();
            }

            let mut groups: Vec<SearchGroup> = Vec::new();
            for (page, href, title, text) in raw.into_iter().take(30) {
              match groups.iter_mut().find(|(p, _, _)| p == &page) {
                Some((_, _, items)) => items.push((title, text)),
                None => groups.push((page, href, vec![(title, text)])),
              }
            }
            let keyword = trimmed.to_owned();

            view! {
              <div class="space-y-3">
                {groups
                  .into_iter()
                  .map(|(page, href, items)| {
                    let keyword = keyword.clone();
                    view! {
                      <div>
                        <div class="mb-1 flex items-center gap-2 px-2 text-xs font-semibold text-muted-foreground">
                          <span>{page.clone()}</span>
                          <span class="h-px flex-1 bg-border"></span>
                        </div>
                        <ul class="space-y-0.5">
                          {items
                            .into_iter()
                            .map(|(title, text)| {
                              let kw = keyword.clone();
                              let href = href.clone();
                              view! {
                                <li>
                                  <a
                                    href=href
                                    class="block rounded-md px-3 py-2 transition-colors hover:bg-accent"
                                    on:click=move |_| open.set(false)
                                  >
                                    <div class="text-sm font-medium" inner_html=highlight(&title, &kw)></div>
                                    <div class="mt-0.5 truncate text-xs text-muted-foreground" inner_html=highlight(&text, &kw)></div>
                                  </a>
                                </li>
                              }
                            })
                            .collect_view()}
                        </ul>
                      </div>
                    }
                  })
                  .collect_view()}
              </div>
            }
            .into_any()
          }}
        </div>

        // 底部提示
        <div class="flex items-center justify-between border-t pt-3 text-[11px] text-muted-foreground">
          <span>{move || t("点击结果跳转")}</span>
          <span>{move || t("Esc 关闭 · / 唤起")}</span>
        </div>
      </div>
    </Dialog>
  }
}
