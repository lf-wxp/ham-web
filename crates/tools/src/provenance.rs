//! 生成物的来源戳：记录「这份数据来自哪里、哪一版、什么时候抓的」。
//!
//! 为什么需要：`dxcc`、`dxcc-map`、`dataset` 都从外部拉数据，产物入库（`git` 里能看到），
//! 但产物本身不写来源，于是「上游出了新一期」「本地落后几期」只能靠人记。此前判断
//! `dxcc.txt` 是哪一期，只能靠 `=VERSION` 那条规则恰好落在哪个实体上反推——上游一换
//! 版本实体就失效。
//!
//! 这里给每种产物配一份戳：抓取地址、内容 sha256、抓取时间，以及能确定时的上游版本号与
//! HTTP 校验头（`ETag` / `Last-Modified`）。文本产物把戳写成 `#` 注释块（不破坏解析），
//! 二进制产物写同名 `.source.json` 边车。
//!
//! 有了戳，`check-data-freshness` 对 `dxcc.txt` 这类逐行转写的产物可以直接比上游摘要，
//! 对派生产物则带着记下的校验头发一次条件 `HEAD`（`If-None-Match` / `If-Modified-Since`），
//! 服务端答 `304` 就是没变——都不必整份重下。

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// 一份产物的来源戳。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Stamp {
  /// 实际抓取的地址；离线 `--input` 时是 `file://<文件名>`（不写绝对路径，见 [`local_source`]）。
  pub source: String,
  /// 上游原始字节的 sha256（十六进制）。**补戳产物**（见 `backfilled`）里是产物自身的摘要。
  pub digest: String,
  /// 抓取时间（UTC，RFC 3339）。
  pub fetched_at: String,
  /// 上游版本标识（能确定时才有，如 `CTY-3632`）。
  #[serde(skip_serializing_if = "Option::is_none")]
  pub version: Option<String>,
  /// HTTP `ETag`（下载时由服务端给出）。
  #[serde(skip_serializing_if = "Option::is_none")]
  pub etag: Option<String>,
  /// HTTP `Last-Modified`（下载时由服务端给出）。
  #[serde(skip_serializing_if = "Option::is_none")]
  pub last_modified: Option<String>,
  /// 生成这份产物时用到的**仓库内**输入：相对路径 → 文件内容 sha256。
  ///
  /// 这是「上游没变、但依赖变了」的唯一线索。最关键的一例：
  /// `dxcc-entities.bin` 的实体名来自 `dxcc.txt`，`dxcc.txt` 一变，`.bin` 就该重建，
  /// 但两者之间原本没有任何记录，看不出来。
  #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
  pub inputs: BTreeMap<String, String>,
  /// 这是给**既有产物补写**的戳（`stamp-data`），不是抓取上游时写下的。
  ///
  /// 补戳时拿不到当初那份上游字节，`digest` 只能是产物自己的摘要；若照常拿它去比上游，
  /// 第一次比对就必然误报「有更新」。所以带这个标记的戳在新鲜度检查里一律判为
  /// 「需要重新生成一次来换真戳」，而不是去比摘要。
  #[serde(default, skip_serializing_if = "std::ops::Not::not")]
  pub backfilled: bool,
}

/// 注释块里各字段的前缀。解析时按前缀取值，格式改了也能读回旧文件。
const TAG_SOURCE: &str = "源: ";
const TAG_DIGEST: &str = "摘要: sha256:";
const TAG_FETCHED: &str = "抓取: ";
const TAG_VERSION: &str = "上游版本: ";
const TAG_ETAG: &str = "ETag: ";
const TAG_MODIFIED: &str = "Last-Modified: ";
const TAG_INPUT: &str = "输入: ";
const TAG_BACKFILLED: &str = "补戳: ";

impl Stamp {
  /// 按内容算戳（时间为当前 UTC 时间）。
  #[must_use]
  pub fn of(source: impl Into<String>, bytes: &[u8]) -> Self {
    Self::of_digest(source, digest_hex(bytes))
  }

  /// 摘要已经算好时用（上游由多个文件组成，合并摘要由调用方给）。
  #[must_use]
  pub fn of_digest(source: impl Into<String>, digest: String) -> Self {
    Self {
      source: source.into(),
      digest,
      fetched_at: now_utc(),
      version: None,
      etag: None,
      last_modified: None,
      inputs: BTreeMap::new(),
      backfilled: false,
    }
  }

  /// 离线读取的本地文件：来源记成 `file://<文件名>`，便于一眼看出「不是从上游抓的」。
  #[must_use]
  pub fn of_local(path: &std::path::Path, bytes: &[u8]) -> Self {
    Self::of(local_source(path), bytes)
  }

  /// 标成「补写的戳」（摘要是产物自身的，不是上游的）。
  #[must_use]
  pub fn into_backfilled(mut self) -> Self {
    self.backfilled = true;
    self
  }

  #[must_use]
  pub fn with_version(mut self, version: impl Into<String>) -> Self {
    self.version = Some(version.into());
    self
  }

  /// 记下 HTTP 校验头；空值不记，避免产出 `ETag: ` 这种无效行。
  #[must_use]
  pub fn with_validators(mut self, etag: Option<&str>, last_modified: Option<&str>) -> Self {
    self.etag = etag.filter(|v| !v.is_empty()).map(str::to_owned);
    self.last_modified = last_modified.filter(|v| !v.is_empty()).map(str::to_owned);
    self
  }

  /// 记一个仓库内输入（相对路径 + 其内容摘要，口径见 [`input_digest`]）。
  /// 文件读不到时跳过，不阻断生成。
  #[must_use]
  pub fn with_input(mut self, root: &Path, rel: &str) -> Self {
    if let Ok(bytes) = std::fs::read(root.join(rel)) {
      self
        .inputs
        .insert(rel.to_owned(), input_digest(rel, &bytes));
    }
    self
  }

  /// 渲染成 `#` 注释块（每行一个前缀，末行带换行）。文本产物直接拼在文件头。
  #[must_use]
  pub fn header_block(&self, tool: &str) -> String {
    let mut out = String::new();
    let _ = writeln!(
      out,
      "# 由 `ham-web-tools {tool}` 从 {} 生成，请勿手工编辑。",
      self.source
    );
    let _ = writeln!(out, "# {TAG_DIGEST}{}", self.digest);
    let _ = writeln!(out, "# {TAG_FETCHED}{}", self.fetched_at);
    if self.backfilled {
      let _ = writeln!(
        out,
        "# {TAG_BACKFILLED}是（摘要是既有产物自身的，不是上游的；下次 `cargo make {tool}` 会写入真实戳）"
      );
    }
    if let Some(v) = &self.version {
      let _ = writeln!(out, "# {TAG_VERSION}{v}");
    }
    if let Some(v) = &self.last_modified {
      let _ = writeln!(out, "# {TAG_MODIFIED}{v}");
    }
    if let Some(v) = &self.etag {
      let _ = writeln!(out, "# {TAG_ETAG}{v}");
    }
    for (rel, digest) in &self.inputs {
      let _ = writeln!(out, "# {TAG_INPUT}{rel} sha256:{digest}");
    }
    out
  }

  /// 从文件头的注释块解析。没有戳（旧文件）时返回 `None`，调用方据此提示「需要重新生成」。
  #[must_use]
  pub fn parse(text: &str) -> Option<Self> {
    let mut stamp = Self::default();
    let mut seen = false;
    for line in text.lines() {
      let Some(body) = line.strip_prefix('#') else {
        break;
      };
      let body = body.trim_start();
      if let Some(v) = body.strip_prefix(TAG_DIGEST) {
        stamp.digest = v.trim().to_owned();
        seen = true;
      } else if let Some(v) = body.strip_prefix(TAG_FETCHED) {
        stamp.fetched_at = v.trim().to_owned();
      } else if let Some(v) = body.strip_prefix(TAG_BACKFILLED) {
        stamp.backfilled = v.trim_start().starts_with('是');
      } else if let Some(v) = body.strip_prefix(TAG_VERSION) {
        stamp.version = Some(v.trim().to_owned());
      } else if let Some(v) = body.strip_prefix(TAG_MODIFIED) {
        stamp.last_modified = Some(v.trim().to_owned());
      } else if let Some(v) = body.strip_prefix(TAG_ETAG) {
        stamp.etag = Some(v.trim().to_owned());
      } else if let Some(v) = body.strip_prefix(TAG_INPUT) {
        // 形如 `<相对路径> sha256:<摘要>`；路径可能含空格，按最后一个 ` sha256:` 切。
        if let Some((rel, digest)) = v.trim().rsplit_once(" sha256:")
          && !rel.is_empty()
        {
          stamp
            .inputs
            .insert(rel.to_owned(), digest.trim().to_owned());
        }
      } else if let Some(v) = body.strip_prefix(TAG_SOURCE) {
        stamp.source = v.trim().to_owned();
      } else if let Some(v) = body.strip_prefix("由 `ham-web-tools") {
        // 首行「由 … 从 <源> 生成」里也带地址：结构化字段缺失时用它兜底。
        if stamp.source.is_empty()
          && let Some(rest) = v.split("从 ").nth(1)
          && let Some(src) = rest.split(" 生成").next()
        {
          stamp.source = src.trim().to_owned();
        }
      }
    }
    // 只有摘要才是「这份文件有戳」的证据：地址可能是从首行兜底来的。
    seen.then_some(stamp)
  }
}

/// 边车存放目录（相对仓库根）。
///
/// **不放 `public/`**：`public/questions/` 是整目录 `copy-dir` 进产物的，边车放那里会被
/// 发布出去，还会因为 `fetched_at` 每次生成都变而改掉 Service Worker 的预缓存版本，
/// 让用户在数据没变时也重下整个预缓存。来源戳是构建元数据，不该是发布物。
pub const SIDECAR_DIR: &str = "data/provenance";

/// 二进制产物的边车路径：`<root>/data/provenance/<产物相对路径>.source.json`。
///
/// 按相对路径镜像，避免不同目录下同名产物（如两处 `config.json`）互相覆盖。
#[must_use]
pub fn sidecar_path(root: &Path, artifact: &Path) -> std::path::PathBuf {
  let rel = artifact.strip_prefix(root).unwrap_or_else(|_| {
    // 产物不在仓库内（测试里的临时路径）：退回只用文件名。
    artifact.file_name().map_or(artifact, Path::new)
  });
  let mut name = rel.file_name().unwrap_or_default().to_os_string();
  name.push(".source.json");
  root.join(SIDECAR_DIR).join(rel).with_file_name(name)
}

/// 写二进制产物的边车（自动创建 `data/provenance/` 下的目录）。
pub fn write_sidecar(root: &Path, artifact: &Path, stamp: &Stamp) -> anyhow::Result<()> {
  use anyhow::Context as _;

  let sidecar = sidecar_path(root, artifact);
  if let Some(dir) = sidecar.parent() {
    std::fs::create_dir_all(dir).with_context(|| format!("failed to create {}", dir.display()))?;
  }
  let json = serde_json::to_string_pretty(stamp).context("serialize provenance")?;
  std::fs::write(&sidecar, format!("{json}\n"))
    .with_context(|| format!("failed to write {}", sidecar.display()))
}

/// 读边车；不存在或格式不对都返回 `None`（旧产物没有边车，属正常情况）。
#[must_use]
pub fn read_sidecar(root: &Path, artifact: &Path) -> Option<Stamp> {
  let text = std::fs::read_to_string(sidecar_path(root, artifact)).ok()?;
  serde_json::from_str(&text).ok()
}

/// 把边车里**已记录**的每个输入，重新记成它们此刻的摘要。返回有没有改动。
///
/// 用在「产物已经按最新输入重建、但重建路径不经过联网生成器」的场合：题库的解析由离线的
/// `explanations-apply` 写回，它执行完题库就与当前 `data/explanations.json` 一致了，
/// 边车里记的旧摘要必须同步，否则 `check-data-sync` 会要求联网重跑 `cargo make dataset`。
///
/// 只刷新已有的输入键，不新增；边车不存在时什么也不做（那是 `stamp-data` 的职责）。
pub fn refresh_inputs(root: &Path, artifact: &Path) -> anyhow::Result<bool> {
  use anyhow::Context as _;

  let Some(mut stamp) = read_sidecar(root, artifact) else {
    return Ok(false);
  };
  let mut changed = false;
  for (rel, recorded) in &mut stamp.inputs {
    let path = root.join(rel);
    let bytes =
      std::fs::read(&path).with_context(|| format!("failed to read {}", path.display()))?;
    let now = input_digest(rel, &bytes);
    if *recorded != now {
      *recorded = now;
      changed = true;
    }
  }
  if changed {
    write_sidecar(root, artifact, &stamp)?;
  }
  Ok(changed)
}

/// 本地输入文件的来源记法：`file://<文件名>`。
///
/// 只记名字、不记绝对路径：戳会随产物入库，`file:///Users/<谁>/…` 既泄露本机用户名，
/// 也让同一份数据在不同机器上生成出不同的戳。
#[must_use]
pub fn local_source(path: &Path) -> String {
  let name = path.file_name().map_or_else(
    || path.display().to_string(),
    |n| n.to_string_lossy().into_owned(),
  );
  format!("file://{name}")
}

/// 仓库内输入的摘要口径。
///
/// `.txt` 数据文件（目前只有 `dxcc.txt`）顶部是生成器写的 `#` 注释头，里面的抓取时间每次
/// 重新生成都变；摘要若把它算进去，上游没变、只是重跑了一遍 `cargo make dxcc`，依赖它的
/// `.bin` 就会被误判成过期。所以对 `.txt` 只摘**数据行**，其余文件按原始字节。
#[must_use]
pub fn input_digest(rel: &str, bytes: &[u8]) -> String {
  if !rel.ends_with(".txt") {
    return digest_hex(bytes);
  }
  let text = String::from_utf8_lossy(bytes);
  let body: String = text
    .lines()
    .filter(|l| !l.starts_with('#'))
    .map(|l| format!("{l}\n"))
    .collect();
  digest_hex(body.as_bytes())
}

/// 一次下载的结果：原始字节 + 服务端给的校验头。
pub struct Fetched {
  pub bytes: Vec<u8>,
  pub etag: Option<String>,
  pub last_modified: Option<String>,
}

/// 下载 `url`，限制响应体大小。
///
/// 三个生成器原先各写一遍 `ureq::get(..).call()`；这里统一，并把 `ETag` / `Last-Modified`
/// 取出来存进戳——下次 `check-data-freshness` 可以用条件请求，不必整份重下。
pub fn download(url: &str, limit: u64) -> anyhow::Result<Fetched> {
  download_with(&ureq::Agent::new_with_defaults(), url, limit)
}

/// [`download`] 的可注入 agent 版本（测试用本地桩服务器时要绕开环境里的代理设置）。
pub fn download_with(agent: &ureq::Agent, url: &str, limit: u64) -> anyhow::Result<Fetched> {
  use anyhow::Context as _;

  let mut res = agent
    .get(url)
    .call()
    .with_context(|| format!("failed to download {url}"))?;
  let header = |name: &str| {
    res
      .headers()
      .get(name)
      .map(|v| v.to_str().unwrap_or_default().to_owned())
  };
  let (etag, last_modified) = (header("etag"), header("last-modified"));
  let bytes = res
    .body_mut()
    .with_config()
    .limit(limit)
    .read_to_vec()
    .with_context(|| format!("failed to download {url}"))?;
  Ok(Fetched {
    bytes,
    etag,
    last_modified,
  })
}

/// 条件请求的结论。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Probe {
  /// `304 Not Modified`：上游没变。
  NotModified,
  /// 其余成功响应（通常是 `200`）：上游变了。
  Modified,
}

/// 带着记下的校验头发一次条件 `HEAD`，不下载内容。
///
/// 按 RFC 9110 §13.1.3，`If-None-Match` 存在时 `If-Modified-Since` 会被忽略，所以有 `ETag`
/// 就只发它，没有再退到 `Last-Modified`。两者都没有时调用方不该走到这里。
pub fn probe_with(
  agent: &ureq::Agent,
  url: &str,
  etag: Option<&str>,
  last_modified: Option<&str>,
) -> anyhow::Result<Probe> {
  use anyhow::Context as _;

  let mut req = agent.head(url);
  if let Some(v) = etag {
    req = req.header("If-None-Match", v);
  } else if let Some(v) = last_modified {
    req = req.header("If-Modified-Since", v);
  }
  let res = req
    .call()
    .with_context(|| format!("failed to probe {url}"))?;
  Ok(if res.status().as_u16() == 304 {
    Probe::NotModified
  } else {
    Probe::Modified
  })
}

/// 内容 sha256（十六进制小写）。
#[must_use]
pub fn digest_hex(bytes: &[u8]) -> String {
  let digest = Sha256::digest(bytes);
  digest.iter().fold(String::with_capacity(64), |mut s, b| {
    let _ = write!(s, "{b:02x}");
    s
  })
}

/// 当前 UTC 时间，格式 `2026-10-22T05:12:33Z`。
#[must_use]
pub fn now_utc() -> String {
  let secs = SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .map_or(0, |d| d.as_secs());
  format_rfc3339(secs)
}

/// 秒级时间戳 → RFC 3339（UTC）。不引入日期库：只用到「年-月-日」，换算三十行足够。
#[must_use]
pub fn format_rfc3339(secs: u64) -> String {
  let days = (secs / 86_400) as i64;
  let rem = secs % 86_400;
  let (hour, minute, second) = (rem / 3_600, (rem % 3_600) / 60, rem % 60);
  let (year, month, day) = civil_from_days(days);
  format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

/// 自 1970-01-01 起的天数 → `(年, 月, 日)`（Howard Hinnant 的 `civil_from_days`）。
fn civil_from_days(days: i64) -> (i64, u32, u32) {
  let z = days + 719_468;
  let era = z.div_euclid(146_097);
  let doe = z.rem_euclid(146_097);
  let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
  let year = yoe + era * 400;
  let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
  let mp = (5 * doy + 2) / 153;
  let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
  let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
  let year = if month <= 2 { year + 1 } else { year };
  (year, month, day)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn formats_known_instants() {
    assert_eq!(format_rfc3339(0), "1970-01-01T00:00:00Z");
    assert_eq!(format_rfc3339(1_700_000_000), "2023-11-14T22:13:20Z");
    // 闰日：2024-02-29 12:00:00 UTC
    assert_eq!(format_rfc3339(1_709_208_000), "2024-02-29T12:00:00Z");
  }

  #[test]
  fn digest_is_stable_and_hex() {
    assert_eq!(
      digest_hex(b"abc"),
      "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    assert_eq!(digest_hex(b"").len(), 64);
  }

  #[test]
  fn header_block_round_trips() {
    let stamp = Stamp::of("https://example.test/a.csv", b"body")
      .with_version("CTY-3632")
      .with_validators(Some("\"abc\""), Some("Sat, 17 Oct 2026 10:44:30 GMT"));
    let text = format!("{}460|3D2/r|Rotuma Island\n", stamp.header_block("dxcc"));
    let back = Stamp::parse(&text).expect("有戳");
    assert_eq!(back, stamp);
  }

  #[test]
  fn parse_returns_none_without_stamp() {
    // 旧产物只有一行说明，没有摘要：不能假装知道版本。
    let old = "# 由 `ham-web-tools dxcc` 从 country-files.com 的 cty.csv 生成，请勿手工编辑。\n110|KH6|Hawaii\n";
    assert!(Stamp::parse(old).is_none());
  }

  #[test]
  fn parse_reads_source_from_the_legacy_first_line() {
    let text = "# 由 `ham-web-tools dxcc` 从 https://example.test/cty.csv 生成，请勿手工编辑。\n\
                # 摘要: sha256:deadbeef\n";
    let stamp = Stamp::parse(text).expect("有戳");
    assert_eq!(stamp.source, "https://example.test/cty.csv");
    assert_eq!(stamp.digest, "deadbeef");
  }

  /// 边车必须落在 `data/provenance/` 下、按产物相对路径镜像——
  /// 放回 `public/` 会被 `copy-dir` 带进发布物（见 `SIDECAR_DIR` 的注释）。
  #[test]
  fn sidecar_path_mirrors_under_data_provenance() {
    let root = std::path::Path::new("/repo");
    assert_eq!(
      sidecar_path(root, &root.join("public/dxcc-entities.bin")),
      std::path::Path::new("/repo/data/provenance/public/dxcc-entities.bin.source.json")
    );
    assert_eq!(
      sidecar_path(root, &root.join("public/questions/config.json")),
      std::path::Path::new("/repo/data/provenance/public/questions/config.json.source.json")
    );
    // 绝不能在 public 下生成任何边车。
    for artifact in ["public/a.bin", "public/questions/b.json"] {
      let p = sidecar_path(root, &root.join(artifact));
      assert!(
        !p.starts_with(root.join("public")),
        "边车不该落在 public/：{}",
        p.display()
      );
    }
  }

  /// 产物不在仓库内（测试临时目录）时退回只用文件名，不 panic。
  #[test]
  fn sidecar_path_handles_artifact_outside_root() {
    let root = std::path::Path::new("/repo");
    let outside = std::path::Path::new("/tmp/elsewhere/x.bin");
    assert_eq!(
      sidecar_path(root, outside),
      std::path::Path::new("/repo/data/provenance/x.bin.source.json")
    );
  }

  #[test]
  fn header_block_round_trips_inputs() {
    let root = std::env::temp_dir().join(format!("prov-test-{}", std::process::id()));
    let dir = root.join("crates/core/data");
    std::fs::create_dir_all(&dir).expect("mkdir");
    std::fs::write(dir.join("dxcc.txt"), b"110|KH6|Hawaii\n").expect("write");

    let stamp = Stamp::of("https://example.test/ne.geojson", b"{}")
      .with_input(&root, "crates/core/data/dxcc.txt");
    assert!(stamp.inputs.contains_key("crates/core/data/dxcc.txt"));

    let text = stamp.header_block("dxcc-map");
    let back = Stamp::parse(&text).expect("有戳");
    assert_eq!(back.inputs, stamp.inputs);
    assert_eq!(
      back.inputs["crates/core/data/dxcc.txt"],
      digest_hex(b"110|KH6|Hawaii\n")
    );
    let _ = std::fs::remove_dir_all(&root);
  }

  #[test]
  fn missing_input_is_skipped_not_fatal() {
    let root = std::env::temp_dir().join("prov-test-missing");
    let stamp = Stamp::of("x", b"y").with_input(&root, "no/such/file.txt");
    assert!(stamp.inputs.is_empty());
  }

  #[test]
  fn sidecar_round_trips_through_json() {
    let dir = std::env::temp_dir().join(format!("prov-sidecar-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("mkdir");
    let bin = dir.join("dxcc-entities.bin");
    std::fs::write(&bin, [0u8, 1, 2]).expect("write");

    let stamp = Stamp::of("https://example.test", &[0, 1, 2]).with_version("VER20261017");
    write_sidecar(&dir, &bin, &stamp).expect("write sidecar");
    assert_eq!(read_sidecar(&dir, &bin), Some(stamp));

    // 没有边车时返回 None，而不是报错：旧产物就长这样。
    let other = dir.join("none.bin");
    assert!(read_sidecar(&dir, &other).is_none());
    let _ = std::fs::remove_dir_all(&dir);
  }

  #[test]
  fn parses_input_line_with_spaces_in_path() {
    let text = "# 摘要: sha256:aa\n# 输入: data/some dir/x.json sha256:bb\n";
    let stamp = Stamp::parse(text).expect("有戳");
    assert_eq!(stamp.inputs["data/some dir/x.json"], "bb");
  }

  /// 补戳标记必须两种载体都能往返，否则新鲜度检查读回来就认不出它是「补的」。
  #[test]
  fn backfilled_flag_round_trips_in_header_and_sidecar() {
    let stamp = Stamp::of("https://example.test/cty.csv", b"body").into_backfilled();
    let text = format!("{}110|KH6|Hawaii\n", stamp.header_block("dxcc"));
    assert!(text.contains("# 补戳: 是"), "{text}");
    assert_eq!(Stamp::parse(&text), Some(stamp.clone()));

    // 没标记的戳不能被读成补戳；JSON 里默认也不写这个字段。
    let plain = Stamp::of("https://example.test/cty.csv", b"body");
    assert!(!plain.header_block("dxcc").contains("补戳"));
    assert!(
      !Stamp::parse(&plain.header_block("dxcc"))
        .expect("有戳")
        .backfilled
    );
    let json = serde_json::to_string(&plain).expect("json");
    assert!(!json.contains("backfilled"), "{json}");
    let back: Stamp =
      serde_json::from_str(&serde_json::to_string(&stamp).expect("json")).expect("读回");
    assert!(back.backfilled);
  }

  /// 回归：`.txt` 输入的摘要若连注释头一起算，重跑一次生成器（抓取时间变了、数据没变）
  /// 就会让依赖它的产物被误判成过期。
  #[test]
  fn text_input_digest_ignores_the_comment_header() {
    let data = "110|KH6|Hawaii\n460|3D2/r|Rotuma\n";
    let a = format!("# 抓取: 2026-10-01T00:00:00Z\n{data}");
    let b = format!("# 抓取: 2026-10-22T08:00:00Z\n# ETag: x\n{data}");
    assert_eq!(
      input_digest("crates/core/data/dxcc.txt", a.as_bytes()),
      input_digest("crates/core/data/dxcc.txt", b.as_bytes())
    );
    assert_eq!(
      input_digest("crates/core/data/dxcc.txt", a.as_bytes()),
      digest_hex(data.as_bytes())
    );
    // 数据行一变摘要就变。
    assert_ne!(
      input_digest("x.txt", a.as_bytes()),
      input_digest("x.txt", format!("# h\n{data}110|KH7|Other\n").as_bytes())
    );
    // 非 .txt 文件按原始字节，哪怕首字符是 `#`。
    assert_eq!(
      input_digest("data/x.json", b"# not a header"),
      digest_hex(b"# not a header")
    );
  }

  #[test]
  fn local_source_never_leaks_an_absolute_path() {
    assert_eq!(
      local_source(Path::new("/Users/someone/work/cty.csv")),
      "file://cty.csv"
    );
    assert_eq!(local_source(Path::new("/data/dataset/")), "file://dataset");
    let stamp = Stamp::of_local(Path::new("/Users/someone/ne.geojson"), b"{}");
    assert!(!stamp.source.contains("someone"), "{}", stamp.source);
    assert!(stamp.source.starts_with("file://"));
  }

  #[test]
  fn refresh_inputs_follows_the_current_files() {
    let root = std::env::temp_dir().join(format!("prov-refresh-{}", std::process::id()));
    std::fs::create_dir_all(root.join("data")).expect("mkdir");
    std::fs::write(root.join("data/explanations.json"), b"{\"a\":1}").expect("write");
    let artifact = root.join("public/questions/config.json");
    let stamp = Stamp::of("https://example.test/", b"").with_input(&root, "data/explanations.json");
    write_sidecar(&root, &artifact, &stamp).expect("sidecar");

    // 没变化：不动边车。
    assert!(!refresh_inputs(&root, &artifact).expect("refresh"));

    // 解析库变了：刷新后记录的摘要等于新内容，其余字段（来源、抓取时间）保持原样。
    std::fs::write(root.join("data/explanations.json"), b"{\"a\":2}").expect("rewrite");
    assert!(refresh_inputs(&root, &artifact).expect("refresh"));
    let after = read_sidecar(&root, &artifact).expect("sidecar");
    assert_eq!(
      after.inputs["data/explanations.json"],
      digest_hex(b"{\"a\":2}")
    );
    assert_eq!(after.source, stamp.source);
    assert_eq!(after.fetched_at, stamp.fetched_at);

    // 没有边车的产物：什么也不做，也不凭空建一个。
    let other = root.join("public/other.bin");
    assert!(!refresh_inputs(&root, &other).expect("refresh"));
    assert!(read_sidecar(&root, &other).is_none());
    let _ = std::fs::remove_dir_all(&root);
  }

  // ───────────── 联网路径：用本地桩服务器验证（不碰真实上游） ─────────────

  /// 起一个只接一次连接的本地 HTTP 桩，返回它的地址。`reply` 拿到小写化的请求头文本，
  /// 返回整段响应。桩线程里的断言失败会让连接断开，主线程随即报错，测试照样变红。
  fn serve_once(reply: impl FnOnce(&str) -> String + Send + 'static) -> String {
    use std::io::{Read, Write};

    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
    let addr = listener.local_addr().expect("addr");
    std::thread::spawn(move || {
      let (mut conn, _) = listener.accept().expect("accept");
      let mut buf = Vec::new();
      let mut chunk = [0u8; 512];
      while !buf.windows(4).any(|w| w == b"\r\n\r\n") {
        let n = conn.read(&mut chunk).expect("read");
        if n == 0 {
          break;
        }
        buf.extend_from_slice(&chunk[..n]);
      }
      let request = String::from_utf8_lossy(&buf).to_ascii_lowercase();
      conn.write_all(reply(&request).as_bytes()).expect("write");
    });
    format!("http://{addr}/upstream")
  }

  /// 不读环境里的代理变量：桩在 127.0.0.1 上，公司代理会把它转成 502。
  fn direct_agent() -> ureq::Agent {
    ureq::Agent::new_with_config(ureq::Agent::config_builder().proxy(None).build())
  }

  const NOT_MODIFIED: &str = "HTTP/1.1 304 Not Modified\r\nConnection: close\r\n\r\n";
  const CHANGED: &str =
    "HTTP/1.1 200 OK\r\nETag: \"v2\"\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";

  #[test]
  fn conditional_head_tells_304_from_200() {
    // 服务端只在收到匹配的 If-None-Match 时才答 304。
    let reply = |req: &str| {
      assert!(req.starts_with("head "), "应当是 HEAD：{req}");
      if req.contains("if-none-match: \"v1\"") {
        NOT_MODIFIED.to_owned()
      } else {
        CHANGED.to_owned()
      }
    };
    let url = serve_once(reply);
    assert_eq!(
      probe_with(&direct_agent(), &url, Some("\"v1\""), None).expect("probe"),
      Probe::NotModified
    );
    let url = serve_once(reply);
    assert_eq!(
      probe_with(&direct_agent(), &url, Some("\"v0\""), None).expect("probe"),
      Probe::Modified
    );
  }

  #[test]
  fn conditional_head_falls_back_to_last_modified_and_never_sends_both() {
    let date = "Sat, 17 Oct 2026 10:44:30 GMT";
    // 只有 Last-Modified：发 If-Modified-Since。
    let url = serve_once(|req| {
      if req.contains(&format!("if-modified-since: {}", date.to_ascii_lowercase())) {
        NOT_MODIFIED.to_owned()
      } else {
        CHANGED.to_owned()
      }
    });
    assert_eq!(
      probe_with(&direct_agent(), &url, None, Some(date)).expect("probe"),
      Probe::NotModified
    );
    // 两者都有：RFC 9110 规定 If-None-Match 优先，所以只能发它，不能再带 If-Modified-Since。
    let url = serve_once(|req| {
      if req.contains("if-none-match:") && !req.contains("if-modified-since") {
        NOT_MODIFIED.to_owned()
      } else {
        CHANGED.to_owned()
      }
    });
    assert_eq!(
      probe_with(&direct_agent(), &url, Some("\"v1\""), Some(date)).expect("probe"),
      Probe::NotModified
    );
  }

  #[test]
  fn probe_reports_http_errors_instead_of_claiming_unchanged() {
    let url = serve_once(|_| {
      "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_owned()
    });
    assert!(probe_with(&direct_agent(), &url, Some("\"v1\""), None).is_err());
  }

  /// 下载要把服务端给的校验头原样存进戳，下次条件请求才有东西可发。
  #[test]
  fn download_captures_body_and_validators() {
    let url = serve_once(|req| {
      assert!(req.starts_with("get "), "应当是 GET：{req}");
      "HTTP/1.1 200 OK\r\nETag: W/\"abc\"\r\nLast-Modified: Sat, 17 Oct 2026 10:44:30 GMT\r\nContent-Length: 4\r\nConnection: close\r\n\r\nabcd"
        .to_owned()
    });
    let fetched = download_with(&direct_agent(), &url, 1024).expect("download");
    assert_eq!(fetched.bytes, b"abcd");
    assert_eq!(fetched.etag.as_deref(), Some("W/\"abc\""));
    assert_eq!(
      fetched.last_modified.as_deref(),
      Some("Sat, 17 Oct 2026 10:44:30 GMT")
    );
  }
}
