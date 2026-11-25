/// field（20°×10°）热力：按密度分档的填充类。
pub(super) fn field_fill_class(n: usize, max: usize) -> &'static str {
  let r = n as f64 / max.max(1) as f64;
  if r <= 0.25 {
    "fill-primary/15"
  } else if r <= 0.5 {
    "fill-primary/25"
  } else if r <= 0.75 {
    "fill-primary/40"
  } else {
    "fill-primary/60"
  }
}

/// square（2°×1°）热力：更细粒度分档。
pub(super) fn square_fill_class(n: usize, max: usize) -> &'static str {
  let r = n as f64 / max.max(1) as f64;
  if r <= 0.2 {
    "fill-primary/30"
  } else if r <= 0.4 {
    "fill-primary/50"
  } else if r <= 0.6 {
    "fill-primary/70"
  } else if r <= 0.8 {
    "fill-primary/85"
  } else {
    "fill-primary"
  }
}

/// square 已全部确认（QSL_RCVD）时的热力：emerald 色系分档，与未确认的 primary 区分。
pub(super) fn square_fill_class_confirmed(n: usize, max: usize) -> &'static str {
  let r = n as f64 / max.max(1) as f64;
  if r <= 0.2 {
    "fill-win/30"
  } else if r <= 0.4 {
    "fill-win/50"
  } else if r <= 0.6 {
    "fill-win/70"
  } else if r <= 0.8 {
    "fill-win/85"
  } else {
    "fill-win/70"
  }
}

/// square 处于「进行中」状态（部分确认或已寄出未确认）时的热力：amber 色系分档。
pub(super) fn square_fill_class_sent(n: usize, max: usize) -> &'static str {
  let r = n as f64 / max.max(1) as f64;
  if r <= 0.2 {
    "fill-gold/30"
  } else if r <= 0.4 {
    "fill-gold/50"
  } else if r <= 0.6 {
    "fill-gold/70"
  } else if r <= 0.8 {
    "fill-gold/85"
  } else {
    "fill-gold/70"
  }
}

/// 波段 → 通联路径与图例的颜色。
///
/// 已知波段取 core 的数据身份色（`log_stats::band_color`，不随配色方案变，见 `docs/ui/pixel.md` §2）；
/// 未知波段走中性的语义 token，不和任何真实波段撞色。
pub(super) fn band_color(band: &str) -> String {
  ham_web_core::log_stats::band_color(band).unwrap_or_else(|| "var(--muted-foreground)".to_owned())
}
