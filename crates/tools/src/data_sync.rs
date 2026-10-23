//! 离线一致性检查：产物里的来源戳是否与仓库现状对得上。
//!
//! 只读本地文件，不联网，所以可以放心挂进 `cargo make check`。它回答三个问题：
//!
//! 1. **产物有没有戳**：没有戳说明是旧版本生成的，或者被人手工改过。
//! 2. **产物内容变没变**：戳里记的摘要要和文件当前摘要一致（防手改、防半途中断）。
//! 3. **依赖变没变**：`dxcc-entities.bin` 记着它依据的 `dxcc.txt` 摘要，题库记着
//!    `data/explanations.json` 的摘要——这两处是「上游没变但产物该重建」的唯一线索。
//!
//! 发现问题时只**报告并要求重新生成**，不尝试自动修复：重建 `dxcc-entities.bin` 需要联网，
//! 把一个网络依赖塞进本地检查会让离线环境无故变红。

use std::path::Path;

use ham_web_core::staleness::{Age, age, curated};

use crate::fsutil::Paths;
use crate::provenance::{self, Probe, Stamp, digest_hex, input_digest};

/// 一处检查的结论。
struct Finding {
  target: String,
  problem: String,
  fix: String,
}

// ─────────────── 给既有产物补戳 ───────────────

/// 给**已存在**的产物补写来源戳，内容一个字不动。
///
/// 用途：产物是引入戳之前生成的（比如 `dxcc.txt` 已经在库里放了一个月）。此时拿不到
/// 当初那份 `cty.csv`，所以**不能编一个假的"上游摘要"**——那会让 `check-data-freshness`
/// 永远认为「已最新」。这里记的是「既有产物本身的摘要」，并用 `backfilled` 明确标出：
/// 新鲜度检查见到这个标记就不去比摘要（比了必然误报），而是要求联网重新生成一次换真戳。
pub fn backfill(root: &Path) -> anyhow::Result<()> {
  let mut done = 0;

  // dxcc.txt：重写注释头，保留全部数据行。
  let dxcc_path = root.join("crates/core/data/dxcc.txt");
  let text = std::fs::read_to_string(&dxcc_path)
    .map_err(|e| anyhow::anyhow!("读不到 {}：{e}", dxcc_path.display()))?;
  if Stamp::parse(&text).is_none() {
    let body: String = text
      .lines()
      .filter(|l| !l.starts_with('#'))
      .map(|l| format!("{l}\n"))
      .collect();
    // 摘要按「数据部分」算：能自证内容没被改，同时不假冒上游。
    let stamp = Stamp::of(
      "https://www.country-files.com/bigcty/cty.csv",
      body.as_bytes(),
    )
    .into_backfilled();
    std::fs::write(&dxcc_path, format!("{}{body}", stamp.header_block("dxcc")))
      .map_err(|e| anyhow::anyhow!("写 {} 失败：{e}", dxcc_path.display()))?;
    println!("✓ 已为 crates/core/data/dxcc.txt 补戳（数据未改动）");
    done += 1;
  }

  // 二进制/JSON 产物：写边车，摘要按「产物自身内容」算。
  for (rel, url) in [
    (
      "public/dxcc-entities.bin",
      "https://raw.githubusercontent.com/nvkelso/natural-earth-vector/master/geojson/ne_110m_admin_0_countries.geojson",
    ),
    (
      "public/questions/config.json",
      "https://raw.githubusercontent.com/xiedada05/crac-amateur-radio-exam-questions-2025-csv/main",
    ),
  ] {
    let path = root.join(rel);
    if provenance::read_sidecar(root, &path).is_some() {
      continue;
    }
    let bytes =
      std::fs::read(&path).map_err(|e| anyhow::anyhow!("读不到 {}：{e}", path.display()))?;
    let mut stamp = Stamp::of(url, &bytes).into_backfilled();
    // 依赖能算就算：`.bin` 依赖 dxcc.txt，题库依赖解析库。
    if rel.ends_with(".bin") {
      stamp = stamp.with_input(root, "crates/core/data/dxcc.txt");
    } else {
      stamp = stamp.with_input(root, "data/explanations.json");
    }
    provenance::write_sidecar(root, &path, &stamp)?;
    println!("✓ 已为 {rel} 补边车");
    done += 1;
  }

  if done == 0 {
    println!("· 所有产物都已有来源戳，无需补");
  } else {
    println!(
      "共补 {done} 处；这些戳带「补戳」标记，`data-freshness` 会要求重新生成一次：\
       下次 `cargo make dxcc` / `dxcc-map` / `dataset` 会写入真实戳"
    );
  }
  Ok(())
}

pub fn run(root: &Path) -> anyhow::Result<()> {
  let paths = Paths::new(root);
  let mut findings: Vec<Finding> = Vec::new();

  // ── 1. dxcc.txt：注释头里的戳 ────────────────────────────────
  let dxcc_path = root.join("crates/core/data/dxcc.txt");
  match std::fs::read(&dxcc_path) {
    Ok(bytes) => {
      let text = String::from_utf8_lossy(&bytes).into_owned();
      match provenance::Stamp::parse(&text) {
        None => findings.push(Finding {
          target: "crates/core/data/dxcc.txt".to_owned(),
          problem: "没有来源戳（无法判断来自哪一期上游）".to_owned(),
          fix: "cargo make dxcc".to_owned(),
        }),
        Some(stamp) => {
          // 戳记的是**上游原始字节**的摘要，不是生成后文本的摘要，所以这里只能查
          // 「字段齐不齐」，不能直接比对文件摘要。版本号缺失时也提示一下。
          if stamp.digest.len() != 64 {
            findings.push(Finding {
              target: "crates/core/data/dxcc.txt".to_owned(),
              problem: format!("摘要字段不合法：{:?}", stamp.digest),
              fix: "cargo make dxcc".to_owned(),
            });
          }
          if stamp.fetched_at.is_empty() {
            findings.push(Finding {
              target: "crates/core/data/dxcc.txt".to_owned(),
              problem: "缺少抓取时间".to_owned(),
              fix: "cargo make dxcc".to_owned(),
            });
          }
        }
      }
    }
    Err(e) => findings.push(Finding {
      target: "crates/core/data/dxcc.txt".to_owned(),
      problem: format!("读不到：{e}"),
      fix: "cargo make dxcc".to_owned(),
    }),
  }

  // ── 2. dxcc-entities.bin：依赖的 dxcc.txt 摘要是否还对得上 ────
  let bin_path = root.join("public/dxcc-entities.bin");
  match provenance::read_sidecar(root, &bin_path) {
    None => findings.push(Finding {
      target: "public/dxcc-entities.bin".to_owned(),
      problem: "没有 .source.json 边车".to_owned(),
      fix: "cargo make dxcc-map（需联网）".to_owned(),
    }),
    Some(stamp) => {
      // 边车记的是**输入** `dxcc.txt` 的摘要，逐个核对。
      for (rel, recorded) in &stamp.inputs {
        match std::fs::read(root.join(rel)) {
          Ok(bytes) => {
            let now = input_digest(rel, &bytes);
            if now != *recorded {
              findings.push(Finding {
                target: "public/dxcc-entities.bin".to_owned(),
                problem: format!(
                  "依赖的 {rel} 已变化（生成时 {}，当前 {}）→ 实体名可能对不上",
                  &recorded[..recorded.len().min(12)],
                  &now[..now.len().min(12)]
                ),
                fix: "cargo make dxcc-map（需联网）".to_owned(),
              });
            }
          }
          Err(e) => findings.push(Finding {
            target: "public/dxcc-entities.bin".to_owned(),
            problem: format!("依赖的 {rel} 读不到：{e}"),
            fix: "cargo make dxcc-map".to_owned(),
          }),
        }
      }
    }
  }

  // ── 3. 题库：解析库变更后是否忘了重建 ────────────────────────
  let config = paths.questions.join("config.json");
  match provenance::read_sidecar(root, &config) {
    None => findings.push(Finding {
      target: "public/questions/config.json".to_owned(),
      problem: "没有 .source.json 边车（无法判断题库基于哪份解析库构建）".to_owned(),
      fix: "cargo make dataset（需联网或 DATASET_DIR）".to_owned(),
    }),
    Some(stamp) => {
      for (rel, recorded) in &stamp.inputs {
        let Ok(bytes) = std::fs::read(root.join(rel)) else {
          findings.push(Finding {
            target: "public/questions/config.json".to_owned(),
            problem: format!("依赖的 {rel} 读不到"),
            fix: "cargo make dataset".to_owned(),
          });
          continue;
        };
        let now = input_digest(rel, &bytes);
        if now != *recorded {
          findings.push(Finding {
            target: "public/questions/config.json".to_owned(),
            problem: format!(
              "依赖的 {rel} 已变化（生成时 {}，当前 {}）→ 题库里的解析可能已过期",
              &recorded[..recorded.len().min(12)],
              &now[..now.len().min(12)]
            ),
            fix: "cargo make explanations-apply（离线）；重新拉取上游则 cargo make dataset"
              .to_owned(),
          });
        }
      }
    }
  }

  if findings.is_empty() {
    println!("✓ 数据产物来源戳齐全，依赖摘要与现状一致");
    return Ok(());
  }
  println!("✗ 数据产物的来源戳有问题 {} 处：", findings.len());
  for f in &findings {
    println!("    {}：{}", f.target, f.problem);
    println!("      → 修复：{}", f.fix);
  }
  anyhow::bail!("数据产物需要重新生成（见上方修复命令）")
}

// ─────────────── 新鲜度：比对上游 ───────────────

/// 上游字节与产物字节的关系，决定了能不能直接比摘要。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Relation {
  /// 产物是上游文件的**直接转写**（`dxcc.txt` 一行行来自 `cty.csv`）：摘要可直接比。
  Verbatim,
  /// 产物由上游**派生**（`.bin` 是从 GeoJSON 算出来的几何、题库 JSON 是从 CSV 编译的）：
  /// 上游摘要与产物摘要没有可比性，只能靠 `ETag` / `Last-Modified` 判断上游有没有变。
  Derived,
}

/// 一个待比对的数据源。
struct Target {
  name: &'static str,
  /// 产物路径（相对仓库根），用来读戳。
  artifact: &'static str,
  /// 上游地址；产物戳里记的地址优先。
  url: &'static str,
  /// 重建命令。
  fix: &'static str,
  relation: Relation,
}

const TARGETS: &[Target] = &[
  Target {
    name: "dxcc",
    artifact: "crates/core/data/dxcc.txt",
    url: "https://www.country-files.com/bigcty/cty.csv",
    fix: "cargo make dxcc",
    relation: Relation::Verbatim,
  },
  Target {
    name: "dxcc-map",
    artifact: "public/dxcc-entities.bin",
    url: "https://raw.githubusercontent.com/nvkelso/natural-earth-vector/master/geojson/ne_110m_admin_0_countries.geojson",
    fix: "cargo make dxcc-map",
    relation: Relation::Derived,
  },
  Target {
    name: "dataset",
    artifact: "public/questions/config.json",
    url: "https://raw.githubusercontent.com/xiedada05/crac-amateur-radio-exam-questions-2025-csv/main",
    fix: "cargo make dataset",
    relation: Relation::Derived,
  },
];

/// 单个数据源的比对结论。
#[derive(Debug, PartialEq, Eq)]
pub enum Verdict {
  /// 上游内容与产物记录的一致（逐行转写看摘要，派生产物看条件请求是否 `304`）。
  UpToDate,
  /// 上游已有新内容。
  Changed,
  /// 产物没有戳（旧产物），无从比对。
  NoStamp,
  /// 戳是 `stamp-data` 补写的：摘要是产物自己的，不是上游的，比对没有意义（比了必然误报）。
  /// 只能重新生成一次换真戳。
  Backfilled,
  /// 产物来源是本地文件（`--input` / `DATASET_DIR`），不该拿去比上游。
  LocalOnly,
  /// 上游连不上：只报告，按 `strict` 决定是否失败。
  Unreachable,
  /// 派生类产物：戳里没有校验头，无法判断上游有没有变（不猜）。
  Unknown,
}

/// 逐行转写产物的纯判定：给定产物戳与上游摘要，得出结论。
///
/// 抽成纯函数是为了能离线跑单测。`None` 表示没拿到上游摘要（连不上）。
#[must_use]
pub fn compare(stamp: &Stamp, upstream_digest: Option<&str>) -> Verdict {
  match upstream_digest {
    None => Verdict::Unreachable,
    Some(now) if now == stamp.digest => Verdict::UpToDate,
    Some(_) => Verdict::Changed,
  }
}

/// 上游访问的抽象：把「真的发请求」与「怎么下结论」分开，结论部分就能离线测全。
trait Upstream {
  /// 下载上游并返回它的 sha256；连不上返回 `None`。
  fn digest(&self, url: &str) -> Option<String>;
  /// 带校验头发条件 `HEAD`；连不上或服务端报错返回 `None`。
  fn probe(&self, url: &str, etag: Option<&str>, last_modified: Option<&str>) -> Option<Probe>;
}

/// 真实网络实现。
struct Http(ureq::Agent);

/// 只提示一次原因（错误链压成一行），不打印整条 `Debug`。
fn first_line(e: &anyhow::Error) -> String {
  format!("{e:#}")
    .lines()
    .next()
    .unwrap_or_default()
    .to_owned()
}

impl Upstream for Http {
  fn digest(&self, url: &str) -> Option<String> {
    match provenance::download_with(&self.0, url, 64 * 1024 * 1024) {
      Ok(fetched) => Some(digest_hex(&fetched.bytes)),
      Err(e) => {
        println!("   （下载失败：{}）", first_line(&e));
        None
      }
    }
  }

  fn probe(&self, url: &str, etag: Option<&str>, last_modified: Option<&str>) -> Option<Probe> {
    match provenance::probe_with(&self.0, url, etag, last_modified) {
      Ok(p) => Some(p),
      Err(e) => {
        println!("   （探测失败：{}）", first_line(&e));
        None
      }
    }
  }
}

/// 对一个数据源下结论。上游只在**确实需要**时才访问：没戳、本地来源、补戳、没有校验头的
/// 派生产物，都不联网。
fn judge(relation: Relation, stamp: Option<&Stamp>, url: &str, upstream: &dyn Upstream) -> Verdict {
  let Some(stamp) = stamp else {
    return Verdict::NoStamp;
  };
  if stamp.source.starts_with("file://") {
    return Verdict::LocalOnly;
  }
  if stamp.backfilled {
    return Verdict::Backfilled;
  }
  match relation {
    Relation::Verbatim => compare(stamp, upstream.digest(url).as_deref()),
    // `.bin` / 题库 JSON 都不是上游文件的逐字节转写，拿上游摘要比没有意义（会永远报
    // 「有更新」），只认校验头：有就发条件请求，没有就老实说不知道。
    Relation::Derived => {
      // 多文件来源（题库）：上游地址是目录，没有「那一个文件的校验头」，只能逐文件探。
      if !stamp.files.is_empty() {
        return judge_files(stamp, url, upstream);
      }
      if stamp.etag.is_none() && stamp.last_modified.is_none() {
        return Verdict::Unknown;
      }
      match upstream.probe(url, stamp.etag.as_deref(), stamp.last_modified.as_deref()) {
        Some(Probe::NotModified) => Verdict::UpToDate,
        Some(Probe::Modified) => Verdict::Changed,
        None => Verdict::Unreachable,
      }
    }
  }
}

/// 多文件来源的判定：对戳里记下的每个上游文件发一次条件 `HEAD`。
///
/// 优先级有意写成「变了 > 连不上 > 不知道 > 最新」：只要有一个文件确实变了就该重新生成，
/// 别让另一个文件的网络故障把它盖成「不可达」而白等一周。
fn judge_files(stamp: &Stamp, base: &str, upstream: &dyn Upstream) -> Verdict {
  let base = base.trim_end_matches('/');
  let (mut unreachable, mut unknown) = (false, false);
  for (rel, file) in &stamp.files {
    if file.etag.is_none() && file.last_modified.is_none() {
      // 下载时上游就没给校验头：这个文件判不了，只好说不知道（不猜）。
      unknown = true;
      continue;
    }
    let url = format!("{base}/{rel}");
    match upstream.probe(&url, file.etag.as_deref(), file.last_modified.as_deref()) {
      Some(Probe::NotModified) => {}
      Some(Probe::Modified) => return Verdict::Changed,
      None => unreachable = true,
    }
  }
  if unreachable {
    Verdict::Unreachable
  } else if unknown {
    Verdict::Unknown
  } else {
    Verdict::UpToDate
  }
}

pub fn freshness(root: &Path, only: &[String], strict: bool) -> anyhow::Result<()> {
  let selected: Vec<&Target> = TARGETS
    .iter()
    .filter(|t| only.is_empty() || only.iter().any(|n| n == t.name))
    .collect();
  if selected.is_empty() {
    anyhow::bail!("没有匹配的数据源：{only:?}（可用：dxcc / dxcc-map / dataset）");
  }

  let http = Http(ureq::Agent::new_with_defaults());
  let mut changed = false;
  let mut unreachable = false;
  for target in &selected {
    let path = root.join(target.artifact);
    let stamp = load_stamp(root, &path);
    // 本地来源不联网；上游地址优先用戳里记的，便于换镜像。
    let url = stamp
      .as_ref()
      .map(|s| s.source.clone())
      .filter(|s| !s.starts_with("file://"))
      .unwrap_or_else(|| target.url.to_owned());

    let verdict = judge(target.relation, stamp.as_ref(), &url, &http);

    match verdict {
      Verdict::UpToDate => println!("✓ {}：上游无变化", target.name),
      Verdict::Changed => {
        changed = true;
        println!("↑ {}：上游有新版本 → {}", target.name, target.fix);
      }
      Verdict::NoStamp => {
        changed = true;
        println!("? {}：产物没有来源戳 → {}", target.name, target.fix);
      }
      Verdict::Backfilled => {
        changed = true;
        println!(
          "? {}：来源戳是补写的（摘要不是上游的），无法判断新旧 → {}（重新生成后换成真戳）",
          target.name, target.fix
        );
      }
      Verdict::LocalOnly => {
        println!(
          "· {}：产物来自本地输入（{}），跳过上游比对",
          target.name,
          stamp.as_ref().map(|s| s.source.as_str()).unwrap_or("-")
        );
      }
      Verdict::Unknown => {
        println!(
          "· {}：产物是派生数据，戳里没有 ETag / Last-Modified，无法判断上游是否变化（不猜）",
          target.name
        );
      }
      Verdict::Unreachable => {
        unreachable = true;
        println!(
          "! {}：连不上上游（{url}）——本机网络不可用时属正常，CI 上才算问题",
          target.name
        );
      }
    }
  }

  if changed {
    // 退出码 1 = 「有更新」，定时任务据此决定要不要重新生成。
    anyhow::bail!("有数据源需要更新");
  }
  if unreachable && strict {
    anyhow::bail!("上游不可达（--strict）");
  }
  if unreachable {
    println!("（存在不可达的数据源；未要求严格模式，不影响结论）");
  }
  Ok(())
}

// ─────────────── 时效性数据的核对日期 ───────────────

/// 时间到了就该有人回去看：人工转录的外部事实没有可比的上游摘要，只能按「多久没复核」报警。
///
/// 与 [`freshness`] 的分工：那边的上游是可下载的文件（能比摘要 / 校验头），这边的上游是
/// HTML 页与法规原文（页面模板一改校验头就变，比了也没意义），所以只认 `checked_on` 的年龄。
/// 登记表在 `ham_web_core::staleness`（数据文件 / 常量各留一份日期，这里只做判定）。
///
/// `warn_only` 只放过「到期」：到期随时间自然出现，挂进每个 PR 都要跑的 `cargo make check`
/// 会让无关改动在某天突然变红；而日期读不出来 / 写在未来是数据本身的错误，任何模式都判失败。
pub fn staleness(max_months: i64, warn_only: bool) -> anyhow::Result<()> {
  let now = provenance::now_utc();
  let today = now.get(..7).unwrap_or(now.as_str());
  let items = curated();
  println!("时效性数据的核对日期（今天 {today}，阈值 {max_months} 个月）：");

  let mut stale: Vec<&str> = Vec::new();
  let mut broken: Vec<&str> = Vec::new();
  for item in &items {
    let (key, date, source) = (item.key, item.checked_on, item.source);
    match age(date, today, max_months) {
      Age::Fresh(months) => println!("✓ {key}：{date}（{months} 个月）—— {source}"),
      Age::Stale(months) => {
        println!("⚠ {key}：{date}（{months} 个月）—— {source}");
        stale.push(key);
      }
      Age::Future(_) => {
        println!("✗ {key}：核对日期 {date} 晚于今天 —— {source}");
        broken.push(key);
      }
      Age::Unreadable => {
        println!("✗ {key}：核对日期读不出来（{date:?}）—— {source}");
        broken.push(key);
      }
    }
  }

  if stale.is_empty() && broken.is_empty() {
    println!("共 {} 项，全部在 {} 个月内复核过", items.len(), max_months);
    return Ok(());
  }

  println!("→ 复核后把日期改成当前月份，改在这些地方：");
  for key in stale.iter().chain(broken.iter()) {
    println!("    {key}：{}", fix_hint(key));
  }
  if !broken.is_empty() {
    anyhow::bail!(
      "{} 项时效性数据的核对日期不可用（读不出来，或晚于今天；格式应为 YYYY-MM 或 YYYY-MM-DD）",
      broken.len()
    );
  }
  if warn_only {
    println!(
      "（--warn-only：{} 项超过 {max_months} 个月未复核，只告警不失败）",
      stale.len()
    );
    return Ok(());
  }
  anyhow::bail!("{} 项时效性数据超过 {max_months} 个月未复核", stale.len())
}

/// 每个登记项的核对日期写在哪（复核完要改的地方）。
fn fix_hint(key: &str) -> &'static str {
  match key {
    "gear-rx" => "`gear_rx::CHECKED_ON` 与 `crates/core/data/gear-rx.txt` 文件头",
    "radio-law-text" => "`crates/core/data/radio-law.txt` 文件头的 `# 核对日期：`",
    "toroid" => "`toroid::CORES[].verified`（逐条）",
    "diy-cost" => "`diy_projects::COST_CHECKED_ON`",
    _ => "`crates/core/src/provenance.rs` 的 `PROVENANCES` 里该域的 `checked_on`",
  }
}

/// 读产物的戳：文本产物读注释头，二进制读边车。
fn load_stamp(root: &Path, path: &Path) -> Option<Stamp> {
  if let Some(stamp) = provenance::read_sidecar(root, path) {
    return Some(stamp);
  }
  let text = std::fs::read_to_string(path).ok()?;
  Stamp::parse(&text)
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::provenance::Stamp;

  /// 造一个最小仓库骨架，用来验证三类问题都能被抓到。
  fn scaffold(name: &str) -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!("sync-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("crates/core/data")).expect("mkdir dxcc");
    std::fs::create_dir_all(root.join("public/questions")).expect("mkdir questions");
    std::fs::create_dir_all(root.join("data")).expect("mkdir data");
    root
  }

  fn write_stamped_dxcc(root: &Path, body: &str) -> Stamp {
    let stamp = Stamp::of("https://example.test/cty.csv", body.as_bytes());
    let text = format!("{}{body}", stamp.header_block("dxcc"));
    std::fs::write(root.join("crates/core/data/dxcc.txt"), text).expect("write dxcc.txt");
    stamp
  }

  #[test]
  fn reports_no_findings_when_everything_matches() {
    let root = scaffold("ok");
    write_stamped_dxcc(&root, "110|KH6|Hawaii\n");
    // .bin 的边车：记录当前 dxcc.txt 的摘要。
    let bin = root.join("public/dxcc-entities.bin");
    std::fs::write(&bin, [0u8]).expect("write bin");
    let dxcc_bytes = std::fs::read(root.join("crates/core/data/dxcc.txt")).expect("read");
    provenance::write_sidecar(
      &root,
      &bin,
      &Stamp::of("https://example.test/ne.geojson", &[0])
        .with_input(&root, "crates/core/data/dxcc.txt"),
    )
    .expect("sidecar");
    assert_eq!(digest_hex(&dxcc_bytes).len(), 64);

    // 题库边车与解析库
    std::fs::write(root.join("data/explanations.json"), b"{}").expect("write expl");
    provenance::write_sidecar(
      &root,
      &root.join("public/questions/config.json"),
      &Stamp::of("x", b"y").with_input(&root, "data/explanations.json"),
    )
    .expect("sidecar2");

    run(&root).expect("应当无问题");
    let _ = std::fs::remove_dir_all(&root);
  }

  #[test]
  fn catches_missing_stamp_and_sidecars() {
    let root = scaffold("missing");
    // 旧式无戳的 dxcc.txt
    std::fs::write(
      root.join("crates/core/data/dxcc.txt"),
      "# 由 `ham-web-tools dxcc` 从 country-files.com 的 cty.csv 生成，请勿手工编辑。\n110|KH6|Hawaii\n",
    )
    .expect("write");
    let err = run(&root).expect_err("应当报错");
    let msg = err.to_string();
    assert!(msg.contains("重新生成"), "{msg}");
    let _ = std::fs::remove_dir_all(&root);
  }

  #[test]
  fn catches_stale_input_digest() {
    let root = scaffold("stale");
    write_stamped_dxcc(&root, "110|KH6|Hawaii\n");
    let bin = root.join("public/dxcc-entities.bin");
    std::fs::write(&bin, [0u8]).expect("write bin");
    // 先按旧内容记戳，然后改掉 dxcc.txt → 摘要对不上。
    let stale = Stamp::of("https://example.test/ne.geojson", &[0])
      .with_input(&root, "crates/core/data/dxcc.txt");
    provenance::write_sidecar(&root, &bin, &stale).expect("sidecar");
    std::fs::write(root.join("crates/core/data/dxcc.txt"), "changed\n").expect("change");

    let err = run(&root).expect_err("依赖变了应当报错");
    assert!(err.to_string().contains("重新生成"));
    let _ = std::fs::remove_dir_all(&root);
  }

  /// 回归:只是重跑了一遍 `cargo make dxcc`(抓取时间变了,数据行没变),依赖它的 `.bin`
  /// 不该被判成过期——输入摘要只认数据行,不认注释头。
  #[test]
  fn regenerating_dxcc_with_unchanged_data_does_not_stale_the_bin() {
    let root = scaffold("rerun");
    write_stamped_dxcc(&root, "110|KH6|Hawaii\n");
    let bin = root.join("public/dxcc-entities.bin");
    std::fs::write(&bin, [0u8]).expect("write bin");
    provenance::write_sidecar(
      &root,
      &bin,
      &Stamp::of("https://example.test/ne.geojson", &[0])
        .with_input(&root, "crates/core/data/dxcc.txt"),
    )
    .expect("sidecar");
    std::fs::write(root.join("data/explanations.json"), b"{}").expect("write expl");
    provenance::write_sidecar(
      &root,
      &root.join("public/questions/config.json"),
      &Stamp::of("x", b"y").with_input(&root, "data/explanations.json"),
    )
    .expect("sidecar2");

    // 同样的数据行,换一个抓取时间 / 校验头重新写头。
    let mut again = Stamp::of("https://example.test/cty.csv", b"110|KH6|Hawaii\n");
    again.fetched_at = "2030-01-01T00:00:00Z".to_owned();
    let again = again.with_validators(Some("\"new\""), None);
    std::fs::write(
      root.join("crates/core/data/dxcc.txt"),
      format!("{}110|KH6|Hawaii\n", again.header_block("dxcc")),
    )
    .expect("rewrite");

    run(&root).expect("数据行没变,不应判过期");
    let _ = std::fs::remove_dir_all(&root);
  }

  /// 回归(1.2):改了解析库后走离线的 `explanations-apply`,题库已经是新的——
  /// 刷新边车输入之后 `check-data-sync` 必须转绿;没刷新时则如实报红并指向离线命令。
  #[test]
  fn explanations_change_is_cleared_by_refreshing_the_sidecar() {
    let root = scaffold("expl");
    write_stamped_dxcc(&root, "110|KH6|Hawaii\n");
    let bin = root.join("public/dxcc-entities.bin");
    std::fs::write(&bin, [0u8]).expect("write bin");
    provenance::write_sidecar(
      &root,
      &bin,
      &Stamp::of("https://example.test/ne.geojson", &[0])
        .with_input(&root, "crates/core/data/dxcc.txt"),
    )
    .expect("sidecar");
    std::fs::write(root.join("data/explanations.json"), b"{\"a\":1}").expect("write expl");
    let config = root.join("public/questions/config.json");
    provenance::write_sidecar(
      &root,
      &config,
      &Stamp::of("x", b"y").with_input(&root, "data/explanations.json"),
    )
    .expect("sidecar2");
    run(&root).expect("起点应当干净");

    std::fs::write(root.join("data/explanations.json"), b"{\"a\":2}").expect("edit expl");
    let err = run(&root).expect_err("解析变了还没 apply,应当报红");
    assert!(err.to_string().contains("重新生成"));

    assert!(provenance::refresh_inputs(&root, &config).expect("refresh"));
    run(&root).expect("apply 之后应当转绿");
    let _ = std::fs::remove_dir_all(&root);
  }

  /// 回归(1.3):补戳的摘要是产物自己的。新鲜度检查见到补戳标记必须直接判「需重新生成」,
  /// 而且根本不该联网(联网比对必然误报)。
  #[test]
  fn backfilled_stamps_are_marked_and_never_compared_with_upstream() {
    let root = scaffold("backfill");
    let dxcc = "110|KH6|Hawaii\n";
    std::fs::write(
      root.join("crates/core/data/dxcc.txt"),
      format!("# 旧式说明\n{dxcc}"),
    )
    .expect("write dxcc");
    std::fs::write(root.join("public/dxcc-entities.bin"), [1u8, 2]).expect("write bin");
    std::fs::write(root.join("public/questions/config.json"), b"{}").expect("write cfg");
    std::fs::write(root.join("data/explanations.json"), b"{}").expect("write expl");

    backfill(&root).expect("backfill");

    let text = std::fs::read_to_string(root.join("crates/core/data/dxcc.txt")).expect("read");
    let stamp = Stamp::parse(&text).expect("补戳后有戳");
    assert!(stamp.backfilled, "{text}");
    assert_eq!(
      stamp.digest,
      digest_hex(dxcc.as_bytes()),
      "摘要是数据部分自己的"
    );
    assert!(text.ends_with(dxcc), "数据行一个字不能动");
    for rel in ["public/dxcc-entities.bin", "public/questions/config.json"] {
      let side = provenance::read_sidecar(&root, &root.join(rel)).expect(rel);
      assert!(side.backfilled, "{rel}");
    }
    run(&root).expect("补戳产物离线检查应当通过");

    for relation in [Relation::Verbatim, Relation::Derived] {
      assert_eq!(
        judge(relation, Some(&stamp), "https://example.test/x", &Offline),
        Verdict::Backfilled
      );
    }
    // 再补一次是幂等的:已有戳的不重写。
    let before = std::fs::read_to_string(root.join("crates/core/data/dxcc.txt")).expect("read");
    backfill(&root).expect("backfill again");
    assert_eq!(
      std::fs::read_to_string(root.join("crates/core/data/dxcc.txt")).expect("read"),
      before
    );
    let _ = std::fs::remove_dir_all(&root);
  }

  /// 不允许联网的上游:判定逻辑如果在不该联网的分支里访问它,测试直接 panic。
  struct Offline;
  impl Upstream for Offline {
    fn digest(&self, url: &str) -> Option<String> {
      panic!("不该联网:{url}");
    }
    fn probe(&self, url: &str, _: Option<&str>, _: Option<&str>) -> Option<Probe> {
      panic!("不该联网:{url}");
    }
  }

  /// 固定应答的上游。
  struct Fake {
    digest: Option<String>,
    probe: Option<Probe>,
  }
  impl Upstream for Fake {
    fn digest(&self, _: &str) -> Option<String> {
      self.digest.clone()
    }
    fn probe(&self, _: &str, _: Option<&str>, _: Option<&str>) -> Option<Probe> {
      self.probe
    }
  }

  /// 按 URL 应答的上游：多文件来源要逐个文件探活，得能区分请求的是哪个文件。
  struct PerUrl<F: Fn(&str) -> Option<Probe>>(F);

  impl<F: Fn(&str) -> Option<Probe>> Upstream for PerUrl<F> {
    fn digest(&self, _: &str) -> Option<String> {
      panic!("多文件来源不该整份下载")
    }
    fn probe(&self, url: &str, _: Option<&str>, _: Option<&str>) -> Option<Probe> {
      (self.0)(url)
    }
  }

  #[test]
  fn compare_matrix() {
    let remote = Stamp::of("https://example.test/cty.csv", b"body");
    assert_eq!(compare(&remote, Some(&remote.digest)), Verdict::UpToDate);
    assert_eq!(
      compare(&remote, Some(&digest_hex(b"other"))),
      Verdict::Changed
    );
    assert_eq!(compare(&remote, None), Verdict::Unreachable);
  }

  #[test]
  fn verbatim_judgement_follows_the_upstream_digest() {
    let stamp = Stamp::of("https://example.test/cty.csv", b"body");
    let url = "https://example.test/cty.csv";
    let same = Fake {
      digest: Some(stamp.digest.clone()),
      probe: None,
    };
    let other = Fake {
      digest: Some(digest_hex(b"x")),
      probe: None,
    };
    let down = Fake {
      digest: None,
      probe: None,
    };
    assert_eq!(
      judge(Relation::Verbatim, Some(&stamp), url, &same),
      Verdict::UpToDate
    );
    assert_eq!(
      judge(Relation::Verbatim, Some(&stamp), url, &other),
      Verdict::Changed
    );
    assert_eq!(
      judge(Relation::Verbatim, Some(&stamp), url, &down),
      Verdict::Unreachable
    );
  }

  /// 回归(1.1):带 `ETag` 的派生产物以前被直接判成「连不上上游」,条件请求从未发出。
  /// 现在:`304` → 最新,`200` → 有更新,探测失败 → 才是不可达。
  #[test]
  fn derived_judgement_uses_the_conditional_probe() {
    let url = "https://example.test/ne.geojson";
    let with_etag = Stamp::of(url, b"derived").with_validators(Some("\"v1\""), None);
    let with_date =
      Stamp::of(url, b"derived").with_validators(None, Some("Sat, 17 Oct 2026 10:44:30 GMT"));
    for stamp in [&with_etag, &with_date] {
      let probe = |p| Fake {
        digest: None,
        probe: p,
      };
      assert_eq!(
        judge(
          Relation::Derived,
          Some(stamp),
          url,
          &probe(Some(Probe::NotModified))
        ),
        Verdict::UpToDate
      );
      assert_eq!(
        judge(
          Relation::Derived,
          Some(stamp),
          url,
          &probe(Some(Probe::Modified))
        ),
        Verdict::Changed
      );
      assert_eq!(
        judge(Relation::Derived, Some(stamp), url, &probe(None)),
        Verdict::Unreachable
      );
    }
    // 派生产物绝不拿上游摘要去比:即使上游给了摘要也不看。
    let never = Fake {
      digest: Some(digest_hex(b"anything")),
      probe: Some(Probe::NotModified),
    };
    assert_eq!(
      judge(Relation::Derived, Some(&with_etag), url, &never),
      Verdict::UpToDate
    );
  }

  #[test]
  fn derived_without_validators_is_unknown_and_stays_offline() {
    let stamp = Stamp::of("https://example.test/ne.geojson", b"derived");
    assert_eq!(
      judge(
        Relation::Derived,
        Some(&stamp),
        "https://example.test/ne.geojson",
        &Offline
      ),
      Verdict::Unknown
    );
  }

  /// 回归：题库的上游地址是个目录，戳里只有单份校验头时永远判不出新旧（一直是「无法判断」）。
  /// 逐文件记下校验头之后，全部 `304` 才算最新，任何一个 `200` 就是有新版本。
  #[test]
  fn multi_file_source_probes_every_recorded_file() {
    let base = "https://example.test/main/";
    let stamp = Stamp::of(base, b"derived")
      .with_file("class_a.csv", "aa", Some("\"a1\""), None)
      .with_file("class_b.csv", "bb", Some("\"b1\""), None);

    let all_same = PerUrl(|url: &str| {
      assert!(
        url.starts_with("https://example.test/main/class_"),
        "文件要拼在目录地址后面：{url}"
      );
      Some(Probe::NotModified)
    });
    assert_eq!(
      judge(Relation::Derived, Some(&stamp), base, &all_same),
      Verdict::UpToDate
    );

    let one_changed = PerUrl(|url: &str| {
      Some(if url.ends_with("class_b.csv") {
        Probe::Modified
      } else {
        Probe::NotModified
      })
    });
    assert_eq!(
      judge(Relation::Derived, Some(&stamp), base, &one_changed),
      Verdict::Changed
    );

    let down = PerUrl(|_: &str| None);
    assert_eq!(
      judge(Relation::Derived, Some(&stamp), base, &down),
      Verdict::Unreachable
    );
  }

  /// 一个文件变了、另一个连不上：要判「有新版本」，别让网络故障把它盖成「不可达」白等一周。
  #[test]
  fn changed_file_wins_over_an_unreachable_one() {
    let base = "https://example.test/main/";
    let stamp = Stamp::of(base, b"derived")
      .with_file("a.csv", "a", Some("\"a1\""), None)
      .with_file("b.csv", "b", Some("\"b1\""), None);
    let mixed = PerUrl(|url: &str| {
      if url.ends_with("a.csv") {
        None
      } else {
        Some(Probe::Modified)
      }
    });
    assert_eq!(
      judge(Relation::Derived, Some(&stamp), base, &mixed),
      Verdict::Changed
    );
  }

  /// 下载时上游就没给校验头的文件判不了：不猜，报「无法判断」，也不为它联网。
  #[test]
  fn multi_file_source_without_validators_is_unknown() {
    let base = "https://example.test/main/";
    let bare = Stamp::of(base, b"derived").with_file("a.csv", "a", None, None);
    assert_eq!(
      judge(Relation::Derived, Some(&bare), base, &Offline),
      Verdict::Unknown
    );

    // 一半有、一半没有：其余文件都是 304 也不能说「最新」。
    let half = Stamp::of(base, b"derived")
      .with_file("a.csv", "a", Some("\"a1\""), None)
      .with_file("b.csv", "b", None, None);
    let same = PerUrl(|_: &str| Some(Probe::NotModified));
    assert_eq!(
      judge(Relation::Derived, Some(&half), base, &same),
      Verdict::Unknown
    );
  }

  #[test]
  fn no_stamp_and_local_sources_never_touch_the_network() {
    for relation in [Relation::Verbatim, Relation::Derived] {
      assert_eq!(
        judge(relation, None, "https://example.test/x", &Offline),
        Verdict::NoStamp
      );
      let local = Stamp::of_local(Path::new("/tmp/cty.csv"), b"body");
      assert_eq!(
        judge(relation, Some(&local), "https://example.test/x", &Offline),
        Verdict::LocalOnly
      );
    }
  }

  /// 回归：`dxcc-map` 曾因为拿 `.bin` 自身摘要去比上游 GeoJSON 摘要而误报「有新版本」。
  #[test]
  fn derived_target_is_never_verbatim() {
    let map = TARGETS
      .iter()
      .find(|t| t.name == "dxcc-map")
      .expect("dxcc-map");
    assert_eq!(map.relation, Relation::Derived);
    // dxcc.txt 是逐行转写，可以比摘要。
    let dxcc = TARGETS.iter().find(|t| t.name == "dxcc").expect("dxcc");
    assert_eq!(dxcc.relation, Relation::Verbatim);
  }

  /// 过期提示要指到「改哪个文件 / 常量」：指不到具体位置，复核的人还得自己找。
  #[test]
  fn file_backed_keys_point_at_the_place_to_edit() {
    assert!(fix_hint("gear-rx").contains("gear_rx::CHECKED_ON"));
    assert!(fix_hint("radio-law-text").contains("radio-law.txt"));
    assert!(fix_hint("toroid").contains("verified"));
    assert!(fix_hint("diy-cost").contains("COST_CHECKED_ON"));
    // 页面溯源表的域落在同一处，别给每个域各写一句。
    assert!(fix_hint("bands").contains("PROVENANCES"));
  }

  #[test]
  fn load_stamp_prefers_sidecar_then_header() {
    let root = scaffold("load");
    // 文本产物：读注释头
    let stamp = write_stamped_dxcc(&root, "110|KH6|Hawaii\n");
    assert_eq!(
      load_stamp(&root, &root.join("crates/core/data/dxcc.txt")),
      Some(stamp)
    );
    // 二进制：读边车
    let bin = root.join("public/dxcc-entities.bin");
    std::fs::write(&bin, [7u8]).expect("write");
    let sidecar = Stamp::of("https://x/y", &[7]);
    provenance::write_sidecar(&root, &bin, &sidecar).expect("sidecar");
    assert_eq!(load_stamp(&root, &bin), Some(sidecar));
    let _ = std::fs::remove_dir_all(&root);
  }
}
