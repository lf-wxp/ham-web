//! 弱信号 / 窄带干扰下，本仓解码器与 fldigi 4.2.13 接收机的差分对拍。
//!
//! 放在 thor 的 examples 里而不是 mfsk 的：它要同时调用 `ham_web_mfsk` 与 `ham_web_thor`，而 thor 本来就依赖 mfsk；
//! 放进 mfsk 就得给 mfsk 加一个指向 thor 的反向 dev-dependency（Cargo 允许，但依赖图成环、读起来绕）。
//! 只编成 example，不进发布物（见 `docs/engineering/licensing.md`）。
//!
//! 同一段加噪音频喂给两边：右边是进程内的 [`ham_web_thor::decode_audio`]（`--family mfsk`
//! 时换 [`ham_web_mfsk::decode_audio`]），左边是 fldigi 的 WASM 接收机（`reference/fldigi_web/dec.mjs`，
//! 需要 node 与一份 fldigi-web 克隆）。
//!
//! ```text
//! cargo run -p ham-web-thor --release --example noise_sweep -- \
//!     --family thor --mode 16 /tmp/fw/fwlong_THOR16.wav --snr -12 -13 --seeds 4
//! cargo run -p ham-web-thor --release --example noise_sweep -- \
//!     --family thor --mode 16 /tmp/fw/fwlong_THOR16.wav --snr -5 --cw 1550 --cw-db 3 --skip-fldigi
//! ```
//!
//! 噪声与打分刻意复刻迁移前的 Python 脚本（已不在仓库里；CPython 的 MT19937 + `random.gauss`、
//! `difflib.SequenceMatcher`），所以同一命令两边给出的表格逐点可比 —— 这是迁移脚本时唯一的验收标准，
//! 见 `reference/fldigi_web/README.md`。

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

/// 对拍的报文：`gen_all.sh` 用同一句生成测试录音，长录音把它重复 8 遍。
const TEXT: &str = "the quick brown fox jumps over the lazy dog 1234567890";

fn main() {
  let opts = Options::parse();
  let bytes = std::fs::read(&opts.wav).expect("读取 WAV 失败");
  let (samples, rate) = ham_web_wav::parse(&bytes).expect("解析 WAV 失败");

  let dec_mjs = Path::new(env!("CARGO_MANIFEST_DIR")).join("../mfsk/reference/fldigi_web/dec.mjs");
  if !opts.skip_fldigi && !dec_mjs.exists() {
    panic!(
      "找不到 {}（fldigi 对照需要仓库里的 reference/fldigi_web）",
      dec_mjs.display()
    );
  }

  println!(
    "{:<8} {:<6} | {:<18} | {:<18}",
    "SNR(dB)", "seed", "fldigi 遍数 / 吻合率", "本仓 遍数 / 吻合率"
  );
  for &snr in &opts.snr {
    let (mut theirs_sum, mut theirs_ratio, mut ours_sum, mut ours_ratio) =
      (0u32, 0.0f64, 0u32, 0.0f64);
    for seed in 0..opts.seeds {
      let noisy = impair(&samples, rate, snr, opts.cw, opts.cw_db, 1000 + seed);
      // 与原脚本同一口径：加噪样本先落成 16 位 PCM 再读回。弱信号（−13 dB 以下）时噪声幅度
      // 是信号的好几倍，写盘会大量削顶；本仓若直接吃内存里的浮点样本，吃到的就不是「同一段
      // 音频」，各项会明显偏乐观（实测 THOR16 −13.5 dB：不落盘 31 / 32 遍，落盘 12 / 32 遍）。
      let bytes = ham_web_wav::encode(&noisy, rate);
      let path = temp_path(u64::from(seed));
      std::fs::write(&path, &bytes).expect("写临时 WAV 失败");
      let (audio, audio_rate) = ham_web_wav::parse(&bytes).expect("解析临时 WAV 失败");
      let theirs = if opts.skip_fldigi {
        String::new()
      } else {
        run_fldigi(&dec_mjs, &opts, &path)
      };
      let ours = decode_text(opts.family, &opts.mode, &audio, audio_rate, opts.center);
      let _ = std::fs::remove_file(&path);

      let (theirs_n, theirs_r) = score(&theirs, opts.repeats);
      let (ours_n, ours_r) = score(&ours, opts.repeats);
      theirs_sum += theirs_n;
      theirs_ratio += theirs_r;
      ours_sum += ours_n;
      ours_ratio += ours_r;
      println!(
        "{:<8} {:<6} | {:>2} / {:.3}          | {:>2} / {:.3}",
        snr.map_or_else(|| "None".to_owned(), |v| format!("{v}")),
        seed,
        theirs_n,
        theirs_r,
        ours_n,
        ours_r
      );
    }
    let seeds = opts.seeds.max(1);
    println!(
      "{:<8} {:<6} | {:>2} / {:.3} (均)     | {:>2} / {:.3} (均)",
      snr.map_or_else(|| "None".to_owned(), |v| format!("{v}")),
      "合计",
      theirs_sum,
      theirs_ratio / f64::from(seeds),
      ours_sum,
      ours_ratio / f64::from(seeds)
    );
  }
}

/// 解码器家族：两个 crate 的 `decode_audio` 签名一致，只有类型不同。
#[derive(Clone, Copy, PartialEq)]
enum Family {
  Mfsk,
  Thor,
}

struct Options {
  family: Family,
  /// 本仓的子模式名（`16` / `25x4` / `micro` / `64L` …）。
  mode: String,
  wav: PathBuf,
  center: f64,
  /// 每个值跑一轮；`None` 表示干净信号（只加载波，如果给了 `--cw`）。
  snr: Vec<Option<f64>>,
  seeds: u32,
  cw: f64,
  cw_db: f64,
  repeats: usize,
  fldigi_mode: String,
  /// fldigi-web 克隆目录；没有默认值：默认指向 `/tmp` 等于从任何人都能写的目录里加载并执行代码。
  fldigi_web: Option<PathBuf>,
  skip_fldigi: bool,
}

impl Options {
  fn parse() -> Self {
    let mut family = Family::Thor;
    let mut mode = None;
    let mut wav = None;
    let mut center = 1500.0;
    let mut snr = vec![None];
    let mut seeds = 3;
    let mut cw = 0.0;
    let mut cw_db = 0.0;
    let mut repeats = 8;
    let mut fldigi_mode = None;
    let mut fldigi_web = std::env::var_os("FLDIGI_WEB").map(PathBuf::from);
    let mut skip_fldigi = false;

    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut i = 0;
    while i < args.len() {
      match args[i].as_str() {
        "--family" => {
          i += 1;
          family = match args.get(i).map(String::as_str) {
            Some("mfsk") => Family::Mfsk,
            Some("thor") => Family::Thor,
            other => panic!("--family 只认 thor / mfsk，收到 {other:?}"),
          };
        }
        "--mode" => {
          i += 1;
          mode = Some(args.get(i).cloned().expect("--mode 需要一个子模式名"));
        }
        "--center" => {
          i += 1;
          center = parse_f64(&args, i);
        }
        "--snr" => {
          // 一直吃到下一个选项：`--snr -12 -13` 是同一个列表，不是两个参数。
          let mut list = Vec::new();
          while let Some(next) = args.get(i + 1) {
            if next.starts_with("--") {
              break;
            }
            i += 1;
            list.push(Some(parse_f64(&args, i)));
          }
          snr = list;
        }
        "--seeds" => {
          i += 1;
          seeds = args
            .get(i)
            .and_then(|v| v.parse().ok())
            .expect("--seeds 需要整数");
        }
        "--cw" => {
          i += 1;
          cw = parse_f64(&args, i);
        }
        "--cw-db" => {
          i += 1;
          cw_db = parse_f64(&args, i);
        }
        "--repeats" => {
          i += 1;
          repeats = args
            .get(i)
            .and_then(|v| v.parse().ok())
            .expect("--repeats 需要整数");
        }
        "--fldigi-mode" => {
          i += 1;
          fldigi_mode = Some(args.get(i).cloned().expect("--fldigi-mode 需要一个模式名"));
        }
        "--fldigi-web" => {
          i += 1;
          fldigi_web = Some(PathBuf::from(
            args.get(i).expect("--fldigi-web 需要一个目录"),
          ));
        }
        "--skip-fldigi" => skip_fldigi = true,
        "-h" | "--help" => {
          println!("{}", HELP);
          std::process::exit(0);
        }
        other if other.starts_with("--") => panic!("不认识的选项 {other}（--help 看用法）"),
        other => wav = Some(PathBuf::from(other)),
      }
      i += 1;
    }

    let mode = mode.expect("缺 --mode");
    let wav = wav.expect("缺 WAV 路径");
    let fldigi_mode = fldigi_mode.unwrap_or_else(|| fldigi_name(family, &mode));
    Self {
      family,
      mode,
      wav,
      center,
      snr,
      seeds,
      cw,
      cw_db,
      repeats,
      fldigi_mode,
      fldigi_web,
      skip_fldigi,
    }
  }
}

const HELP: &str = "\
用法：noise_sweep --mode <子模式> <录音.wav> [选项]

  --family thor|mfsk   解码器家族（默认 thor）
  --mode <名>          本仓子模式：16 / 25x4 / micro（thor），16 / 64L（mfsk）
  --center <Hz>        中心频率（默认 1500；两边都用它，不做自动检测）
  --snr <dB>...        每个值跑一轮；数字之间用空格分隔（默认干净信号）
  --seeds <n>          每个 SNR 的种子数（默认 3，种子从 1000 起）
  --cw <Hz>            在该频率加一个窄带载波（默认不加）
  --cw-db <dB>         载波相对信号功率的 dB（默认 0）
  --repeats <n>        录音里报文重复的遍数（默认 8，用于数完整遍数）
  --fldigi-mode <名>   fldigi 侧模式名（默认由 family + mode 推出）
  --fldigi-web <目录>  fldigi-web 克隆（默认 $FLDIGI_WEB；两者都没有就必须加 --skip-fldigi）
  --skip-fldigi        只跑本仓（不装 node / 没有 fldigi-web 时用）";

fn parse_f64(args: &[String], i: usize) -> f64 {
  args
    .get(i)
    .and_then(|v| v.parse().ok())
    .unwrap_or_else(|| panic!("需要一个数字，收到 {:?}", args.get(i)))
}

/// 本仓子模式名 → fldigi 模式名。THOR 的 micro 在 fldigi 里叫 THORM，其余直接拼接。
fn fldigi_name(family: Family, mode: &str) -> String {
  match family {
    Family::Mfsk => format!("MFSK{mode}"),
    Family::Thor if mode == "micro" => "THORM".to_owned(),
    Family::Thor => format!("THOR{mode}"),
  }
}

/// 进程内解码（原脚本走 `--decoder` 外部二进制）。
fn decode_text(family: Family, mode: &str, samples: &[f32], rate: u32, center: f64) -> String {
  match family {
    Family::Mfsk => {
      let mode = ham_web_mfsk::parse_mode(mode).expect("不认识的 MFSK 子模式");
      ham_web_mfsk::decode_audio(samples, rate, mode, Some(center), false)
        .map(|d| d.text)
        .unwrap_or_default()
    }
    Family::Thor => {
      let mode = ham_web_thor::parse_mode(mode).expect("不认识的 THOR 子模式");
      ham_web_thor::decode_audio(samples, rate, mode, Some(center), false)
        .map(|d| d.text)
        .unwrap_or_default()
    }
  }
}

/// 加噪声 / 载波，口径与原脚本一致：
/// `sigma = rms / 10^(snr/20)`（全带宽，信号总功率 / 噪声总功率），
/// `载波幅度 = rms × √2 × 10^(cw_db/20)`（所以 `cw_db` 是相对信号功率的 dB）。
fn impair(
  samples: &[f32],
  rate: u32,
  snr_db: Option<f64>,
  cw_hz: f64,
  cw_db: f64,
  seed: u32,
) -> Vec<f32> {
  let n = samples.len();
  let sum: f64 = samples.iter().map(|&s| f64::from(s) * f64::from(s)).sum();
  let rms = (sum / n as f64).sqrt();
  let sigma = snr_db.map_or(0.0, |snr| rms / 10f64.powf(snr / 20.0));
  let amp = if cw_hz == 0.0 {
    0.0
  } else {
    rms * std::f64::consts::SQRT_2 * 10f64.powf(cw_db / 20.0)
  };
  let w = 2.0 * std::f64::consts::PI * cw_hz / f64::from(rate);

  let mut rnd = PyRandom::new(seed);
  let mut out = Vec::with_capacity(n);
  for (i, &s) in samples.iter().enumerate() {
    let mut v = f64::from(s);
    if sigma > 0.0 {
      v += rnd.gauss(sigma);
    }
    if cw_hz != 0.0 {
      v += amp * (w * i as f64).sin();
    }
    out.push(v as f32);
  }
  out
}

fn temp_path(seed: u64) -> PathBuf {
  std::env::temp_dir().join(format!("noise_sweep_{}_{seed}.wav", std::process::id()))
}

/// 调 fldigi 的 WASM 接收机（`dec.mjs`），返回它解出的正文。
fn run_fldigi(dec_mjs: &Path, opts: &Options, wav: &Path) -> String {
  let fldigi_web = opts.fldigi_web.as_ref().expect(
    "没有 fldigi-web：设置 $FLDIGI_WEB 或加 --fldigi-web <目录>，只跑本仓就加 --skip-fldigi",
  );
  let out = Command::new("node")
    .arg(dec_mjs)
    .arg(&opts.fldigi_mode)
    .arg(wav)
    .arg(opts.center.to_string())
    .env("FLDIGI_WEB", fldigi_web)
    .output()
    .unwrap_or_else(|e| panic!("跑不了 node（fldigi 对照需要 node 与 fldigi-web 克隆）：{e}"));
  if !out.status.success() {
    panic!(
      "dec.mjs 失败（{}）：{}",
      out.status,
      String::from_utf8_lossy(&out.stderr)
    );
  }
  let stdout = String::from_utf8_lossy(&out.stdout);
  let last = stdout.lines().last().unwrap_or("{}");
  json_field(last, "text").unwrap_or_default()
}

/// 从 dec.mjs 打的那一行 JSON 里取一个字符串字段（不为这一个字段引 serde_json）。
fn json_field(line: &str, key: &str) -> Option<String> {
  let needle = format!("\"{key}\":\"");
  let start = line.find(&needle)? + needle.len();
  let bytes = line.as_bytes();
  let mut out = String::new();
  let mut chunk = start;
  let mut i = start;
  while i < bytes.len() {
    match bytes[i] {
      b'"' => {
        out.push_str(&line[chunk..i]);
        return Some(out);
      }
      b'\\' => {
        out.push_str(&line[chunk..i]);
        i += 1;
        match *bytes.get(i)? {
          b'n' => out.push('\n'),
          b't' => out.push('\t'),
          b'r' => out.push('\r'),
          b'b' => out.push('\u{8}'),
          b'f' => out.push('\u{c}'),
          b'u' => {
            let hex = line.get(i + 1..i + 5)?;
            let code = u32::from_str_radix(hex, 16).ok()?;
            out.push(char::from_u32(code).unwrap_or('\u{fffd}'));
            i += 4;
          }
          other => out.push(char::from(other)),
        }
        i += 1;
        chunk = i;
      }
      _ => {
        // 可能是 UTF-8 多字节：走到下一个字符边界，切片才不会落在中间。
        i += 1;
        while i < bytes.len() && bytes[i] & 0xC0 == 0x80 {
          i += 1;
        }
      }
    }
  }
  None
}

/// 与原脚本同一套指标：完整出现几遍，以及字符吻合率（`difflib.SequenceMatcher`）。
fn score(text: &str, repeats: usize) -> (u32, f64) {
  let whole = text.matches(TEXT).count() as u32;
  let expected = format!("{TEXT} ").repeat(repeats);
  (whole, ratio(text.trim(), &expected))
}

/// `difflib.SequenceMatcher(None, a, b, autojunk=False).ratio()`。
///
/// 不是最长公共子序列：Python 是「反复取最长匹配块」再递归两侧，然后在块边界上向左右扩展。
/// 这里照抄那套递归，否则同一段文本会算出不同的数，与已记录的表格没法比。
fn ratio(a: &str, b: &str) -> f64 {
  let a: Vec<char> = a.chars().collect();
  let b: Vec<char> = b.chars().collect();
  let mut index: HashMap<char, Vec<usize>> = HashMap::new();
  for (j, &c) in b.iter().enumerate() {
    index.entry(c).or_default().push(j);
  }

  let mut queue = vec![(0usize, a.len(), 0usize, b.len())];
  let mut total = 0usize;
  while let Some((alo, ahi, blo, bhi)) = queue.pop() {
    let (i, j, k) = longest_match(&a, &b, &index, (alo, ahi, blo, bhi));
    if k == 0 {
      continue;
    }
    total += k;
    if alo < i && blo < j {
      queue.push((alo, i, blo, j));
    }
    if i + k < ahi && j + k < bhi {
      queue.push((i + k, ahi, j + k, bhi));
    }
  }

  let len = a.len() + b.len();
  if len == 0 {
    1.0
  } else {
    2.0 * total as f64 / len as f64
  }
}

fn longest_match(
  a: &[char],
  b: &[char],
  index: &HashMap<char, Vec<usize>>,
  (alo, ahi, blo, bhi): (usize, usize, usize, usize),
) -> (usize, usize, usize) {
  let (mut besti, mut bestj, mut bestsize) = (alo, blo, 0);
  let mut prev: HashMap<usize, usize> = HashMap::new();
  for (i, ch) in a.iter().enumerate().take(ahi).skip(alo) {
    let mut next = HashMap::new();
    if let Some(positions) = index.get(ch) {
      for &j in positions {
        if j < blo {
          continue;
        }
        if j >= bhi {
          break;
        }
        // `j == 0` 时 Python 查的是 key `-1`（不存在），不能折到 key 0 上。
        let k = if j == 0 {
          1
        } else {
          prev.get(&(j - 1)).copied().unwrap_or(0) + 1
        };
        next.insert(j, k);
        if k > bestsize {
          besti = i + 1 - k;
          bestj = j + 1 - k;
          bestsize = k;
        }
      }
    }
    prev = next;
  }
  // 块边界外的同字符不算在 DP 里，Python 在这里继续向两侧扩展（autojunk=False 时没有
  // junk 字符要跳过，所以只比较字符本身）。
  while besti > alo && bestj > blo && a[besti - 1] == b[bestj - 1] {
    besti -= 1;
    bestj -= 1;
    bestsize += 1;
  }
  while besti + bestsize < ahi
    && bestj + bestsize < bhi
    && a[besti + bestsize] == b[bestj + bestsize]
  {
    bestsize += 1;
  }
  (besti, bestj, bestsize)
}

/// CPython `random.Random(seed)` 的最小复刻：MT19937 + `init_by_array` + 53 位 `random()`
/// + `gauss()`（Box–Muller，第二个值缓存到下一次）。只为让新旧脚本喂出同一份噪声。
struct PyRandom {
  state: [u32; 624],
  index: usize,
  spare: Option<f64>,
}

impl PyRandom {
  fn new(seed: u32) -> Self {
    let mut rnd = Self {
      state: [0; 624],
      index: 624,
      spare: None,
    };
    rnd.seed_from(&[seed]);
    rnd
  }

  fn init_genrand(&mut self, seed: u32) {
    self.state[0] = seed;
    for i in 1..624 {
      let prev = self.state[i - 1];
      self.state[i] = 1_812_433_253u32
        .wrapping_mul(prev ^ (prev >> 30))
        .wrapping_add(i as u32);
    }
    self.index = 624;
  }

  /// CPython 的 `random.seed(int)` 走的就是 `init_by_array`（整数的 32 位字数组）。
  fn seed_from(&mut self, key: &[u32]) {
    self.init_genrand(19_650_218);
    let mut i = 1;
    let mut j = 0;
    for _ in 0..624.max(key.len()) {
      let prev = self.state[i - 1];
      let mixed = (prev ^ (prev >> 30)).wrapping_mul(1_664_525);
      self.state[i] = (self.state[i] ^ mixed)
        .wrapping_add(key[j])
        .wrapping_add(j as u32);
      i += 1;
      j += 1;
      if i >= 624 {
        self.state[0] = self.state[623];
        i = 1;
      }
      if j >= key.len() {
        j = 0;
      }
    }
    for _ in 0..623 {
      let prev = self.state[i - 1];
      let mixed = (prev ^ (prev >> 30)).wrapping_mul(1_566_083_941);
      self.state[i] = (self.state[i] ^ mixed).wrapping_sub(i as u32);
      i += 1;
      if i >= 624 {
        self.state[0] = self.state[623];
        i = 1;
      }
    }
    self.state[0] = 0x8000_0000;
    self.index = 624;
  }

  fn next_u32(&mut self) -> u32 {
    const MATRIX_A: u32 = 0x9908_b0df;
    if self.index >= 624 {
      for kk in 0..624 {
        let y = (self.state[kk] & 0x8000_0000) | (self.state[(kk + 1) % 624] & 0x7fff_ffff);
        let mut next = self.state[(kk + 397) % 624] ^ (y >> 1);
        if y & 1 != 0 {
          next ^= MATRIX_A;
        }
        self.state[kk] = next;
      }
      self.index = 0;
    }
    let mut y = self.state[self.index];
    self.index += 1;
    y ^= y >> 11;
    y ^= (y << 7) & 0x9d2c_5680;
    y ^= (y << 15) & 0xefc6_0000;
    y ^= y >> 18;
    y
  }

  fn random(&mut self) -> f64 {
    let a = self.next_u32() >> 5;
    let b = self.next_u32() >> 6;
    (f64::from(a) * 67_108_864.0 + f64::from(b)) * (1.0 / 9_007_199_254_740_992.0)
  }

  fn gauss(&mut self, sigma: f64) -> f64 {
    let z = self.spare.take().unwrap_or_else(|| {
      let angle = self.random() * std::f64::consts::TAU;
      let radius = (-2.0 * (1.0 - self.random()).ln()).sqrt();
      self.spare = Some(angle.sin() * radius);
      angle.cos() * radius
    });
    z * sigma
  }
}

/// 复刻的正确性靠 CPython 的实际输出钉住：随机数与 scoring 一旦偏了，「同一命令两边逐点可比」
/// 这个迁移验收标准就失效了。参照值由 Python 3.8 / 3.11 的 `random` 与 `difflib` 生成。
#[cfg(test)]
mod tests {
  use super::*;

  fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-12
  }

  #[test]
  fn py_random_matches_cpython() {
    let mut rnd = PyRandom::new(1000);
    for want in [
      0.777_356_642_700_563_9,
      0.669_825_559_559_249_7,
      0.099_139_603_924_817_02,
    ] {
      let got = rnd.random();
      assert!(close(got, want), "{got} != {want}");
    }
  }

  #[test]
  fn py_gauss_matches_cpython() {
    let mut rnd = PyRandom::new(1000);
    let want = [
      0.254_632_116_649_684,
      -1.466_775_647_039_671_1,
      0.757_869_999_938_949_9,
      0.544_389_454_999_776_1,
    ];
    for w in want {
      let got = rnd.gauss(1.0);
      assert!(close(got, w), "{got} != {w}");
    }
  }

  #[test]
  fn ratio_matches_difflib() {
    let expected = format!("{TEXT} ").repeat(3);
    assert!(close(
      ratio(expected.trim(), &expected),
      0.996_960_486_322_188_5
    ));
    let dirty = expected.replace("lazy", "laser").replace("quick", "quack");
    assert!(close(
      ratio(dirty.trim(), &expected),
      0.933_734_939_759_036_1
    ));
    // 结尾的重复串会让「递归取最长块」与朴素 LCS 给出不同的数。
    assert!(close(ratio("abcabc", "abcxabc"), 0.923_076_923_076_923_1));
  }
}
