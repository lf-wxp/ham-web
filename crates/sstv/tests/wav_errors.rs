//! WAV 层的错误必须保留各自的稳定错误码：此前 `decode` 把所有 WAV 错误都压成 `NotWav`，
//! 页面只能提示「不是有效的 WAV」，用户分不清是格式不支持、数据被截断还是文件里没有音频。

fn wav_header(data_len: u32, declared_len: u32) -> Vec<u8> {
  let mut b = Vec::new();
  b.extend_from_slice(b"RIFF");
  b.extend_from_slice(&(36 + declared_len).to_le_bytes());
  b.extend_from_slice(b"WAVEfmt ");
  b.extend_from_slice(&16u32.to_le_bytes());
  b.extend_from_slice(&[1, 0, 1, 0]); // PCM / 单声道
  b.extend_from_slice(&12_000u32.to_le_bytes());
  b.extend_from_slice(&24_000u32.to_le_bytes());
  b.extend_from_slice(&[2, 0, 16, 0]);
  b.extend_from_slice(b"data");
  b.extend_from_slice(&declared_len.to_le_bytes());
  b.extend(std::iter::repeat_n(0u8, data_len as usize));
  b
}

#[test]
fn not_a_wav_keeps_not_wav() {
  assert_eq!(
    ham_web_sstv::decode(b"definitely not a wav")
      .unwrap_err()
      .code(),
    "not-wav"
  );
}

#[test]
fn empty_data_chunk_is_no_audio_not_not_wav() {
  let code = ham_web_sstv::decode(&wav_header(0, 0)).unwrap_err().code();
  assert_eq!(code, "no-audio");
}

#[test]
fn too_short_audio_is_not_reported_as_not_wav() {
  let code = ham_web_sstv::decode(&wav_header(200, 200))
    .unwrap_err()
    .code();
  assert_ne!(code, "not-wav", "合法 WAV 不能被报成不是 WAV");
}
