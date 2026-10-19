//! 信号听辨向导：把「我听到的是什么」从听感特征收窄到具体模式。
//!
//! 只描述**音频层听得出来**的东西（有没有音高、几个音、节奏、占用带宽），
//! 不进入协议细节 —— 协议留给各模式的知识页，能不能解交给对应解码器。
//! 特征表是「排除法」而不是识别器：同一组特征下并存多个模式是正常的，
//! 页面要如实把它们并列出来，不给唯一答案。

/// 听感形态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sound {
  /// 人声（SSB / AM / FM 语音）。
  Speech,
  /// 一个有音高的稳定音（CW 拍频、PSK31 的颤动哨声）。
  Single,
  /// 两个音来回切换（RTTY、NAVTEX、AFSK1200）。
  Dual,
  /// 多个音同时存在或快速跳变（MFSK 家族、FT8）。
  Multi,
  /// 没有音高、像噪声（宽带数据、传真底噪）。
  Noise,
  /// 音高连续滑动（LoRa chirp）。
  Chirp,
}

/// 节奏。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rhythm {
  /// 连续不断。
  Continuous,
  /// 人工键入：长短不齐、有词间隔。
  Keyed,
  /// 周期性突发（FT8 每 15 秒、WSPR 每 2 分钟）。
  Burst,
  /// 单次短促突发（APRS / Packet）。
  Packet,
  /// 有规律的慢速扫描（图像：SSTV / WEFAX / APT）。
  Sweep,
}

/// 占用带宽分档。
///
/// 分档是给「不测频谱的人」用的粗粒度开关，真正把模式分开的是相邻两档的边界；
/// 判定用区间**交集**，所以落在 250 Hz 上的模式在窄 / 中两档都会出现。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bandwidth {
  /// 小于 250 Hz。
  Narrow,
  /// 250 Hz – 1 kHz。
  Medium,
  /// 1 kHz – 3 kHz。
  Wide,
  /// 3 kHz 以上。
  Broad,
}

impl Bandwidth {
  /// 该档覆盖的频率区间（Hz）。
  #[must_use]
  pub const fn range(self) -> (f64, f64) {
    match self {
      Self::Narrow => (0.0, 250.0),
      Self::Medium => (250.0, 1000.0),
      Self::Wide => (1000.0, 3000.0),
      Self::Broad => (3000.0, f64::INFINITY),
    }
  }
}

/// 一个可听辨的模式。
#[derive(Debug, Clone, Copy)]
pub struct SignalProfile {
  /// 稳定标识：页面据此取 `signal.<key>-name` / `signal.<key>-hint` 文案。
  pub key: &'static str,
  /// 可能出现的听感形态（满足其一即算命中）。
  pub sounds: &'static [Sound],
  /// 可能的节奏。
  pub rhythms: &'static [Rhythm],
  /// 典型占用带宽区间（Hz），用于与 [`Bandwidth`] 求交集。
  pub bandwidth_hz: (f64, f64),
  /// 站内可以继续深入或直接解码的页面；`None` 表示只有知识页提到过。
  pub page: Option<&'static str>,
}

/// 用户已经确认的听感特征；`None` 表示「不确定，不限」。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SignalQuery {
  pub sound: Option<Sound>,
  pub rhythm: Option<Rhythm>,
  pub bandwidth: Option<Bandwidth>,
}

impl SignalQuery {
  fn matches(self, p: &SignalProfile) -> bool {
    self.sound.is_none_or(|s| p.sounds.contains(&s))
      && self.rhythm.is_none_or(|r| p.rhythms.contains(&r))
      && self
        .bandwidth
        .is_none_or(|b| overlaps(b.range(), p.bandwidth_hz))
  }
}

/// 区间是否有交集（闭区间，边界点算重叠）。
fn overlaps(a: (f64, f64), b: (f64, f64)) -> bool {
  a.0 <= b.1 && b.0 <= a.1
}

/// 模式特征表。
///
/// 顺序即页面展示顺序：从最常听到的窄带模式排到宽带 / 图像模式。
/// 带宽取「典型占用」而不是「标准允许」，目的是排除明显不可能的模式。
pub const PROFILES: &[SignalProfile] = &[
  SignalProfile {
    key: "cw",
    sounds: &[Sound::Single],
    rhythms: &[Rhythm::Keyed, Rhythm::Continuous],
    bandwidth_hz: (80.0, 500.0),
    page: Some("/morse"),
  },
  SignalProfile {
    key: "rtty",
    sounds: &[Sound::Dual],
    rhythms: &[Rhythm::Continuous],
    bandwidth_hz: (170.0, 300.0),
    page: Some("/rtty-decode"),
  },
  SignalProfile {
    key: "psk31",
    sounds: &[Sound::Single, Sound::Multi],
    rhythms: &[Rhythm::Keyed, Rhythm::Continuous],
    bandwidth_hz: (31.0, 200.0),
    page: Some("/psk-decode"),
  },
  SignalProfile {
    key: "ft8",
    sounds: &[Sound::Multi],
    rhythms: &[Rhythm::Burst],
    bandwidth_hz: (30.0, 100.0),
    page: Some("/ft8-decode"),
  },
  SignalProfile {
    key: "ft4",
    sounds: &[Sound::Multi],
    rhythms: &[Rhythm::Burst],
    bandwidth_hz: (40.0, 120.0),
    page: Some("/ft8-decode"),
  },
  SignalProfile {
    key: "wspr",
    sounds: &[Sound::Multi],
    rhythms: &[Rhythm::Burst],
    bandwidth_hz: (4.0, 20.0),
    page: Some("/wspr-decode"),
  },
  SignalProfile {
    key: "olivia",
    sounds: &[Sound::Multi],
    rhythms: &[Rhythm::Continuous],
    bandwidth_hz: (250.0, 1000.0),
    page: Some("/olivia-decode"),
  },
  SignalProfile {
    key: "dominoex",
    sounds: &[Sound::Multi],
    rhythms: &[Rhythm::Continuous],
    bandwidth_hz: (200.0, 600.0),
    page: Some("/dominoex-decode"),
  },
  SignalProfile {
    key: "hell",
    sounds: &[Sound::Single, Sound::Noise],
    rhythms: &[Rhythm::Continuous],
    bandwidth_hz: (300.0, 1200.0),
    page: Some("/hell-decode"),
  },
  SignalProfile {
    key: "navtex",
    sounds: &[Sound::Dual],
    rhythms: &[Rhythm::Continuous],
    bandwidth_hz: (200.0, 500.0),
    page: Some("/navtex-decode"),
  },
  SignalProfile {
    key: "packet",
    sounds: &[Sound::Dual],
    rhythms: &[Rhythm::Packet],
    bandwidth_hz: (1000.0, 3000.0),
    page: Some("/kiss-tnc"),
  },
  SignalProfile {
    key: "sstv",
    sounds: &[Sound::Multi, Sound::Chirp],
    rhythms: &[Rhythm::Sweep],
    bandwidth_hz: (1000.0, 2500.0),
    page: Some("/sstv-decode"),
  },
  SignalProfile {
    key: "wefax",
    sounds: &[Sound::Noise],
    rhythms: &[Rhythm::Sweep, Rhythm::Continuous],
    bandwidth_hz: (1000.0, 2400.0),
    page: Some("/wefax-decode"),
  },
  SignalProfile {
    key: "apt",
    sounds: &[Sound::Noise],
    rhythms: &[Rhythm::Sweep],
    bandwidth_hz: (30_000.0, 45_000.0),
    page: Some("/apt-decoder"),
  },
  SignalProfile {
    key: "ssb",
    sounds: &[Sound::Speech],
    rhythms: &[Rhythm::Continuous],
    bandwidth_hz: (1800.0, 3000.0),
    page: Some("/modes"),
  },
  SignalProfile {
    key: "fm",
    sounds: &[Sound::Speech],
    rhythms: &[Rhythm::Continuous],
    bandwidth_hz: (8000.0, 16_000.0),
    page: Some("/modes"),
  },
  SignalProfile {
    key: "lora",
    sounds: &[Sound::Chirp, Sound::Noise],
    rhythms: &[Rhythm::Burst, Rhythm::Continuous],
    bandwidth_hz: (125_000.0, 500_000.0),
    page: Some("/lora"),
  },
];

/// 按已确认的特征筛选候选模式。
///
/// 没有任何条件时返回全部（页面用它做初始列表）；条件互相矛盾时返回空表，
/// 页面应给出「放宽某一项」的提示而不是硬塞一个猜测。
#[must_use]
pub fn narrow(query: &SignalQuery) -> Vec<&'static SignalProfile> {
  PROFILES.iter().filter(|p| query.matches(p)).collect()
}

/// 按稳定标识取一个模式。
#[must_use]
pub fn profile(key: &str) -> Option<&'static SignalProfile> {
  PROFILES.iter().find(|p| p.key == key)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn empty_query_returns_everything() {
    assert_eq!(narrow(&SignalQuery::default()).len(), PROFILES.len());
    assert!(PROFILES.len() >= 15);
  }

  #[test]
  fn keyed_single_tone_finds_cw_and_psk31() {
    let q = SignalQuery {
      sound: Some(Sound::Single),
      rhythm: Some(Rhythm::Keyed),
      bandwidth: None,
    };
    let keys: Vec<&str> = narrow(&q).iter().map(|p| p.key).collect();
    assert!(keys.contains(&"cw"));
    assert!(keys.contains(&"psk31"));
    assert!(!keys.contains(&"ft8"), "FT8 是周期突发而不是人工键入");
  }

  #[test]
  fn burst_narrow_tone_finds_wsjt_modes_only() {
    let q = SignalQuery {
      sound: Some(Sound::Multi),
      rhythm: Some(Rhythm::Burst),
      bandwidth: Some(Bandwidth::Narrow),
    };
    let keys: Vec<&str> = narrow(&q).iter().map(|p| p.key).collect();
    assert_eq!(keys, ["ft8", "ft4", "wspr"]);
  }

  #[test]
  fn speech_only_returns_voice_modes() {
    let q = SignalQuery {
      sound: Some(Sound::Speech),
      ..SignalQuery::default()
    };
    let keys: Vec<&str> = narrow(&q).iter().map(|p| p.key).collect();
    assert_eq!(keys, ["ssb", "fm"]);
  }

  #[test]
  fn contradictory_query_is_empty_not_guessed() {
    let q = SignalQuery {
      sound: Some(Sound::Speech),
      rhythm: Some(Rhythm::Burst),
      bandwidth: Some(Bandwidth::Narrow),
    };
    assert!(narrow(&q).is_empty());
  }

  #[test]
  fn bandwidth_class_boundaries_overlap() {
    // 250 Hz 正好落在窄 / 中两档的交界，两档都该看得见 RTTY（170–300 Hz）。
    let in_narrow = SignalQuery {
      bandwidth: Some(Bandwidth::Narrow),
      ..SignalQuery::default()
    };
    let in_medium = SignalQuery {
      bandwidth: Some(Bandwidth::Medium),
      ..SignalQuery::default()
    };
    for q in [in_narrow, in_medium] {
      let keys: Vec<&str> = narrow(&q).iter().map(|p| p.key).collect();
      assert!(keys.contains(&"rtty"));
    }
  }

  #[test]
  fn every_profile_is_well_formed() {
    for p in PROFILES {
      assert!(!p.sounds.is_empty(), "{} 缺少听感形态", p.key);
      assert!(!p.rhythms.is_empty(), "{} 缺少节奏", p.key);
      assert!(
        p.bandwidth_hz.0 < p.bandwidth_hz.1 && p.bandwidth_hz.0 >= 0.0,
        "{} 的带宽区间不合法：{:?}",
        p.key,
        p.bandwidth_hz
      );
    }
  }

  #[test]
  fn keys_are_unique_and_lookup_works() {
    let mut keys: Vec<&str> = PROFILES.iter().map(|p| p.key).collect();
    keys.sort_unstable();
    keys.dedup();
    assert_eq!(keys.len(), PROFILES.len(), "key 有重复");
    for p in PROFILES {
      assert_eq!(profile(p.key).map(|x| x.key), Some(p.key));
    }
    assert!(profile("不存在").is_none());
  }

  #[test]
  fn pages_exist_in_registry() {
    for p in PROFILES {
      let Some(page) = p.page else { continue };
      assert!(
        crate::registry::MODULES.iter().any(|m| m.path == page),
        "{} 指向的页面 {page} 没有注册",
        p.key
      );
    }
  }
}
