//! VCDU / MPDU 层：RS 后的 892 字节 VCDU → 源包流 → MSU-MR 段。
//!
//! 转写自 lrpt-encoder（MIT）`ccsds.rs` 的封装结构：
//! VCDU = [6B 主头 | 2B 插入区 | 2B MPDU 头（首包头指针）| 882B 数据区]；
//! 源包 = [2B 版本+APID | 2B 序号 | 2B 包长（-1）| 载荷]。
//! MSU-MR 段载荷 = [day(2) ms(4) us(2) MCUN(1) QT(1) DCAC(1) QFM(2) QF(1) | 压缩数据…]。

use std::collections::BTreeMap;

/// VCDU 数据区长度。
pub const VCDU_DATA: usize = 892;
/// VCDU 主头。
pub const VCDU_HEADER: usize = 6;
/// 插入区。
pub const INSERT_ZONE: usize = 2;
/// MPDU 头（首包头指针）。
pub const MPDU_HEADER: usize = 2;
/// MPDU 数据区。
pub const MPDU_DATA: usize = VCDU_DATA - VCDU_HEADER - INSERT_ZONE - MPDU_HEADER; // 882

/// 空闲（填充）虚拟信道：CCSDS 约定 VCID 全 1，不携带图像也不参与计数。
pub const IDLE_VCID: u8 = 0x3F;
/// 帧计数的取值空间（24 位）。
const COUNTER_MODULUS: u32 = 1 << 24;

/// 帧里没有包起始时的首包头指针（全 1）。
pub const NO_PACKET_START: u16 = 0x7FF;

/// 一个 MSU-MR 图像段包（APID 64/65/66 = R/G/B）。
#[derive(Debug, Clone)]
pub struct Segment {
  /// 通道 APID（64/65/66）。
  pub apid: u16,
  /// MCU 序号（0, 14, 28, …, 182）。
  pub mcu_number: u8,
  /// 质量因子。
  pub quality: u8,
  /// 压缩数据（14 个 MCU 的 Huffman 位流）。
  pub data: Vec<u8>,
}

/// VCDU 帧头字段。
pub struct VcduHeader {
  /// 虚拟信道 ID。
  pub vcid: u8,
  /// 帧计数（0..2^24-1）。
  pub counter: u32,
  /// 首包头指针。
  pub first_header_ptr: u16,
}

/// 解析 VCDU 主头。
#[must_use]
pub fn parse_header(vcdu: &[u8; VCDU_DATA]) -> VcduHeader {
  let word0 = u16::from_be_bytes([vcdu[0], vcdu[1]]);
  let vcid = (word0 & 0x3F) as u8;
  let counter = u32::from(vcdu[2]) << 16 | u32::from(vcdu[3]) << 8 | u32::from(vcdu[4]);
  let first_header_ptr = u16::from_be_bytes([vcdu[8], vcdu[9]]) & NO_PACKET_START;
  VcduHeader {
    vcid,
    counter,
    first_header_ptr,
  }
}

/// 把 VCDU 流聚合成源包序列，再按通道分拣出 MSU-MR 段。
///
/// 帧的数据区首尾相接成一条包流（参照实现的 `pending` 缓冲）；从第一条帧
/// （计数 0，包起点 0）起顺序解析包边界（包长字段自描述）。
pub struct MuxStream {
  /// 相接的 MPDU 数据区字节流。
  stream: Vec<u8>,
  /// 每帧起点的流内偏移（用于 fhp 校验）。
  frame_offsets: Vec<usize>,
  /// 每帧的首包头指针。
  fhps: Vec<u16>,
  /// 帧计数（连续性校验）。
  counters: Vec<u32>,
  /// 已锁定的虚拟信道：第一个非空闲帧的 VCID。各 VC 的帧计数相互独立，不同 VC 的帧混进来
  /// 会让单调性检查误判整段录音、还会把别的信道的字节拼进包流。
  vcid: Option<u8>,
}

impl MuxStream {
  #[must_use]
  pub fn new() -> Self {
    MuxStream {
      stream: Vec::new(),
      frame_offsets: Vec::new(),
      fhps: Vec::new(),
      counters: Vec::new(),
      vcid: None,
    }
  }

  /// 追加一帧（按流顺序）。
  ///
  /// 以下帧被忽略：空闲信道（VCID 63）、与已锁定信道不同的 VC、与上一帧计数相同的重复帧
  /// （重复会让单调性检查拒绝整段录音）。
  pub fn push(&mut self, vcdu: &[u8; VCDU_DATA]) {
    let h = parse_header(vcdu);
    if h.vcid == IDLE_VCID || self.vcid.is_some_and(|v| v != h.vcid) {
      return;
    }
    if self.counters.last() == Some(&h.counter) {
      return;
    }
    self.vcid.get_or_insert(h.vcid);
    self.frame_offsets.push(self.stream.len());
    self.fhps.push(h.first_header_ptr);
    self.counters.push(h.counter);
    self
      .stream
      .extend_from_slice(&vcdu[VCDU_HEADER + INSERT_ZONE + MPDU_HEADER..]);
  }

  /// 帧计数是否连续（0,1,2,…）。
  #[must_use]
  pub fn counters_are_consecutive(&self) -> bool {
    // 首帧被滑窗 Viterbi 的收敛区吃掉是常态 —— 允许从任意值起连续递增，
    // 不要求从 0 开始。
    if self.counters.len() < 2 {
      return true;
    }
    let first = self.counters[0];
    self
      .counters
      .iter()
      .enumerate()
      // `wrapping_add`：`first` 接近 2^24 时普通加法会溢出（debug panic）。
      .all(|(i, &c)| c == first.wrapping_add(i as u32))
  }

  /// 帧计数是否单调递增（允许空洞）。
  ///
  /// 真实过境录音丢帧是常态，空洞不该让整段录音解不出图像；但乱序说明同步到了错位的
  /// 帧，必须拒绝。
  #[must_use]
  pub fn counters_are_monotonic(&self) -> bool {
    // 24 位计数会回绕（0xFFFFFF → 0）：用模减法判断「前进了不到半个空间」，而不是直接比大小。
    self.counters.windows(2).all(|w| {
      let step = w[1].wrapping_sub(w[0]) % COUNTER_MODULUS;
      step != 0 && step < COUNTER_MODULUS / 2
    })
  }

  /// 第 `i` 帧与上一帧之间是否丢了帧（计数不连续）。
  fn gap_before(&self, i: usize) -> bool {
    i > 0 && self.counters[i].wrapping_sub(self.counters[i - 1]) % COUNTER_MODULUS != 1
  }

  /// 从第 `from` 帧起，第一个带包起点（fhp 有效）的帧的流内起点。
  fn next_packet_start(&self, from: usize) -> Option<(usize, usize)> {
    (from..self.fhps.len())
      .find(|&i| usize::from(self.fhps[i]) < MPDU_DATA)
      .map(|i| (i, self.frame_offsets[i] + usize::from(self.fhps[i])))
  }

  /// 解析包流：`(apid, 序号, 载荷)`。遇到填充（0xFF）即停止。
  ///
  /// 流首可能截在包中间（首帧被滑窗 Viterbi 的收敛区吃掉是常态）—— 用各帧的
  /// 首包头指针（fhp）重新同步：fhp 指向本帧里第一个包的起点。
  #[must_use]
  pub fn packets(&self) -> Vec<(u16, u16, Vec<u8>)> {
    let mut out = Vec::new();
    let Some(mut frame_idx) = self.fhps.iter().position(|&p| usize::from(p) < MPDU_DATA) else {
      return out;
    };
    let mut pos = self.frame_offsets[frame_idx] + usize::from(self.fhps[frame_idx]);
    let fill_start = self
      .stream
      .iter()
      .rposition(|&b| b != 0xFF)
      .map_or(0, |p| p + 1);
    while pos + 6 <= self.stream.len() {
      // 推进到包含 pos 的帧。
      while frame_idx + 1 < self.frame_offsets.len() && self.frame_offsets[frame_idx + 1] <= pos {
        frame_idx += 1;
      }
      let fhp = self.fhps[frame_idx];
      if usize::from(fhp) < MPDU_DATA {
        let target = self.frame_offsets[frame_idx] + usize::from(fhp);
        if target > pos {
          pos = target; // 包在本帧更晚处开始
          continue;
        }
      }
      let s = &self.stream[pos..];
      let word0 = u16::from_be_bytes([s[0], s[1]]);
      let apid = word0 & 0x7FF;
      let seq = u16::from_be_bytes([s[2], s[3]]);
      let pkt_len = u16::from_be_bytes([s[4], s[5]]) as usize + 1;
      let total = 6 + pkt_len;
      if pos + total > self.stream.len() {
        break;
      }
      // 填充帧：全 0xFF → APID = 0x7FF，到此为止。
      if apid == 0x7FF && pos >= fill_start {
        break;
      }
      // 跨过丢帧的包：流是各帧数据区首尾相接的，丢帧后包长字段会把**后一帧**的字节当成
      // 本包的尾巴，拼出夹杂别处字节的垃圾包（Huffman 对任意位流都可能「解码成功」，
      // 图像上就是花块）。这种包整个丢弃，在丢帧之后第一个带包起点的帧重新同步。
      // `- 1` 不会下溢：`frame_offsets[0] == 0`（`push` 首次入栈时 `stream` 还是空的），
      // 而此处 `pos + total >= 6 + 1`，所以 `partition_point` 至少命中第 0 帧。
      let end_frame = self.frame_offsets.partition_point(|&o| o < pos + total) - 1;
      if (frame_idx + 1..=end_frame).any(|k| self.gap_before(k)) {
        let gap = (frame_idx + 1..=end_frame)
          .find(|&k| self.gap_before(k))
          .unwrap_or(end_frame);
        match self.next_packet_start(gap) {
          Some((f, target)) => {
            frame_idx = f;
            pos = target;
            continue;
          }
          None => break,
        }
      }
      out.push((apid, seq, s[6..total].to_vec()));
      pos += total;
    }
    out
  }

  /// 分拣 MSU-MR 段（按 (通道, MCUN) 排序）。跳过 APID 70 遥测。
  #[must_use]
  pub fn segments(&self) -> BTreeMap<(u16, u8), Segment> {
    let mut out = BTreeMap::new();
    for (apid, _seq, payload) in self.packets() {
      if !(64..=66).contains(&apid) || payload.len() < 14 {
        continue;
      }
      let mcu_number = payload[8];
      let quality = payload[13];
      out.insert(
        (apid, mcu_number),
        Segment {
          apid,
          mcu_number,
          quality,
          data: payload[14..].to_vec(),
        },
      );
    }
    out
  }
}

impl Default for MuxStream {
  fn default() -> Self {
    Self::new()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  /// 手工构造一帧：一个源包 + 填充。
  fn make_vcdu(counter: u32, packet: &[u8]) -> [u8; VCDU_DATA] {
    let mut v = [0u8; VCDU_DATA];
    v[0] = 0x01; // 版本 01
    v[1] = 0x05; // VCID 5（SCID 57 → word0 = 0b01_111001_000101 = 0x1CE5）
    v[0] = 0x1C;
    v[1] = 0xE5;
    v[2] = (counter >> 16) as u8;
    v[3] = (counter >> 8) as u8;
    v[4] = counter as u8;
    // 插入区 v[6..8] = 0；首包头指针 v[8..10] = 0（包从数据区起点开始）。
    v[10..10 + packet.len()].copy_from_slice(packet);
    for b in v.iter_mut().skip(10 + packet.len()) {
      *b = 0xFF;
    }
    v
  }

  /// 参照实现 build_source_packet 的头：word0 = (0b000_0_1 << 11) | APID。
  fn source_packet(apid: u16, payload_len: usize) -> Vec<u8> {
    let mut p = vec![0u8; 6 + payload_len];
    let word0 = 0x0800 | apid;
    p[0] = (word0 >> 8) as u8;
    p[1] = word0 as u8;
    p[2] = 0xC0; // 序号标志 11 + 计数 0（高 6 位）
    let len = (payload_len as u16 - 1).to_be_bytes();
    p[4] = len[0];
    p[5] = len[1];
    p
  }

  #[test]
  fn walks_packets_across_frames() {
    // 包 1 长 900 字节（跨帧），包 2 长 100 字节紧随其后 —— 与真实复用器一致：
    // 帧 1 的数据区 = 包 1 的尾巴 + 包 2 + 填充（包连续相接，填充只在流尾）。
    let pkt1 = source_packet(64, 900);
    let pkt2 = source_packet(65, 100);
    let mut frame1 = vec![0xFFu8; 882];
    frame1[..24].copy_from_slice(&pkt1[882..]); // 906 - 882 = 24
    frame1[24..24 + pkt2.len()].copy_from_slice(&pkt2);
    let mut mux = MuxStream::new();
    mux.push(&make_vcdu(0, &pkt1[..882]));
    let mut vcdu1 = [0u8; VCDU_DATA];
    vcdu1[..VCDU_HEADER + INSERT_ZONE + MPDU_HEADER]
      .copy_from_slice(&make_vcdu(1, &[])[..VCDU_HEADER + INSERT_ZONE + MPDU_HEADER]);
    vcdu1[VCDU_HEADER + INSERT_ZONE + MPDU_HEADER..].copy_from_slice(&frame1);
    mux.push(&vcdu1);
    assert!(mux.counters_are_consecutive());
    let packets = mux.packets();
    assert_eq!(packets.len(), 2, "两个源包");
    assert_eq!(packets[0].0, 64);
    assert_eq!(packets[0].2.len(), 900);
    assert_eq!(packets[1].0, 65);
    assert_eq!(packets[1].2.len(), 100);
  }

  #[test]
  fn monotonic_counters_tolerate_holes_but_not_reordering() {
    let mut mux = MuxStream::new();
    for c in [0u32, 1, 5, 6] {
      mux.push(&make_vcdu(c, &[]));
    }
    assert!(mux.counters_are_monotonic(), "允许空洞");
    assert!(!mux.counters_are_consecutive(), "空洞不算连续");

    let mut reordered = MuxStream::new();
    for c in [3u32, 2] {
      reordered.push(&make_vcdu(c, &[]));
    }
    assert!(!reordered.counters_are_monotonic(), "乱序必须拒绝");
  }

  #[test]
  fn skips_leading_continuations_until_a_valid_packet_pointer() {
    let packet = source_packet(64, 14);
    let mut continuation = make_vcdu(12, &[]);
    continuation[8..10].copy_from_slice(&NO_PACKET_START.to_be_bytes());
    let mut mux = MuxStream::new();
    mux.push(&continuation);
    assert!(mux.packets().is_empty());
    mux.push(&make_vcdu(13, &packet));
    let packets = mux.packets();
    assert!(mux.counters_are_monotonic());
    assert_eq!(packets.len(), 1);
    assert_eq!(packets[0].0, 64);
    assert_eq!(packets[0].2, packet[6..]);
  }

  #[test]
  fn packet_pointer_reserved_bits_are_not_part_of_the_offset() {
    let packet = source_packet(65, 14);
    let mut frame = make_vcdu(1, &packet);
    frame[8] = 0xF8;
    assert_eq!(parse_header(&frame).first_header_ptr, 0);
    let mut mux = MuxStream::new();
    mux.push(&frame);
    assert_eq!(mux.packets()[0].0, 65);
    frame[8..10].copy_from_slice(&1000u16.to_be_bytes());
    let mut mux = MuxStream::new();
    mux.push(&frame);
    assert!(mux.packets().is_empty());
  }

  #[test]
  fn stops_at_fill_padding() {
    let pkt = source_packet(64, 14);
    let mut mux = MuxStream::new();
    mux.push(&make_vcdu(0, &pkt));
    assert_eq!(mux.packets().len(), 1, "填充帧只出一个包");
  }

  /// 用指定 VCID / 计数 / 首包头指针 / 数据区构造一帧。
  fn frame_with(vcid: u8, counter: u32, fhp: u16, data: &[u8]) -> [u8; VCDU_DATA] {
    let mut v = make_vcdu(counter, &[]);
    v[1] = (v[1] & 0xC0) | vcid;
    v[8..10].copy_from_slice(&fhp.to_be_bytes());
    let body = &mut v[VCDU_HEADER + INSERT_ZONE + MPDU_HEADER..];
    body.fill(0xFF);
    body[..data.len()].copy_from_slice(data);
    v
  }

  #[test]
  fn a_packet_spanning_a_dropped_frame_is_discarded_and_resynced() {
    // 帧 0 里的包 P1（900 字节载荷）本应延续到帧 1，但帧 1 丢了；到手的是帧 2（计数 2）。
    // 旧实现把帧 2 的字节当 P1 的尾巴，拼出一个夹杂别处字节的垃圾包，并吞掉紧随其后的 P2。
    let p1 = source_packet(64, 900);
    let p2 = source_packet(65, 100);
    let mut mux = MuxStream::new();
    mux.push(&frame_with(5, 0, 0, &p1[..882]));
    // 帧 2：开头是 P1 的又一段尾巴（fhp 指向其后的 P2 起点），不能被当成 P1 的延续。
    let mut data = vec![0xAAu8; 40];
    data.extend_from_slice(&p2);
    mux.push(&frame_with(5, 2, 40, &data));
    let packets = mux.packets();
    assert_eq!(packets.len(), 1, "P1 跨丢帧被丢弃，只剩 P2：{packets:?}");
    assert_eq!(packets[0].0, 65);
    assert_eq!(packets[0].2.len(), 100);
  }

  #[test]
  fn idle_other_vc_and_duplicate_frames_are_ignored() {
    let packet = source_packet(64, 14);
    let mut mux = MuxStream::new();
    mux.push(&frame_with(IDLE_VCID, 0, 0, &[]));
    mux.push(&frame_with(5, 10, 0, &packet));
    mux.push(&frame_with(9, 99, 0, &[0x11; 30])); // 另一虚拟信道：计数独立，不能混进来
    mux.push(&frame_with(5, 10, 0, &packet)); // 重复帧
    mux.push(&frame_with(5, 11, NO_PACKET_START, &[]));
    assert_eq!(mux.counters, [10, 11]);
    assert!(mux.counters_are_monotonic() && mux.counters_are_consecutive());
    assert_eq!(mux.packets().len(), 1);
  }

  #[test]
  fn counter_wraparound_is_not_reordering() {
    let mut mux = MuxStream::new();
    for c in [0x00FF_FFFEu32, 0x00FF_FFFF, 0, 1] {
      mux.push(&make_vcdu(c, &[]));
    }
    assert!(mux.counters_are_monotonic(), "24 位回绕是正常前进");
    assert!(!mux.gap_before(2), "0xFFFFFF → 0 连续");
  }
}
