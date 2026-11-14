//! 全局搜索命令面板：`/` 快捷键或导航栏搜索按钮唤起，居中弹出、不打断当前流程。
//!
//! 两套皮肤共用这一个弹层（外观由 `ui::Dialog` / `ui::Input` 按皮肤分派）；
//! 现代风的顶栏搜索按钮与 `/` 也指向它 —— 现代风没有「另一套搜索」。

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use ham_web_core::QuestionSearchEntry;
use ham_web_core::registry;
use ham_web_core::search::{match_strength, parse_query, qso_matches, search, tokenize};
use ham_web_core::unit_conversions::UNIT_CONVERSION_GROUPS;
use leptos::prelude::*;

use crate::components::radio::{SignalBars, TuningScale};
use crate::data;
use crate::data::core_text_tracked;
use crate::i18n::{t, tf};
use crate::icons::{Icon, IconKind};
use crate::pages::SLANG_CATEGORY;
use crate::ui::{
  ControlSize, Dialog, Input, InputType, Led, LedState, LedTone, Readout, ReadoutTone,
};

/// 站内搜索的开闭状态（App 层 `provide_context`，两套顶栏的按钮与 `/` 共用）。
///
/// **必须是独立的 newtype，不能直接 provide `RwSignal<bool>`**：现代风顶栏过去也
/// provide 过自己的 `RwSignal<bool>`（设置面板开关），`expect_context` 按类型查找，
/// 于是切换风格后像素风顶栏拿到的是前者留下的、已随组件销毁的信号 ——
/// 点击搜索按钮时 `set(true)` 写在死信号上，弹层永远不出现。
#[derive(Clone, Copy)]
pub struct SearchOpen(pub RwSignal<bool>);

/// 日志 QSO 命中上限：本地日志动辄上千条，搜索面板里只放最相关的几条，
/// 其余交给 `/log` 页的筛选。
const MAX_QSO_RESULTS: usize = 5;

/// 页面命中上限：页面是「直达」类结果，排在内容之前但不能挤满整屏。
///
/// 曾经是 3：181 条路由里只有前 3 条能被「输入页面名直达」，第 4 条以后的路由
/// 在站内搜索里等于不存在。提到 8 之后仍给内容留 22 条（总量上限 30），
/// 单屏也不会被页面占满 —— 页面组天然排在内容前面，8 条是「一屏能扫完」的上限
/// （上限与分档规则见 `docs/ui/modern.md` §4.4）。
const MAX_PAGE_RESULTS: usize = 8;

/// 页面命中排序的分档：完全同名 < 标题前缀 < 标题包含 < 仅路由包含。
///
/// 只按 `registry` 顺序截断会让长尾页永远进不来（见 `docs/ui/modern.md` §4.4）：键入「天线」时
/// 「天线建模」应该排在「天线长度计算器」之前，而不是按注册表里谁先登记谁在前。
fn page_rank(title: &str, href: &str, needle: &str) -> usize {
  let title = title.to_lowercase();
  let href = href.to_lowercase();
  if title == needle {
    0
  } else if title.starts_with(needle) {
    1
  } else if title.contains(needle) {
    2
  } else if href.contains(needle) {
    3
  } else {
    4
  }
}

/// 按页面标题与路由搜页面（标题是中文原文，渲染时再过一层词典）。
///
/// 知识索引只覆盖有正文的页面，`/log`、`/backup` 这类入口页不在里面；
/// 少了这一段，站内搜索就找不回「输入页面名直接跳过去」的能力。
fn search_pages(q: &str) -> Vec<(String, String, String, String)> {
  let needle = q.trim().to_lowercase();
  if needle.is_empty() {
    return Vec::new();
  }
  let group = t("shell.search-pages");
  let mut hits: Vec<(usize, usize, &registry::Module)> = registry::MODULES
    .iter()
    .enumerate()
    .filter_map(|(i, m)| {
      let mut rank = page_rank(m.title, m.nav_href(), &needle);
      // 曾用名（`registry::FORMER_TITLES`）算作标题命中的下一档：老用户输入旧名还能直达。
      if rank == 4
        && registry::former_title(m.path).is_some_and(|old| page_rank(old, "", &needle) < 4)
      {
        rank = 3;
      }
      (rank < 4).then_some((rank, i, m))
    })
    .collect();
  // 同档内保持注册表顺序（`sort_by_key` 稳定），分档只看匹配质量。
  hits.sort_by_key(|(rank, i, _)| (*rank, *i));
  hits
    .into_iter()
    .take(MAX_PAGE_RESULTS)
    .map(|(_, _, m)| {
      let href = m.nav_href().to_owned();
      (
        group.clone(),
        href.clone(),
        core_text_tracked(m.title),
        href,
      )
    })
    .collect()
}

/// 在本地日志里搜索 QSO（根 `LogStore` 的内存状态，按引用读取、不克隆）。
///
/// 判定（自由词 + `dxcc:` / `band:` / `mode:` 前缀）在核心 [`qso_matches`]，
/// 与静态条目同一套口径。
fn search_qsos(
  terms: &[String],
  filters: &[(String, String)],
) -> Vec<(String, String, String, String)> {
  // 直接读根 store 里的日志：它既是最新状态，也免掉「每敲一个键就重新反序列化整本
  // 日志快照」（localStorage 里上千条记录，`serde_json` 解析比字符串比较贵得多）。
  crate::pages::log::use_log_store()
    .logbook
    .with_untracked(|logbook| {
      let mut out = Vec::new();
      for e in &logbook.entries {
        if !qso_matches(e, terms, filters) {
          continue;
        }
        let title = format!("{} · {} {}", e.callsign, e.date, e.time);
        let text = format!("{} · {} · {}", e.mode, e.band, e.freq);
        // 分组标题复用既有词条（`radio.log` = 「日志」）：新造同义词条会撞上
        // 「中文释义必须唯一」的反向索引测试。
        out.push((t("radio.log"), "/log".to_owned(), title, text));
        if out.len() >= MAX_QSO_RESULTS {
          break;
        }
      }
      out
    })
}

/// 搜索结果分组：`(页面名, [(标题, 链接, 内容)])`。
///
/// 链接随**条目**走而不是随分组走：题目命中的 `href` 含 bank（`/browse?bank=…`），
/// 同一个「题库」分组里会有多个不同链接；若把 `href` 挂在分组上，组内所有条目都会
/// 跳向第一条命中的链接。
type SearchGroup = (String, Vec<(String, String, String)>);

/// 像素风扫描面板的状态：待命 → 扫描中（输入还在防抖期）→ 锁定 N 条 / 无信号。
#[derive(Clone, Copy, PartialEq, Eq)]
enum ScanState {
  Idle,
  Scanning,
  Lock(usize),
  NoSignal,
}

/// 面板里最多列多少条结果（各来源合计）。
const MAX_RESULTS: usize = 30;

/// 一个查询的命中（分组名、链接、标题、摘要，至多 `MAX_RESULTS` 条），按来源优先级排好：
/// 自己的日志 → 页面 → 术语 → 知识条目 → 单位换算 → 题目。
fn collect_results(q: &str) -> Vec<(String, String, String, String)> {
  let (qso_terms, qso_filters) = parse_query(q);
  // 自己的日志排最前：用户搜呼号时第一条就该是那条通联。
  let mut raw = search_qsos(&qso_terms, &qso_filters);
  // 其次是页面：键入的若是页面名（「史密斯」「通联日志」），第一条就该能直达。
  raw.extend(search_pages(q));
  raw.extend(
    search_glossary(q)
      .into_iter()
      .map(|(p, h, t, d)| (p.to_owned(), h.to_owned(), t, d)),
  );
  for e in search(q) {
    raw.push((
      e.page.to_owned(),
      e.href.to_owned(),
      e.title.clone(),
      e.text.clone(),
    ));
  }
  raw.extend(
    search_unit_conversions(q)
      .into_iter()
      .map(|(p, h, t, d)| (p, h.to_owned(), t, d)),
  );
  for e in search_questions(q) {
    raw.push((e.0, e.1, e.2, e.3));
  }
  // 面板只列前 `MAX_RESULTS` 条、状态条也只读 `len().min(MAX_RESULTS)`：在这里就封顶，
  // 题目命中成百上千时 `Memo` 的整表比较与渲染处的克隆都只碰这几十条。
  raw.truncate(MAX_RESULTS);
  raw
}

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
    result.push_str("<mark class=\" bg-primary/20 px-0.5 text-primary\">");
    result.push_str(&escape_html(&text[s..e]));
    result.push_str("</mark>");
    last = e;
  }
  result.push_str(&escape_html(&text[last..]));
  result
}

/// 术语表 / 简语关键词搜索（运行时 JSON 数据），返回 `(页面名, 路由, 标题, 内容)`。
fn search_glossary(query: &str) -> Vec<(String, &'static str, String, String)> {
  // 与知识库检索一致地切词：搜「天线 驻波比」要的是同时含两个词的条目，
  // 而不是把「天线 驻波比」整串（含空格）当一个子串去查 —— 后者永远查不到。
  let terms = tokenize(query);
  if terms.is_empty() {
    return Vec::new();
  }
  let Some(glossary) = data::glossary_loaded() else {
    return Vec::new();
  };
  let mut matched: Vec<(u8, String, &'static str, String, String)> = glossary
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
        (t("shell.q-codes"), "/q-code")
      } else {
        (t("shell.glossary"), "/glossary")
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

/// 单位换算速查搜索，返回 `(页面名, 路由, 标题, 内容)`。
///
/// 为什么不放进 core 的 `knowledge_index`：这张表的名称、示例是 **i18n key**（`exam.units-*`），
/// core 拿不到译文；这里按当前界面语言取译文后再匹配，所以搜「dBm」「波长」「功率比」都能命中，
/// 英文 / 西语界面则按各自的译文搜。公式本身（`dBm = 10·lg(P/mW)`）是符号表达式，跨语言一致，
/// 也一并参与匹配。
fn search_unit_conversions(query: &str) -> Vec<(String, &'static str, String, String)> {
  let terms = tokenize(query);
  if terms.is_empty() {
    return Vec::new();
  }
  // 与 core 索引里公式条目同一个页面名（`/formulas` 页），搜索结果里归到同一组。
  let page = t("exam.formula-reference");
  UNIT_CONVERSION_GROUPS
    .iter()
    .flat_map(|g| g.items.iter())
    .filter_map(|it| {
      let name = t(it.name);
      let example = t(it.example);
      let hay = format!("{name} {} {example}", it.expr).to_lowercase();
      terms
        .iter()
        .all(|term| hay.contains(term.as_str()))
        .then(|| {
          (
            page.clone(),
            "/formulas",
            name,
            format!("{} · {example}", it.expr),
          )
        })
    })
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

/// 题目索引每条的小写检索文本，连同生成它时的那份索引（索引换新就重建）。
type QuestionTexts = (Arc<Vec<QuestionSearchEntry>>, Rc<Vec<String>>);

thread_local! {
  static QUESTION_TEXTS: RefCell<Option<QuestionTexts>> = const { RefCell::new(None) };
}

/// 题目索引的检索文本（会话内缓存）：此前每输入一个字都要对约 3500 条各拼接、转小写一遍。
fn question_texts(index: &Arc<Vec<QuestionSearchEntry>>) -> Rc<Vec<String>> {
  if let Some(hit) = QUESTION_TEXTS.with_borrow(|c| {
    c.as_ref()
      .filter(|(src, _)| Arc::ptr_eq(src, index))
      .map(|(_, texts)| Rc::clone(texts))
  }) {
    return hit;
  }
  let texts = Rc::new(
    index
      .iter()
      .map(QuestionSearchEntry::search_text)
      .collect::<Vec<_>>(),
  );
  QUESTION_TEXTS.with_borrow_mut(|c| *c = Some((Arc::clone(index), Rc::clone(&texts))));
  texts
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
  let texts = question_texts(&index);
  // 链接里的查询串必须编码：含空格、`&`、`#` 时未编码会把 URL 截断或注入额外参数。
  let href_q = crate::util::encode_uri_component(query.trim());
  let mut matched: Vec<(String, String, String, String)> = index
    .iter()
    .zip(texts.iter())
    .filter(|(_, text)| terms.iter().all(|t| text.contains(t.as_str())))
    .map(|(e, _)| e)
    .take(20)
    .map(|e| {
      let title = truncate(&e.q, 42);
      let text = if e.exp.is_empty() {
        title.clone()
      } else {
        truncate(&e.exp, 90)
      };
      (
        t("exam.bank"),
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
      "<mark class=\" bg-primary/20 px-0.5 text-primary\">a</mark>&lt;b"
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

/// 全局搜索命令面板。开闭状态由 App 层 `provide_context(SearchOpen(..))` 提供。
#[component]
pub fn SearchDialog() -> impl IntoView {
  let SearchOpen(open) = expect_context::<SearchOpen>();
  let query = RwSignal::new(String::new());
  // 输入框绑 `query`，结果按防抖后的副本算（见 `debounced`）：每次重算要扫题目、知识库、术语表与日志。
  let query_settled = crate::debounced::use_debounced(query, crate::debounced::SEARCH_DEBOUNCE);

  // 术语表 / 题目搜索索引按需加载：首次打开面板时拉取，加载完成后结果自动刷新。
  let glossary_ready = RwSignal::new(data::glossary_loaded().is_some());
  let question_ready = RwSignal::new(data::question_index_loaded().is_some());
  // 索引真正换新（修订号变化）后加一：题库数据更新会换出新索引，用它驱动结果重算。
  let index_epoch = RwSignal::new(0u32);

  // 分组标题：像素风是点阵小标签，现代风用面板里的小节标题档（12px / 600 / 字距）。
  let group_label = if crate::kit::is_modern_at_mount() {
    "md-group-title mb-1 flex items-center gap-2"
  } else {
    "pxl-label mb-1 flex items-center gap-2 px-2 text-xs text-muted-foreground"
  };

  // 结果只算一遍：列表与像素风的扫描状态条（要知道条数）共用。
  let results = Memo::new(move |_| {
    glossary_ready.track();
    question_ready.track();
    // 索引刷新（题库数据更新）后重算结果
    index_epoch.track();
    let q = query_settled.get();
    if q.trim().is_empty() {
      Vec::new()
    } else {
      collect_results(&q)
    }
  });

  // 像素风的扫描面板（canvas「站内搜索 · 扫描」，`docs/ui/pixel.md` §3.4）：标题栏的灯、刻度尺与读数屏
  // 都读同一个状态 —— 待命 → 扫描中（防抖期内）→ 锁定 N 条 / 无信号。全是装饰（`aria-hidden`）：
  // 结果本身与「没有结果」的提示在下面的列表里，两套皮肤的无障碍语义一致。
  // 弹层的 children 会被多次构建（`Fn`），所以下面都是闭包而不是建好的视图。
  let pixel = !crate::kit::is_modern_at_mount();
  let state = move || {
    let typed = query.with(|q| q.trim().to_owned());
    let settled = query_settled.with(|q| q.trim().to_owned());
    let shown = results.with(Vec::len).min(MAX_RESULTS);
    if typed.is_empty() && settled.is_empty() {
      ScanState::Idle
    } else if typed != settled {
      ScanState::Scanning
    } else if shown > 0 {
      ScanState::Lock(shown)
    } else {
      ScanState::NoSignal
    }
  };
  let lamp = move || match state() {
    ScanState::Scanning => (LedTone::Warn, LedState::Blink),
    ScanState::Lock(_) => (LedTone::Rx, LedState::On),
    ScanState::Idle | ScanState::NoSignal => (LedTone::Rx, LedState::Off),
  };
  let title_bar = move || {
    pixel.then(|| {
      view! {
        // 负外边距抵消弹层面板的内边距（`ui/dialog/dialog_root.rs` 像素分支的 `p-4 sm:p-6`），
        // 让标题栏贴边铺满；那边的内边距改了，这里的 `-m*` 要跟着改。
        <div class="pxl-section-title -mx-4 -mt-4 sm:-mx-6 sm:-mt-6" aria-hidden="true">
          {move || t("shell.scan-title")}
          <span class="ml-auto">
            {move || {
              let (tone, s) = lamp();
              view! { <Led tone=tone state=s small=true /> }
            }}
          </span>
        </div>
      }
    })
  };
  // 刻度上印的是这一次结果里真实出现的分组（最多 4 个，按结果顺序），锁定时指针停在第一组。
  let marks = Signal::derive(move || {
    let mut groups: Vec<String> = Vec::new();
    results.with(|r| {
      for (page, ..) in r.iter().take(MAX_RESULTS) {
        if groups.len() < 4 && !groups.contains(page) {
          groups.push(page.clone());
        }
      }
    });
    let n = groups.len().max(1);
    groups
      .into_iter()
      .enumerate()
      .map(|(i, g)| (g, u8::try_from((i * 2 + 1) * 50 / n).unwrap_or(50)))
      .collect::<Vec<_>>()
  });
  let needle = Signal::derive(move || match state() {
    ScanState::Lock(_) => marks.with(|m| m.first().map_or(1, |(_, at)| *at)),
    _ => 1,
  });
  let scan = move || {
    pixel.then(|| {
      view! {
        <div class="space-y-0" aria-hidden="true">
          <TuningScale
            marks=Signal::derive(move || if matches!(state(), ScanState::Lock(_)) { marks.get() } else { Vec::new() })
            needle=needle
            sweeping=Signal::derive(move || state() == ScanState::Scanning)
          />
          <div class="pxl-display flex items-center gap-2 px-2.5 py-1.5">
            {move || {
              let (code, class) = match state() {
                ScanState::Idle => (t("shell.scan-code-idle"), "pxl-d-dim"),
                ScanState::Scanning => (t("shell.scan-code-busy"), "pxl-d-hot"),
                ScanState::Lock(_) => (t("learning.rig-lock"), "pxl-d-ok"),
                ScanState::NoSignal => (t("shell.no-signal"), "pxl-d-dim"),
              };
              view! { <span class=format!("pxl-label text-xs {class}")>{code}</span> }
            }}
            <span class="ml-auto flex items-center gap-1.5">
              <Readout
                text=Signal::derive(move || match state() {
                  ScanState::Lock(n) => format!("{n:02}"),
                  ScanState::NoSignal => "00".to_owned(),
                  _ => "--".to_owned(),
                })
                tone=Signal::derive(move || {
                  if state() == ScanState::Scanning { ReadoutTone::Hot } else { ReadoutTone::Normal }
                })
              />
              <span class="pxl-label pxl-d-dim text-xs">{move || t("shell.scan-unit")}</span>
            </span>
          </div>
        </div>
      }
    })
  };

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
    <Dialog open=open class="sm:max-w-2xl" show_close=false label="shell.site-search">
      <div class="flex flex-col gap-3">
        {title_bar}
        // 搜索框
        <Input
          value=query
          on_change=Callback::new(move |v: String| query.set(v))
          kind=InputType::Search
          size=ControlSize::Lg
          aria_label=Signal::derive(move || t("shell.search-keywords"))
          placeholder=Signal::derive(move || t("shell.search-terms-frequencies-callsigns"))
          prefix=move || view! { <Icon kind=IconKind::Search /> }
          clearable=true
        />
        {scan}

        // 结果列表。`md-search-text` 给现代风的命中高亮（`mark` 底色），像素风不能挂 ——
        // 样式表不分层，像素风里挂上会命中现代的 `mark` 配色。
        <div class=if crate::kit::is_modern_at_mount() {
          "md-search-text max-h-[60svh] overflow-y-auto overscroll-contain"
        } else {
          "max-h-[60svh] overflow-y-auto overscroll-contain"
        }>
          {move || {
            let q = query_settled.get();
            let trimmed = q.trim();
            if trimmed.is_empty() {
              return view! {
                <div class="px-2 py-8 text-center text-sm text-muted-foreground">
                  {move || t("shell.type-keywords-to-search")}
                </div>
              }
              .into_any();
            }

            // `results` 在 `collect_results` 里已封顶，这里最多克隆 `MAX_RESULTS` 条。
            let raw = results.with(Clone::clone);
            if raw.is_empty() {
              return view! {
                <div class="px-2 py-8 text-center text-sm text-muted-foreground">
                  {move || t("shell.no-results-for-u")} <span class="text-foreground">{q.clone()}</span> {move || t("shell.u-201d-try-another")}
                </div>
              }
              .into_any();
            }

            let mut groups: Vec<SearchGroup> = Vec::new();
            for (page, href, title, text) in raw.into_iter().take(MAX_RESULTS) {
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
                        <div class=group_label>
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
                                    class="flex items-center gap-3 px-3 py-2 transition-colors hover:bg-accent"
                                    on:click=move |_| open.set(false)
                                  >
                                    <div class="min-w-0 flex-1">
                                      <div class="text-sm" inner_html=highlight(&title, &kw)></div>
                                      <div class="mt-0.5 truncate text-xs text-muted-foreground" inner_html=highlight(&text, &kw)></div>
                                    </div>
                                    // 像素风：右侧信号格 = 匹配度（`core::search::match_strength`，标题命中高于正文命中）。
                                    // 格数与高度都是视觉通道，旁边补一句文字版（`SignalBars` 自己 `aria-hidden`）。
                                    {pixel.then(|| {
                                      let lit = match_strength(&kw, &title, &text);
                                      view! {
                                        <SignalBars lit=lit total=4 />
                                        <span class="sr-only">
                                          {tf("shell.match-strength", &[&lit.to_string(), "4"])}
                                        </span>
                                      }
                                    })}
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
        <div class="flex items-center justify-between border-t pt-3 text-xs text-muted-foreground">
          <span>{move || t("shell.click-a-result-to")}</span>
          <span>{move || t("shell.esc-to-close-to")}</span>
        </div>
      </div>
    </Dialog>
  }
}
