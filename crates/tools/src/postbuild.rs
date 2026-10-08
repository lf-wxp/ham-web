//! Trunk 构建后处理：
//!
//! 1. 生成 `sw.js`：预缓存全部静态资源（带内容哈希的版本号），题库 JSON 走 NetworkFirst，
//!    题目图片走 StaleWhileRevalidate，**运行时语言包**（`data/i18n/*.json`）同样按需
//!    缓存而不预取（否则中文用户也要为 en / es 付约 400 KB），页面导航离线回退到
//!    `index.html`；
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
///
/// 语言包（`data/i18n/*.json`）**故意排除**：预缓存清单会被 `sw.js` 的 `install`
/// 无条件 `cache.addAll` 拉一遍，而中文用户（默认语言，词典全量内嵌）根本用不到
/// en / es 两个包（合计约 400 KB）。它们改走 `sw.js` 的运行时缓存按需拉取，
/// 详见 `templates/sw.js` 的 `RUNTIME.i18n`。
///
/// 题库的**聚合产物**（`full.json` / `search-index.json`）同理排除：全站搜索与整卷
/// 模拟都是按需加载的（见 `core/question.rs` 的 `search-index` 说明），预取等于让
/// 每个用户（包括从不用搜索的人）在 install 阶段付 1.6 MB + 1.7 MB。
/// `/questions/*.json` 已被 `sw.js` 的 `RUNTIME.questions` 覆盖，离线仍可用。
fn is_excluded(rel: &str) -> bool {
  is_precompressed(rel)
    || rel == "sw.js"
    || rel == "sitemap.xml"
    || rel == "questions/full.json"
    || rel == "questions/search-index.json"
    || (rel.starts_with("data/i18n/") && rel.ends_with(".json"))
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

/// 运行时语言包（`dist/data/i18n/*.json`）的内容指纹。
///
/// 这些文件不进预缓存清单，但要参与 `__CACHE_VERSION__`（理由见 `write_service_worker`）。
fn pack_fingerprint(dist: &Path) -> Result<String> {
  let mut files: Vec<std::path::PathBuf> = match fs::read_dir(dist.join("data/i18n")) {
    Ok(entries) => entries.filter_map(Result::ok).map(|e| e.path()).collect(),
    // 没有语言包目录（未跑 `i18n-pack` 的极简构建）：指纹为空，不影响其它资源。
    Err(_) => return Ok(String::new()),
  };
  files.sort();
  let mut hasher = Sha256::new();
  for path in files {
    if path.extension().is_none_or(|e| e != "json") {
      continue;
    }
    hasher.update(path.to_string_lossy().as_bytes());
    hasher.update(fs::read(&path)?);
  }
  Ok(hex(&hasher.finalize()))
}

fn write_service_worker(dist: &Path) -> Result<usize> {
  let manifest = precache_manifest(dist)?;
  let manifest_json = serde_json::to_string(&manifest)?;
  // 版本号要同时覆盖「预缓存清单」与「运行时语言包」两类内容：语言包不进清单
  // （理由见 `is_excluded`），但只改一条译文也必须让版本变，否则运行时缓存
  // `i18n-<版本>` 不会换新，线上会静默地一直用旧译文。
  let version =
    hex(&Sha256::digest(format!("{manifest_json}{}", pack_fingerprint(dist)?).as_bytes())[..8]);
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
  // `lastmod` 取**已入库的**当前版本日期，而不是 `Utc::now()`：后者让同一份源码两次
  // 构建产出不同的 `sitemap.xml` 字节，破坏产物可复现，也让镜像层缓存与产物指纹比对
  // 无谓失效（`write_changelog` 用的是同一个日期，两者口径一致）。
  let lastmod = changelog::current();
  // 页面清单来自能力注册表：新增页面只要注册一次，站点地图自动跟上。
  // 此前这里是一份手写数组，长期漏掉了大批新页面（且没人会发现）。
  let pages = ham_web_core::registry::sitemap_entries();
  let mut xml = String::from(
    "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n",
  );
  for (path, freq, priority) in &pages {
    let _ = write!(
      xml,
      "<url>\n<loc>{site_url}{path}</loc>\n<lastmod>{lastmod}</lastmod>\n<changefreq>{freq}</changefreq>\n<priority>{priority}</priority>\n</url>\n"
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
///
/// 除了构建后处理写进 `dist/`，题库文本变化时（`cargo make dataset` /
/// `cargo make questions-normalize`）也会用同一函数刷新 `public/questions/search-index.json`。
pub(crate) fn write_question_search_index(dist: &Path) -> Result<()> {
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
    // 聚合产物都不预缓存：`install` 阶段的 `addAll` 是无条件拉取的，
    // 而搜索索引 1.7 MB 只在用户真的搜索时才需要（`RUNTIME.questions` 已覆盖离线）。
    assert!(is_excluded("questions/search-index.json"));
    assert!(!is_excluded("questions/A.json"));
    assert!(!is_excluded("index.html"));
    assert!(is_excluded("app_bg.wasm.br"));
    assert!(is_excluded("index.html.gz"));
  }

  /// 语言包不进预缓存：预缓存会在 `install` 阶段被无条件拉一遍，中文用户不该为
  /// 用不到的 en / es 付约 400 KB（改走 `RUNTIME.i18n` 按需缓存）。
  #[test]
  fn excludes_runtime_language_packs() {
    assert!(is_excluded("data/i18n/en.json"));
    assert!(is_excluded("data/i18n/es.json"));
    assert!(is_excluded("data/i18n/en.json.br"));
    // 同目录下的非 JSON（若将来有）不在此列，避免误伤。
    assert!(!is_excluded("data/i18n/index.txt"));
    assert!(!is_excluded("data/glossary/en.json"));
  }

  /// 语言包必须参与 `__CACHE_VERSION__`：不进清单时只改译文不会让清单变，
  /// 若不额外带上内容哈希，运行时缓存不会换新，线上就会一直用旧译文。
  #[test]
  fn pack_fingerprint_tracks_content() {
    let dist = std::env::temp_dir().join(format!("ham-postbuild-packs-{}", std::process::id()));
    let dir = dist.join("data/i18n");
    let _ = fs::remove_dir_all(&dist);
    // 目录不存在（未跑 `i18n-pack` 的极简构建）：指纹为空且不报错。
    assert_eq!(pack_fingerprint(&dist).unwrap(), "");
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("en.json"), r#"{"flat":{"a":"A"}}"#).unwrap();
    let first = pack_fingerprint(&dist).unwrap();
    assert!(!first.is_empty());
    assert_eq!(
      first,
      pack_fingerprint(&dist).unwrap(),
      "同一内容指纹应稳定"
    );
    fs::write(dir.join("en.json"), r#"{"flat":{"a":"B"}}"#).unwrap();
    assert_ne!(
      first,
      pack_fingerprint(&dist).unwrap(),
      "内容变了指纹必须变"
    );
    let _ = fs::remove_dir_all(&dist);
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

  /// `sitemap.xml` 必须**可复现**：`lastmod` 取已入库的版本日期，而不是构建时刻。
  ///
  /// 曾经用 `Utc::now()`：同一份源码两次构建产出不同字节，镜像层缓存与产物指纹比对
  /// 都会无谓失效（`sitemap` 本身不进预缓存，所以不影响 SW 版本号，但可复现性是硬要求）。
  #[test]
  fn sitemap_is_reproducible_and_uses_the_release_date() {
    let dir = std::env::temp_dir().join(format!("ham-sitemap-{}", std::process::id()));
    let mut seen: Vec<String> = Vec::new();
    for run in 0..2 {
      let dist = dir.join(format!("run{run}"));
      let _ = fs::remove_dir_all(&dist);
      fs::create_dir_all(&dist).unwrap();
      write_sitemap(&dist, "https://example.test").unwrap();
      seen.push(fs::read_to_string(dist.join("sitemap.xml")).unwrap());
    }
    assert_eq!(seen[0], seen[1], "两次构建的 sitemap 必须逐字节相同");
    assert!(
      seen[0].contains(&format!("<lastmod>{}</lastmod>", changelog::current())),
      "lastmod 应取版本日期而不是构建时刻：{}",
      seen[0].lines().take(6).collect::<Vec<_>>().join(" | ")
    );
    assert!(!seen[0].contains("T00:"), "lastmod 里不该出现构建时刻");
    let _ = fs::remove_dir_all(&dir);
  }
}
