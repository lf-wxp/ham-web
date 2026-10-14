//! DominoEX FEC 的独立参考实现：fldigi（GPL-3.0）`dominoex.cxx` 的 `sendMuPskEX` /
//! `decodeMuPskEX`、`viterbi.cxx` 的 `encoder` / `viterbi`、`interleave.cxx` 的 `interleave`
//! **逐行直译**（C 风格的下标、`unsigned` 回绕、平铺的交织表），与 `src/fec.rs` 不共享代码。
//!
//! 码表从 e2e 的 TS 数据模块里现场解析（与 `src/mfsk_varicode.rs` 同由
//! `examples/extract_mfsk_varicode.rs` 从 `mfskvaricode.cxx` 生成，但走的是另一份产物）。

// 刻意保留 C 的写法（下标循环、手写取模）：参考实现的价值在于**一眼能对上原文**，
// 改成迭代器风格就不再是逐行直译了。
#![allow(dead_code, clippy::needless_range_loop, clippy::manual_is_multiple_of)]

const TS_TABLE: &str = include_str!("../../../../e2e/tests/mfsk_varicode_data.ts");

fn varicode() -> Vec<String> {
  TS_TABLE
    .lines()
    .filter_map(|l| l.trim().strip_prefix('"'))
    .map(|l| l.split('"').next().unwrap().to_owned())
    .collect()
}

fn parity(mut w: u32) -> i32 {
  let mut p = 0;
  while w != 0 {
    p ^= (w & 1) as i32;
    w >>= 1;
  }
  p
}

// ---------------------------------------------------------------- encoder

pub struct Encoder {
  output: Vec<i32>,
  shreg: u32,
  shregmask: u32,
}

impl Encoder {
  pub fn new(k: u32, poly1: u32, poly2: u32) -> Self {
    let size = 1u32 << k;
    let output = (0..size)
      .map(|i| parity(poly1 & i) | (parity(poly2 & i) << 1))
      .collect();
    Self {
      output,
      shreg: 0,
      shregmask: size - 1,
    }
  }

  pub fn encode(&mut self, bit: i32) -> i32 {
    self.shreg = (self.shreg << 1) | u32::from(bit != 0);
    self.output[(self.shreg & self.shregmask) as usize]
  }
}

// ---------------------------------------------------------------- viterbi

const PATHMEM: u32 = 256;

pub struct Viterbi {
  traceback: i32,
  chunksize: u32,
  nstates: usize,
  output: Vec<i32>,
  metrics: Vec<Vec<i32>>,
  history: Vec<Vec<i32>>,
  sequence: Vec<i32>,
  mettab: [[i32; 256]; 2],
  ptr: u32,
}

impl Viterbi {
  pub fn new(k: u32, poly1: u32, poly2: u32) -> Self {
    let outsize = 1u32 << k;
    let nstates = 1usize << (k - 1);
    let mut mettab = [[0i32; 256]; 2];
    for i in 0..256 {
      mettab[0][i] = 128 - i as i32;
      mettab[1][i] = i as i32 - 128;
    }
    Self {
      traceback: (k * 12) as i32,
      chunksize: 8,
      nstates,
      output: (0..outsize)
        .map(|i| parity(poly1 & i) | (parity(poly2 & i) << 1))
        .collect(),
      metrics: vec![vec![0; nstates]; PATHMEM as usize],
      history: vec![vec![0; nstates]; PATHMEM as usize],
      sequence: vec![0; PATHMEM as usize],
      mettab,
      ptr: 0,
    }
  }

  pub fn settraceback(&mut self, t: i32) {
    self.traceback = t;
  }

  pub fn setchunksize(&mut self, c: u32) {
    self.chunksize = c;
  }

  fn do_traceback(&mut self) -> i32 {
    let mut p = self.ptr.wrapping_sub(1) % PATHMEM;
    let mut bestmetric = i32::MIN;
    let mut beststate = 0;
    for i in 0..self.nstates {
      if self.metrics[p as usize][i] > bestmetric {
        bestmetric = self.metrics[p as usize][i];
        beststate = i as i32;
      }
    }
    self.sequence[p as usize] = beststate;
    for _ in 0..self.traceback {
      let prev = p.wrapping_sub(1) % PATHMEM;
      self.sequence[prev as usize] = self.history[p as usize][self.sequence[p as usize] as usize];
      p = prev;
    }
    let mut c = 0;
    for _ in 0..self.chunksize {
      c = (c << 1) | (self.sequence[p as usize] & 1);
      p = (p + 1) % PATHMEM;
    }
    c
  }

  pub fn decode(&mut self, sym: &[u8; 2]) -> i32 {
    let currptr = self.ptr;
    let prevptr = currptr.wrapping_sub(1) % PATHMEM;
    let m = &self.mettab;
    let (s0v, s1v) = (sym[0] as usize, sym[1] as usize);
    let met = [
      m[0][s1v] + m[0][s0v],
      m[0][s1v] + m[1][s0v],
      m[1][s1v] + m[0][s0v],
      m[1][s1v] + m[1][s0v],
    ];
    for n in 0..self.nstates {
      let s0 = n;
      let s1 = n + self.nstates;
      let p0 = s0 >> 1;
      let p1 = s1 >> 1;
      let m0 = self.metrics[prevptr as usize][p0] + met[self.output[s0] as usize];
      let m1 = self.metrics[prevptr as usize][p1] + met[self.output[s1] as usize];
      if m0 > m1 {
        self.metrics[currptr as usize][n] = m0;
        self.history[currptr as usize][n] = p0 as i32;
      } else {
        self.metrics[currptr as usize][n] = m1;
        self.history[currptr as usize][n] = p1 as i32;
      }
    }
    self.ptr = (self.ptr + 1) % PATHMEM;
    if self.ptr % self.chunksize == 0 {
      return self.do_traceback();
    }
    -1
  }
}

// ---------------------------------------------------------------- interleave

pub struct Interleave {
  size: usize,
  depth: usize,
  fwd: bool,
  table: Vec<u8>,
}

impl Interleave {
  pub fn new(size: usize, depth: usize, fwd: bool) -> Self {
    Self {
      size,
      depth,
      fwd,
      table: vec![if fwd { 0 } else { 128 }; size * size * depth],
    }
  }

  fn tab(&self, i: usize, j: usize, k: usize) -> usize {
    (self.size * self.size * i) + (self.size * j) + k
  }

  pub fn symbols(&mut self, psyms: &mut [u8]) {
    let size = self.size;
    for k in 0..self.depth {
      for i in 0..size {
        for j in 0..size - 1 {
          let (a, b) = (self.tab(k, i, j), self.tab(k, i, j + 1));
          self.table[a] = self.table[b];
        }
      }
      for i in 0..size {
        let a = self.tab(k, i, size - 1);
        self.table[a] = psyms[i];
      }
      for i in 0..size {
        let a = if self.fwd {
          self.tab(k, i, size - i - 1)
        } else {
          self.tab(k, i, i)
        };
        psyms[i] = self.table[a];
      }
    }
  }

  pub fn bits(&mut self, pbits: &mut u32) {
    let size = self.size;
    let mut syms = vec![0u8; size];
    for i in 0..size {
      syms[i] = ((*pbits >> (size - i - 1)) & 1) as u8;
    }
    self.symbols(&mut syms);
    *pbits = 0;
    for i in 0..size {
      *pbits = (*pbits << 1) | u32::from(syms[i]);
    }
  }
}

// ---------------------------------------------------------------- dominoex

/// fldigi `MuPskSec2Pri` 去掉重音折叠表（参考实现只用到 ASCII）。
fn sec2pri(mut c: i32) -> i32 {
  if (b'a' as i32..=b'z' as i32).contains(&c) {
    c -= 32;
  }
  let r = |lo: u8, hi: u8| (lo as i32..=hi as i32).contains(&c);
  if r(b'A', b'Z') {
    c - b'A' as i32 + 127
  } else if r(b'0', b'9') {
    c - b'0' as i32 + 14
  } else if r(b' ', b'"') {
    c - b' ' as i32 + 1
  } else if c == b'_' as i32 {
    4
  } else if r(b'$', b'&') {
    c - b'$' as i32 + 5
  } else if r(b'\'', b'*') {
    c - b'\'' as i32 + 9
  } else if r(b'+', b'/') {
    c - b'+' as i32 + 24
  } else if r(b':', b'<') {
    c - b':' as i32 + 29
  } else if r(b'=', b'@') {
    c - b'=' as i32 + 153
  } else if r(b'[', b']') {
    c - b'[' as i32 + 157
  } else {
    b'_' as i32
  }
}

fn prisec(c: i32) -> i32 {
  let r = |lo: i32, hi: i32| (lo..hi).contains(&c);
  if r(127, 153) {
    c + (b'A' as i32 - 127) + 0x100
  } else if r(14, 24) {
    c + (b'0' as i32 - 14) + 0x100
  } else if r(1, 4) {
    c + (b' ' as i32 - 1) + 0x100
  } else if c == 4 {
    b'_' as i32 + 0x100
  } else if r(5, 8) {
    c + (b'$' as i32 - 5) + 0x100
  } else if r(9, 13) {
    c + (b'\'' as i32 - 9) + 0x100
  } else if r(24, 29) {
    c + (b'+' as i32 - 24) + 0x100
  } else if r(29, 32) {
    c + (b':' as i32 - 29) + 0x100
  } else if r(153, 157) {
    c + (b'=' as i32 - 153) + 0x100
  } else if r(157, 160) {
    c + (b'[' as i32 - 157) + 0x100
  } else {
    c
  }
}

pub struct Tx {
  enc: Encoder,
  inlv: Interleave,
  bitshreg: u32,
  bitstate: i32,
  table: Vec<String>,
  pub symbols: Vec<u8>,
}

impl Tx {
  pub fn new() -> Self {
    Self {
      enc: Encoder::new(7, 0x6d, 0x4f),
      inlv: Interleave::new(4, 4, true),
      bitshreg: 0,
      bitstate: 0,
      table: varicode(),
      symbols: Vec::new(),
    }
  }

  pub fn clearbits(&mut self) {
    let data = self.enc.encode(0);
    for _ in 0..100 {
      for i in 0..2 {
        self.bitshreg = (self.bitshreg << 1) | ((data as u32 >> i) & 1);
        self.bitstate += 1;
        if self.bitstate == 4 {
          self.inlv.bits(&mut self.bitshreg);
          self.bitstate = 0;
          self.bitshreg = 0;
        }
      }
    }
  }

  pub fn send(&mut self, c: u8, secondary: bool) {
    let mut c = i32::from(c);
    if secondary {
      c = sec2pri(c);
    } else {
      if c == 10 {
        return;
      }
      if (1..=7).contains(&c)
        || (9..=12).contains(&c)
        || (14..=31).contains(&c)
        || (127..=159).contains(&c)
      {
        c = b'_' as i32;
      }
    }
    let code = self.table[c as usize].clone();
    for ch in code.bytes() {
      let data = self.enc.encode(i32::from(ch - b'0'));
      for i in 0..2 {
        self.bitshreg = (self.bitshreg << 1) | ((data as u32 >> i) & 1);
        self.bitstate += 1;
        if self.bitstate == 4 {
          self.inlv.bits(&mut self.bitshreg);
          self.symbols.push(self.bitshreg as u8);
          self.bitstate = 0;
          self.bitshreg = 0;
        }
      }
    }
  }
}

pub struct Rx {
  inlv: Interleave,
  dec: Viterbi,
  pair: [u8; 2],
  symcounter: i32,
  datashreg: u32,
  decode_table: Vec<u32>,
  pub text: Vec<i32>,
}

impl Rx {
  pub fn new() -> Self {
    let mut dec = Viterbi::new(7, 0x6d, 0x4f);
    dec.settraceback(45);
    dec.setchunksize(1);
    Self {
      inlv: Interleave::new(4, 4, false),
      dec,
      pair: [0; 2],
      symcounter: 0,
      datashreg: 1,
      decode_table: varicode()
        .iter()
        .map(|s| u32::from_str_radix(s, 2).unwrap())
        .collect(),
      text: Vec::new(),
    }
  }

  fn symbol(&mut self, symbol: u8) {
    self.pair[0] = self.pair[1];
    self.pair[1] = symbol;
    self.symcounter = if self.symcounter != 0 { 0 } else { 1 };
    if self.symcounter != 0 {
      return;
    }
    let c = self.dec.decode(&self.pair);
    if c == -1 {
      return;
    }
    self.datashreg = (self.datashreg << 1) | u32::from(c != 0);
    if (self.datashreg & 7) == 1 {
      let want = self.datashreg >> 1;
      let ch = self
        .decode_table
        .iter()
        .position(|&v| v == want)
        .map_or(-1, |i| i as i32);
      if ch != -1 {
        self.text.push(prisec(ch));
      }
      self.datashreg = 1;
    }
  }

  pub fn feed(&mut self, ch: i32) {
    let mut symbols = [0u8; 4];
    let mut c = ch;
    for i in 0..4 {
      symbols[3 - i] = if (c & 1) == 1 { 255 } else { 1 };
      c /= 2;
    }
    self.inlv.symbols(&mut symbols);
    for s in symbols {
      self.symbol(s);
    }
  }

  pub fn primary(&self) -> String {
    self
      .text
      .iter()
      .filter(|&&c| c < 0x100)
      .map(|&c| char::from(c as u8))
      .collect()
  }
}
