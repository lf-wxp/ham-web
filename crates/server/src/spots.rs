//! DX 实时热点（DX Cluster）代理：拉取 DXWatch 全球 DX spot（无浏览器 CORS 限制）。
//!
//! spot 更新频繁，缓存 60 秒。返回报告者、频率、DX 台、备注、时间与年龄。

use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use axum::response::Response;
use serde::Serialize;

use crate::cache::JsonCache;

/// DXWatch DX spot 数据源。
const DXWATCH_URL: &str = "https://www.dxwatch.com/dxsd1/s.php?s=0&r=50";
/// 服务端缓存时长。
const CACHE_TTL: Duration = Duration::from_secs(60);
/// 上游响应体上限。
const MAX_BODY: u64 = 1 << 20;
/// 返回的最大 spot 数。
const MAX_SPOTS: usize = 50;

/// 一条 DX spot。
#[derive(Serialize, Clone)]
struct Spot {
  spotter: String,
  freq_khz: u32,
  dx: String,
  country: String,
  comment: String,
  time: String,
  age: u32,
}

/// 上游频率字段 → 整 kHz。
///
/// 上游对不同来源给不同类型：整数（`14074`）、浮点（`1841.6`、`14294.3`）、偶尔是数字串。
/// 只认 `as_u64()` 时浮点一律落成 `None` 再被 `unwrap_or(0)` 吃掉，页面上就是「0 MHz」；
/// 因此三种形态都认，四舍五入到 kHz（对外契约仍是 `u32` kHz）。非法 / 负数 / 超范围取 0，
/// 由前端按「频率未知」展示。
fn freq_khz_of(v: &serde_json::Value) -> u32 {
  let khz = match v {
    serde_json::Value::Number(n) => n.as_f64(),
    serde_json::Value::String(s) => s.trim().parse::<f64>().ok(),
    _ => None,
  };
  match khz {
    Some(f) if (0.0..f64::from(u32::MAX)).contains(&f) => f.round() as u32,
    _ => 0,
  }
}

/// 数字实体（`&#NN;` / `&#xHH;`）里的那个字符。
///
/// 两处比 `parse` / `from_str_radix` 严：只收纯数字（它们会放过前导 `+`，
/// 把 `&#x+41;` 解成 `A`，而实体语法里没有符号位）；不产出控制字符 ——
/// 解码结果直接进展示字段与通知正文，`&#27;` 这类实体不该绕过原文里本来会被拦下的控制字符。
fn numeric_entity(digits: &str, radix: u32) -> Option<char> {
  if digits.is_empty() || !digits.chars().all(|c| c.is_digit(radix)) {
    return None;
  }
  let ch = char::from_u32(u32::from_str_radix(digits, radix).ok()?)?;
  (!ch.is_control()).then_some(ch)
}

/// 解码 HTML 实体（`&lt;` `&gt;` `&amp;` `&quot;` `&#39;` `&#NN;` `&#xHH;`）。
///
/// 上游的备注 / 呼号是 HTML 转义过的文本（`FN41&lt;F2&gt;EM63`），而页面按纯文本渲染，
/// 不解码就把实体原样露给用户。只解码这一小撮：不认识的 `&xxx;` 原样保留，
/// 绝不做「二次解码」（`&amp;lt;` 只解一层，得到 `&lt;`）。
fn unescape_html(s: &str) -> String {
  let mut out = String::with_capacity(s.len());
  let mut rest = s;
  while let Some(amp) = rest.find('&') {
    out.push_str(&rest[..amp]);
    let tail = &rest[amp..];
    // 实体最长是 `&#x10FFFF;`（10 个字符），只在开头 12 个**字符**里找分号：
    // 备注里常混着中文，按字节切 `[..12]` 会落在多字节字符中间而 panic。
    let semi = tail
      .char_indices()
      .take(12)
      .find_map(|(i, c)| (c == ';').then_some(i));
    let decoded = semi.and_then(|semi| {
      let ch = match &tail[1..semi] {
        "lt" => '<',
        "gt" => '>',
        "amp" => '&',
        "quot" => '"',
        "apos" => '\'',
        num if num.starts_with("#x") || num.starts_with("#X") => numeric_entity(&num[2..], 16)?,
        num if num.starts_with('#') => numeric_entity(&num[1..], 10)?,
        _ => return None,
      };
      Some((ch, semi + 1))
    });
    match decoded {
      Some((ch, used)) => {
        out.push(ch);
        rest = &tail[used..];
      }
      None => {
        out.push('&');
        rest = &tail[1..];
      }
    }
  }
  out.push_str(rest);
  out
}

/// 从 `ci` 缓存对象读取呼号对应的国家/地区名。
fn country_of(ci: Option<&serde_json::Map<String, serde_json::Value>>, call: &str) -> String {
  ci.and_then(|c| c.get(call))
    .and_then(|val| val.as_array())
    .and_then(|arr| arr.get(1))
    .and_then(|x| x.as_str())
    .unwrap_or("")
    .to_owned()
}

/// 拉取并解析 DXWatch（同步，在 `spawn_blocking` 中执行）。
fn fetch_and_parse() -> anyhow::Result<Vec<Spot>> {
  let mut res = crate::util::http_agent()
    .get(DXWATCH_URL)
    .call()
    .context("fetch DXWatch failed")?;
  let body = res
    .body_mut()
    .with_config()
    .limit(MAX_BODY)
    .read_to_string()
    .context("read DXWatch body failed")?;

  let v: serde_json::Value = serde_json::from_str(&body).context("parse DXWatch json failed")?;
  let s = v.get("s").and_then(|x| x.as_object()).context("no spots")?;
  let ci = v.get("ci").and_then(|x| x.as_object());

  let mut spots: Vec<Spot> = s
    .values()
    .filter_map(|val| {
      let arr = val.as_array()?;
      if arr.len() < 6 {
        return None;
      }
      let dx = unescape_html(arr[2].as_str().unwrap_or(""));
      if dx.is_empty() {
        return None;
      }
      Some(Spot {
        spotter: unescape_html(arr[0].as_str().unwrap_or("")),
        freq_khz: freq_khz_of(&arr[1]),
        // `ci` 的键是上游原文：用解码前的呼号查，解码只作用于展示字段。
        country: country_of(ci, arr[2].as_str().unwrap_or("")),
        dx,
        comment: unescape_html(arr[3].as_str().unwrap_or("")),
        time: arr[4].as_str().unwrap_or("").to_owned(),
        age: arr[5].as_u64().unwrap_or(0) as u32,
      })
    })
    .collect();
  spots.sort_by_key(|s| s.age);
  spots.truncate(MAX_SPOTS);
  Ok(spots)
}

/// DX spot 缓存。
pub type Cache = JsonCache;

/// `GET /api/spots` 处理器。
pub async fn handler(cache: Arc<Cache>) -> Response {
  cache
    .get(CACHE_TTL, fetch_and_parse, "dx spots unavailable")
    .await
}

#[cfg(test)]
mod tests {
  use super::{freq_khz_of, unescape_html};
  use serde_json::json;

  #[test]
  fn freq_accepts_integer_float_and_numeric_string() {
    assert_eq!(freq_khz_of(&json!(14074)), 14074);
    // 实测上游给的浮点：此前一律被当成 0。
    assert_eq!(freq_khz_of(&json!(1841.6)), 1842);
    assert_eq!(freq_khz_of(&json!(14294.3)), 14294);
    assert_eq!(freq_khz_of(&json!(3573.5)), 3574);
    assert_eq!(freq_khz_of(&json!("7075.2")), 7075);
    assert_eq!(freq_khz_of(&json!(" 7074 ")), 7074);
  }

  #[test]
  fn freq_rejects_garbage_as_zero() {
    for bad in [
      json!(null),
      json!("abc"),
      json!(-5),
      json!(-0.4),
      json!(1e30),
      json!([1]),
      json!({}),
      json!(true),
    ] {
      assert_eq!(freq_khz_of(&bad), 0, "{bad}");
    }
  }

  #[test]
  fn unescape_decodes_the_common_entities() {
    assert_eq!(
      unescape_html("FN41&lt;F2&gt;EM63 FT8  Sent: -20"),
      "FN41<F2>EM63 FT8  Sent: -20"
    );
    assert_eq!(unescape_html("DM78&lt;&gt;DL82"), "DM78<>DL82");
    assert_eq!(unescape_html("JN85oo -&gt; BI01xa"), "JN85oo -> BI01xa");
    assert_eq!(
      unescape_html("R&amp;D &quot;x&quot; it&#39;s"),
      "R&D \"x\" it's"
    );
    assert_eq!(unescape_html("&#x41;&#66;"), "AB");
  }

  #[test]
  fn unescape_is_single_pass_and_keeps_unknowns() {
    // 只解一层：`&amp;lt;` 是字面上的 `&lt;`，不是 `<`。
    assert_eq!(unescape_html("&amp;lt;"), "&lt;");
    // 不认识 / 残缺的实体原样保留，不吞字符。
    assert_eq!(unescape_html("a & b"), "a & b");
    assert_eq!(unescape_html("&nosuch; &lt"), "&nosuch; &lt");
    assert_eq!(unescape_html("&#xZZ; &#99999999;"), "&#xZZ; &#99999999;");
    assert_eq!(unescape_html("无实体，中文 OK"), "无实体，中文 OK");
    // `&` 后面紧跟多字节字符：窗口按字节切会落在字符中间而 panic。
    assert_eq!(
      unescape_html("&中文中文中文中文中文"),
      "&中文中文中文中文中文"
    );
    assert_eq!(unescape_html("日本&lt;JA&gt;"), "日本<JA>");
    assert_eq!(unescape_html(""), "");
    assert_eq!(unescape_html("&"), "&");
  }

  #[test]
  fn numeric_entities_take_plain_digits_and_printable_chars_only() {
    // 前导正号：`from_str_radix` / `parse` 会放过，实体语法里没有符号位。
    for raw in ["&#x+41;", "&#+65;", "&#x;", "&#;", "&#x-1;", "&#1 2;"] {
      assert_eq!(unescape_html(raw), raw, "{raw}");
    }
    // 控制字符不解码：`\n`、ESC、DEL 与 C1 区的 NEL 都原样留着。
    for raw in ["&#0;", "&#10;", "&#27;", "&#x1B;", "&#127;", "&#x85;"] {
      assert_eq!(unescape_html(raw), raw, "{raw}");
    }
    // 可显示字符照常解码（含十六进制大写前缀与非 ASCII）。
    assert_eq!(unescape_html("&#x4e2d;&#25991;&#X41;"), "中文A");
  }

  #[test]
  fn parses_dxwatch_spots() {
    let json = r#"{"s":{"1":["OH6NUW",7015,"VE7CA","","0549z 30 Sep",340,12,1]}}"#;
    let v: serde_json::Value = serde_json::from_str(json).unwrap();
    let s = v.get("s").and_then(|x| x.as_object()).unwrap();
    let arr = s.values().next().unwrap().as_array().unwrap();
    assert_eq!(arr[0].as_str().unwrap(), "OH6NUW");
    assert_eq!(arr[1].as_u64().unwrap(), 7015);
    assert_eq!(arr[2].as_str().unwrap(), "VE7CA");
  }
}
