//! 编译并调用 fldigi 上游代码的对拍程序（`reference/fldigi_ref.cpp`）。
//!
//! 需要环境变量 `FLDIGI_SRC` 指向 fldigi 源码根目录（含 `src/`）。调用它的用例都标了 `#[ignore]`
//! （默认的 `cargo test` 里显示为 ignored，而不是悄悄「通过」）；显式运行时没设环境变量就 panic，
//! 不会空转。上游源码不进本仓库；设了但编译失败同样是真问题，也 panic。
//!
//! ```text
//! git clone --depth 1 https://github.com/w1hkj/fldigi /tmp/fldigi
//! FLDIGI_SRC=/tmp/fldigi cargo test -p ham-web-mfsk -p ham-web-thor --release --test upstream_xcheck -- --include-ignored
//! ```

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

/// 编译好的对拍程序。
pub struct Reference {
  exe: PathBuf,
}

/// 上游源码根目录 → 编译产物（整个测试进程只编一次）。
///
/// # Panics
/// 没设 `FLDIGI_SRC`（调用方都是 `#[ignore]` 用例，显式运行就该有上游源码）、路径不对或编译失败。
pub fn reference() -> &'static Reference {
  static REF: OnceLock<Reference> = OnceLock::new();
  REF.get_or_init(build)
}

fn build() -> Reference {
  let root = PathBuf::from(std::env::var_os("FLDIGI_SRC").expect(
    "请设置 FLDIGI_SRC 指向 fldigi 源码根目录（git clone --depth 1 https://github.com/w1hkj/fldigi）",
  ));
  let src = root.join("src");
  assert!(
    src.join("filters/viterbi.cxx").exists(),
    "FLDIGI_SRC={} 不是 fldigi 源码根目录（找不到 src/filters/viterbi.cxx）",
    root.display()
  );
  // 放在 cargo 的 target/tmp 下、按 crate 分目录且不带进程号：随 `cargo clean` 一起清掉，
  // 反复运行也只覆盖同一个目录，不会像系统临时目录那样每跑一次留一份。
  let dir =
    Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("fldigi-ref-{}", env!("CARGO_PKG_NAME")));
  std::fs::create_dir_all(&dir).expect("建临时目录");
  // 上游文件都 `#include <config.h>`（autoconf 生成），这里给个空的。
  std::fs::write(dir.join("config.h"), "").expect("写 config.h");
  let exe = dir.join("fldigi_ref");
  // 本文件被 mfsk 与 thor 两个 crate 的测试各自 `#[path]` 引入，`CARGO_MANIFEST_DIR` 是引入方的目录；
  // 对拍程序只有一份，住在 mfsk 里，所以按「同级的 mfsk 目录」找（两个 crate 都适用）。
  let harness = Path::new(env!("CARGO_MANIFEST_DIR")).join("../mfsk/reference/fldigi_ref.cpp");
  let out = Command::new("c++")
    .args(["-std=c++17", "-O1", "-w"])
    .arg("-I")
    .arg(&dir)
    .args(
      [
        "src/include",
        "src/filters",
        "src/mfsk",
        "src/thor",
        "src/misc",
      ]
      .map(|d| {
        let mut a = std::ffi::OsString::from("-I");
        a.push(root.join(d));
        a
      }),
    )
    .arg(&harness)
    .args(
      [
        "src/filters/viterbi.cxx",
        "src/mfsk/interleave.cxx",
        "src/mfsk/mfskvaricode.cxx",
        "src/thor/thorvaricode.cxx",
      ]
      .map(|f| root.join(f)),
    )
    .arg("-o")
    .arg(&exe)
    .output()
    .expect("调用 c++ 失败（需要 C++ 编译器）");
  assert!(
    out.status.success(),
    "编译对拍程序失败：\n{}",
    String::from_utf8_lossy(&out.stderr)
  );
  Reference { exe }
}

impl Reference {
  /// 运行对拍程序，返回标准输出。
  pub fn run(&self, args: &[&str]) -> String {
    let out = Command::new(&self.exe)
      .args(args)
      .output()
      .expect("运行对拍程序失败");
    assert!(out.status.success(), "对拍程序退出码非零：{args:?}");
    String::from_utf8(out.stdout).expect("输出不是 UTF-8")
  }

  /// 运行并把输出解析成整数序列（音调号）。
  pub fn tones(&self, args: &[&str]) -> Vec<u8> {
    self
      .run(args)
      .split_whitespace()
      .map(|t| t.parse().expect("音调号"))
      .collect()
  }
}
