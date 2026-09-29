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
    ("/bands", "monthly", "0.7"),
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
