//! 台站档案：一次安装往往不止一个台站（固定台 / 车载 / 野外 / POTA），每条通联要说清
//! 「是在哪个台站通的」。
//!
//! # 为什么通联存的是档案 id 而不是呼号
//!
//! **呼号不是身份**：同一个呼号可以对应多个台站（家里固定台与 POTA 野外，网格不同），
//! 反过来同一个台站也可能在不同时间挂不同呼号。所以本地用自增 `id` 标识档案。
//! 但 id 出了这台机器就没有意义，因此 ADIF 里写的是 `STATION_CALLSIGN` / `MY_GRIDSQUARE`
//! 这些**标准字段**；导入时按「呼号 + 网格」找回档案，找不到就新建一条 ——
//! 导出再导入不会把两个同名不同格的台站并成一个，也不会把别人的 id 当自己的用。
//!
//! # 老的单值本台信息
//!
//! [`StationInfo`]（存 `station-info`）作为**当前台站的镜像**继续写：RBN / PSK Reporter
//! 页面与备份合并都在读它，语义（「本机当前台站」）也没变。档案册是唯一真相来源，
//! 镜像在同一个持久化函数里派生，因此不会漂移。

use serde::{Deserialize, Serialize};

use crate::logbook::{LogEntry, StationInfo};

/// 一个台站档案。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StationProfile {
  /// 本地自增标识（0 表示「还没分配」，由 [`StationBook::upsert`] 分配）。
  #[serde(default)]
  pub id: u64,
  /// 档案名，如「固定台」「车载」「POTA 千岛湖」。空则界面回落到呼号；
  /// 呼号也空时界面给翻译过的默认名（core 不产出界面文案，见 [`StationProfile::title`]）。
  #[serde(default)]
  pub label: String,
  /// 本台呼号 `STATION_CALLSIGN`。
  #[serde(default)]
  pub callsign: String,
  /// 操作员 `OPERATOR`（空则默认同呼号）。
  #[serde(default)]
  pub operator: String,
  /// 本台网格 `MY_GRIDSQUARE`。
  #[serde(default)]
  pub gridsquare: String,
  /// 设备 `MY_RIG`。
  #[serde(default)]
  pub rig: String,
  /// 天线 `MY_ANTENNA`。
  #[serde(default)]
  pub antenna: String,
}

impl StationProfile {
  /// 新档案（未分配 id）。
  #[must_use]
  pub fn new(label: &str) -> Self {
    Self {
      label: label.to_owned(),
      ..Self::default()
    }
  }

  /// 导出 / 卡片 / 标签要用的形态。
  #[must_use]
  pub fn info(&self) -> StationInfo {
    StationInfo {
      callsign: self.callsign.clone(),
      operator: self.operator.clone(),
      gridsquare: self.gridsquare.clone(),
      rig: self.rig.clone(),
      antenna: self.antenna.clone(),
    }
  }

  /// 界面上的名字：档案名优先，其次呼号，都没有返回空串。
  ///
  /// 空串的含义由界面兜底成**翻译过的**默认名（core 不产界面文案，中文兜底会漏进
  /// en / es 界面）。
  #[must_use]
  pub fn title(&self) -> String {
    let label = self.label.trim();
    if !label.is_empty() {
      return label.to_owned();
    }
    self.callsign.trim().to_owned()
  }

  /// 与「呼号 + 网格」是否相符（都不区分大小写与首尾空白）。
  ///
  /// 导入时用它认领档案：只比呼号会把家里与野外并成一个，只比网格又认不出改名后的同一台站。
  #[must_use]
  pub fn matches(&self, callsign: &str, gridsquare: &str) -> bool {
    self.callsign.trim().eq_ignore_ascii_case(callsign.trim())
      && self
        .gridsquare
        .trim()
        .eq_ignore_ascii_case(gridsquare.trim())
  }

  /// 是否还没填身份信息（呼号 / 操作员 / 网格 / 设备 / 天线全空）。
  ///
  /// 档案名不算：新档案的默认显示名由界面兜底，拿它当「填过了」会让这个判断永远为假。
  #[must_use]
  pub fn is_blank(&self) -> bool {
    self.info().is_empty()
  }
}

/// [`StationBook::remove`] 的结果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Removal {
  /// 删掉了。
  Removed,
  /// 没有这个档案。
  NotFound,
  /// 只剩这一条，不许删（至少要有一个台站，否则日志不知道该署谁的名）。
  LastStation,
}

/// 台站档案册。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StationBook {
  /// 下一个可用的档案 id。
  #[serde(default)]
  pub next_id: u64,
  /// 档案列表（顺序即界面顺序）。
  #[serde(default)]
  pub profiles: Vec<StationProfile>,
  /// 当前台站（新通联默认归属、ADIF 镜像取它）。
  #[serde(default)]
  pub active_id: u64,
}

impl Default for StationBook {
  fn default() -> Self {
    Self::from_info(&StationInfo::default())
  }
}

impl StationBook {
  /// 由一份本台信息构造（迁移老数据的入口）：一条档案，设为当前。
  #[must_use]
  pub fn from_info(info: &StationInfo) -> Self {
    Self {
      next_id: 2,
      profiles: vec![StationProfile {
        id: 1,
        // 档案名留空：界面给翻译过的默认名（「本台」），core 不存界面文案。
        label: String::new(),
        callsign: info.callsign.trim().to_owned(),
        operator: info.operator.trim().to_owned(),
        gridsquare: info.gridsquare.trim().to_owned(),
        rig: info.rig.trim().to_owned(),
        antenna: info.antenna.trim().to_owned(),
      }],
      active_id: 1,
    }
  }

  /// 当前台站（状态异常时回落到第一条）。
  #[must_use]
  pub fn active(&self) -> &StationProfile {
    self
      .by_id(self.active_id)
      .or_else(|| self.profiles.first())
      .unwrap_or(&FALLBACK)
  }

  /// 按 id 取。
  #[must_use]
  pub fn by_id(&self, id: u64) -> Option<&StationProfile> {
    self.profiles.iter().find(|p| p.id == id)
  }

  /// 设为当前台站（id 不存在时不改）。
  pub fn set_active(&mut self, id: u64) -> bool {
    if self.by_id(id).is_some() {
      self.active_id = id;
      true
    } else {
      false
    }
  }

  /// 按 id 取档案，取不到（`0` = 未指定，或已被删掉）时回落到当前台站。
  ///
  /// 回落而不是「无归属」：老日志（没有归属字段）与删掉的档案都该照样能显示、
  /// 照样能算距离，不该在界面上冒出一排「未知台站」。
  #[must_use]
  pub fn resolve(&self, id: u64) -> &StationProfile {
    self.by_id(id).unwrap_or_else(|| self.active())
  }

  /// 一条通联归属的档案（见 [`StationBook::resolve`] 的回落规则）。
  #[must_use]
  pub fn of_entry(&self, e: &LogEntry) -> &StationProfile {
    self.resolve(e.station_id)
  }

  /// 给新通联打上当前台站的归属。
  pub fn assign(&mut self, e: &mut LogEntry) {
    e.station_id = self.active().id;
  }

  /// 新增或更新一条档案，返回它的 id。
  ///
  /// `id == 0`（或 id 不存在）时分配新 id —— 界面里「新增台站」新增的那条还没 id。
  pub fn upsert(&mut self, profile: StationProfile) -> u64 {
    let id = if profile.id == 0 || self.by_id(profile.id).is_none() {
      let id = self.next_id.max(1);
      // 饱和加法：手改过的 `next_id` 可能顶到 `u64::MAX`，溢出在 debug 下 panic、
      // release 下回绕到 0，而 0 是「未分配」哨兵 —— 会把 id 1 再发一遍。
      self.next_id = id.saturating_add(1);
      id
    } else {
      profile.id
    };
    let mut profile = StationProfile { id, ..profile };
    profile.callsign = profile.callsign.trim().to_ascii_uppercase();
    profile.gridsquare = profile.gridsquare.trim().to_ascii_uppercase();
    profile.operator = profile.operator.trim().to_owned();
    match self.profiles.iter_mut().find(|p| p.id == id) {
      Some(slot) => *slot = profile,
      None => self.profiles.push(profile),
    }
    if self.by_id(self.active_id).is_none() {
      self.active_id = id;
    }
    id
  }

  /// 新增一条空档案（只填档案名），返回它的 id。
  pub fn add(&mut self, label: &str) -> u64 {
    self.upsert(StationProfile::new(label))
  }

  /// 删除一条档案。**最后一条不许删**：日志总得有个署名的地方。
  ///
  /// 删掉当前台站时，当前台站切到剩下第一条；指向它的通联会回落到新的当前台站
  /// （`station_id` 保持原值，读的时候按 [`StationBook::of_entry`] 回落），
  /// 因此删档案**不会**顺手改写历史记录。
  pub fn remove(&mut self, id: u64) -> Removal {
    if self.by_id(id).is_none() {
      return Removal::NotFound;
    }
    if self.profiles.len() <= 1 {
      return Removal::LastStation;
    }
    self.profiles.retain(|p| p.id != id);
    if self.active_id == id {
      self.active_id = self.profiles.first().map_or(0, |p| p.id);
    }
    Removal::Removed
  }

  /// 按「呼号 + 网格」认领档案，找不到就新建一条。
  ///
  /// 导入 ADIF 时用：报告里的 `STATION_CALLSIGN` / `MY_GRIDSQUARE` 决定这条通联属于谁。
  /// 空白呼号一律回落到当前台站（很多导出工具不写这两个字段，凭空新建档案只会堆垃圾）。
  ///
  /// 匹配分三轮，因为**只写一半**是常态（ADIF 常常只有 `STATION_CALLSIGN`、没写
  /// `MY_GRIDSQUARE`，本机新档案也可能还没填网格）：
  ///
  /// 1. 两边网格都有值 → 按「呼号 + 网格」比，这是唯一能分清「家里 / 野外」的情形；
  /// 2. 本机那条没填网格 → 只比呼号，并把报告里的网格补进档案；
  /// 3. 报告没写网格 → 只比呼号，多条同呼号档案时优先当前台站。
  ///
  /// 少了后两轮，导入一份没写网格的报告就会给同一台站凭空堆出一条空档案，那批通联随后
  /// 全归到它上面，导出的 `OPERATOR` / `MY_RIG` / `MY_ANTENNA` 也跟着变空。
  pub fn claim(&mut self, callsign: &str, gridsquare: &str) -> u64 {
    if callsign.trim().is_empty() {
      return self.active().id;
    }
    let (call, grid) = (callsign.trim(), gridsquare.trim());
    let same_call = |p: &StationProfile| p.callsign.trim().eq_ignore_ascii_case(call);
    // 第一轮：两边网格都有值 → 按「呼号 + 网格」比。这是唯一能分清「家里 / 野外」的情形，
    // 只比呼号会把它们并成一个。
    if let Some(id) = self
      .profiles
      .iter()
      .find(|p| !p.gridsquare.trim().is_empty() && !grid.is_empty() && p.matches(call, grid))
      .map(|p| p.id)
    {
      return id;
    }
    // 第二轮：本机这条还没填网格 → 认下来，顺手把报告里的网格补上（补齐身份，
    // 下次就能精确匹配）。
    if let Some(p) = self
      .profiles
      .iter_mut()
      .find(|p| p.gridsquare.trim().is_empty() && same_call(p))
    {
      if !grid.is_empty() {
        p.gridsquare = grid.to_ascii_uppercase();
      }
      return p.id;
    }
    // 第三轮：报告里没写 `MY_GRIDSQUARE`（很多导出工具都不写）→ 归到同呼号的台站，
    // 而不是凭空再堆一条空档案 —— 否则那批通联全归到空档案上，导出的 `OPERATOR` /
    // `MY_RIG` / `MY_ANTENNA` 也跟着变空。多条同呼号档案时优先**当前台站**
    // （用户多半就是为它导入的），否则取第一条。
    if grid.is_empty() {
      let at = self
        .profiles
        .iter()
        .position(|p| same_call(p) && p.id == self.active_id)
        .or_else(|| self.profiles.iter().position(same_call));
      if let Some(at) = at {
        return self.profiles[at].id;
      }
    }
    // 全新用户（册里只有一条还没填过的占位档案）导入别人的日志时不「认领」那条占位档案：
    // 把人家的呼号填成自己的，之后新记的通联会跟着署上别人的名。宁可多一条待清理的
    // 空档案，也不要悄悄改掉用户自己的身份。
    //
    // 只差网格的同呼号台站：新建一条并给出可读的档案名（呼号 · 网格）。
    // 名字按**规整后**的写法拼（`upsert` 会把呼号与网格转成大写），否则会存下
    // 「BG4XXX · ol99aa」这种半大写的档案名。
    let call = callsign.trim().to_ascii_uppercase();
    let label = match gridsquare.trim() {
      "" => call,
      grid => format!("{call} · {}", grid.to_ascii_uppercase()),
    };
    self.upsert(StationProfile {
      label,
      callsign: callsign.to_owned(),
      gridsquare: gridsquare.to_owned(),
      ..StationProfile::default()
    })
  }

  /// 修复持久化数据里的不一致：空册、重复 / 归零的 id、指向不存在的当前台站。
  ///
  /// 每次加载都跑一遍：这些状态只会来自手改 localStorage 或旧版本，但一旦出现就会让
  /// `active()` 落到兜底值上，界面上表现为「本台信息全空」。
  pub fn migrate(&mut self) {
    if self.profiles.is_empty() {
      self.profiles.push(StationProfile {
        id: 1,
        ..StationProfile::default()
      });
    }
    let mut used: Vec<u64> = Vec::new();
    // 重分配的起点必须大于现存的最大 id：`next_id` 本身可能被手改坏（例如两份档案
    // 都拿着 id 2、next_id 却还是 2），从它起步会把撞车的 id 再发一遍，修复静默失败。
    let mut next = self.next_id.max(
      self
        .profiles
        .iter()
        .map(|p| p.id)
        .max()
        .unwrap_or(0)
        .saturating_add(1),
    );
    for p in &mut self.profiles {
      if p.id == 0 || used.contains(&p.id) {
        p.id = next;
        next = next.saturating_add(1);
      }
      used.push(p.id);
    }
    self.next_id = next.max(used.iter().copied().max().unwrap_or(0).saturating_add(1));
    if !used.contains(&self.active_id) {
      self.active_id = used[0];
    }
  }
}

/// `active()` 在空册上的兜底（`migrate` 保证不会走到，但 `active()` 不该 panic）。
static FALLBACK: StationProfile = StationProfile {
  id: 0,
  label: String::new(),
  callsign: String::new(),
  operator: String::new(),
  gridsquare: String::new(),
  rig: String::new(),
  antenna: String::new(),
};

impl StationInfo {
  /// 是否什么都没填。
  #[must_use]
  pub fn is_empty(&self) -> bool {
    self.callsign.trim().is_empty()
      && self.operator.trim().is_empty()
      && self.gridsquare.trim().is_empty()
      && self.rig.trim().is_empty()
      && self.antenna.trim().is_empty()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn info() -> StationInfo {
    StationInfo {
      callsign: " bg4xxx ".into(),
      operator: "WXP".into(),
      gridsquare: " om89ew ".into(),
      rig: "FT-710".into(),
      antenna: "DP 20m".into(),
    }
  }

  fn entry(id: u64) -> LogEntry {
    LogEntry {
      id,
      callsign: "JA1AAA".into(),
      ..Default::default()
    }
  }

  #[test]
  fn migration_carries_every_field_and_trims() {
    let book = StationBook::from_info(&info());
    assert_eq!(book.profiles.len(), 1);
    let p = book.active();
    assert_eq!(p.callsign, "bg4xxx", "迁移不做大小写改写，原样搬过来");
    assert_eq!(p.gridsquare, "om89ew");
    assert_eq!(p.operator, "WXP");
    assert_eq!(p.rig, "FT-710");
    assert_eq!(p.antenna, "DP 20m");
    assert_eq!(p.id, book.active_id);
    assert!(!p.is_blank());
  }

  #[test]
  fn a_fresh_book_still_has_one_station_to_sign_with() {
    let book = StationBook::default();
    assert_eq!(book.profiles.len(), 1);
    assert!(book.active().is_blank());
    // 档案名留空（core 不产界面文案），显示名由界面兜底成翻译过的默认名。
    assert_eq!(book.active().title(), "");
  }

  #[test]
  fn an_empty_book_is_repaired_on_migrate() {
    let mut book = StationBook {
      next_id: 0,
      profiles: Vec::new(),
      active_id: 7,
    };
    book.migrate();
    assert_eq!(book.profiles.len(), 1);
    assert_eq!(book.active_id, book.profiles[0].id);
    assert!(book.next_id > book.active_id);
  }

  #[test]
  fn duplicate_and_zero_ids_are_reassigned() {
    let mut book = StationBook {
      next_id: 3,
      profiles: vec![
        StationProfile {
          id: 1,
          label: "固定台".into(),
          ..Default::default()
        },
        StationProfile {
          id: 1,
          label: "车载".into(),
          ..Default::default()
        },
        StationProfile {
          id: 0,
          label: "野外".into(),
          ..Default::default()
        },
      ],
      active_id: 1,
    };
    book.migrate();
    let ids: Vec<u64> = book.profiles.iter().map(|p| p.id).collect();
    assert_eq!(ids, vec![1, 3, 4], "撞车的与归零的都要换新 id");
    assert_eq!(book.next_id, 5);
    assert_eq!(book.active_id, 1, "当前台站仍然有效就不动");
  }

  #[test]
  fn duplicate_ids_above_next_id_are_still_repaired() {
    // 手改坏 localStorage 的一种真实形态：两份档案都拿着 id 2，而 next_id 停在了 2。
    // 修复必须从「现存最大 id + 1」起步，否则撞车的那条会再拿到 2，修复静默失败。
    let mut book = StationBook {
      next_id: 2,
      profiles: vec![
        StationProfile {
          id: 2,
          label: "固定台".into(),
          ..Default::default()
        },
        StationProfile {
          id: 2,
          label: "车载".into(),
          ..Default::default()
        },
      ],
      active_id: 2,
    };
    book.migrate();
    let ids: Vec<u64> = book.profiles.iter().map(|p| p.id).collect();
    assert_eq!(ids, vec![2, 3], "撞车的 id 必须换成没被占用的");
    assert_eq!(book.next_id, 4);
    assert_eq!(book.active_id, 2);
  }

  #[test]
  fn upsert_assigns_ids_and_normalizes_the_callsign() {
    let mut book = StationBook::default();
    let id = book.upsert(StationProfile {
      callsign: " bg4xxx ".into(),
      gridsquare: " om89ew ".into(),
      operator: " wxp ".into(),
      label: "  ".into(),
      ..Default::default()
    });
    assert_eq!(book.profiles.len(), 2, "迁移来的那条还在");
    let p = book.by_id(id).expect("新档案");
    assert_eq!(p.callsign, "BG4XXX", "呼号 / 网格规整成大写");
    assert_eq!(p.gridsquare, "OM89EW");
    assert_eq!(p.operator, "wxp", "操作员只是去空白，不改大小写");
    assert_eq!(p.title(), "BG4XXX", "档案名为空时回落到呼号");

    // 再 upsert 同一条是更新，不是新增。
    let again = book.upsert(StationProfile {
      id,
      label: "POTA".into(),
      callsign: "BG4XXX".into(),
      ..Default::default()
    });
    assert_eq!(again, id);
    assert_eq!(book.profiles.len(), 2);
    assert_eq!(book.by_id(id).expect("仍在").title(), "POTA");
  }

  #[test]
  fn entries_fall_back_to_the_active_station() {
    let mut book = StationBook::from_info(&info());
    let home = book.active_id;
    let other = book.add("POTA");
    let mut e = entry(1);
    // 老日志没有归属字段（station_id = 0）→ 落到当前台站。
    assert_eq!(book.of_entry(&e).id, home);
    // 打上归属后就跟着那条走。
    book.assign(&mut e);
    assert_eq!(e.station_id, home);
    book.set_active(other);
    assert_eq!(
      book.of_entry(&e).id,
      home,
      "历史记录跟着自己的台站，不跟着当前台站变"
    );
    // 指向已删除的档案 → 回落到当前台站，而不是「无归属」。
    let mut e2 = entry(2);
    e2.station_id = other;
    assert_eq!(book.remove(other), Removal::Removed);
    assert_eq!(book.of_entry(&e2).id, book.active_id);
  }

  #[test]
  fn removing_the_active_station_switches_to_another() {
    let mut book = StationBook::from_info(&info());
    let pota = book.add("POTA");
    book.set_active(pota);
    assert_eq!(book.remove(pota), Removal::Removed);
    assert_eq!(book.profiles.len(), 1);
    assert_eq!(
      book.active_id, book.profiles[0].id,
      "当前台站要落到还在的那条"
    );
    assert_eq!(
      book.remove(book.active_id),
      Removal::LastStation,
      "最后一条不许删"
    );
    assert_eq!(book.remove(999), Removal::NotFound);
    assert_eq!(book.profiles.len(), 1);
  }

  #[test]
  fn claim_matches_on_callsign_and_grid_then_creates() {
    let mut book = StationBook::from_info(&info());
    let home = book.active_id;
    // 大小写与首尾空白不影响认领。
    assert_eq!(book.claim("BG4XXX", "OM89EW"), home);
    // 同呼号、不同网格（家里 vs 野外）要各自成条，否则导出再导入会并成一个。
    let field = book.claim("bg4xxx", "ol99aa");
    assert_ne!(field, home);
    assert_eq!(book.profiles.len(), 2);
    assert_eq!(
      book.by_id(field).expect("新档案").title(),
      "BG4XXX · OL99AA"
    );
    // 幂等：再认领一次还是同一条。
    assert_eq!(book.claim("BG4XXX", "OL99AA"), field);
    assert_eq!(book.profiles.len(), 2);
    // 空白呼号（多数导出工具不写）回落到当前台站，不凭空造档案。
    book.set_active(home);
    assert_eq!(book.claim("", ""), home);
    assert_eq!(book.claim("  ", "PM95"), home);
    assert_eq!(book.profiles.len(), 2);
  }

  #[test]
  fn claim_falls_back_to_the_callsign_when_a_grid_is_missing() {
    let mut book = StationBook::from_info(&info());
    let home = book.active_id;
    // ADIF 只写 `STATION_CALLSIGN`、不写 `MY_GRIDSQUARE` 是常态：少一项不能算「另一台站」，
    // 否则导入一份这样的报告就凭空多出一条空档案 —— 那批通联全归到它上面，导出时
    // `OPERATOR` / `MY_RIG` / `MY_ANTENNA` 也跟着变空。
    assert_eq!(book.claim("BG4XXX", ""), home);
    assert_eq!(book.profiles.len(), 1);
    // 两边网格都填了才按两项比：那才是真的两台站（家里 vs 野外）。
    let field = book.claim("BG4XXX", "OL99AA");
    assert_ne!(field, home);
    assert_eq!(book.profiles.len(), 2);
    // 精确匹配优先于「只比呼号」那一轮：网格对得上的报告仍归「家里」。
    assert_eq!(book.claim("BG4XXX", "OM89EW"), home);
  }

  #[test]
  fn claim_fills_a_missing_grid_on_the_matched_station() {
    // 本机档案还没填网格时，报告里的网格归到它上面并补进档案 —— 补齐身份后，
    // 下一次导入就能走精确匹配。补的是**同一个呼号**的档案，不会张冠李戴。
    let mut book = StationBook::from_info(&StationInfo {
      callsign: "BG4XXX".into(),
      ..Default::default()
    });
    let only = book.active_id;
    assert_eq!(book.claim("BG4XXX", "OM89EW"), only);
    assert_eq!(book.profiles.len(), 1);
    assert_eq!(book.by_id(only).expect("本机那条").gridsquare, "OM89EW");
  }

  #[test]
  fn claim_never_hijacks_a_fresh_placeholder() {
    // 全新用户导入别人的日志：不能把人家的呼号填进自己那条占位档案 —— 之后新记的
    // 通联会跟着署上别人的名。宁可多一条待清理的空档案。
    let mut book = StationBook::default();
    let placeholder = book.active_id;
    let theirs = book.claim("JA1AAA", "PM95");
    assert_ne!(theirs, placeholder);
    assert_eq!(book.profiles.len(), 2);
    assert_eq!(book.active_id, placeholder, "当前台站仍是用户自己那条");
    assert!(book.by_id(placeholder).expect("占位档案").is_blank());
  }

  #[test]
  fn info_round_trips_through_the_profile() {
    let book = StationBook::from_info(&info());
    let back = book.active().info();
    assert_eq!(back.callsign, info().callsign.trim());
    assert_eq!(back.antenna, info().antenna);
    assert!(!back.is_empty());
    assert!(StationInfo::default().is_empty());
  }

  #[test]
  fn book_round_trips_through_json() {
    let mut book = StationBook::from_info(&info());
    book.add("POTA");
    let json = serde_json::to_string(&book).expect("序列化");
    let back: StationBook = serde_json::from_str(&json).expect("反序列化");
    assert_eq!(back, book);
    // 老数据（只有一份 StationInfo）也能读进来：字段全部 `serde(default)`。
    let legacy: StationBook = serde_json::from_str("{}").expect("空对象");
    assert!(legacy.profiles.is_empty());
    assert!(legacy.active().is_blank());
  }
}
