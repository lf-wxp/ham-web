//! `ham-web-tools`：构建与题库维护命令行工具。
//!
//! 常用入口见 `Makefile.toml`（`cargo make <task>`），也可直接运行：
//!
//! ```text
//! cargo run -p ham-web-tools -- <SUBCOMMAND> --help
//! ```

mod csv;
mod dataset;
mod dxcc;
mod dxcc_map;
mod explanations;
mod fsutil;
mod i18n;
mod icons;
mod knowledge_i18n;
mod postbuild;
mod psk31_gen;
mod spectrum_gen;

use std::path::PathBuf;

use anyhow::{Result, bail};
use clap::{Parser, Subcommand};

const DEFAULT_REMOTE: &str =
  "https://raw.githubusercontent.com/xiedada05/crac-amateur-radio-exam-questions-2025-csv/main";

#[derive(Debug, Parser)]
#[command(
  name = "ham-web-tools",
  version,
  about = "业余无线电考试模拟：构建与题库维护工具"
)]
struct Cli {
  /// 项目根目录（默认自动向上查找 Makefile.toml）
  #[arg(long, global = true, env = "HAM_WEB_ROOT")]
  root: Option<PathBuf>,

  #[command(subcommand)]
  command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
  /// 从 CSV 数据源构建题库 JSON 与题目图片（public/questions）
  Dataset {
    /// 本地数据集目录（优先于远程）
    #[arg(long, env = "DATASET_DIR")]
    dataset_dir: Option<PathBuf>,
    /// 远程数据集基础地址
    #[arg(long, env = "DATASET_REMOTE", default_value = DEFAULT_REMOTE)]
    remote: String,
  },
  /// 把 explanations.json 中的解析重新应用到已构建的题库 JSON（无需联网）
  ApplyExplanations,
  /// 按题目 ID 批量合并解析到 explanations.json
  AddExplanations {
    /// 形如 {"A-1": "解析…", "B-5": "解析…"} 的 JSON 文件
    batch: PathBuf,
  },
  /// 用术语表 data/glossary/ 中 inject=true 的词条为解析注入通俗解释（幂等）
  EnhanceExplanations,
  /// 校验 data/glossary/（分类、参见、重复、括号等）并输出统计
  CheckGlossary,
  /// 校验 crates/app/src/i18n.rs 的 EN / ES 词典（重复 key、占位符数量）并统计覆盖率
  CheckI18n {
    /// 导出「源码里有但词典里没有」的文案模板（{zh, text}）
    #[arg(long)]
    missing: Option<PathBuf>,
    /// 配合 --missing 使用的语言（en / es）
    #[arg(long, default_value = "en")]
    lang: String,
  },
  /// 合并已填写的界面文案模板回 i18n.rs 词典
  AddI18n {
    /// 已填写的模板文件（{zh, text}）
    batch: PathBuf,
    /// 目标语言（en / es）
    #[arg(long, default_value = "en")]
    lang: String,
  },
  /// 列出 crates/core 中含中文的模块（按字符数从大到小），供决定先翻哪个
  KnowledgeI18nInventory,
  /// 导出某模块的待译模板（{zh, text}，text 留空表示未翻）
  KnowledgeI18nMissing {
    /// core 模块名，如 `wspr`
    #[arg(long)]
    module: String,
    /// 目标语言（en / es）
    #[arg(long, default_value = "en")]
    lang: String,
    /// 模板输出路径
    #[arg(long, default_value = "tmp/knowledge-i18n.json")]
    output: PathBuf,
  },
  /// 合并已填写的模板回 data/knowledge-i18n/
  KnowledgeI18nAdd {
    /// core 模块名
    #[arg(long)]
    module: String,
    /// 目标语言（en / es）
    #[arg(long, default_value = "en")]
    lang: String,
    /// 已填写的模板文件
    batch: PathBuf,
  },
  /// 查看某模块某语言的翻译覆盖率
  KnowledgeI18nCoverage {
    /// core 模块名
    #[arg(long)]
    module: String,
    /// 目标语言（en / es）；省略时两种语言都看
    #[arg(long)]
    lang: Option<String>,
  },
  /// 统计缺失解析的题目，并导出待填写模板
  MissingExplanations {
    /// 仅统计某个题库（A/B/C）
    #[arg(long)]
    bank: Option<ham_web_core::Bank>,
    /// 导出 {"id": ""} 模板，填写后可直接用 add-explanations 合并
    #[arg(long)]
    output: Option<PathBuf>,
    /// 导出题目上下文（题干/选项/答案）供人工或 AI 撰写解析
    #[arg(long)]
    context: Option<PathBuf>,
    /// 导出数量上限
    #[arg(long)]
    limit: Option<usize>,
  },
  /// 由 cty.csv 生成内置 DXCC 前缀表（crates/core/data/dxcc.txt）
  Dxcc {
    /// 本地 cty.csv（默认从 country-files.com 下载）
    #[arg(long)]
    input: Option<PathBuf>,
    /// 下载地址
    #[arg(long, default_value = dxcc::DEFAULT_URL)]
    url: String,
  },
  /// 由 Natural Earth 国界 GeoJSON 生成 DXCC 实体边界（public/dxcc-entities.bin）
  DxccMap {
    /// 本地 GeoJSON（默认从 Natural Earth 下载）
    #[arg(long)]
    input: Option<PathBuf>,
    /// 下载地址
    #[arg(long, default_value = dxcc_map::DEFAULT_URL)]
    url: String,
  },
  /// 由 public/pwa-icon.svg 生成 PWA 与 Apple Touch 图标
  Icons,
  /// 生成合成 PSK31 测试样本 WAV（用于手动测试 /psk-decode）
  GenPsk31 {
    /// 输出 WAV 路径
    #[arg(long, default_value = "tmp/psk31-sample.wav")]
    output: PathBuf,
    /// 要编码的文本
    #[arg(long, default_value = "CQ CQ CQ DE BG4XXX BG4XXX K")]
    text: String,
    /// 采样率（Hz）
    #[arg(long, default_value = "8000")]
    rate: u32,
    /// 载波频率（Hz）
    #[arg(long, default_value = "1000")]
    center: f32,
    /// 载波频偏（Hz，模拟真实接收）
    #[arg(long, default_value = "0")]
    offset: f32,
    /// 噪声标准差（0 表示无噪声）
    #[arg(long, default_value = "0.1")]
    noise: f32,
  },
  /// 生成合成频谱测试样本 WAV（用于手动测试 /sdr-waterfall）
  GenSpectrum {
    /// 输出 WAV 路径
    #[arg(long, default_value = "tmp/spectrum-sample.wav")]
    output: PathBuf,
    /// 采样率（Hz）
    #[arg(long, default_value = "8000")]
    rate: u32,
    /// 时长（秒）
    #[arg(long, default_value = "2.0")]
    seconds: f32,
    /// 音调列表，逗号分隔的「频率:幅度」，如 700:0.8,1500:0.5
    #[arg(long, default_value = "700:0.8,1500:0.5,3000:0.3")]
    tones: String,
    /// 噪声幅度（0 表示无噪声）
    #[arg(long, default_value = "0.05")]
    noise: f32,
  },
  /// 读取 WAV 并做频谱分析，打印峰值频率（验证测试样本）
  AnalyzeSpectrum {
    /// 输入 WAV 路径
    #[arg(long, default_value = "tmp/spectrum-sample.wav")]
    input: PathBuf,
    /// 打印峰值数量
    #[arg(long, default_value = "5")]
    peaks: usize,
  },
  /// Trunk 构建后处理：生成 Service Worker、sitemap.xml，并替换站点地址
  Postbuild {
    /// 构建产物目录
    #[arg(long, default_value = "dist")]
    dist: PathBuf,
    /// 站点地址（用于 Open Graph 与 sitemap）
    #[arg(long, env = "SITE_URL", default_value = "https://ham.onlyxp.me")]
    site_url: String,
  },
}

fn main() -> Result<()> {
  let cli = Cli::parse();
  let root = fsutil::project_root(cli.root)?;
  let paths = fsutil::Paths::new(&root);

  match cli.command {
    Command::Dataset {
      dataset_dir,
      remote,
    } => dataset::build(&paths, dataset_dir.as_deref(), &remote),
    Command::ApplyExplanations => explanations::apply(&paths),
    Command::AddExplanations { batch } => explanations::add(&paths, &batch),
    Command::EnhanceExplanations => explanations::enhance(&paths),
    Command::CheckGlossary => explanations::check_glossary(&paths),
    Command::CheckI18n { missing, lang } => {
      if let Some(out) = missing {
        let items = i18n::missing(&root, &lang)?;
        if let Some(dir) = out.parent() {
          std::fs::create_dir_all(dir)?;
        }
        let payload: Vec<serde_json::Value> = items
          .iter()
          .map(|zh| serde_json::json!({ "zh": zh, "text": "" }))
          .collect();
        crate::fsutil::write_json(&out, &payload)?;
        println!("已导出 {} 条待译界面文案 → {}", items.len(), out.display());
        return Ok(());
      }
      if !i18n::run(&root)? {
        bail!("界面文案词典校验未通过");
      }
      Ok(())
    }
    Command::AddI18n { batch, lang } => {
      let n = i18n::add(&root, &lang, &batch)?;
      println!("已合并 {n} 条界面文案（{lang}）");
      Ok(())
    }
    Command::KnowledgeI18nInventory => {
      let items = knowledge_i18n::inventory(&root)?;
      let total: usize = items.iter().map(|(_, n)| n).sum();
      println!(
        "crates/core 含中文模块 {} 个，合计 {} 字符\n",
        items.len(),
        total
      );
      for (m, n) in items.iter().take(20) {
        println!("  {n:>6}  {m}");
      }
      if items.len() > 20 {
        println!("  … 另有 {} 个模块", items.len() - 20);
      }
      Ok(())
    }
    Command::KnowledgeI18nMissing {
      module,
      lang,
      output,
    } => {
      let (p, n) = knowledge_i18n::export(&root, &module, &lang, &output)?;
      println!("已导出 {n} 条待译条目 → {}", p.display());
      Ok(())
    }
    Command::KnowledgeI18nAdd {
      module,
      lang,
      batch,
    } => {
      let n = knowledge_i18n::add(&root, &module, &lang, &batch)?;
      println!("已合并 {n} 条译文（{module}/{lang}）");
      Ok(())
    }
    Command::KnowledgeI18nCoverage { module, lang } => {
      let langs: Vec<String> = match lang {
        Some(l) => vec![l],
        None => knowledge_i18n::LANGS
          .iter()
          .map(|s| (*s).to_owned())
          .collect(),
      };
      for l in langs {
        let (done, total) = knowledge_i18n::coverage(&root, &module, &l)?;
        let pct = if total == 0 {
          100.0
        } else {
          done as f64 * 100.0 / total as f64
        };
        println!("{module}/{l}: {done} / {total}（{pct:.1}%）");
      }
      Ok(())
    }
    Command::MissingExplanations {
      bank,
      output,
      context,
      limit,
    } => explanations::missing(&paths, bank, output.as_deref(), context.as_deref(), limit),
    Command::Dxcc { input, url } => dxcc::generate(&root, input.as_deref(), &url),
    Command::DxccMap { input, url } => dxcc_map::generate(&root, input.as_deref(), &url),
    Command::Icons => icons::generate(&paths),
    Command::GenPsk31 {
      output,
      text,
      rate,
      center,
      offset,
      noise,
    } => psk31_gen::generate(&output, &text, rate, center + offset, noise),
    Command::GenSpectrum {
      output,
      rate,
      seconds,
      tones,
      noise,
    } => {
      let tones = spectrum_gen::parse_tones(&tones)?;
      spectrum_gen::generate(&output, rate, seconds, &tones, noise)
    }
    Command::AnalyzeSpectrum { input, peaks } => spectrum_gen::analyze(&input, peaks),
    Command::Postbuild { dist, site_url } => {
      let dist = if dist.is_absolute() {
        dist
      } else {
        root.join(dist)
      };
      postbuild::run(&root, &dist, &site_url)
    }
  }
}
