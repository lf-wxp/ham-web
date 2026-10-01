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
    "fill-emerald-500/30"
  } else if r <= 0.4 {
    "fill-emerald-500/50"
  } else if r <= 0.6 {
    "fill-emerald-500/70"
  } else if r <= 0.8 {
    "fill-emerald-500/85"
  } else {
    "fill-emerald-500"
  }
}

/// square 处于「进行中」状态（部分确认或已寄出未确认）时的热力：amber 色系分档。
pub(super) fn square_fill_class_sent(n: usize, max: usize) -> &'static str {
  let r = n as f64 / max.max(1) as f64;
  if r <= 0.2 {
    "fill-amber-500/30"
  } else if r <= 0.4 {
    "fill-amber-500/50"
  } else if r <= 0.6 {
    "fill-amber-500/70"
  } else if r <= 0.8 {
    "fill-amber-500/85"
  } else {
    "fill-amber-500"
  }
}

/// 波段 → 路径颜色（低频暖色 → 高频冷色）。
pub(super) fn band_color(band: &str) -> &'static str {
  match band {
    "160m" => "#b91c1c",
    "80m" => "#ea580c",
    "60m" | "40m" => "#f59e0b",
    "30m" => "#84cc16",
    "20m" => "#22c55e",
    "17m" => "#14b8a6",
    "15m" => "#06b6d4",
    "12m" => "#3b82f6",
    "10m" => "#6366f1",
    "6m" => "#a855f7",
    "2m" | "70cm" => "#ec4899",
    _ => "#94a3b8",
  }
}
