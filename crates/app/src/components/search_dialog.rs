//! 全局搜索命令面板：`/` 快捷键或导航栏搜索按钮唤起，居中弹出、不打断当前流程。

use ham_web_core::search::{search, tokenize};
use leptos::prelude::*;

use crate::data;
use crate::i18n::t;
use crate::icons::{Icon, IconKind};
use crate::pages::SLANG_CATEGORY;
use crate::ui::{ControlSize, Dialog, Input, InputType};

/// 搜索结果分组：`(页面名, [(标题, 链接, 内容)])`。
///
/// 链接随**条目**走而不是随分组走：题目命中的 `href` 含 bank（`/browse?bank=…`），
/// 同一个「题库」分组里会有多个不同链接；若把 `href` 挂在分组上，组内所有条目都会
/// 跳向第一条命中的链接。
type SearchGroup = (String, Vec<(String, String, String)>);

/// HTML 转义。
fn escape_html(s: &str) -> String {
  s.replace('&', "&amp;")
    .replace('<', "&lt;")
    .replace('>', "&gt;")
    .replace('"', "&quot;")
}

/// 找出 `term` 在 `text` 中的全部命中区间（字节区间，必定落在字符边界上）。
///
/// 不能直接拿 `text.to_lowercase()` 的字节偏移去切原文：少数字符小写化后字节数会变
/// （如 `İ` → `i̇`，2 字节变 3 字节），偏移一旦漂移，`text[s..e]` 就可能落在 UTF-8
/// 字符中间而 panic —— 在 CSR 里那意味着整页白屏。因此这里分两条路：长度不变时走快的
/// 子串查找并逐条校验边界，长度变化时退回到逐字符比较。
fn match_ranges(text: &str, term: &str) -> Vec<(usize, usize)> {
  let mut out = Vec::new();
  let lower = text.to_lowercase();
  if lower.len() == text.len() {
    // 快路径：小写化未改变字节长度（ASCII / 中文都是这种情况），偏移可直接映射。
    let mut from = 0;
    while let Some(pos) = lower[from..].find(term) {
      let (start, end) = (from + pos, from + pos + term.len());
      if text.get(start..end).is_some() {
        out.push((start, end));
      }
      from = (start + term.len()).max(start + 1);
    }
    return out;
  }
  // 慢路径：在原文的字符边界上逐点比较。慢，但绝不会切出非法切片。
  let n = term.chars().count();
  for (i, _) in text.char_indices() {
    if text[i..].chars().count() < n {
      break;
    }
    let hit = text[i..]
      .chars()
      .zip(term.chars())
      .all(|(a, b)| a.to_lowercase().eq(b.to_lowercase()));
    if hit {
      let end = i + text[i..].chars().take(n).map(char::len_utf8).sum::<usize>();
      out.push((i, end));
    }
  }
  out
}

/// 高亮文本中的关键词（忽略大小写），返回 HTML。
///
/// 查询串会被切成多个检索词分别高亮 —— 与检索侧的多词 AND 匹配保持一致，
/// 否则搜「天线 驻波比」时结果能出来却一个字都不高亮。
///
/// 在原始文本上定位关键词、再对非匹配片段做转义，避免关键词命中 `&amp;` 等
/// HTML 实体内部而破坏标记。多个词的命中区间先合并再输出，避免嵌套 `<mark>`。
fn highlight(text: &str, query: &str) -> String {
  let terms = tokenize(query);
  if terms.is_empty() {
    return escape_html(text);
  }
  let mut ranges: Vec<(usize, usize)> = Vec::new();
  for t in &terms {
    ranges.extend(match_ranges(text, t));
  }
  if ranges.is_empty() {
    return escape_html(text);
  }
  ranges.sort_unstable();
  let mut merged: Vec<(usize, usize)> = Vec::new();
  for (s, e) in ranges {
    match merged.last_mut() {
      Some(last) if s <= last.1 => last.1 = last.1.max(e),
      _ => merged.push((s, e)),
    }
  }

  let mut result = String::new();
  let mut last = 0;
  for (s, e) in merged {
    if s < last {
      continue;
    }
    result.push_str(&escape_html(&text[last..s]));
    result.push_str("<mark class=\"rounded-sm bg-primary/20 px-0.5 text-primary\">");
    result.push_str(&escape_html(&text[s..e]));
    result.push_str("</mark>");
    last = e;
  }
  result.push_str(&escape_html(&text[last..]));
  result
}

/// 术语表 / 简语关键词搜索（运行时 JSON 数据），返回 `(页面名, 路由, 标题, 内容)`。
fn search_glossary(query: &str) -> Vec<(&'static str, &'static str, String, String)> {
  // 与知识库检索一致地切词：搜「天线 驻波比」要的是同时含两个词的条目，
  // 而不是把「天线 驻波比」整串（含空格）当一个子串去查 —— 后者永远查不到。
  let terms = tokenize(query);
  if terms.is_empty() {
    return Vec::new();
  }
  let Some(glossary) = data::glossary_loaded() else {
    return Vec::new();
  };
  let mut matched: Vec<(u8, &'static str, &'static str, String, String)> = glossary
    .entries()
    .iter()
    .filter_map(|e| {
      // 多词 AND：每个词都要命中，整体排名取最弱的那个词（否则「一个词很匹配、
      // 另一个词只是提到」的条目会挤到前面）。
      let mut worst = 0u8;
      for t in &terms {
        worst = worst.max(e.match_rank(t)?);
      }
      let is_slang = e.category_key() == SLANG_CATEGORY;
      let rank = worst;
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
  let terms = tokenize(query);
  if terms.is_empty() {
    return Vec::new();
  }
  let Some(index) = data::question_index_loaded() else {
    return Vec::new();
  };
  // 链接里的查询串必须编码：含空格、`&`、`#` 时未编码会把 URL 截断或注入额外参数。
  let href_q = crate::util::encode_uri_component(query.trim());
  let mut matched: Vec<(String, String, String, String)> = index
    .iter()
    .filter(|e| {
      let text = e.search_text();
      terms.iter().all(|t| text.contains(t.as_str()))
    })
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
        format!("/browse?bank={}&q={href_q}", e.bank),
        title,
        text,
      )
    })
    .collect();
  // 题干越短通常越精确，靠前展示。
  matched.sort_by_key(|a| a.2.chars().count());
  matched
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn escapes_html_outside_matches() {
    assert_eq!(highlight("a<b", "zzz"), "a&lt;b");
    assert_eq!(
      highlight("a<b", "a"),
      "<mark class=\"rounded-sm bg-primary/20 px-0.5 text-primary\">a</mark>&lt;b"
    );
  }

  #[test]
  fn highlights_every_term() {
    let out = highlight("天线与驻波比", "天线 驻波比");
    assert_eq!(out.matches("<mark").count(), 2, "{out}");
  }

  #[test]
  fn merges_overlapping_ranges() {
    // 「ab」与「bc」的命中区间重叠，合并后只应产生一个 <mark>，不能嵌套。
    let out = highlight("abc", "ab bc");
    assert_eq!(out.matches("<mark").count(), 1, "{out}");
    assert!(out.contains(">abc<"), "{out}");
  }

  #[test]
  fn empty_query_is_plain_escaped() {
    assert_eq!(highlight("x&y", ""), "x&amp;y");
    assert_eq!(highlight("x&y", "   "), "x&amp;y");
  }

  #[test]
  fn survives_lowercasing_that_changes_byte_length() {
    // `İ` 小写后从 2 字节变 3 字节，小写串的偏移无法映射回原文。
    // 这里验证的是「不 panic 且仍能正确高亮」—— 直接切片会在这里崩掉整页。
    let out = highlight("İstanbul 天线", "天线");
    assert!(out.contains("<mark"), "{out}");
    assert!(out.contains("İstanbul"), "{out}");
    // 纯 ASCII / 中文（小写化不改变长度）走快路径，同样要高亮出来。
    assert!(highlight("istanbul 天线", "天线").contains("<mark"));
  }

  #[test]
  fn encodes_query_in_question_links() {
    // 查询串里的空格与 `&` 必须编码，否则链接会被截断。
    assert_eq!(crate::util::encode_uri_component("a b"), "a%20b");
    assert_eq!(crate::util::encode_uri_component("a&b=c"), "a%26b%3Dc");
  }
}

/// 全局搜索命令面板。开闭状态由 App 层 `provide_context::<RwSignal<bool>>()` 提供。
#[component]
pub fn SearchDialog() -> impl IntoView {
  let open = expect_context::<RwSignal<bool>>();
  let query = RwSignal::new(String::new());

  // 术语表 / 题目搜索索引按需加载：首次打开面板时拉取，加载完成后结果自动刷新。
  let glossary_ready = RwSignal::new(data::glossary_loaded().is_some());
  let question_ready = RwSignal::new(data::question_index_loaded().is_some());
  // 索引真正换新（修订号变化）后加一：题库数据更新会换出新索引，用它驱动结果重算。
  let index_epoch = RwSignal::new(0u32);

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
      // 每次打开都核对索引新鲜度：修订号未变时只是一次内存比较，变了才重新拉取。
      // 这样「题库更新后又打开搜索」不会再用旧题号去跳转。
      leptos::task::spawn_local(async move {
        let (_, refreshed) = data::load_question_index().await;
        question_ready.set(true);
        // 仅当索引真的换新（题库数据更新）时才驱动结果重算，避免每次打开都无谓重算。
        if refreshed {
          index_epoch.update(|n| *n = n.wrapping_add(1));
        }
      });
    }
  });

  view! {
    <Dialog open=open class="sm:max-w-2xl" show_close=false label=t("全站搜索")>
      <div class="flex flex-col gap-3">
        // 搜索框
        <Input
          value=query
          on_change=Callback::new(move |v: String| query.set(v))
          kind=InputType::Search
          size=ControlSize::Lg
          aria_label=Signal::derive(move || t("搜索关键词"))
          placeholder=Signal::derive(move || t("搜索术语、频率、呼号、天线、元件……"))
          prefix=move || view! { <Icon kind=IconKind::Search /> }
          clearable=true
        />

        // 结果列表
        <div class="max-h-[60vh] overflow-y-auto">
          {move || {
            glossary_ready.track();
            question_ready.track();
            // 索引刷新（题库数据更新）后重算结果
            index_epoch.track();
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
              match groups.iter_mut().find(|(p, _)| p == &page) {
                Some((_, items)) => items.push((title, href, text)),
                None => groups.push((page, vec![(title, href, text)])),
              }
            }
            let keyword = trimmed.to_owned();

            view! {
              <div class="space-y-3">
                {groups
                  .into_iter()
                  .map(|(page, items)| {
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
                            .map(|(title, href, text)| {
                              let kw = keyword.clone();
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
