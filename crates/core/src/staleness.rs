//! 时效性数据的核对日期登记：谁需要定期复核、多久算过期。
//!
//! 仓库里其实有三套「来源」机制，分工别混：
//!
//! - `ham-web-tools` 的来源戳（`crates/tools/src/provenance.rs`）：回答「产物由上游哪一份
//!   文件生成」，靠摘要 / 校验头判断上游动没动；
//! - [`crate::provenance`]：回答「这一页的知识依据是什么」，把依据与核对月份标在页面上；
//! - 这里：回答「这份**人工转录**的外部事实，上次核对是什么时候」。
//!
//! 为什么后者不能并入前两者：这些数据的上游是 HTML 页或法规原文，人工抄进仓库后摘要就再
//! 没有可比性（页面模板一改，校验头也会变）。唯一可靠的信号是**时间**——超过多久没人复核，
//! 就该有人回去看一遍。
//!
//! 登记项集中在这一个文件里，避免每个模块各记一个日期、各判各的过期。

use crate::{diy_projects, gear_rx, provenance, radio_law, toroid};

/// 一条需要定期复核的外部数据。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Curated {
  /// 稳定 key：命令行输出、文档、修复提示都用它。
  pub key: &'static str,
  /// 依据（一句话出处，人看的）。
  pub source: &'static str,
  /// 最近一次核对的日期：`YYYY-MM` 或 `YYYY-MM-DD`。
  pub checked_on: &'static str,
}

/// 全仓登记的时效性数据（顺序固定，便于输出对账）。
#[must_use]
pub fn curated() -> Vec<Curated> {
  let mut out: Vec<Curated> = provenance::PROVENANCES
    .iter()
    .map(|(key, p)| Curated {
      key,
      source: p.source,
      checked_on: p.checked_on,
    })
    .collect();

  // 页面溯源表之外，还有几处「数据文件 / 常量」自己带核对日期。它们的依据比页面上那句更
  // 具体（厂商零件页、法规原文、市场价），过期时也各改各自的日期，所以单列，不并进上面。
  out.push(Curated {
    key: "gear-rx",
    source: "Sherwood (NC0B) Receiver Test Data",
    checked_on: gear_rx::CHECKED_ON,
  });
  if let Some(date) = radio_law::checked_on() {
    out.push(Curated {
      key: "radio-law-text",
      source: "《业余无线电台管理办法》条文库（工信部令第 67 号，原文照录）",
      checked_on: date,
    });
  }
  if let Some(date) = toroid::oldest_verified() {
    out.push(Curated {
      key: "toroid",
      source: "Fair-Rite 官方零件页（磁环 AL / 尺寸，按最旧的一条算）",
      checked_on: date,
    });
  }
  out.push(Curated {
    key: "diy-cost",
    source: "DIY 项目成本档位（市场价，只给档位不给售价）",
    checked_on: diy_projects::COST_CHECKED_ON,
  });
  out
}

/// `YYYY-MM` 或 `YYYY-MM-DD` → `(年, 月)`；格式不对返回 `None`（不猜）。
fn year_month(date: &str) -> Option<(i64, i64)> {
  let mut parts = date.split('-');
  let (year, month) = (parts.next()?, parts.next()?);
  // 只认定长数字：`2026-1`、`26-10` 这类写法宁可当读不出来，也不当成某个月。
  if year.len() != 4 || month.len() != 2 {
    return None;
  }
  let year: i64 = year.parse().ok()?;
  let month: i64 = month.parse().ok()?;
  (1..=12).contains(&month).then_some((year, month))
}

/// `checked_on` 到 `today` 之间过了多少个月（同月为 0，`checked_on` 在未来则为负）。
#[must_use]
pub fn months_since(checked_on: &str, today: &str) -> Option<i64> {
  let (from_year, from_month) = year_month(checked_on)?;
  let (to_year, to_month) = year_month(today)?;
  Some((to_year - from_year) * 12 + (to_month - from_month))
}

/// 一条核对日期的年龄判定。
///
/// 把「到期」和「数据写坏了」分开：前者随时间自然出现，后者（日期读不出来、写成了未来）
/// 是确定性的数据错误，任何时候都该判失败，不能被「只告警」模式放过。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Age {
  /// 月龄在阈值内（含阈值）。
  Fresh(i64),
  /// 月龄超过阈值：该回去复核了。
  Stale(i64),
  /// 核对日期晚于今天：数据里写了还没到的日期，复核记录不可信。
  Future(i64),
  /// 格式不对，读不出来。
  Unreadable,
}

/// 判定 `checked_on` 相对 `today` 的年龄。
#[must_use]
pub fn age(checked_on: &str, today: &str, max_months: i64) -> Age {
  match months_since(checked_on, today) {
    None => Age::Unreadable,
    Some(months) if months < 0 => Age::Future(months),
    Some(months) if months > max_months => Age::Stale(months),
    Some(months) => Age::Fresh(months),
  }
}

/// 该不该回去复核：月龄**超过** `max_months`，或日期读不出来 / 在未来（不可信就不能当新鲜）。
#[must_use]
pub fn is_stale(checked_on: &str, today: &str, max_months: i64) -> bool {
  !matches!(age(checked_on, today, max_months), Age::Fresh(_))
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn months_are_counted_in_calendar_months() {
    assert_eq!(months_since("2026-10", "2026-10"), Some(0));
    assert_eq!(months_since("2026-04", "2026-10"), Some(6));
    assert_eq!(months_since("2025-10", "2026-10"), Some(12));
    // 带日期的写法只看到月：同月内怎么变都是 0。
    assert_eq!(months_since("2026-10-09", "2026-10-23"), Some(0));
    assert_eq!(months_since("2026-09-30", "2026-10-01"), Some(1));
    // 未来日期是负数：说明有人在数据里写了还没到的核对日期。
    assert_eq!(months_since("2027-01", "2026-10"), Some(-3));
  }

  #[test]
  fn malformed_dates_are_not_guessed() {
    for bad in [
      "", "2026", "2026-1", "26-10", "2026-13", "2026-00", "abc-def",
    ] {
      assert_eq!(months_since(bad, "2026-10"), None, "{bad}");
    }
    assert_eq!(months_since("2026-10", "xx"), None);
  }

  #[test]
  fn staleness_is_strictly_over_the_limit() {
    assert!(!is_stale("2026-04", "2026-10", 6), "正好 6 个月不算过期");
    assert!(is_stale("2026-03", "2026-10", 6));
    // 读不出日期时按「该复核」处理，别把一个坏值当成新鲜。
    assert!(is_stale("", "2026-10", 6));
    // 未来日期同理：它说明有人写了还没发生的复核，不能当新鲜。
    assert!(is_stale("2027-01", "2026-10", 6));
  }

  #[test]
  fn age_separates_expiry_from_broken_data() {
    assert_eq!(age("2026-10", "2026-10", 6), Age::Fresh(0));
    assert_eq!(age("2026-04", "2026-10", 6), Age::Fresh(6));
    assert_eq!(age("2026-03", "2026-10", 6), Age::Stale(7));
    assert_eq!(age("2027-01", "2026-10", 6), Age::Future(-3));
    assert_eq!(age("2026-1", "2026-10", 6), Age::Unreadable);
  }

  #[test]
  fn every_registered_entry_has_a_usable_date_and_unique_key() {
    let all = curated();
    assert!(all.len() >= provenance::PROVENANCES.len() + 2);
    for item in &all {
      assert!(
        months_since(item.checked_on, "2026-01").is_some(),
        "{} 的核对日期读不出来：{}",
        item.key,
        item.checked_on
      );
      assert!(!item.source.is_empty(), "{} 缺少出处", item.key);
    }
    let mut keys: Vec<&str> = all.iter().map(|c| c.key).collect();
    keys.sort_unstable();
    keys.dedup();
    assert_eq!(keys.len(), all.len(), "key 必须唯一（输出、文档都靠它）");
  }

  /// 数据文件里的日期必须被登记项读到：漏了就等于「没人知道上次核对是什么时候」。
  #[test]
  fn file_backed_entries_are_registered() {
    let all = curated();
    let find = |key: &str| {
      all
        .iter()
        .find(|c| c.key == key)
        .unwrap_or_else(|| panic!("缺少登记项：{key}"))
        .checked_on
    };
    assert_eq!(find("gear-rx"), gear_rx::CHECKED_ON);
    assert_eq!(
      find("radio-law-text"),
      radio_law::checked_on().expect("条文核对日期")
    );
    assert_eq!(
      find("toroid"),
      toroid::oldest_verified().expect("磁环核对日期")
    );
    assert_eq!(find("diy-cost"), diy_projects::COST_CHECKED_ON);
  }
}
