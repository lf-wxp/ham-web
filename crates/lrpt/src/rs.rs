//! Reed-Solomon (255,223) 译码 —— CCSDS 约定，转写自 lrpt-encoder
//! （MIT）`reed_solomon.rs` 的编码侧（生成多项式、交织结构），译码侧为标准的
//! 伴随式 + Berlekamp-Massey + Chien 搜索 + Forney。
//!
//! 参数：本原多项式 0x187（x⁸+x⁷+x²+x+1）、首个根指数 112、根步进 11
//! （即生成多项式根 α^(11·(112+i))，i = 0..31）、交织深度 4（892 = 4×223）。
//!
//! 基底：普通 α 基底，**不做** CCSDS 双基底转换。这不是缺口 —— Meteor-M 的 LRPT 本来就
//! 不用双基底：SatDump（`plugins/meteor_support/meteor/module_meteor_lrpt_decoder.cpp`，
//! 2026-10 核对）两条解码路径都以 `decode_interlaved(&cadu[4], false, 4, errors)` 调用，
//! 第二个参数就是「是否双基底」。早先这里与页面提示都把它列为「真实录音还缺的一层」，是误判。

pub const N: usize = 255;
pub const K: usize = 223;
pub const TWO_T: usize = N - K; // 32
pub const INTERLEAVE: usize = 4;
/// 每帧 RS 数据区（VCDU）= 892 字节。
pub const DATA_LEN: usize = K * INTERLEAVE;

const PRIM_POLY: u16 = 0x187;
const FIRST_ROOT: u16 = 112;
const PRIM: u16 = 11;

/// GF(2⁸) 表：`exp[i]` = α^i，`log[x]` = x 的离散对数（log[0] 未定义，置 0）。
struct Gf {
  exp: [u8; 512], // 加倍长度，免取模
  log: [u8; 256],
}

/// 进程内共享的 GF 表：每帧重建一次（~1 KB 初始化）在上万帧的录音里是纯浪费。
fn gf() -> &'static Gf {
  static GF: std::sync::OnceLock<Gf> = std::sync::OnceLock::new();
  GF.get_or_init(Gf::new)
}

impl Gf {
  fn new() -> Self {
    let mut exp = [0u8; 512];
    let mut log = [0u8; 256];
    let mut x = 1u16;
    for (i, slot) in exp.iter_mut().enumerate().take(255) {
      *slot = x as u8;
      log[x as usize] = i as u8;
      x <<= 1;
      if x & 0x100 != 0 {
        x ^= PRIM_POLY;
      }
    }
    for i in 255..512 {
      exp[i] = exp[i - 255];
    }
    log[0] = 0;
    Gf { exp, log }
  }

  fn mul(&self, a: u8, b: u8) -> u8 {
    if a == 0 || b == 0 {
      return 0;
    }
    self.exp[usize::from(self.log[a as usize]) + usize::from(self.log[b as usize])]
  }

  fn inv(&self, a: u8) -> u8 {
    debug_assert_ne!(a, 0, "0 没有逆元");
    self.exp[255 - usize::from(self.log[a as usize])]
  }
}

/// RS 生成多项式（常数项在前，32+1 个系数），与参照实现的编码侧一致。
fn generator(gf: &Gf) -> [u8; TWO_T + 1] {
  let mut poly = vec![1u8];
  for i in 0..TWO_T {
    let root_exp = (PRIM * (FIRST_ROOT + i as u16)) % 255;
    let root = gf.exp[root_exp as usize];
    let mut next = vec![0u8; poly.len() + 1];
    for (j, &c) in poly.iter().enumerate() {
      next[j] ^= gf.mul(c, root);
      next[j + 1] ^= c;
    }
    poly = next;
  }
  let mut out = [0u8; TWO_T + 1];
  out.copy_from_slice(&poly);
  out
}

/// 编码一个 892 字节 VCDU → 1020 字节 RS 码字（测试与对拍用，镜像参照实现）。
#[must_use]
pub fn encode_frame(data: &[u8; DATA_LEN]) -> [u8; 1020] {
  let gf = gf();
  let genpoly = generator(gf);
  let mut sub_blocks = [[0u8; K]; INTERLEAVE];
  for (i, &b) in data.iter().enumerate() {
    sub_blocks[i % INTERLEAVE][i / INTERLEAVE] = b;
  }
  let mut parities = [[0u8; TWO_T]; INTERLEAVE];
  for (j, sub) in sub_blocks.iter().enumerate() {
    parities[j] = encode_block(gf, &genpoly, sub);
  }
  let mut out = [0u8; 1020];
  out[..DATA_LEN].copy_from_slice(data);
  for i in 0..TWO_T {
    for (j, p) in parities.iter().enumerate() {
      out[DATA_LEN + i * INTERLEAVE + j] = p[i];
    }
  }
  out
}

/// 单码字编码：数据 223 字节（最高阶系数在前）→ 32 校验字节。
fn encode_block(gf: &Gf, genpoly: &[u8; TWO_T + 1], data: &[u8; K]) -> [u8; TWO_T] {
  let mut modpoly = [0u8; N];
  for (i, &byte) in data.iter().enumerate() {
    modpoly[N - 1 - i] = byte;
  }
  for i in (TWO_T..N).rev() {
    let top = modpoly[i];
    if top == 0 {
      continue;
    }
    let q_coeff = if genpoly[TWO_T] == 1 {
      top
    } else {
      gf.mul(top, gf.inv(genpoly[TWO_T]))
    };
    let q_order = i - TWO_T;
    for (j, &g) in genpoly.iter().enumerate() {
      if g != 0 {
        modpoly[q_order + j] ^= gf.mul(g, q_coeff);
      }
    }
  }
  let mut parity = [0u8; TWO_T];
  for (i, p) in parity.iter_mut().enumerate() {
    *p = modpoly[TWO_T - 1 - i];
  }
  parity
}

/// 译码一帧：1020 字节（去扰码后）→ 892 字节 VCDU。
///
/// 4 个交织码字各自独立译码（可纠至多 16 字节错/码字）；任一字不可纠则整帧
/// 返回 `None`（干净的合成链路不会发生；真实信号的容错属后续层）。
#[must_use]
pub fn decode_frame(frame: &[u8; 1020]) -> Option<[u8; DATA_LEN]> {
  let gf = gf();
  let mut out = [0u8; DATA_LEN];
  for j in 0..INTERLEAVE {
    // 交织还原：码字 j = 数据 [j..892 step 4] + 校验 [892 + i*4 + j]
    let mut cw = [0u8; N];
    for i in 0..K {
      cw[i] = frame[i * INTERLEAVE + j];
    }
    for i in 0..TWO_T {
      cw[K + i] = frame[DATA_LEN + i * INTERLEAVE + j];
    }
    // 任一码字不可纠 → 整帧丢弃。
    decode_codeword(gf, &mut cw)?;
    for i in 0..K {
      out[i * INTERLEAVE + j] = cw[i];
    }
  }
  Some(out)
}

/// 单码字译码（原地纠正）。返回纠正的字节数；`None` = 不可纠。
fn decode_codeword(gf: &Gf, cw: &mut [u8; N]) -> Option<usize> {
  // 伴随式 S_k = cw(α^(11·(112+k)))
  let mut synd = [0u8; TWO_T];
  for (k, s) in synd.iter_mut().enumerate() {
    let root_exp = (PRIM * (FIRST_ROOT + k as u16)) % 255;
    let root = gf.exp[root_exp as usize];
    // Horner：系数从 x^254 到 x^0（cw[0] 是最高阶）
    let mut acc = 0u8;
    for &c in cw.iter() {
      acc = gf.mul(acc, root) ^ c;
    }
    *s = acc;
  }
  if synd.iter().all(|&s| s == 0) {
    return Some(0);
  }

  // Berlekamp-Massey：求错误位置多项式 Λ（常数项在前）。
  let mut lambda = [0u8; TWO_T + 1];
  lambda[0] = 1;
  let mut b = [0u8; TWO_T + 1];
  b[0] = 1;
  let mut l = 0usize;
  let mut m = 1usize;
  let mut b_inv = 1u8;
  for n in 0..TWO_T {
    // 偏差 d_n = S_n + Σ Λ_i · S_{n-i}
    let mut d = synd[n];
    for i in 1..=l {
      d ^= gf.mul(lambda[i], synd[n - i]);
    }
    if d == 0 {
      m += 1;
      continue;
    }
    let t = lambda;
    let coef = gf.mul(d, b_inv);
    for i in 0..=TWO_T - m {
      lambda[i + m] ^= gf.mul(coef, b[i]);
    }
    if 2 * l <= n {
      l = n + 1 - l;
      b = t;
      b_inv = gf.inv(d);
      m = 1;
    } else {
      m += 1;
    }
  }
  if l > TWO_T / 2 {
    return None; // 错误太多，不可纠
  }

  // Chien 搜索：根的步进是 11（生成多项式根 = α^(212+11k)），因此要在
  // Λ(α^(-11·j)) = 0 的 j 上找根 —— 步进会在 mod 255 的逆映射里消掉，
  // 出错位置 i = 254 − j（cw[i] 是 x^(254-i) 的系数）。这是「首根 112、
  // 步进 11」的非连续根约定 —— 忽略步进会搜出 5 个与真实位置无关的根
  // （对拍测试当场抓到）。
  let mut positions = Vec::new();
  for j in 0..N {
    let mut acc = 0u8;
    for (k, &lk) in lambda.iter().enumerate() {
      if lk != 0 {
        let e = (255 - (11 * j * k) % 255) % 255;
        acc ^= gf.mul(lk, gf.exp[e]);
      }
    }
    if acc == 0 {
      positions.push((N - 1 - j) % N);
    }
  }
  if positions.len() != l {
    return None;
  }

  // Forney：错误值 e_i = - X_i · Ω(X_i⁻¹) / Λ'(X_i⁻¹)，X_i = α^i。
  // Ω（错误求值多项式）可省去：RS 在特征 2 里 e_i = X_i^{1-b} · Ω(X_i⁻¹)/Λ'(X_i⁻¹)；
  // 这里用「错误幅值多项式」重算太繁 —— 用伴随式直接构造 Ω：Ω(x) = Λ(x)·S(x) mod x^2T。
  let mut omega = [0u8; TWO_T];
  for i in 0..TWO_T {
    let mut acc = 0u8;
    for j in 0..=i.min(l) {
      acc ^= gf.mul(lambda[j], synd[i - j]);
    }
    omega[i] = acc;
  }
  // Forney：全部在 y 域计算 —— Y = X^11，S_k = Σ e_i·X_i^b·Y_i^k。
  //   e_i = X_i^(11−b) · Ω(Y⁻¹) / Λ'(Y⁻¹)
  // b = (112×11) mod 255 = 212；X_i = α^(254−i)；Y⁻¹ = α^(−11j)，j = 254−i。
  let b_first = (PRIM * FIRST_ROOT) % 255;
  let mut corrected = 0usize;
  for &pos in &positions {
    let j = (N - 1 - pos) % N;
    // Λ'(Y⁻¹)：特征 2 只保留奇次项并降一阶。
    let mut lambda_prime = 0u8;
    for k in (1..=l).step_by(2) {
      if lambda[k] != 0 {
        let e = (255 - (11 * j * (k - 1)) % 255) % 255;
        lambda_prime ^= gf.mul(lambda[k], gf.exp[e]);
      }
    }
    if lambda_prime == 0 {
      return None;
    }
    // Ω(Y⁻¹)
    let mut omega_at = 0u8;
    for (k, &ok) in omega.iter().enumerate() {
      if ok != 0 {
        let e = (255 - (11 * j * k) % 255) % 255;
        omega_at ^= gf.mul(ok, gf.exp[e]);
      }
    }
    // X_i^(11−b) = α^(j·(11−b) mod 255)，X_i = α^j = α^(254−i)。
    let x_factor = gf.exp[(j * ((11 + 255 - b_first) % 255) as usize) % 255];
    let mag = gf.mul(x_factor, gf.mul(omega_at, gf.inv(lambda_prime)));
    cw[pos] ^= mag;
    corrected += 1;
  }
  // 纠正后校验伴随式归零。
  for k in 0..TWO_T {
    let root_exp = (PRIM * (FIRST_ROOT + k as u16)) % 255;
    let root = gf.exp[root_exp as usize];
    let mut acc = 0u8;
    for &c in cw.iter() {
      acc = gf.mul(acc, root) ^ c;
    }
    if acc != 0 {
      return None;
    }
  }
  Some(corrected)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn encode_decode_round_trips() {
    let mut data = [0u8; DATA_LEN];
    for (i, b) in data.iter_mut().enumerate() {
      *b = (i * 13 % 251) as u8;
    }
    let coded = encode_frame(&data);
    let decoded = decode_frame(&coded).expect("译码");
    assert_eq!(decoded, data);
  }

  #[test]
  fn corrects_injected_errors() {
    let mut data = [0u8; DATA_LEN];
    for (i, b) in data.iter_mut().enumerate() {
      *b = (i * 7 + 3) as u8;
    }
    let mut coded = encode_frame(&data);
    // 每码字注入 5 个字节错（< 16，可纠）。
    for cw in 0..INTERLEAVE {
      for k in 0..5 {
        let idx = (cw + k * 37) % N;
        let pos = if idx < K {
          idx * INTERLEAVE + cw
        } else {
          DATA_LEN + (idx - K) * INTERLEAVE + cw
        };
        coded[pos] ^= 0xA5;
      }
    }
    let decoded = decode_frame(&coded).expect("译码");
    assert_eq!(decoded, data, "每码字 5 字节错应全部纠正");
  }

  #[test]
  fn rejects_uncorrectable_frames() {
    let mut data = [0u8; DATA_LEN];
    for (i, b) in data.iter_mut().enumerate() {
      *b = i as u8;
    }
    let mut coded = encode_frame(&data);
    // 码字 0 注入 17 字节错（> 16，不可纠）。
    for k in 0..17 {
      let pos = k * INTERLEAVE;
      coded[pos] ^= 0x5A;
    }
    assert!(decode_frame(&coded).is_none(), "超过纠错能力的帧应被拒绝");
  }
}
