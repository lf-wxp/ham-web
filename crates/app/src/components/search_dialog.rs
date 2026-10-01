//! 全局搜索命令面板：`/` 快捷键或导航栏搜索按钮唤起，居中弹出、不打断当前流程。

use ham_web_core::search::search;
use leptos::prelude::*;

use crate::data;
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

/// 全局搜索命令面板。开闭状态由 App 层 `provide_context::<RwSignal<bool>>()` 提供。
#[component]
pub fn SearchDialog() -> impl IntoView {
  let open = expect_context::<RwSignal<bool>>();
  let query = RwSignal::new(String::new());

  // 术语表按需加载：首次打开面板时拉取，加载完成后结果自动刷新。
  let glossary_ready = RwSignal::new(data::glossary_loaded().is_some());

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
    }
  });

  view! {
    <Dialog open=open class="sm:max-w-2xl" show_close=false label="全站搜索">
      <div class="flex flex-col gap-3">
        // 搜索框
        <div class="relative">
          <Icon
            kind=IconKind::Search
            class="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground"
          />
          <input
            type="search"
            aria-label="搜索关键词"
            placeholder="搜索术语、频率、呼号、天线、元件……"
            class=input_class("h-11 pl-9")
            prop:value=move || query.get()
            on:input=move |e| query.set(event_target_value(&e))
          />
        </div>

        // 结果列表
        <div class="max-h-[60vh] overflow-y-auto">
          {move || {
            glossary_ready.track();
            let q = query.get();
            let trimmed = q.trim();
            if trimmed.is_empty() {
              return view! {
                <div class="px-2 py-8 text-center text-sm text-muted-foreground">
                  "输入关键词检索全站知识，如「驻波比」「FT8」「三极管」「DXCC」「APRS」。"
                </div>
              }
              .into_any();
            }

            let mut raw = search_glossary(&q);
            for e in search(&q) {
              raw.push((e.page, e.href, e.title.clone(), e.text.clone()));
            }
            if raw.is_empty() {
              return view! {
                <div class="px-2 py-8 text-center text-sm text-muted-foreground">
                  "未找到与「" <span class="font-medium text-foreground">{q.clone()}</span> "」相关的内容，换个关键词试试。"
                </div>
              }
              .into_any();
            }

            let mut groups: Vec<SearchGroup> = Vec::new();
            for (page, href, title, text) in raw.into_iter().take(30) {
              match groups.iter_mut().find(|(p, _, _)| p == page) {
                Some((_, _, items)) => items.push((title, text)),
                None => groups.push((page.to_owned(), href.to_owned(), vec![(title, text)])),
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
          <span>"点击结果跳转"</span>
          <span>"Esc 关闭 · / 唤起"</span>
        </div>
      </div>
    </Dialog>
  }
}
