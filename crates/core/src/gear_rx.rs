//! 接收机实测指标：互调动态范围与噪声底，数据来自 Rob Sherwood 的 Receiver Test Data 表。
//!
//! 数据文件是 `data/gear-rx.txt`，文件头写明了来源、核对日期与**取数规则**（前放状态、
//! 一格多值、同型号多样本、相位噪声受限的脚注）。本模块只做两件事：解析、按机型查。
//!
//! 为什么单独一层而不是塞进 [`crate::gear`]：器材库那层是「公开且稳定的高层次信息」，
//! 而这里是**有出处、有测量条件、会随批次变化**的实测值 —— 两者的时效性要求不同，
//! 分开才能各自带自己的出处与更新日期（P3 的「知识溯源」要的正是这个）。

use std::sync::LazyLock;

/// 内嵌的数据文件（含来源与取数规则）。
const DATA: &str = include_str!("../data/gear-rx.txt");

/// 一台机型的接收机实测指标。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GearRx {
  /// 器材 id（[`crate::gear::Gear::id`]）。
  pub id: &'static str,
  /// 型号（照抄原表的写法，便于与原始表逐行对照）。
  pub model: &'static str,
  /// 20 kHz 间隔的三阶互调动态范围（dB，越大越好）。
  pub imd_wide_db: f64,
  /// 窄间隔（通常 2 kHz，个别 3 kHz）的三阶互调动态范围（dB）。
  ///
  /// 原表脚注 f 标明「测量受相位噪声限制」时，这一列**就是** ARRL RMDR —— 本模块照原值收录，
  /// 界面也不能把它说成「纯互调指标」。
  pub imd_narrow_db: f64,
  /// 窄间隔用的频率间隔（kHz）。
  pub narrow_khz: f64,
  /// 噪声底（dBm，未开前放）。
  pub noise_floor_dbm: f64,
  /// 原表里该行的时间（`YYYY-MM-DD`）：指标会随样本与测试条件变，必须带时间。
  pub measured: &'static str,
}

/// 解析数据文件：跳过注释与空行，每行 7 个 `|` 分隔字段。
fn parse(text: &'static str) -> Vec<GearRx> {
  text
    .lines()
    .map(str::trim)
    .filter(|l| !l.is_empty() && !l.starts_with('#'))
    .filter_map(|line| {
      let mut f = line.split('|').map(str::trim);
      let id = f.next()?;
      let model = f.next()?;
      let wide = f.next()?.parse().ok()?;
      let narrow = f.next()?.parse().ok()?;
      let narrow_khz = f.next()?.parse().ok()?;
      let noise_floor = f.next()?.parse().ok()?;
      let measured = f.next()?;
      if f.next().is_some() {
        return None; // 多出来的字段说明格式写错了，宁可丢掉这一行也不要猜
      }
      Some(GearRx {
        id,
        model,
        imd_wide_db: wide,
        imd_narrow_db: narrow,
        narrow_khz,
        noise_floor_dbm: noise_floor,
        measured,
      })
    })
    .collect()
}

static TABLE: LazyLock<Vec<GearRx>> = LazyLock::new(|| parse(DATA));

/// 全部已收录的机型指标（原表内的机型）。
#[must_use]
pub fn all() -> &'static [GearRx] {
  &TABLE
}

/// 按器材 id 取指标。
///
/// 返回 `None` 是常态：原表只测 HF / 50 MHz 级别的接收机，手持台、车载台、SDR 与附件
/// 都不在表内 —— 界面据此显示「未收录」，而不是拿别的机型的值凑。
#[must_use]
pub fn for_gear(id: &str) -> Option<&'static GearRx> {
  TABLE.iter().find(|r| r.id == id)
}

/// 数据来源（界面要显示它，用户才知道这些数字是谁测的）。
pub const SOURCE: &str = "Rob Sherwood (NC0B), Receiver Test Data";
/// 数据来源地址。
pub const SOURCE_URL: &str = "http://sherweng.com/table.html";
/// 本次核对日期。
pub const CHECKED_ON: &str = "2026-10-08";

#[cfg(test)]
mod tests {
  use super::*;
  use crate::gear;

  #[test]
  fn every_row_parses_and_is_sane() {
    let rows = all();
    assert!(rows.len() >= 8, "收录太少（{} 行）", rows.len());
    for r in rows {
      assert!(!r.id.is_empty() && !r.model.is_empty());
      // 动态范围：40–140 dB 是这个量级的合理区间；窄间隔不会比宽间隔更好。
      for value in [r.imd_wide_db, r.imd_narrow_db] {
        assert!(
          (40.0..=140.0).contains(&value),
          "{} 的动态范围 {value} 不像实测值",
          r.id
        );
      }
      assert!(
        r.imd_narrow_db <= r.imd_wide_db + 1e-9,
        "{} 的窄间隔（{}）比宽间隔（{}）还好，抄错了吧",
        r.id,
        r.imd_narrow_db,
        r.imd_wide_db
      );
      // 噪声底：HF 接收机的合理区间。
      assert!(
        (-150.0..=-90.0).contains(&r.noise_floor_dbm),
        "{} 的噪声底 {} dBm 不像实测值",
        r.id,
        r.noise_floor_dbm
      );
      // 窄间隔就是 2 kHz 或 3 kHz（原表只有这两种）。
      assert!(
        (r.narrow_khz - 2.0).abs() < 1e-9 || (r.narrow_khz - 3.0).abs() < 1e-9,
        "{} 的窄间隔 {} kHz 超出原表的两种取值",
        r.id,
        r.narrow_khz
      );
      // 带日期的 ISO 写法，且不能是「未来」的核对日期之后。
      assert_eq!(r.measured.len(), 10, "{} 的测量日期格式不对", r.id);
      assert!(r.measured < CHECKED_ON, "{} 的测量日期晚于核对日期", r.id);
    }
    let mut ids: Vec<&str> = rows.iter().map(|r| r.id).collect();
    let before = ids.len();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), before, "同一机型收录了多行");
  }

  #[test]
  fn rows_point_at_gear_we_actually_list() {
    for r in all() {
      let g =
        gear::by_id(r.id).unwrap_or_else(|| panic!("{} 不在器材库里：指标要挂在真实机型上", r.id));
      assert!(
        r.model.contains(g.model),
        "{} 的型号写法（{}）与器材库（{}）对不上",
        r.id,
        r.model,
        g.model
      );
      // 原表只测 HF / 50 MHz 级接收机：手持 / 车载 / SDR / 附件不该有行。
      assert!(
        matches!(g.category, "hf" | "portable"),
        "{} 属于 {}，原表不会测它",
        r.id,
        g.category
      );
    }
  }

  #[test]
  fn unlisted_rigs_have_no_data_instead_of_a_guess() {
    // 原表里没有的机型必须是 None：界面据此显示「未收录」，不能拿别的数字凑。
    for id in [
      "uv-5r",
      "ft-60r",
      "ft-5dr",
      "at-d878uv-ii",
      "id-52a",
      "rspdx",
      "z-100plus",
    ] {
      assert!(for_gear(id).is_none(), "{id} 不该有接收机指标");
    }
    assert!(for_gear("no-such-rig").is_none());
  }

  #[test]
  fn the_file_states_its_provenance() {
    // 数据来自外部测量，出处与核对日期必须写在文件里 —— 改数据的人先看到它。
    // 来源、地址、核对日期与取数规则都要写在数据文件里（常量与文件必须是同一份事实）。
    for needle in [
      "Rob Sherwood",
      "Receiver Test Data",
      SOURCE_URL,
      "Updated",
      "取数规则",
    ] {
      assert!(DATA.contains(needle), "数据文件里少了「{needle}」");
    }
  }

  #[test]
  fn malformed_lines_are_dropped_rather_than_guessed() {
    let text: &'static str = "# c\n\
      ok|M|100|90|2|-130|2020-01-01\n\
      short|M|100|90\n\
      badnum|M|abc|90|2|-130|2020-01-01\n\
      extra|M|100|90|2|-130|2020-01-01|tail\n";
    let rows = parse(text);
    assert_eq!(rows.len(), 1, "只有第一行是合法的");
    assert_eq!(rows[0].id, "ok");
  }
}
