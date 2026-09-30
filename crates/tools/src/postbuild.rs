//! Trunk 构建后处理：
//!
//! 1. 生成 `sw.js`：预缓存全部静态资源（带内容哈希的版本号），题库 JSON 走 NetworkFirst，
//!    题目图片走 StaleWhileRevalidate，页面导航离线回退到 `index.html`；
//! 2. 生成 `sitemap.xml`；
//! 3. 把 `index.html` 中的 `__SITE_URL__` 替换为实际站点地址。

use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result, ensure};
use serde::Serialize;
use sha2::{Digest, Sha256};
use walkdir::WalkDir;

const SW_TEMPLATE: &str = include_str!("../templates/sw.js");

/// 不进入预缓存的文件。
fn is_excluded(rel: &str) -> bool {
  rel == "sw.js"
    || rel == "sitemap.xml"
    || rel == "questions/full.json"
    || rel.ends_with(".map")
    || rel.rsplit('/').next().is_some_and(|f| f.starts_with('.'))
}

#[derive(Serialize)]
struct Entry {
  url: String,
  revision: String,
}

fn hex(bytes: &[u8]) -> String {
  bytes
    .iter()
    .fold(String::with_capacity(bytes.len() * 2), |mut s, b| {
      let _ = write!(s, "{b:02x}");
      s
    })
}

fn precache_manifest(dist: &Path) -> Result<Vec<Entry>> {
  let mut entries = Vec::new();
  for entry in WalkDir::new(dist).sort_by_file_name() {
    let entry = entry?;
    if !entry.file_type().is_file() {
      continue;
    }
    let rel = entry
      .path()
      .strip_prefix(dist)?
      .to_string_lossy()
      .replace('\\', "/");
    if is_excluded(&rel) {
      continue;
    }
    let digest = Sha256::digest(fs::read(entry.path())?);
    entries.push(Entry {
      url: format!("/{rel}"),
      revision: hex(&digest[..8]),
    });
  }
  Ok(entries)
}

fn write_service_worker(dist: &Path) -> Result<usize> {
  let manifest = precache_manifest(dist)?;
  let manifest_json = serde_json::to_string(&manifest)?;
  let version = hex(&Sha256::digest(manifest_json.as_bytes())[..8]);
  let sw = SW_TEMPLATE
    .replace("__CACHE_VERSION__", &version)
    .replace("__PRECACHE_MANIFEST__", &manifest_json);
  fs::write(dist.join("sw.js"), sw)?;
  println!(
    "Generated sw.js (version {version}, {} precached files)",
    manifest.len()
  );
  Ok(manifest.len())
}

fn write_sitemap(dist: &Path, site_url: &str) -> Result<()> {
  let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
  let pages = [
    ("", "weekly", "1"),
    ("/photo-processor", "monthly", "0.8"),
    ("/practice", "weekly", "0.9"),
    ("/exam", "weekly", "0.9"),
    ("/browse", "weekly", "0.8"),
    ("/glossary", "monthly", "0.7"),
    ("/q-code", "monthly", "0.7"),
    ("/morse", "monthly", "0.7"),
    ("/phonetic", "monthly", "0.7"),
    ("/bands", "monthly", "0.7"),
    ("/reference", "monthly", "0.7"),
    ("/antennas", "monthly", "0.7"),
    ("/bandplan", "monthly", "0.7"),
    ("/prefixes", "monthly", "0.7"),
    ("/modes", "monthly", "0.7"),
    ("/analog-modes", "monthly", "0.7"),
    ("/frequencies", "monthly", "0.7"),
    ("/satellites", "monthly", "0.7"),
    ("/operating", "monthly", "0.7"),
    ("/rst", "monthly", "0.7"),
    ("/propagation", "monthly", "0.7"),
    ("/log", "monthly", "0.7"),
    ("/countdown", "monthly", "0.7"),
    ("/mistakes", "monthly", "0.7"),
    ("/bookmarks", "monthly", "0.7"),
    ("/flashcards", "monthly", "0.7"),
    ("/contest", "monthly", "0.7"),
    ("/solar", "monthly", "0.7"),
    ("/safety", "monthly", "0.7"),
    ("/license", "monthly", "0.7"),
    ("/electronics", "monthly", "0.7"),
    ("/feedline", "monthly", "0.7"),
    ("/meters", "monthly", "0.7"),
    ("/power", "monthly", "0.7"),
    ("/awards", "monthly", "0.7"),
    ("/aprs", "monthly", "0.7"),
    ("/sdr", "monthly", "0.7"),
    ("/emcomm", "monthly", "0.7"),
    ("/beginner", "monthly", "0.7"),
    ("/organizations", "monthly", "0.7"),
    ("/ardf", "monthly", "0.7"),
    ("/special-prop", "monthly", "0.7"),
    ("/antenna-diy", "monthly", "0.7"),
    ("/transceiver", "monthly", "0.7"),
    ("/dx", "monthly", "0.7"),
    ("/eqsl", "monthly", "0.7"),
    ("/grid", "monthly", "0.7"),
    ("/history", "monthly", "0.7"),
    ("/muf", "monthly", "0.7"),
    ("/portable", "monthly", "0.7"),
    ("/cw-operating", "monthly", "0.7"),
    ("/antenna-installation", "monthly", "0.7"),
    ("/ft8", "monthly", "0.7"),
    ("/repeater", "monthly", "0.7"),
    ("/wspr", "monthly", "0.7"),
    ("/logging-software", "monthly", "0.7"),
    ("/microwave", "monthly", "0.7"),
    ("/remote", "monthly", "0.7"),
    ("/qsl-card", "monthly", "0.7"),
    ("/eme", "monthly", "0.7"),
    ("/antenna-tuning", "monthly", "0.7"),
    ("/rfi", "monthly", "0.7"),
    ("/qrp", "monthly", "0.7"),
    ("/dxpedition", "monthly", "0.7"),
    ("/regulations", "monthly", "0.7"),
    ("/antenna-farm", "monthly", "0.7"),
    ("/nvis", "monthly", "0.7"),
    ("/rtty", "monthly", "0.7"),
    ("/iota", "monthly", "0.7"),
    ("/gnuradio", "monthly", "0.7"),
    ("/swl", "monthly", "0.7"),
    ("/amplifier", "monthly", "0.7"),
    ("/atv", "monthly", "0.7"),
    ("/filters", "monthly", "0.7"),
    ("/antenna-modeling", "monthly", "0.7"),
    ("/most-wanted", "monthly", "0.7"),
    ("/repeater-build", "monthly", "0.7"),
    ("/cabrillo", "monthly", "0.7"),
    ("/polarization", "monthly", "0.7"),
    ("/dv-network", "monthly", "0.7"),
    ("/dx-spots", "weekly", "0.8"),
    ("/dashboard", "weekly", "0.9"),
    ("/progress", "weekly", "0.8"),
    ("/grid-map", "weekly", "0.8"),
    ("/grayline", "weekly", "0.8"),
    ("/stats", "weekly", "0.8"),
    ("/grounding", "monthly", "0.7"),
    ("/antenna-analyzer", "monthly", "0.7"),
    ("/power-supply", "monthly", "0.7"),
    ("/sstv", "monthly", "0.7"),
    ("/weather-sat", "monthly", "0.7"),
    ("/packet", "monthly", "0.7"),
    ("/mobile", "monthly", "0.7"),
    ("/license-classes", "monthly", "0.7"),
    ("/receiver", "monthly", "0.7"),
    ("/antenna-array", "monthly", "0.7"),
    ("/tools", "monthly", "0.7"),
  ];
  let mut xml = String::from(
    "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n",
  );
  for (path, freq, priority) in pages {
    let _ = write!(
      xml,
      "<url>\n<loc>{site_url}{path}</loc>\n<lastmod>{now}</lastmod>\n<changefreq>{freq}</changefreq>\n<priority>{priority}</priority>\n</url>\n"
    );
  }
  xml.push_str("</urlset>\n");
  fs::write(dist.join("sitemap.xml"), xml)?;
  println!("Generated sitemap.xml");
  Ok(())
}

fn replace_site_url(dist: &Path, site_url: &str) -> Result<()> {
  let index = dist.join("index.html");
  let html =
    fs::read_to_string(&index).with_context(|| format!("failed to read {}", index.display()))?;
  fs::write(&index, html.replace("__SITE_URL__", site_url))?;
  Ok(())
}

pub fn run(dist: &Path, site_url: &str) -> Result<()> {
  ensure!(
    dist.join("index.html").is_file(),
    "{} 不存在，请先执行 trunk build",
    dist.join("index.html").display()
  );
  let site_url = site_url.trim_end_matches('/');
  replace_site_url(dist, site_url)?;
  write_sitemap(dist, site_url)?;
  write_service_worker(dist)?;
  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn excludes_non_precache_files() {
    assert!(is_excluded("sw.js"));
    assert!(is_excluded("questions/full.json"));
    assert!(is_excluded("questions/.DS_Store"));
    assert!(!is_excluded("questions/A.json"));
    assert!(!is_excluded("index.html"));
  }
}
