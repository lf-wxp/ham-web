//! `ham-exam-tools`：构建与题库维护命令行工具。
//!
//! 常用入口见 `Makefile.toml`（`cargo make <task>`），也可直接运行：
//!
//! ```text
//! cargo run -p ham-exam-tools -- <SUBCOMMAND> --help
//! ```

mod csv;
mod dataset;
mod explanations;
mod fsutil;
mod icons;
mod postbuild;

use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};

const DEFAULT_REMOTE: &str =
  "https://raw.githubusercontent.com/xiedada05/crac-amateur-radio-exam-questions-2025-csv/main";

#[derive(Debug, Parser)]
#[command(
  name = "ham-exam-tools",
  version,
  about = "业余无线电考试模拟：构建与题库维护工具"
)]
struct Cli {
  /// 项目根目录（默认自动向上查找 Makefile.toml）
  #[arg(long, global = true, env = "HAM_EXAM_ROOT")]
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
  /// 用术语表 glossary.json 中 inject=true 的词条为解析注入通俗解释（幂等）
  EnhanceExplanations,
  /// 校验 data/glossary.json（分类、参见、重复、括号等）并输出统计
  CheckGlossary,
  /// 统计缺失解析的题目，并导出待填写模板
  MissingExplanations {
    /// 仅统计某个题库（A/B/C）
    #[arg(long)]
    bank: Option<ham_exam_core::Bank>,
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
  /// 由 public/pwa-icon.svg 生成 PWA 与 Apple Touch 图标
  Icons,
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
    Command::MissingExplanations {
      bank,
      output,
      context,
      limit,
    } => explanations::missing(&paths, bank, output.as_deref(), context.as_deref(), limit),
    Command::Icons => icons::generate(&paths),
    Command::Postbuild { dist, site_url } => {
      let dist = if dist.is_absolute() {
        dist
      } else {
        root.join(dist)
      };
      postbuild::run(&dist, &site_url)
    }
  }
}
