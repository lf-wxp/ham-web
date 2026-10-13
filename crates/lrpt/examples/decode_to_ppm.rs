//! 把 CADU 流解码成图像并写 PPM（P6）—— 视觉验证用。
use ham_web_lrpt::{decode_image, prand::PRAND};
fn main() {
  let mut args = std::env::args().skip(1);
  let in_path = args
    .next()
    .unwrap_or_else(|| "crates/lrpt/samples/lrpt_encoder_cadus.bin".into());
  let out_path = args
    .next()
    .unwrap_or_else(|| "/tmp/lrpt_decoded.ppm".into());
  let bytes = std::fs::read(&in_path).unwrap();
  let frames: Vec<[u8; 1020]> = bytes
    .chunks(1024)
    .filter(|c| c.len() == 1024)
    .map(|c| {
      let mut d = [0u8; 1020];
      d.copy_from_slice(&c[4..]);
      for (i, b) in d.iter_mut().enumerate() {
        *b ^= PRAND[i % PRAND.len()];
      }
      d
    })
    .collect();
  let image = decode_image(&frames).expect("整链解码");
  let mut ppm = Vec::with_capacity(image.height * image.width * 3 + 32);
  ppm.extend_from_slice(format!("P6\n{} {}\n255\n", image.width, image.height).as_bytes());
  for y in 0..image.height {
    for x in 0..image.width {
      for ch in 0..3 {
        ppm.push(image.channels[ch][y][x]);
      }
    }
  }
  std::fs::write(&out_path, &ppm).unwrap();
  println!("已写 {out_path}（{}×{}）", image.width, image.height);
}
