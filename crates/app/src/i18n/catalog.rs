//! 词典索引。
//!
//! 词条本体不在代码里，而在 `data/i18n/{lang}/{domain}.json`（按域拆分，见
//! `docs/i18n-refactor.md`）；[`build.rs`](../../build.rs) 在构建期把它们编成下面的
//! 静态表，这里只负责聚成 O(1) 的哈希索引。
//!
//! 只有中文全量入表，其它语言只编进内嵌域（`BUNDLED_DOMAINS`）—— 其余译文由
//! [`super::pack`] 在运行时按语言拉一次，不在 wasm 里。
//!
//! 词条以语义 key（`<domain>.<slug>`）索引，`zh` 与其他语言同等对待 —— 源码里调用
//! `t("shell.home")`，中文不再散落在 Rust 字面量里，改中文文案不会再让译文失效。
//!
//! 值为复数对象的词条单独成表（`EN_PLURAL` / `ES_PLURAL`）：一个 key 对应若干个
//! CLDR 类别变体，由 [`plural`] 按 [`Category`] 选取。中文没有复数变化，不生成
//! 复数表。

use std::collections::HashMap;
use std::sync::OnceLock;

use ham_web_core::plural::Category;

// 生成物：`pub static ZH / EN / ES: &[(&str, &str)]` 与 `EN_PLURAL / ES_PLURAL`，
// 源文件是 data/i18n/ 下的 JSON。
include!(concat!(env!("OUT_DIR"), "/i18n_catalog.rs"));

/// 复数词条表：`key → [(类别, 文本)]`。
type PluralMap = HashMap<&'static str, &'static [(&'static str, &'static str)]>;

/// 中文词典的哈希索引（惰性构建）。
pub(super) fn zh_map() -> &'static HashMap<&'static str, &'static str> {
  static MAP: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();
  MAP.get_or_init(|| ZH.iter().copied().collect())
}

/// 英文词典的哈希索引（惰性构建）。
pub(super) fn en_map() -> &'static HashMap<&'static str, &'static str> {
  static MAP: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();
  MAP.get_or_init(|| EN.iter().copied().collect())
}

/// 西班牙文词典的哈希索引。
pub(super) fn es_map() -> &'static HashMap<&'static str, &'static str> {
  static MAP: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();
  MAP.get_or_init(|| ES.iter().copied().collect())
}

/// 英文复数词条的哈希索引（惰性构建）。
fn en_plural_map() -> &'static PluralMap {
  static MAP: OnceLock<PluralMap> = OnceLock::new();
  MAP.get_or_init(|| EN_PLURAL.iter().copied().collect())
}

/// 西班牙文复数词条的哈希索引。
fn es_plural_map() -> &'static PluralMap {
  static MAP: OnceLock<PluralMap> = OnceLock::new();
  MAP.get_or_init(|| ES_PLURAL.iter().copied().collect())
}

/// 取内嵌复数表里 `key` 在 `category` 下的文本。
///
/// 没有该类别时回退 `other`（兜底类别），没有这个词条时 `None` —— 调用方据此继续查
/// 运行时语言包与扁平表。中文不生成复数表，直接 `None`。
pub(super) fn plural(lang: &str, key: &str, category: Category) -> Option<&'static str> {
  let map = match lang {
    "en" => en_plural_map(),
    "es" => es_plural_map(),
    _ => return None,
  };
  let variants = map.get(key)?;
  variants
    .iter()
    .find(|(c, _)| *c == category.as_str())
    .or_else(|| {
      variants
        .iter()
        .find(|(c, _)| *c == Category::Other.as_str())
    })
    .map(|(_, text)| *text)
}

/// 复数词条的兜底文本（`other` 变体）：供 [`super::t`] 在扁平表未命中时继续查，
/// 这样「词典里是复数对象、源码里仍写 `t(key)`」的旧调用点不会漏出中文。
pub(super) fn plural_other(lang: &str, key: &str) -> Option<&'static str> {
  plural(lang, key, Category::Other)
}

/// 中文原文 → key 的反向索引。
///
/// 导航分组名、页面标题这类文案定义在 `crates/core` 的 registry 里，是**运行时**传进
/// `t()` 的（`t(m.title)`），拿不到编译期 key。反向索引让这些动态调用继续用中文原文
/// 命中词条 —— 迁移不必一次性改完 `crates/core`，也不必让 registry 额外带一份 key。
fn zh_reverse() -> &'static HashMap<&'static str, &'static str> {
  static MAP: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();
  MAP.get_or_init(|| ZH.iter().map(|(k, v)| (*v, *k)).collect())
}

/// 把可能是「中文原文」的入参还原成 key；本来就是 key 时原样返回。
///
/// CJK 判定与构建工具、知识库译文抽取共用 `ham_web_core::text::has_cjk`
/// （三处口径一致，不会出现「某处漏判、页面上漏出中文」）。
pub(super) fn resolve(key: &str) -> &str {
  // 绝大多数入参是 ASCII 语义 key（`域.词条`）：先按字节快速排除，省掉 `chars()`
  // 解码 + 区段匹配的开销 —— `t()` 是每帧上千次的热路径。
  if key.is_ascii() || !ham_web_core::text::has_cjk(key) {
    return key;
  }
  zh_reverse().get(key).copied().unwrap_or(key)
}

#[cfg(test)]
mod tests {
  use super::{BUNDLED_DOMAINS, EN, EN_PLURAL, ES, ES_PLURAL, ZH};
  use ham_web_core::plural::Category;

  /// 占位符数量必须与中文原文一致，否则 `tf` 会漏填或错填。
  fn placeholders(s: &str) -> usize {
    s.matches("{}").count()
  }

  /// 一种语言的全部 key：扁平表 ∪ 复数表 —— 复数词条不在扁平表里。
  fn keys<'a>(
    flat: &'a [(&'static str, &'static str)],
    plural: &'a [(&'static str, &'static [(&'static str, &'static str)])],
  ) -> std::collections::HashSet<&'a str> {
    flat
      .iter()
      .map(|(k, _)| *k)
      .chain(plural.iter().map(|(k, _)| *k))
      .collect()
  }

  #[test]
  fn dictionaries_have_no_duplicate_keys() {
    for (name, dict) in [("ZH", ZH), ("EN", EN), ("ES", ES)] {
      let mut seen = std::collections::HashSet::new();
      for (k, _) in dict {
        assert!(seen.insert(*k), "{name} 里出现重复词条：{k}");
      }
    }
    for (name, table) in [("EN", EN_PLURAL), ("ES", ES_PLURAL)] {
      let mut seen = std::collections::HashSet::new();
      for (k, _) in table {
        assert!(seen.insert(*k), "{name}_PLURAL 里出现重复词条：{k}");
      }
    }
  }

  /// 非中文只内嵌 `BUNDLED_DOMAINS`：这些域必须在每种语言里都齐全（首屏与离线兜底
  /// 全靠它们），而内嵌表本身不能有中文词典之外的 key。
  ///
  /// 其余域由运行时语言包提供（`super::pack`），入表与否由 `check-i18n` 的语言包
  /// 新鲜度检查兜住 —— 那一步比对的是 `data/i18n/` 与 `public/data/i18n/` 的全量内容。
  #[test]
  fn bundled_domains_are_embedded_in_every_language() {
    let zh: std::collections::HashSet<_> = ZH.iter().map(|(k, _)| *k).collect();
    for (name, dict, table) in [("EN", EN, EN_PLURAL), ("ES", ES, ES_PLURAL)] {
      let flat: std::collections::HashSet<_> = dict.iter().map(|(k, _)| *k).collect();
      assert_eq!(flat.len(), dict.len(), "{name} 里出现重复词条");
      let set = keys(dict, table);
      // 复数词条与扁平词条互斥：写进复数表的 key 不该再出现在扁平表里。
      for (k, _) in table {
        assert!(
          !flat.contains(k),
          "{name} 的 {k} 同时出现在扁平表与复数表里"
        );
      }
      for k in &set {
        assert!(zh.contains(k), "{name} 里的 {k} 不在中文词典里");
      }
      for k in &zh {
        let domain = k.split('.').next().unwrap_or_default();
        if BUNDLED_DOMAINS.contains(&domain) {
          assert!(set.contains(k), "内嵌域词条未编进 {name}：{k}");
        }
      }
    }
  }

  /// `other` 是兜底类别：缺了它，`tp` 在没命中具体类别时会无处可退。
  #[test]
  fn plural_entries_always_have_an_other_variant() {
    for (name, table) in [("EN", EN_PLURAL), ("ES", ES_PLURAL)] {
      for (k, variants) in table {
        assert!(!variants.is_empty(), "{name} 的复数词条没有变体：{k}");
        assert!(
          variants.iter().any(|(c, _)| *c == Category::Other.as_str()),
          "{name} 的复数词条缺少 other 变体：{k}"
        );
        for (c, _) in variants.iter() {
          assert!(
            Category::from_variant(c).is_some(),
            "{name} 的 {k} 含非法复数类别：{c}"
          );
        }
      }
    }
  }

  /// 复数变体的占位符同样以中文侧为准：中文是扁平的，因此每个变体都与中文原文比对。
  #[test]
  fn plural_variants_keep_the_same_placeholders() {
    let zh: std::collections::HashMap<_, _> = ZH.iter().copied().collect();
    for (name, table) in [("EN", EN_PLURAL), ("ES", ES_PLURAL)] {
      for (k, variants) in table {
        let base = zh.get(k).map_or(0, |v| placeholders(v));
        for (c, text) in variants.iter() {
          assert_eq!(
            base,
            placeholders(text),
            "{name} 复数变体占位符数量不一致：{k}#{c} → {text}"
          );
        }
      }
    }
  }

  /// 占位符以中文侧为准：语义 key 本身不含 `{}`。
  #[test]
  fn translations_keep_the_same_placeholders() {
    let zh: std::collections::HashMap<_, _> = ZH.iter().copied().collect();
    for (name, dict) in [("EN", EN), ("ES", ES)] {
      for (k, v) in dict {
        let base = zh.get(k).copied().unwrap_or("");
        assert_eq!(
          placeholders(base),
          placeholders(v),
          "{name} 词条占位符数量不一致：{k} → {v}"
        );
      }
    }
  }

  /// 中文释义必须唯一：`zh_reverse()` 是「中文原文 → key」的**单值**映射，两条 key 共用
  /// 同一句中文时后写的会覆盖先写的 —— 另一条靠反向索引的调用点会静默拿到别的译文。
  #[test]
  fn chinese_texts_are_unique_for_reverse_lookup() {
    let mut seen: std::collections::HashMap<&str, &str> = std::collections::HashMap::new();
    for (k, v) in ZH {
      if let Some(prev) = seen.insert(*v, *k) {
        panic!("{prev} 与 {k} 的中文释义相同（{v}）：反向索引只能命中其中一个");
      }
    }
  }

  /// key 必须带域前缀，否则按域拆分与后续的按需加载都无从下手。
  #[test]
  fn keys_are_namespaced_by_domain() {
    for (k, _) in ZH {
      assert!(k.contains('.'), "key 缺少域前缀：{k}");
    }
    for (name, table) in [("EN", EN_PLURAL), ("ES", ES_PLURAL)] {
      for (k, _) in table {
        assert!(k.contains('.'), "{name}_PLURAL 的 key 缺少域前缀：{k}");
      }
    }
  }
}
