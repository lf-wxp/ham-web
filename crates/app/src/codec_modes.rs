//! MFSK / THOR / DominoEX 的子模式名单：解码页与 `/tools#mode-encoder` 的发端共用这一份。
//!
//! 名单放在这里而不是从核心 crate 读：三个核心 crate 都是 GPL-3.0，主包不能依赖它们
//! （`docs/engineering/licensing.md`，`check-structure` 的「许可边界」守着），所以主包要自己持有
//! 「值 → 标签」表。值必须是核心 crate 的 `parse_mode` 认得的名字；两边漂移时，Worker 对不认识的
//! 名字回 `unsupported-submode:<名>`，`e2e/tests/decode_worker_protocol.spec.ts` 把名单逐个送进
//! Worker 钉死。条数（11 / 15 / 6）也在下面的单测里钉死，与那份 e2e 的名单对应。
//!
//! 每张表第一项是默认（也是最常见的）模式。

/// MFSK 子模式：`(值, 标签)`。
pub const MFSK_MODES: &[(&str, &str)] = &[
  ("16", "MFSK16"),
  ("8", "MFSK8"),
  ("4", "MFSK4"),
  ("11", "MFSK11"),
  ("22", "MFSK22"),
  ("31", "MFSK31"),
  ("32", "MFSK32"),
  ("64", "MFSK64"),
  ("128", "MFSK128"),
  ("64L", "MFSK64L"),
  ("128L", "MFSK128L"),
];

/// THOR 子模式：`(值, 标签)`。
pub const THOR_MODES: &[(&str, &str)] = &[
  ("16", "THOR 16"),
  ("8", "THOR 8"),
  ("4", "THOR 4"),
  ("micro", "THOR micro"),
  ("5", "THOR 5"),
  ("11", "THOR 11"),
  ("22", "THOR 22"),
  ("25", "THOR 25"),
  ("32", "THOR 32"),
  ("44", "THOR 44"),
  ("56", "THOR 56"),
  ("25x4", "THOR 25x4"),
  ("50x1", "THOR 50x1"),
  ("50x2", "THOR 50x2"),
  ("100", "THOR 100"),
];

/// DominoEX 子模式：`(值, 标签)`。
pub const DOMINOEX_MODES: &[(&str, &str)] = &[
  ("8", "DominoEX 8"),
  ("16", "DominoEX 16"),
  ("11", "DominoEX 11"),
  ("4", "DominoEX 4"),
  ("22", "DominoEX 22"),
  ("5", "DominoEX 5"),
];

/// 有图片模式的 MFSK 子模式（值）。
pub const MFSK_PICTURE_MODES: &[&str] = &["16", "32", "64", "128"];

/// 有图片模式的 THOR 子模式（值）。
pub const THOR_PICTURE_MODES: &[&str] = &["11", "16", "22", "25", "32", "44", "56"];

/// THOR 图片模式认得的六种尺寸 `(宽, 高)`；默认（第一项）是 160 × 120。
///
/// 与 `ham_web_thor::PICTURE_SIZES` 一一对应（主包不能依赖那个 GPL crate，所以这里自己持有一份）：
/// 尺寸对不上时 Worker 回 `encode-params:picture`，`decode_worker_protocol.spec.ts` 把名单逐个送进去。
pub const THOR_PICTURE_SIZES: &[(usize, usize)] = &[
  (160, 120),
  (320, 240),
  (59, 74),
  (120, 150),
  (240, 300),
  (640, 480),
];

/// MFSK 图片的「最大边长」档位（像素）；默认（第一项）是 160。
pub const MFSK_PICTURE_EDGES: &[usize] = &[160, 80, 240, 320];

/// 名单 → 下拉选项。
pub fn select_options(modes: &[(&str, &str)]) -> Vec<crate::ui::SelectOption> {
  modes
    .iter()
    .map(|(value, label)| crate::ui::SelectOption::new(*value, (*label).to_owned()))
    .collect()
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn tables_have_the_documented_sizes_and_unique_values() {
    for (name, table, len) in [
      ("MFSK", MFSK_MODES, 11),
      ("THOR", THOR_MODES, 15),
      ("DominoEX", DOMINOEX_MODES, 6),
    ] {
      assert_eq!(table.len(), len, "{name}");
      let mut values: Vec<&str> = table.iter().map(|(v, _)| *v).collect();
      values.sort_unstable();
      values.dedup();
      assert_eq!(values.len(), len, "{name}: 值重复");
      // 标签必须带编解码器名，下拉里才分得清是哪个模式族。
      assert!(table.iter().all(|(_, l)| l.contains(name)), "{name}");
    }
    // 图片模式名单都是各自表里的子集，条数与核心 crate 的 `has_picture` 一致。
    for (name, table, pics, len) in [
      ("MFSK", MFSK_MODES, MFSK_PICTURE_MODES, 4),
      ("THOR", THOR_MODES, THOR_PICTURE_MODES, 7),
    ] {
      assert_eq!(pics.len(), len, "{name}");
      assert!(
        pics.iter().all(|p| table.iter().any(|(v, _)| v == p)),
        "{name}"
      );
    }
    assert_eq!(THOR_PICTURE_SIZES.len(), 6);
    // 各表的默认项与 Worker 里缺省 `mode` 的取值一致（MFSK / THOR 是 "16"，DominoEX 是 "8"）。
    assert_eq!(MFSK_MODES[0].0, "16");
    assert_eq!(THOR_MODES[0].0, "16");
    assert_eq!(DOMINOEX_MODES[0].0, "8");
  }
}
