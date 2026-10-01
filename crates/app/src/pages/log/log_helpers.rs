pub(super) const CELL: &str = "border px-3 py-2 text-left align-top";
pub(super) const PAGE_SIZE: usize = 50;
pub(super) const PROP_MODES: &[(&str, &str)] = &[
  ("", "—"),
  ("SAT", "卫星 SAT"),
  ("EME", "月面反射 EME"),
  ("ES", "E 层突发 ES"),
  ("TR", "对流层 TR"),
  ("MS", "流星余迹 MS"),
  ("AUR", "极光 AUR"),
  ("RPT", "中继 RPT"),
  ("ION", "电离层 ION"),
  ("INTERNET", "网络 INTERNET"),
];

/// 方位角 → 八方位中文。
pub(super) fn compass(deg: f64) -> &'static str {
  const NAMES: [&str; 8] = ["北", "东北", "东", "东南", "南", "西南", "西", "西北"];
  NAMES[((deg.rem_euclid(360.0) + 22.5) / 45.0) as usize % 8]
}

pub(super) fn confirm(msg: &str) -> bool {
  crate::util::window()
    .confirm_with_message(msg)
    .unwrap_or(false)
}
