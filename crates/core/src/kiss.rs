//! KISS（TNC ↔ 主机串行封装）与 AX.25 UI 帧：编解码内核。
//!
//! KISS 是 Packet Radio 里「软件替代硬件 TNC」的封装：0xC0 定界、0xDB 转义，
//! 主机侧软件（如本站的 `/kiss-tnc` 页面）经串口收发 AX.25 帧。
//!
//! # FCS 不在这里
//!
//! KISS 载荷是**不带 FCS** 的 AX.25 帧：HDLC 层（位填充、FCS）由 TNC 负责，
//! 主机给的帧若多带 2 字节，TNC 会把它们当信息字段发出去；TNC 上送的帧同样没有
//! FCS，所以解析侧也不校验。
//!
//! # 与真实设备的互操作
//!
//! 编码规则、命令字节（高 nibble = 端口，低 nibble = 命令，0 = 数据）与 AX.25 地址
//! 位序按公开规范实现，单测用一条手算的字节序列钉住（不依赖本实现自己再算一遍）；
//! 与具体 TNC / Direwolf 的互操作属于**硬件验证**。

/// 帧结束定界。
pub const FEND: u8 = 0xC0;
/// 转义前缀。
pub const FESC: u8 = 0xDB;
/// FEND 的转义形式。
pub const TFEND: u8 = 0xDC;
/// FESC 的转义形式。
pub const TFESC: u8 = 0xDD;

/// 单帧负载上限（KISS 常见实现是 2048；超长视为损坏丢弃）。
pub const MAX_FRAME: usize = 2048;

/// AX.25 地址字段总数上限：目的 + 源 + 8 个中继。
const MAX_ADDRESSES: usize = 10;
/// 最多 8 个中继。
pub const MAX_DIGIPEATERS: usize = MAX_ADDRESSES - 2;
/// UI 帧固定部分：目的 + 源（各 7 字节）+ 控制 + PID。
const UI_FIXED_LEN: usize = 7 + 7 + 2;

/// KISS 帧 → 字节流。命令字节 = `端口 << 4 | 0`（0 = 数据帧）。
///
/// 端口按协议只有 4 位：`port` 的高位会被丢弃（`encode_frame(16, ..)` 等同于端口 0），
/// 调用方不要依赖 > 15 的取值。
#[must_use]
pub fn encode_frame(port: u8, payload: &[u8]) -> Vec<u8> {
  let mut out = Vec::with_capacity(payload.len() + 4);
  out.push(FEND);
  // 命令字节与负载走同一套转义：端口 12 的命令字节恰好是 0xC0（= FEND）。
  for &b in std::iter::once(&((port & 0x0F) << 4)).chain(payload) {
    match b {
      FEND => {
        out.push(FESC);
        out.push(TFEND);
      }
      FESC => {
        out.push(FESC);
        out.push(TFESC);
      }
      _ => out.push(b),
    }
  }
  out.push(FEND);
  out
}

/// 流式 KISS 解码器：任意分块喂入，吐出完整**数据帧**的负载（不含命令字节）。
/// 非数据命令（TXDELAY、P 等参数帧）被丢弃。
#[derive(Debug, Default)]
pub struct KissDecoder {
  /// 当前帧的原始字节（含命令字节）。
  buf: Vec<u8>,
  /// 上一字节是 FESC。
  escape: bool,
  /// 已见过 FEND（帧内）。
  in_frame: bool,
}

impl KissDecoder {
  #[must_use]
  pub fn new() -> Self {
    Self::default()
  }

  /// 喂入一段字节，返回这段里完成的帧（负载，已去命令字节与转义）。
  pub fn feed(&mut self, bytes: &[u8]) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    for &b in bytes {
      if self.escape {
        self.escape = false;
        match b {
          TFEND => self.push(FEND),
          TFESC => self.push(FESC),
          // FESC 后直接跟 FEND：当前帧已损坏，但这个 FEND 仍是下一帧的起点，
          // 吞掉它会连带丢掉下一帧。
          FEND => {
            self.buf.clear();
            self.in_frame = true;
          }
          // 其余非法转义序列：丢弃当前帧，等下一个 FEND。
          _ => self.drop_frame(),
        }
        continue;
      }
      match b {
        FEND => {
          if self.in_frame
            && let Some(payload) = self.data_payload()
          {
            out.push(payload.to_vec());
          }
          self.buf.clear();
          self.in_frame = true;
        }
        FESC => self.escape = true,
        _ => self.push(b),
      }
    }
    out
  }

  /// 丢弃当前半帧（换串口 / 清屏时用）。
  pub fn reset(&mut self) {
    self.drop_frame();
    self.escape = false;
  }

  fn drop_frame(&mut self) {
    self.buf.clear();
    self.in_frame = false;
  }

  /// 帧内追加一字节：普通字节与转义还原后的字节都走这里，上限只在这一处检查
  /// （否则持续喂 `FESC TFEND` 能绕过上限无限增长）。
  fn push(&mut self, b: u8) {
    if !self.in_frame {
      return;
    }
    // buf 含 1 字节命令头，负载上限 MAX_FRAME 对应 buf 上限 MAX_FRAME + 1：
    // 超长按损坏处理，丢弃并等下一个 FEND。
    if self.buf.len() > MAX_FRAME {
      self.drop_frame();
      return;
    }
    self.buf.push(b);
  }

  /// 当前帧是非空数据帧时返回负载。
  ///
  /// 零长度数据帧（只有命令字节）不产出空行：串口上它没有可显示的内容。
  fn data_payload(&self) -> Option<&[u8]> {
    let (&cmd, payload) = self.buf.split_first()?;
    // 低 nibble 是命令，0 = 数据；高 nibble 是端口，这里不区分。
    (cmd & 0x0F == 0 && !payload.is_empty()).then_some(payload)
  }
}

/// AX.25 地址字段编码（7 字节）：呼号 6 字符左移 1 位，SSID 字节 =
/// `0b0110_0000 | (ssid << 1) | (last ? 1 : 0)`（末地址 E 位置 1）。
///
/// **如实说明**：bit 7（命令 / 响应指示 C 位）恒为 0，bit 5 / 6（保留位）恒为 1。UI 帧是无连接
/// 帧，没有「命令 / 响应」之分，这里没有对照规范原文核实「目的地址是否应置 C」 —— 本实现与
/// 测试里独立写的 JS 实现一致，但那只证明两者彼此一致，不是对规范的验证。接收侧
/// （[`decode_address`]）不看这一位，所以互操作上没有已知问题；若要严格贴合 AX.25 v2.0 的习惯
/// （目的地址 C = 1、源地址 C = 0），改这里并同步 e2e 的 JS 实现即可。
#[must_use]
pub fn encode_address(call: &str, ssid: u8, last: bool) -> Option<[u8; 7]> {
  let call = call.trim().to_ascii_uppercase();
  // SSID 只有 4 位：超范围直接拒绝，不静默取低 4 位（`ssid = 99` 会悄悄变成 3）。
  if ssid > 15
    || call.is_empty()
    || call.len() > 6
    || !call.chars().all(|c| c.is_ascii_alphanumeric())
  {
    return None;
  }
  let mut out = [0x20u8 << 1; 7]; // 空格左移一位的填充
  for (i, c) in call.bytes().enumerate() {
    out[i] = c << 1;
  }
  out[6] = 0x60 | ((ssid & 0x0F) << 1) | u8::from(last);
  Some(out)
}

/// 解析 7 字节地址字段 → `(呼号, SSID)`。
///
/// 刻意宽松：不校验 SSID 字节的保留位（bit 5 / 6，规范建议为 `11`）。旧版 APRS 软件与部分
/// TNC 把它们写成 0，严格拒绝会丢掉真实可用的帧；接收侧也不依据这两位做任何决定。
#[must_use]
pub fn decode_address(bytes: &[u8]) -> Option<(String, u8)> {
  if bytes.len() < 7 {
    return None;
  }
  let mut call = String::new();
  for &b in &bytes[..6] {
    let c = (b >> 1) as char;
    if c == ' ' {
      break;
    }
    if !c.is_ascii_alphanumeric() {
      return None;
    }
    call.push(c);
  }
  if call.is_empty() {
    return None;
  }
  let ssid = (bytes[6] >> 1) & 0x0F;
  Some((call, ssid))
}

/// `"BG1AAA-7"` → `("BG1AAA", 7)`；无 `-SSID` 后缀 → SSID 0。呼号非法返回 `None`。
///
/// SSID 只收 1–2 位纯数字且 ≤ 15（`str::parse` 会放过 `+7`，所以单独校验）。
#[must_use]
pub fn split_ssid(call: &str) -> Option<(&str, u8)> {
  if let Some((base, ssid)) = call.rsplit_once('-') {
    if base.is_empty()
      || ssid.is_empty()
      || ssid.len() > 2
      || !ssid.bytes().all(|b| b.is_ascii_digit())
    {
      return None;
    }
    let ssid = ssid.parse::<u8>().ok()?;
    (ssid <= 15).then_some((base, ssid))
  } else {
    Some((call, 0))
  }
}

/// 构建 AX.25 UI 帧（无连接、无重传、**不含 FCS**）：dest + src + digis + 0x03（UI）+
/// 0xF0（无 L3）+ info。呼号可带 `-SSID` 后缀。
///
/// 中继超过 [`MAX_DIGIPEATERS`] 个、或整帧超过 [`MAX_FRAME`] 返回 `None` —— 否则会
/// 生成自己的解码器都会丢弃的帧。
#[must_use]
pub fn ax25_ui_frame(dest: &str, src: &str, digis: &[&str], info: &[u8]) -> Option<Vec<u8>> {
  if digis.len() > MAX_DIGIPEATERS || UI_FIXED_LEN + 7 * digis.len() + info.len() > MAX_FRAME {
    return None;
  }
  let mut frame = Vec::with_capacity(UI_FIXED_LEN + 7 * digis.len() + info.len());
  let last_index = digis.len();
  let (d_base, d_ssid) = split_ssid(dest)?;
  let (s_base, s_ssid) = split_ssid(src)?;
  // dest 永远是第一个地址（E 位不置）；只有**最后一个**地址（无中继时是源呼号）
  // 才带 E 位 —— 这是 AX.25 地址字段的定界方式。
  frame.extend_from_slice(&encode_address(d_base, d_ssid, false)?);
  frame.extend_from_slice(&encode_address(s_base, s_ssid, last_index == 0)?);
  for (i, d) in digis.iter().enumerate() {
    let (base, ssid) = split_ssid(d)?;
    frame.extend_from_slice(&encode_address(base, ssid, i + 1 == last_index)?);
  }
  frame.push(0x03); // UI
  frame.push(0xF0); // No Layer 3
  frame.extend_from_slice(info);
  Some(frame)
}

/// 解析出的 UI 帧。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiFrame {
  /// 源呼号（不含 SSID）。
  pub source: String,
  /// 目的呼号。
  pub destination: String,
  /// 信息字段。
  pub info: Vec<u8>,
}

/// 解析 AX.25 UI 帧（KISS 载荷，不含 FCS；支持 0..8 个中继地址；返回 `None` 表示结构不合法）。
///
/// 控制字节要求**恰好**是 `0x03`（UI 帧、P/F 位为 0）：带 P/F 位的 `0x13` 在部分 TNC /
/// APRS 实现里会出现，这里判为结构非法并退回十六进制显示（有意为之：不猜它的语义），
/// 界面因此不会把这类帧解析成呼号 + 文本。
#[must_use]
pub fn parse_ui_frame(frame: &[u8]) -> Option<UiFrame> {
  if frame.len() < UI_FIXED_LEN {
    return None;
  }
  let dest = decode_address(&frame[0..7])?.0;
  if frame[6] & 1 != 0 {
    return None;
  }
  let source = decode_address(&frame[7..14])?.0;
  let mut offset = 7usize;
  // 源地址固定为第二个；E 位只标识地址链结束，末地址可能是中继。
  loop {
    if offset + 7 > frame.len() {
      return None;
    }
    decode_address(&frame[offset..offset + 7])?;
    let last = frame[offset + 6] & 0x01 != 0;
    offset += 7;
    if !last {
      // 最多 10 个地址（AX.25 上限：dest + 源 + 8 个中继）。offset 已指向
      // 下一个地址，读满 10 个后仍非末地址即为结构非法。
      if offset >= 7 * MAX_ADDRESSES {
        return None;
      }
      continue;
    }
    if offset + 2 > frame.len() {
      return None;
    }
    if frame[offset] != 0x03 || frame[offset + 1] != 0xF0 {
      return None;
    }
    return Some(UiFrame {
      source,
      destination: dest,
      info: frame[offset + 2..].to_vec(),
    });
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn out_of_range_ssid_is_rejected_not_truncated() {
    assert!(encode_address("BG1AAA", 15, true).is_some());
    assert_eq!(encode_address("BG1AAA", 16, true), None);
    assert_eq!(encode_address("BG1AAA", 99, true), None);
  }

  #[test]
  fn kiss_round_trips_with_escapes() {
    let payload = [FEND, 0x01, FESC, 0xAB, 0x02];
    let bytes = encode_frame(0, &payload);
    assert_eq!(bytes[0], FEND);
    assert_eq!(bytes[1], 0x00); // 命令字节：端口 0、数据帧
    let mut dec = KissDecoder::new();
    let frames = dec.feed(&bytes);
    assert_eq!(frames.len(), 1);
    assert_eq!(frames[0], payload);
  }

  #[test]
  fn command_byte_is_escaped_like_payload() {
    // 端口 12 的命令字节是 0xC0：不转义会被当成帧结束。
    let bytes = encode_frame(12, b"x");
    assert_eq!(&bytes[..4], &[FEND, FESC, TFEND, b'x']);
    let mut dec = KissDecoder::new();
    assert_eq!(dec.feed(&bytes), vec![b"x".to_vec()]);
  }

  #[test]
  fn port_goes_into_the_high_nibble() {
    // 规范：高 nibble = 端口，低 nibble = 命令。端口 3 的数据帧命令字节是 0x30。
    assert_eq!(encode_frame(3, b"x")[1], 0x30);
    // 端口只有 4 位：高位被丢弃（文档已写明，不能让调用方以为它被保留）。
    assert_eq!(encode_frame(0x13, b"x")[1], 0x30);
    let mut dec = KissDecoder::new();
    assert_eq!(dec.feed(&encode_frame(3, b"x")), vec![b"x".to_vec()]);
  }

  #[test]
  fn non_data_commands_are_ignored() {
    // 0x01 = TXDELAY 参数命令：不是数据帧，不该当载荷上送。
    let mut dec = KissDecoder::new();
    assert!(dec.feed(&[FEND, 0x01, 0x28, FEND]).is_empty());
    // 紧跟的数据帧不受影响。
    assert_eq!(dec.feed(&encode_frame(0, b"ok")), vec![b"ok".to_vec()]);
  }

  #[test]
  fn decoder_handles_arbitrary_chunking() {
    let payload: Vec<u8> = (0..64).map(|i| i as u8).collect();
    let bytes = encode_frame(0, &payload);
    let mut dec = KissDecoder::new();
    let mut got = Vec::new();
    // 1 字节一块喂
    for &b in &bytes {
      got.extend(dec.feed(&[b]));
    }
    assert_eq!(got.len(), 1);
    assert_eq!(got[0], payload);
  }

  #[test]
  fn decoder_splits_multiple_frames_and_drops_junk() {
    let a = encode_frame(0, b"hello");
    let b = encode_frame(3, b"world");
    let mut stream = a.clone();
    stream.extend_from_slice(&b);
    // 中间夹一段损坏序列：FESC 后跟非法字节。
    stream.insert(a.len(), FESC);
    stream.insert(a.len() + 1, 0x00);
    let mut dec = KissDecoder::new();
    let frames = dec.feed(&stream);
    // 「hello」完成，「坏帧」丢弃，接着的「world」收下。
    assert_eq!(frames.len(), 2);
    assert_eq!(frames[0], b"hello");
    assert_eq!(frames[1], b"world");
  }

  #[test]
  fn fesc_followed_by_fend_keeps_the_next_frame() {
    // 损坏帧以「FESC FEND」收尾：FEND 仍是下一帧的起点，下一帧不能被吞。
    let mut stream = vec![FEND, 0x00, b'a', FESC, FEND];
    stream.extend_from_slice(&[0x00, b'o', b'k', FEND]);
    let mut dec = KissDecoder::new();
    assert_eq!(dec.feed(&stream), vec![b"ok".to_vec()]);
  }

  #[test]
  fn truncated_frame_waits_for_fend() {
    let bytes = encode_frame(0, b"abc");
    let mut dec = KissDecoder::new();
    assert!(dec.feed(&bytes[..5]).is_empty());
    let frames = dec.feed(&bytes[5..]);
    assert_eq!(frames.len(), 1);
    assert_eq!(frames[0], b"abc");
  }

  #[test]
  fn frame_size_limit_is_exact() {
    let mut dec = KissDecoder::new();
    let at_limit = [0x41u8; MAX_FRAME];
    assert_eq!(
      dec.feed(&encode_frame(0, &at_limit)).len(),
      1,
      "恰好上限应收下"
    );
    let over = [0x41u8; MAX_FRAME + 1];
    assert!(
      dec.feed(&encode_frame(0, &over)).is_empty(),
      "超 1 字节即丢弃"
    );
  }

  #[test]
  fn escaped_bytes_cannot_bypass_the_size_limit() {
    // 持续喂 `FESC TFEND`：每对还原成 1 字节，必须和普通字节一样受上限约束。
    let mut stream = vec![FEND, 0x00];
    for _ in 0..(MAX_FRAME * 2) {
      stream.extend_from_slice(&[FESC, TFEND]);
    }
    let mut dec = KissDecoder::new();
    assert!(dec.feed(&stream).is_empty());
    assert!(dec.buf.len() <= MAX_FRAME + 1, "缓冲不得越过上限");
    // 以 FEND 收尾后解码器仍可继续工作。
    stream = encode_frame(0, b"ok");
    assert_eq!(dec.feed(&stream), vec![b"ok".to_vec()]);
  }

  #[test]
  fn ax25_address_bit_layout_is_pinned() {
    // 'B' = 0x42 → 0x84；'G' = 0x47 → 0x8E；SSID 0、非末地址 → 0x60。
    let addr = encode_address("BG1AAA", 0, false).unwrap();
    assert_eq!(addr[0], 0x84);
    assert_eq!(addr[1], 0x8E);
    assert_eq!(addr[6], 0x60);
    // 末地址 E 位置 1；SSID 3 进 nibble。
    let last = encode_address("APRS", 3, true).unwrap();
    assert_eq!(last[6], 0x60 | (3 << 1) | 1);
    // 往返。
    assert_eq!(decode_address(&addr), Some(("BG1AAA".to_owned(), 0)));
    assert_eq!(decode_address(&last), Some(("APRS".to_owned(), 3)));
    // 呼号后缀拆分。
    assert_eq!(split_ssid("BG1AAA-7"), Some(("BG1AAA", 7)));
    assert_eq!(split_ssid("APRS"), Some(("APRS", 0)));
    assert!(split_ssid("BG1AAA-99").is_none());
    assert!(split_ssid("BG1AAA-+7").is_none(), "`+7` 不是合法 SSID");
    assert!(split_ssid("BG1AAA-x").is_none());
  }

  #[test]
  fn ui_frame_matches_hand_computed_bytes() {
    // 手算的字节序列（与本实现的任何代码路径无关）：
    // 目的 "APRS  "：0x41/0x50/0x52/0x53 各左移 1 位 = 82 A0 A4 A6，空格 = 40 40，SSID 字节 60；
    // 源 "BG1AAA-7"：84 8E 62 82 82 82，SSID 7 且为末地址 = 0x60|(7<<1)|1 = 6F；
    // 控制 03、PID F0，信息 ">hello"。
    let expected: Vec<u8> = [
      &[0x82, 0xA0, 0xA4, 0xA6, 0x40, 0x40, 0x60][..],
      &[0x84, 0x8E, 0x62, 0x82, 0x82, 0x82, 0x6F],
      &[0x03, 0xF0],
      b">hello",
    ]
    .concat();
    assert_eq!(
      ax25_ui_frame("APRS", "BG1AAA-7", &[], b">hello").unwrap(),
      expected
    );
  }

  #[test]
  fn ui_frame_round_trips() {
    // 无中继：dest + src + 控制 2 字节 + info，不带 FCS。
    let frame = ax25_ui_frame("APRS", "BG1AAA-7", &[], b">test").unwrap();
    assert_eq!(frame.len(), 7 + 7 + 2 + 5);
    // dest 是第一个地址，E 位不置（0x60）；src 是最后一个地址，E 位置 1。
    assert_eq!(frame[6], 0x60, "dest 不应带 E 位");
    assert_eq!(frame[13], 0x60 | (7 << 1) | 1, "src 带 SSID 7 与 E 位");
    let parsed = parse_ui_frame(&frame).expect("结构合法");
    assert_eq!(parsed.destination, "APRS");
    assert_eq!(parsed.source, "BG1AAA");
    assert_eq!(parsed.info, b">test");
    // 带中继也一样能解回。
    let frame2 = ax25_ui_frame("APRS", "BG1AAA", &["WIDE1-1"], b"hi").unwrap();
    let parsed2 = parse_ui_frame(&frame2).expect("带中继");
    assert_eq!(parsed2.source, "BG1AAA");
    assert_eq!(parsed2.destination, "APRS");
    assert_eq!(parsed2.info, b"hi");
    let repeaters = ["WIDE1-1"; MAX_DIGIPEATERS];
    let full = ax25_ui_frame("APRS", "BG1AAA-7", &repeaters, b"eight").unwrap();
    let parsed = parse_ui_frame(&full).unwrap();
    assert_eq!(parsed.source, "BG1AAA");
    assert_eq!(parsed.destination, "APRS");
    assert_eq!(parsed.info, b"eight");
    // 结构非法：截断、控制字节不是 UI。
    assert!(parse_ui_frame(&frame[..10]).is_none());
    let mut not_ui = frame.clone();
    not_ui[14] = 0x13;
    assert!(parse_ui_frame(&not_ui).is_none());
  }

  #[test]
  fn more_than_ten_addresses_is_structurally_invalid() {
    // 构造器最多产出 10 个地址，所以这条路径只能手工造：11 个地址且都不是末地址。
    let mut frame = Vec::new();
    frame.extend_from_slice(&encode_address("APRS", 0, false).expect("dest"));
    for _ in 0..10 {
      frame.extend_from_slice(&encode_address("BG1AAA", 1, false).expect("hop"));
    }
    frame.extend_from_slice(&[0x03, 0xF0]);
    assert!(parse_ui_frame(&frame).is_none(), "超过 10 个地址必须拒绝");
  }

  #[test]
  fn ui_frame_builder_enforces_limits() {
    let eight = ["A1"; MAX_DIGIPEATERS];
    assert!(ax25_ui_frame("APRS", "BG1AAA", &eight, b"x").is_some());
    let nine = ["A1"; MAX_DIGIPEATERS + 1];
    assert!(
      ax25_ui_frame("APRS", "BG1AAA", &nine, b"x").is_none(),
      "超过 8 个中继"
    );
    const ROOM: usize = MAX_FRAME - UI_FIXED_LEN;
    assert!(ax25_ui_frame("APRS", "BG1AAA", &[], &[b'a'; ROOM]).is_some());
    assert!(
      ax25_ui_frame("APRS", "BG1AAA", &[], &[b'a'; ROOM + 1]).is_none(),
      "整帧超过 MAX_FRAME"
    );
  }
}
