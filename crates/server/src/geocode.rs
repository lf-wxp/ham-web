//! 反向地理编码代理：经纬度 → 国家 / 城市（BigDataCloud 客户端 API，无需密钥）。

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::Context;
use axum::extract::Query;
use axum::http::StatusCode;
use axum::http::header::CONTENT_TYPE;
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};
use tracing::error;

use crate::cache::Inflight;

/// 缓存时长：地理信息基本不变。
const CACHE_TTL: Duration = Duration::from_secs(24 * 3600);
/// 上游响应体上限。
const MAX_BODY: u64 = 1 << 20;

/// 查询参数。
#[derive(Deserialize)]
pub struct GeocodeQuery {
  lat: f64,
  lon: f64,
}

/// 返回给前端的 JSON。
#[derive(Serialize, Clone)]
struct GeocodeResult {
  country: String,
  city: String,
}

/// 上游返回结构（仅取需要的字段）。
#[derive(Deserialize)]
struct Upstream {
  #[serde(default, rename = "countryName")]
  country_name: String,
  #[serde(default, rename = "countryCode")]
  country_code: String,
  #[serde(default)]
  locality: String,
  #[serde(default)]
  city: String,
}

/// ISO 国家代码 → 中文名（覆盖常见业余无线电活跃实体，未覆盖时回退英文名）。
const COUNTRY_ZH: &[(&str, &str)] = &[
  ("CN", "中国"),
  ("TW", "台湾"),
  ("HK", "香港"),
  ("MO", "澳门"),
  ("JP", "日本"),
  ("KR", "韩国"),
  ("KP", "朝鲜"),
  ("MN", "蒙古"),
  ("RU", "俄罗斯"),
  ("US", "美国"),
  ("CA", "加拿大"),
  ("MX", "墨西哥"),
  ("CU", "古巴"),
  ("BR", "巴西"),
  ("AR", "阿根廷"),
  ("CL", "智利"),
  ("CO", "哥伦比亚"),
  ("PE", "秘鲁"),
  ("VE", "委内瑞拉"),
  ("GB", "英国"),
  ("FR", "法国"),
  ("DE", "德国"),
  ("IT", "意大利"),
  ("ES", "西班牙"),
  ("PT", "葡萄牙"),
  ("NL", "荷兰"),
  ("BE", "比利时"),
  ("CH", "瑞士"),
  ("AT", "奥地利"),
  ("PL", "波兰"),
  ("CZ", "捷克"),
  ("SK", "斯洛伐克"),
  ("HU", "匈牙利"),
  ("RO", "罗马尼亚"),
  ("GR", "希腊"),
  ("TR", "土耳其"),
  ("UA", "乌克兰"),
  ("SE", "瑞典"),
  ("NO", "挪威"),
  ("FI", "芬兰"),
  ("DK", "丹麦"),
  ("IS", "冰岛"),
  ("IE", "爱尔兰"),
  ("AU", "澳大利亚"),
  ("NZ", "新西兰"),
  ("PG", "巴布亚新几内亚"),
  ("FJ", "斐济"),
  ("IN", "印度"),
  ("PK", "巴基斯坦"),
  ("BD", "孟加拉国"),
  ("LK", "斯里兰卡"),
  ("NP", "尼泊尔"),
  ("BT", "不丹"),
  ("MM", "缅甸"),
  ("TH", "泰国"),
  ("VN", "越南"),
  ("KH", "柬埔寨"),
  ("LA", "老挝"),
  ("MY", "马来西亚"),
  ("SG", "新加坡"),
  ("ID", "印度尼西亚"),
  ("PH", "菲律宾"),
  ("SA", "沙特阿拉伯"),
  ("AE", "阿联酋"),
  ("QA", "卡塔尔"),
  ("KW", "科威特"),
  ("OM", "阿曼"),
  ("YE", "也门"),
  ("IL", "以色列"),
  ("JO", "约旦"),
  ("IR", "伊朗"),
  ("IQ", "伊拉克"),
  ("AF", "阿富汗"),
  ("EG", "埃及"),
  ("LY", "利比亚"),
  ("DZ", "阿尔及利亚"),
  ("MA", "摩洛哥"),
  ("TN", "突尼斯"),
  ("ZA", "南非"),
  ("KE", "肯尼亚"),
  ("ET", "埃塞俄比亚"),
  ("NG", "尼日利亚"),
  ("GH", "加纳"),
  ("TZ", "坦桑尼亚"),
  ("UG", "乌干达"),
  ("ZM", "赞比亚"),
  ("ZW", "津巴布韦"),
  ("NA", "纳米比亚"),
  ("BW", "博茨瓦纳"),
  ("MG", "马达加斯加"),
  ("MU", "毛里求斯"),
];

/// 拉取并解析反向地理编码。
fn fetch(lat: f64, lon: f64) -> anyhow::Result<GeocodeResult> {
  let url = format!(
    "https://api.bigdatacloud.net/data/reverse-geocode-client?latitude={lat}&longitude={lon}&localityLanguage=zh"
  );
  let mut res = ureq::get(&url).call().context("fetch geocode failed")?;
  let body = res
    .body_mut()
    .with_config()
    .limit(MAX_BODY)
    .read_to_string()
    .context("read geocode body failed")?;
  let up: Upstream = serde_json::from_str(&body).context("parse geocode failed")?;
  let country = COUNTRY_ZH
    .iter()
    .find(|(code, _)| *code == up.country_code)
    .map(|(_, name)| (*name).to_owned())
    .unwrap_or(up.country_name);
  let city = if !up.locality.is_empty() {
    up.locality
  } else {
    up.city
  };
  Ok(GeocodeResult { country, city })
}

/// 缓存条目数上限：防止攻击者用大量不同经纬度把缓存撑满。
const MAX_ENTRIES: usize = 10_000;

/// 缓存。
#[derive(Default)]
pub struct Cache {
  inner: Mutex<HashMap<(i32, i32), (Instant, GeocodeResult)>>,
  inflight: Inflight,
}

/// 经纬度量化到约 1km 精度，避免缓存无限增长。
fn key(lat: f64, lon: f64) -> (i32, i32) {
  ((lat * 100.0).round() as i32, (lon * 100.0).round() as i32)
}

/// `GET /api/geocode` 处理器。
pub async fn handler(Query(q): Query<GeocodeQuery>, cache: Arc<Cache>) -> Response {
  if !q.lat.is_finite()
    || !q.lon.is_finite()
    || !(-90.0..=90.0).contains(&q.lat)
    || !(-180.0..=180.0).contains(&q.lon)
  {
    return (StatusCode::BAD_REQUEST, "invalid lat/lon").into_response();
  }

  let k = key(q.lat, q.lon);
  loop {
    if let Some((t, r)) = cache.inner.lock().ok().and_then(|g| g.get(&k).cloned())
      && t.elapsed() < CACHE_TTL
    {
      return json(&r);
    }

    // 抢刷新权；抢不到则等待，避免并发请求在冷启动/过期瞬间拿到无谓的 503。
    if let Some(_guard) = cache.inflight.acquire() {
      let fetched = tokio::task::spawn_blocking(move || fetch(q.lat, q.lon)).await;

      // `_guard` 存活到缓存写入完成后释放，保证等待者被唤醒时能读到新缓存。
      return match fetched {
        Ok(Ok(r)) => {
          if let Ok(mut g) = cache.inner.lock() {
            crate::cache::evict_oldest(&mut *g, MAX_ENTRIES);
            g.insert(k, (Instant::now(), r.clone()));
          }
          json(&r)
        }
        Ok(Err(e)) => {
          error!("fetch geocode failed: {e:#}");
          (StatusCode::SERVICE_UNAVAILABLE, "geocode unavailable").into_response()
        }
        Err(e) => {
          error!("spawn_blocking failed: {e:#}");
          (StatusCode::SERVICE_UNAVAILABLE, "geocode unavailable").into_response()
        }
      };
    }

    cache.inflight.wait().await;
  }
}

fn json(r: &GeocodeResult) -> Response {
  match serde_json::to_string(r) {
    Ok(j) => ([(CONTENT_TYPE, "application/json")], j).into_response(),
    Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "serialize failed").into_response(),
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn key_quantizes_to_about_1km() {
    assert_eq!(key(39.9042, 116.4074), (3990, 11641));
    assert_eq!(key(39.9042, 116.4074), key(39.9044, 116.4073));
  }

  #[test]
  fn parses_upstream_fields() {
    let j = r#"{"countryName":"China","countryCode":"CN","locality":"北京市","city":"北京市"}"#;
    let u: Upstream = serde_json::from_str(j).unwrap();
    assert_eq!(u.country_name, "China");
    assert_eq!(u.country_code, "CN");
    assert_eq!(u.locality, "北京市");
  }
}
