//! Trunk 构建后处理：
//!
//! 1. 生成 `sw.js`：预缓存全部静态资源（带内容哈希的版本号），题库 JSON 走 NetworkFirst，
//!    题目图片走 StaleWhileRevalidate，页面导航离线回退到 `index.html`；
//! 2. 生成 `sitemap.xml`；
//! 3. 把 `index.html` 中的 `__SITE_URL__` 替换为实际站点地址；
//! 4. 把各题库文件的内容哈希写入 `questions/config.json`（`rev`），并导出 `changelog.json`；
//! 5. 为文本类资源（wasm / js / css / json / svg / html）预生成 `.br`（质量 11）与 `.gz`
//!    （级别 9），由服务端 `ServeDir::precompressed_*` 直接返回，比运行时压缩更小且不耗 CPU。

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result, ensure};
use ham_web_core::{Bank, BankConfig, QuestionItem, QuestionSearchEntry, changelog};
use serde::Serialize;
use sha2::{Digest, Sha256};
use walkdir::WalkDir;

const SW_TEMPLATE: &str = include_str!("../templates/sw.js");

/// 不进入预缓存的文件。
fn is_excluded(rel: &str) -> bool {
  is_precompressed(rel)
    || rel == "sw.js"
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

/// 预压缩产物。
fn is_precompressed(rel: &str) -> bool {
  rel.ends_with(".br") || rel.ends_with(".gz")
}

/// 值得预压缩的文本类资源。
fn is_compressible(rel: &str) -> bool {
  const EXT: &[&str] = &[
    ".wasm", ".js", ".css", ".json", ".svg", ".html", ".xml", ".txt", ".bin",
  ];
  EXT.iter().any(|e| rel.ends_with(e))
}

/// 小于该大小的文件不压缩（收益不抵额外请求协商与磁盘占用）。
const MIN_COMPRESS_BYTES: usize = 1024;

fn brotli_bytes(data: &[u8]) -> Result<Vec<u8>> {
  let mut out = Vec::new();
  let params = brotli::enc::BrotliEncoderParams {
    quality: 11,
    lgwin: 24,
    ..Default::default()
  };
  brotli::BrotliCompress(&mut std::io::Cursor::new(data), &mut out, &params)?;
  Ok(out)
}

fn gzip_bytes(data: &[u8]) -> Result<Vec<u8>> {
  use std::io::Write as _;
  let mut enc = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::best());
  enc.write_all(data)?;
  Ok(enc.finish()?)
}

fn precompress(dist: &Path) -> Result<()> {
  let (mut raw, mut br, mut gz) = (0usize, 0usize, 0usize);
  for entry in WalkDir::new(dist) {
    let entry = entry?;
    if !entry.file_type().is_file() {
      continue;
    }
    let path = entry.path();
    let rel = path
      .strip_prefix(dist)?
      .to_string_lossy()
      .replace('\\', "/");
    if is_precompressed(&rel) || !is_compressible(&rel) {
      continue;
    }
    let data = fs::read(path)?;
    if data.len() < MIN_COMPRESS_BYTES {
      continue;
    }
    for (ext, bytes, total) in [
      ("br", brotli_bytes(&data)?, &mut br),
      ("gz", gzip_bytes(&data)?, &mut gz),
    ] {
      let target = path.with_file_name(format!("{}.{ext}", entry.file_name().to_string_lossy()));
      if bytes.len() * 10 < data.len() * 9 {
        *total += bytes.len();
        fs::write(target, bytes)?;
      } else {
        *total += data.len();
        let _ = fs::remove_file(target);
      }
    }
    raw += data.len();
  }
  println!(
    "Precompressed assets: {} KiB → br {} KiB / gz {} KiB",
    raw / 1024,
    br / 1024,
    gz / 1024
  );
  Ok(())
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
  // 页面清单来自能力注册表：新增页面只要注册一次，站点地图自动跟上。
  // 此前这里是一份手写数组，长期漏掉了大批新页面（且没人会发现）。
  let pages = ham_web_core::registry::sitemap_entries();
  let mut xml = String::from(
    "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n",
  );
  for (path, freq, priority) in &pages {
    let _ = write!(
      xml,
      "<url>\n<loc>{site_url}{path}</loc>\n<lastmod>{now}</lastmod>\n<changefreq>{freq}</changefreq>\n<priority>{priority}</priority>\n</url>\n"
    );
  }
  xml.push_str("</urlset>\n");
  fs::write(dist.join("sitemap.xml"), xml)?;
  println!("Generated sitemap.xml ({} urls)", pages.len());
  Ok(())
}

fn stamp_bank_revisions(dist: &Path) -> Result<()> {
  let path = dist.join("questions/config.json");
  if !path.is_file() {
    return Ok(());
  }
  let mut cfg: BankConfig = serde_json::from_str(&fs::read_to_string(&path)?)
    .with_context(|| format!("failed to parse {}", path.display()))?;
  for v in &mut cfg.versions {
    for bank in Bank::ALL {
      let file = dist.join(v.resolve_url(bank).trim_start_matches('/'));
      let rev = match fs::read(&file) {
        Ok(data) => hex(&Sha256::digest(data)[..4]),
        Err(_) => String::new(),
      };
      match bank {
        Bank::A => v.banks.a.rev = rev,
        Bank::B => v.banks.b.rev = rev,
        Bank::C => v.banks.c.rev = rev,
      }
    }
  }
  fs::write(&path, serde_json::to_string_pretty(&cfg)? + "\n")?;
  Ok(())
}

/// 导出更新日志，供旧版本页面在发现新版本时展示更新内容。
fn write_changelog(dist: &Path) -> Result<()> {
  fs::write(
    dist.join("changelog.json"),
    serde_json::to_string(&changelog::releases())?,
  )?;
  Ok(())
}

fn replace_site_url(dist: &Path, site_url: &str) -> Result<()> {
  let index = dist.join("index.html");
  let html =
    fs::read_to_string(&index).with_context(|| format!("failed to read {}", index.display()))?;
  fs::write(&index, html.replace("__SITE_URL__", site_url))?;
  Ok(())
}

/// `root` 为项目根目录：译文源文件 `data/knowledge-i18n/` 位于源码树而不是 `dist` 中，
/// 必须显式传入，不能依赖进程当前工作目录（否则换个目录执行会静默漏掉全部译文）。
pub fn run(root: &Path, dist: &Path, site_url: &str) -> Result<()> {
  ensure!(
    dist.join("index.html").is_file(),
    "{} 不存在，请先执行 trunk build",
    dist.join("index.html").display()
  );
  let site_url = site_url.trim_end_matches('/');
  replace_site_url(dist, site_url)?;
  write_sitemap(dist, site_url)?;
  stamp_bank_revisions(dist)?;
  write_question_search_index(dist)?;
  write_knowledge_i18n(root, dist)?;
  write_changelog(dist)?;
  write_service_worker(dist)?;
  precompress(dist)?;
  Ok(())
}

/// 从各题库 JSON 提取精简搜索索引（题干 + 解析），供全站搜索按需加载。
fn write_question_search_index(dist: &Path) -> Result<()> {
  let mut entries: Vec<QuestionSearchEntry> = Vec::new();
  for bank in Bank::ALL {
    let path = dist.join(format!("questions/{bank}.json"));
    let Ok(data) = fs::read(&path) else {
      continue;
    };
    let Ok(questions) = serde_json::from_slice::<Vec<QuestionItem>>(&data) else {
      continue;
    };
    for q in questions {
      let Some(id) = q.id_str() else {
        continue;
      };
      entries.push(QuestionSearchEntry {
        id: id.to_owned(),
        bank: bank.to_string(),
        q: q.question,
        exp: q.explanation.unwrap_or_default(),
      });
    }
  }
  let json = serde_json::to_vec(&entries)?;
  fs::write(dist.join("questions/search-index.json"), json)?;
  println!(
    "Generated questions/search-index.json ({} entries)",
    entries.len()
  );
  Ok(())
}

/// 把 `data/knowledge-i18n/{lang}/{module}.json` 合并为每种语言一个文件供前端按需加载。
///
/// 知识库正文按模块分批翻译，构建时再合并：这样翻译进度可以随时推进，而前端只需要
/// 关心「当前语言有没有这条译文」。源目录取自 `root`（源码树），与 `dist` 无关。
fn write_knowledge_i18n(root: &Path, dist: &Path) -> Result<()> {
  let src_dir = root.join("data/knowledge-i18n");
  let mut total = 0usize;
  for lang in ["en", "es"] {
    let lang_dir = src_dir.join(lang);
    let mut merged: BTreeMap<String, String> = BTreeMap::new();
    let Ok(entries) = fs::read_dir(&lang_dir) else {
      continue;
    };
    for e in entries.flatten() {
      let p = e.path();
      if p.extension().is_some_and(|x| x == "json") {
        let dict: BTreeMap<String, String> = serde_json::from_slice(&fs::read(&p)?)?;
        merged.extend(dict);
      }
    }
    let out_dir = dist.join("data/knowledge-i18n");
    fs::create_dir_all(&out_dir)?;
    fs::write(
      out_dir.join(format!("{lang}.json")),
      serde_json::to_vec(&merged)?,
    )?;
    total += merged.len();
    println!(
      "Generated data/knowledge-i18n/{lang}.json ({} entries)",
      merged.len()
    );
  }
  if total == 0 {
    println!("No knowledge translations yet (data/knowledge-i18n/)");
  }
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
    assert!(is_excluded("app_bg.wasm.br"));
    assert!(is_excluded("index.html.gz"));
  }

  #[test]
  fn stamps_bank_revisions() {
    let dist = std::env::temp_dir().join(format!("ham-postbuild-{}", std::process::id()));
    fs::create_dir_all(dist.join("questions")).unwrap();
    let info = r#"{"path":"/questions/A.json","description":""}"#;
    let cfg = format!(
      r#"{{"version":"1","lastModified":"","versions":[{{"id":"v","name":"v","description":"","isLatest":true,"banks":{{"A":{info},"B":{info},"C":{info}}},"updatedAt":""}}]}}"#
    );
    fs::write(dist.join("questions/config.json"), cfg).unwrap();
    fs::write(dist.join("questions/A.json"), "[]").unwrap();
    stamp_bank_revisions(&dist).unwrap();
    write_changelog(&dist).unwrap();
    let cfg: BankConfig =
      serde_json::from_str(&fs::read_to_string(dist.join("questions/config.json")).unwrap())
        .unwrap();
    let url = cfg.versioned_url("/questions/A.json");
    assert_eq!(url.len(), "/questions/A.json?v=".len() + 8, "{url}");
    let releases: Vec<changelog::Release> =
      serde_json::from_str(&fs::read_to_string(dist.join("changelog.json")).unwrap()).unwrap();
    assert_eq!(releases[0].date, changelog::current());
    let _ = fs::remove_dir_all(&dist);
  }

  #[test]
  fn compression_roundtrips_and_shrinks() {
    use std::io::Read as _;
    let data = "业余无线电 CQ CQ DE BG4ABC ".repeat(200).into_bytes();
    let br = brotli_bytes(&data).unwrap();
    let gz = gzip_bytes(&data).unwrap();
    assert!(br.len() < data.len() / 10 && gz.len() < data.len() / 5);
    let mut back = Vec::new();
    brotli::BrotliDecompress(&mut std::io::Cursor::new(&br), &mut back).unwrap();
    assert_eq!(back, data);
    back.clear();
    flate2::read::GzDecoder::new(&gz[..])
      .read_to_end(&mut back)
      .unwrap();
    assert_eq!(back, data);
    assert!(is_compressible("x_bg.wasm") && !is_compressible("a.png"));
  }
}
