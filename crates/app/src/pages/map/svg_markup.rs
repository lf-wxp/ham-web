//! 叠加层的 SVG 标记：拼成一段字符串交给浏览器的 HTML 解析器，点击走事件委托。
//!
//! 世界地图上的区域叠加层（分区、DXCC 着色）每块区域一个 `<polygon>` + `<title>`，整张世界约 3000 个节点，
//! 还要在平铺的三份世界里各建一遍。逐个节点用 `view!` 建，每个节点都是好几次 wasm → JS 调用
//! （创建、设属性、挂监听、挂子节点），4 倍降速下一份世界就是 50 ms 以上的长任务。
//! 拼成字符串、`inner_html` 一次解析，节点数和 DOM 形状完全不变，只是不再一个个从 wasm 里造。
//!
//! 点击不再给每个节点挂监听（3000 个闭包），而是在外层 `<g>` 上挂一个，从事件目标往上找带
//! [`KEY_ATTR`] 的节点取区域标识。

use leptos::wasm_bindgen::JsCast;

/// 区域标识所在的属性：叠加层把它写在每个可点的节点上，[`clicked_key`] 按它找。
pub(super) const KEY_ATTR: &str = "data-key";

/// 把 `text` 作为属性值或文本内容追加到 `out`（转义 `& < > "`）。
///
/// tooltip 里是实体名 / 译文，可能带 `&`；不转义的话整段标记会被解析歪。
pub(super) fn push_escaped(out: &mut String, text: &str) {
  for c in text.chars() {
    match c {
      '&' => out.push_str("&amp;"),
      '<' => out.push_str("&lt;"),
      '>' => out.push_str("&gt;"),
      '"' => out.push_str("&quot;"),
      _ => out.push(c),
    }
  }
}

/// 追加一个属性 ` name="value"`（前面带一个空格，值按 [`push_escaped`] 转义）。
///
/// 叠加层拼标记时用它写每一个文本型属性，不再手写引号和转义：漏转义就是一段被解析歪的标记。
/// 数值、固定的类名这类不会含特殊字符的值也走它，换来的是每个属性写法一致、不会漏掉引号。
pub(super) fn push_attr(out: &mut String, name: &str, value: &str) {
  out.push(' ');
  out.push_str(name);
  out.push_str("=\"");
  push_escaped(out, value);
  out.push('"');
}

/// 点击事件落在哪个区域上：从事件目标往上找第一个带 [`KEY_ATTR`] 的节点，取它的值。
///
/// 命中的节点必须在**挂着处理器的那个 `<g>` 里面**：`closest` 会一路向祖先找，不加这条限制的话，
/// 点在叠加层的空白处（没命中任何区域）却恰好落在某个带同名属性的外层节点里，就会取到一个不属于本层的值。
pub(super) fn clicked_key(ev: &leptos::ev::MouseEvent) -> Option<String> {
  let hit = ev
    .target()?
    .dyn_into::<web_sys::Element>()
    .ok()?
    .closest(&format!("[{KEY_ATTR}]"))
    .ok()??;
  let root = ev.current_target()?.dyn_into::<web_sys::Node>().ok()?;
  let hit_node: &web_sys::Node = &hit;
  root
    .contains(Some(hit_node))
    .then(|| hit.get_attribute(KEY_ATTR))?
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn attr_is_quoted_and_escaped() {
    let mut out = String::from("<g");
    push_attr(&mut out, "data-key", r#"a&"b"#);
    push_attr(&mut out, "fill-opacity", "0.55");
    assert_eq!(out, r#"<g data-key="a&amp;&quot;b" fill-opacity="0.55""#);
  }

  #[test]
  fn escapes_markup_significant_characters_only() {
    let mut out = String::new();
    push_escaped(&mut out, r#"A&B <"x"> 中文"#);
    assert_eq!(out, "A&amp;B &lt;&quot;x&quot;&gt; 中文");
  }

  #[test]
  fn escaping_appends_without_clearing() {
    let mut out = String::from("t=");
    push_escaped(&mut out, "a&b");
    assert_eq!(out, "t=a&amp;b");
  }
}
