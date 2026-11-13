//! 流星雨时刻表：IMO《Meteor Shower Calendar 2026》的 Working List（表 5），用于流星散射与活动日历。
//!
//! # 数据出处与许可
//!
//! 全部数值（活动期、峰值日、太阳黄经 λ⊙、辐射点赤经 / 赤纬、进入速度 V∞、星等指数 r、ZHR）
//! 取自 International Meteor Organization 的 2026 年流星雨年历表 5（2026-11-12 取回，
//! `https://www.imo.net/files/meteor-shower/cal2026.pdf`，第 25 页）。IMO 的
//! [Using IMO Data](https://www.imo.net/about/using-imo-data/) 明确允许「use, reproduce and share」
//! 年历数据（任何目的、不必事先询问），条件是署名、不暗示背书、不把改过的值说成 IMO 的；
//! 只有把数据集**整份**做成独立文件 / 数据库 / API 才要事先问。这里只取表 5 的数值字段
//! 做成站内自己的呈现，属于允许的「short extracts and data used in your own presentation」。
//! 页面上的署名行见 `common.imo-credit`。
//!
//! # 峰值日只对 2026 年精确
//!
//! IMO 表 5 的说明原话是「maximum dates accurate only for 2026」：峰值由太阳黄经 λ⊙ 决定，
//! 每年的公历日期会差一天上下。这里按**月 / 日**存峰值（即 2026 年的对应日期），
//! 跨年推算时按同一个日期处理，所以「距离下次峰值多少天」在别的年份最多差一天。
//! 要精确到小时得按 λ⊙ 反解太阳黄经，本模块不做（ZHR 峰值本来也有几个小时的不确定性）。
//!
//! # 没有收的
//!
//! - Antihelion Source（ANT）：IMO 表里没有固定峰值与辐射点（「见 Table 6」），是全年弥散源，
//!   进不了「峰值倒计时」这种表；
//! - 表 7 的白天射电流星雨（Daytime showers）：只在日间无线电前向散射里用，与本站的
//!   MSK144 夜间流星散射不是同一类观测；
//! - 各年的爆发预报（outburst）、月球干扰：年历正文每颗一句，这里只收数值表。

/// 数据取自 IMO 年历（表 5）的日期，[`crate::staleness`] 用它判过期。
pub const CHECKED_ON: &str = "2026-11-12";

/// 一颗流星雨的 IMO 表 5 条目。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Shower {
  /// IAU 流星雨代号（三字母）。
  pub code: &'static str,
  /// 中文名。
  pub name: &'static str,
  /// IMO 用的英文名（不翻译，直接显示）。
  pub name_en: &'static str,
  /// 活动期：起始月 / 日、结束月 / 日（结束月小于起始月表示跨年）。
  pub activity: (u32, u32, u32, u32),
  /// 峰值：月 / 日（**2026 年**的日期，见模块文档）。
  pub peak: (u32, u32),
  /// 峰值时的太阳黄经 λ⊙（度）。
  pub lambda_sun_deg: f64,
  /// 辐射点赤经（度）。
  pub ra_deg: f64,
  /// 辐射点赤纬（度）。
  pub dec_deg: f64,
  /// 进入速度 V∞（km/s）。
  pub v_inf_km_s: f64,
  /// 星等指数 r（亮度比）。
  pub r: f64,
  /// 天顶每小时出现率；`None` 表示 IMO 标「Var」（可变）。
  pub zhr: Option<f64>,
}

/// IMO 2026 年历表 5 收录的流星雨（按峰值在一年中的先后排列）。
pub const SHOWERS: &[Shower] = &[
  Shower {
    code: "QUA",
    name: "象限仪座流星雨",
    name_en: "Quadrantids",
    activity: (12, 28, 1, 12),
    peak: (1, 3),
    lambda_sun_deg: 283.15,
    ra_deg: 230.0,
    dec_deg: 49.0,
    v_inf_km_s: 41.0,
    r: 2.1,
    zhr: Some(80.0),
  },
  Shower {
    code: "GUM",
    name: "小熊座 γ 流星雨",
    name_en: "Gamma-Ursae Minorids",
    activity: (1, 10, 1, 22),
    peak: (1, 18),
    lambda_sun_deg: 298.0,
    ra_deg: 228.0,
    dec_deg: 67.0,
    v_inf_km_s: 31.0,
    r: 3.0,
    zhr: Some(3.0),
  },
  Shower {
    code: "ACE",
    name: "半人马座 α 流星雨",
    name_en: "Alpha-Centaurids",
    activity: (1, 31, 2, 20),
    peak: (2, 8),
    lambda_sun_deg: 319.4,
    ra_deg: 211.0,
    dec_deg: -58.0,
    v_inf_km_s: 58.0,
    r: 2.0,
    zhr: Some(6.0),
  },
  Shower {
    code: "LYR",
    name: "四月天琴座流星雨",
    name_en: "April Lyrids",
    activity: (4, 14, 4, 30),
    peak: (4, 22),
    lambda_sun_deg: 32.32,
    ra_deg: 271.0,
    dec_deg: 34.0,
    v_inf_km_s: 49.0,
    r: 2.1,
    zhr: Some(18.0),
  },
  Shower {
    code: "PPU",
    name: "船尾座 π 流星雨",
    name_en: "Pi-Puppids",
    activity: (4, 15, 4, 28),
    peak: (4, 24),
    lambda_sun_deg: 33.5,
    ra_deg: 110.0,
    dec_deg: -45.0,
    v_inf_km_s: 18.0,
    r: 2.0,
    zhr: None,
  },
  Shower {
    code: "ETA",
    name: "宝瓶座 η 流星雨",
    name_en: "Eta-Aquariids",
    activity: (4, 19, 5, 28),
    peak: (5, 6),
    lambda_sun_deg: 45.5,
    ra_deg: 338.0,
    dec_deg: -1.0,
    v_inf_km_s: 66.0,
    r: 2.4,
    zhr: Some(50.0),
  },
  Shower {
    code: "ELY",
    name: "天琴座 η 流星雨",
    name_en: "Eta-Lyrids",
    activity: (5, 3, 5, 14),
    peak: (5, 11),
    lambda_sun_deg: 50.0,
    ra_deg: 291.0,
    dec_deg: 43.0,
    v_inf_km_s: 43.0,
    r: 3.0,
    zhr: Some(3.0),
  },
  Shower {
    code: "ARI",
    name: "白日白羊座流星雨",
    name_en: "Daytime Arietids",
    activity: (5, 14, 6, 24),
    peak: (6, 7),
    lambda_sun_deg: 76.7,
    ra_deg: 43.0,
    dec_deg: 24.0,
    v_inf_km_s: 38.0,
    r: 2.8,
    zhr: Some(30.0),
  },
  Shower {
    code: "JBO",
    name: "六月牧夫座流星雨",
    name_en: "June Bootids",
    activity: (6, 22, 7, 2),
    peak: (6, 22),
    lambda_sun_deg: 90.3,
    ra_deg: 221.0,
    dec_deg: 48.0,
    v_inf_km_s: 18.0,
    r: 2.2,
    zhr: None,
  },
  Shower {
    code: "JPE",
    name: "七月飞马座流星雨",
    name_en: "July Pegasids",
    activity: (7, 1, 7, 20),
    peak: (7, 10),
    lambda_sun_deg: 108.0,
    ra_deg: 347.0,
    dec_deg: 11.0,
    v_inf_km_s: 63.0,
    r: 3.0,
    zhr: Some(3.0),
  },
  Shower {
    code: "GDR",
    name: "七月天龙座 γ 流星雨",
    name_en: "July Gamma-Draconids",
    activity: (7, 25, 7, 31),
    peak: (7, 28),
    lambda_sun_deg: 125.13,
    ra_deg: 280.0,
    dec_deg: 51.0,
    v_inf_km_s: 27.0,
    r: 3.0,
    zhr: Some(5.0),
  },
  Shower {
    code: "SDA",
    name: "南宝瓶座 δ 流星雨",
    name_en: "Southern Delta-Aquariids",
    activity: (7, 12, 8, 23),
    peak: (7, 31),
    lambda_sun_deg: 128.0,
    ra_deg: 340.0,
    dec_deg: -16.0,
    v_inf_km_s: 41.0,
    r: 2.5,
    zhr: Some(25.0),
  },
  Shower {
    code: "CAP",
    name: "摩羯座 α 流星雨",
    name_en: "Alpha-Capricornids",
    activity: (7, 3, 8, 15),
    peak: (7, 31),
    lambda_sun_deg: 128.0,
    ra_deg: 307.0,
    dec_deg: -10.0,
    v_inf_km_s: 23.0,
    r: 2.5,
    zhr: Some(5.0),
  },
  Shower {
    code: "ERI",
    name: "波江座 η 流星雨",
    name_en: "Eta-Eridanids",
    activity: (7, 31, 8, 19),
    peak: (8, 7),
    lambda_sun_deg: 135.0,
    ra_deg: 41.0,
    dec_deg: -11.0,
    v_inf_km_s: 64.0,
    r: 3.0,
    zhr: Some(3.0),
  },
  Shower {
    code: "PER",
    name: "英仙座流星雨",
    name_en: "Perseids",
    activity: (7, 17, 8, 24),
    peak: (8, 13),
    lambda_sun_deg: 140.0,
    ra_deg: 48.0,
    dec_deg: 58.0,
    v_inf_km_s: 59.0,
    r: 2.2,
    zhr: Some(100.0),
  },
  Shower {
    code: "KCG",
    name: "天鹅座 κ 流星雨",
    name_en: "Kappa-Cygnids",
    activity: (8, 3, 8, 28),
    peak: (8, 17),
    lambda_sun_deg: 144.0,
    ra_deg: 286.0,
    dec_deg: 59.0,
    v_inf_km_s: 23.0,
    r: 3.0,
    zhr: Some(3.0),
  },
  Shower {
    code: "AUR",
    name: "御夫座流星雨",
    name_en: "Aurigids",
    activity: (8, 28, 9, 5),
    peak: (9, 1),
    lambda_sun_deg: 158.6,
    ra_deg: 91.0,
    dec_deg: 39.0,
    v_inf_km_s: 66.0,
    r: 2.5,
    zhr: Some(6.0),
  },
  Shower {
    code: "SPE",
    name: "九月英仙座 ε 流星雨",
    name_en: "September Epsilon-Perseids",
    activity: (9, 5, 9, 21),
    peak: (9, 9),
    lambda_sun_deg: 166.7,
    ra_deg: 48.0,
    dec_deg: 40.0,
    v_inf_km_s: 64.0,
    r: 2.5,
    zhr: Some(8.0),
  },
  Shower {
    code: "SLY",
    name: "九月天猫座流星雨",
    name_en: "September Lyncids",
    activity: (9, 10, 10, 8),
    peak: (9, 13),
    lambda_sun_deg: 170.0,
    ra_deg: 113.0,
    dec_deg: 56.0,
    v_inf_km_s: 60.0,
    r: 3.0,
    zhr: Some(3.0),
  },
  Shower {
    code: "DSX",
    name: "白日六分仪座流星雨",
    name_en: "Daytime Sextantids",
    activity: (9, 20, 10, 6),
    peak: (10, 1),
    lambda_sun_deg: 188.0,
    ra_deg: 156.0,
    dec_deg: -2.0,
    v_inf_km_s: 32.0,
    r: 2.5,
    zhr: Some(5.0),
  },
  Shower {
    code: "OCT",
    name: "十月鹿豹座流星雨",
    name_en: "October Camelopardalids",
    activity: (10, 5, 10, 6),
    peak: (10, 6),
    lambda_sun_deg: 192.58,
    ra_deg: 164.0,
    dec_deg: 79.0,
    v_inf_km_s: 47.0,
    r: 2.5,
    zhr: Some(5.0),
  },
  Shower {
    code: "DRA",
    name: "天龙座流星雨",
    name_en: "Draconids",
    activity: (10, 6, 10, 10),
    peak: (10, 9),
    lambda_sun_deg: 195.4,
    ra_deg: 262.0,
    dec_deg: 54.0,
    v_inf_km_s: 20.0,
    r: 2.6,
    zhr: Some(5.0),
  },
  Shower {
    code: "EGE",
    name: "双子座 ε 流星雨",
    name_en: "Epsilon-Geminids",
    activity: (10, 14, 10, 27),
    peak: (10, 18),
    lambda_sun_deg: 205.0,
    ra_deg: 102.0,
    dec_deg: 27.0,
    v_inf_km_s: 70.0,
    r: 3.0,
    zhr: Some(3.0),
  },
  Shower {
    code: "ORI",
    name: "猎户座流星雨",
    name_en: "Orionids",
    activity: (10, 2, 11, 7),
    peak: (10, 21),
    lambda_sun_deg: 208.0,
    ra_deg: 95.0,
    dec_deg: 16.0,
    v_inf_km_s: 66.0,
    r: 2.5,
    zhr: Some(20.0),
  },
  Shower {
    code: "LMI",
    name: "小狮座流星雨",
    name_en: "Leonis Minorids",
    activity: (10, 19, 10, 27),
    peak: (10, 24),
    lambda_sun_deg: 211.0,
    ra_deg: 162.0,
    dec_deg: 37.0,
    v_inf_km_s: 62.0,
    r: 3.0,
    zhr: Some(2.0),
  },
  Shower {
    code: "STA",
    name: "南金牛座流星雨",
    name_en: "Southern Taurids",
    activity: (9, 20, 11, 20),
    peak: (11, 5),
    lambda_sun_deg: 223.0,
    ra_deg: 52.0,
    dec_deg: 15.0,
    v_inf_km_s: 27.0,
    r: 2.3,
    zhr: Some(7.0),
  },
  Shower {
    code: "NTA",
    name: "北金牛座流星雨",
    name_en: "Northern Taurids",
    activity: (10, 20, 12, 10),
    peak: (11, 12),
    lambda_sun_deg: 230.0,
    ra_deg: 58.0,
    dec_deg: 22.0,
    v_inf_km_s: 29.0,
    r: 2.3,
    zhr: Some(5.0),
  },
  Shower {
    code: "LEO",
    name: "狮子座流星雨",
    name_en: "Leonids",
    activity: (11, 6, 11, 30),
    peak: (11, 17),
    lambda_sun_deg: 235.27,
    ra_deg: 152.0,
    dec_deg: 22.0,
    v_inf_km_s: 71.0,
    r: 2.5,
    zhr: Some(15.0),
  },
  Shower {
    code: "AMO",
    name: "麒麟座 α 流星雨",
    name_en: "Alpha-Monocerotids",
    activity: (11, 15, 11, 25),
    peak: (11, 22),
    lambda_sun_deg: 239.32,
    ra_deg: 117.0,
    dec_deg: 1.0,
    v_inf_km_s: 65.0,
    r: 2.4,
    zhr: None,
  },
  Shower {
    code: "NOO",
    name: "十一月猎户座流星雨",
    name_en: "November Orionids",
    activity: (11, 13, 12, 6),
    peak: (11, 28),
    lambda_sun_deg: 246.0,
    ra_deg: 91.0,
    dec_deg: 16.0,
    v_inf_km_s: 44.0,
    r: 3.0,
    zhr: Some(3.0),
  },
  Shower {
    code: "PHO",
    name: "凤凰座流星雨",
    name_en: "Phoenicids",
    activity: (12, 1, 12, 5),
    peak: (12, 2),
    lambda_sun_deg: 249.5,
    ra_deg: 8.0,
    dec_deg: -27.0,
    v_inf_km_s: 15.0,
    r: 2.8,
    zhr: None,
  },
  Shower {
    code: "PUP",
    name: "船尾座–船帆座流星雨",
    name_en: "Puppid-Velids",
    activity: (12, 1, 12, 15),
    // IMO 用括号标出：这里的日期只是辐射点的参考时刻，不一定真有峰值。
    peak: (12, 7),
    lambda_sun_deg: 255.0,
    ra_deg: 123.0,
    dec_deg: -45.0,
    v_inf_km_s: 44.0,
    r: 2.9,
    zhr: Some(10.0),
  },
  Shower {
    code: "MON",
    name: "麒麟座流星雨",
    name_en: "Monocerotids",
    activity: (12, 1, 12, 19),
    peak: (12, 9),
    lambda_sun_deg: 257.0,
    ra_deg: 100.0,
    dec_deg: 8.0,
    v_inf_km_s: 41.0,
    r: 3.0,
    zhr: Some(3.0),
  },
  Shower {
    code: "HYD",
    name: "长蛇座 σ 流星雨",
    name_en: "Sigma-Hydrids",
    activity: (12, 3, 12, 20),
    peak: (12, 9),
    lambda_sun_deg: 257.0,
    ra_deg: 125.0,
    dec_deg: 2.0,
    v_inf_km_s: 58.0,
    r: 3.0,
    zhr: Some(7.0),
  },
  Shower {
    code: "GEM",
    name: "双子座流星雨",
    name_en: "Geminids",
    activity: (12, 4, 12, 20),
    peak: (12, 14),
    lambda_sun_deg: 262.2,
    ra_deg: 112.0,
    dec_deg: 33.0,
    v_inf_km_s: 35.0,
    r: 2.6,
    zhr: Some(150.0),
  },
  Shower {
    code: "URS",
    name: "小熊座流星雨",
    name_en: "Ursids",
    activity: (12, 17, 12, 26),
    peak: (12, 22),
    lambda_sun_deg: 270.7,
    ra_deg: 217.0,
    dec_deg: 76.0,
    v_inf_km_s: 33.0,
    r: 2.8,
    zhr: Some(10.0),
  },
  Shower {
    code: "COM",
    name: "后发座贝伦尼斯流星雨",
    name_en: "Comae Berenicids",
    activity: (12, 4, 1, 30),
    peak: (12, 23),
    lambda_sun_deg: 271.0,
    ra_deg: 164.0,
    dec_deg: 29.0,
    v_inf_km_s: 65.0,
    r: 3.0,
    zhr: Some(3.0),
  },
];

/// 时刻表默认的「即将到峰值」窗口（天）：峰值在这之内的流星雨会被列出（正在活动的无论如何都列）。
pub const WINDOW_DAYS: i64 = 30;

/// 时刻表里的一行。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimetableEntry {
  /// 流星雨。
  pub shower: &'static Shower,
  /// 距离下一次峰值还有多少天（峰值当天为 0；峰值已过则指向**明年**，所以可能是三位数）。
  pub days: i64,
  /// 今天是否在活动期内（含首尾两天）。
  pub active: bool,
  /// 正在活动、但**这一轮的峰值已经过了**（`days` 指向明年那一次）。
  /// 页面据此写「峰值已过」，而不是一个容易读成「还有一年才峰值」的 362 天。
  pub peak_passed: bool,
}

/// 公历闰年。
fn is_leap(year: i32) -> bool {
  (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

/// 某年某月的天数（`month` 须在 1–12）。
fn days_in_month(year: i32, month: u32) -> u32 {
  match month {
    4 | 6 | 9 | 11 => 30,
    2 if is_leap(year) => 29,
    2 => 28,
    _ => 31,
  }
}

/// `(年, 月, 日)` 是不是一个真实存在的公历日期。
///
/// 外部给进来的日期先过这一关：下面的日序号换算对越界的月份会下标越界（wasm 里等于整页 panic）。
fn is_valid_date((year, month, day): (i32, u32, u32)) -> bool {
  (1..=12).contains(&month) && (1..=days_in_month(year, month)).contains(&day)
}

/// 一年中的第几天（1–366），纯算术，不依赖时区。`month` 须在 1–12（见 [`is_valid_date`]）。
fn day_of_year(year: i32, month: u32, day: u32) -> i64 {
  let cum = [0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334];
  i64::from(cum[(month - 1) as usize]) + i64::from(day) + i64::from(is_leap(year) && month > 2)
}

/// 从 `today` 起（含当天）到下一个「月 / 日」还有多少天：今年还没过就算今年，过了就落到明年。
/// 同一个月 / 日按「年年相同」处理，见模块文档里「峰值日只对 2026 年精确」的说明。
fn days_until(today: (i32, u32, u32), (month, day): (u32, u32)) -> i64 {
  let (y, m, d) = today;
  let now = day_of_year(y, m, d);
  let this_year = day_of_year(y, month, day);
  if this_year >= now {
    this_year - now
  } else {
    // 跨年：今年剩下的天数 + 明年那一天的序号（明年是不是闰年由 `day_of_year` 自己算）。
    day_of_year(y, 12, 31) - now + day_of_year(y + 1, month, day)
  }
}

/// 某个日期是否落在该流星雨的活动期内（含首尾两天）。
fn is_active(shower: &Shower, today: (i32, u32, u32)) -> bool {
  let (y, m, d) = today;
  let (sm, sd, em, ed) = shower.activity;
  let n = day_of_year(y, m, d);
  let start = day_of_year(y, sm, sd);
  let end = day_of_year(y, em, ed);
  if start <= end {
    (start..=end).contains(&n)
  } else {
    // 跨年活动期：年初那一段（属于上一年开始的活动）与年末那一段都算。
    n >= start || n <= end
  }
}

/// 流星雨时刻表：正在活动的，加上峰值在 `within_days` 天内的，按距下次峰值的天数排列
/// （同一天按代号排，结果确定）。
///
/// `today` = `(年, 月, 日)`（UTC）。日期不是真实存在的公历日期时返回空表，不 panic。
#[must_use]
pub fn timetable(today: (i32, u32, u32), within_days: i64) -> Vec<TimetableEntry> {
  if !is_valid_date(today) {
    return Vec::new();
  }
  let mut out: Vec<TimetableEntry> = SHOWERS
    .iter()
    .map(|shower| {
      let active = is_active(shower, today);
      let days = days_until(today, shower.peak);
      // 活动期还剩多少天：下一次峰值比活动期的尽头还远，说明这一轮的峰值已经过了。
      let peak_passed = active && days > days_until(today, (shower.activity.2, shower.activity.3));
      TimetableEntry {
        shower,
        days,
        active,
        peak_passed,
      }
    })
    .filter(|e| e.active || e.days <= within_days)
    .collect();
  out.sort_by(|a, b| {
    a.days
      .cmp(&b.days)
      .then_with(|| a.shower.code.cmp(b.shower.code))
  });
  out
}

#[cfg(test)]
mod tests {
  use super::*;

  fn by_code(code: &str) -> &'static Shower {
    SHOWERS
      .iter()
      .find(|s| s.code == code)
      .unwrap_or_else(|| panic!("{code} 应收录"))
  }

  #[test]
  fn table_has_the_37_showers_of_imo_table_5() {
    // IMO 表 5 共 38 行，去掉全年弥散的 Antihelion Source（见模块文档）后是 37 条。
    assert_eq!(SHOWERS.len(), 37);
  }

  #[test]
  fn table_is_consistent() {
    for s in SHOWERS {
      assert_eq!(s.code.len(), 3, "{}: 代号应为三个字母", s.code);
      assert!(!s.name.is_empty() && !s.name_en.is_empty());
      for (m, d) in [
        (s.peak.0, s.peak.1),
        (s.activity.0, s.activity.1),
        (s.activity.2, s.activity.3),
      ] {
        // 2028 是闰年：2 月 29 日也算合法，但表里不该出现。
        assert!(
          is_valid_date((2027, m, d)),
          "{}: {m}/{d} 不是真实日期",
          s.code
        );
      }
      assert!((0.0..360.0).contains(&s.lambda_sun_deg), "{}", s.code);
      assert!((0.0..360.0).contains(&s.ra_deg), "{}", s.code);
      assert!((-90.0..=90.0).contains(&s.dec_deg), "{}", s.code);
      assert!(s.v_inf_km_s > 0.0, "{}", s.code);
      assert!(s.r >= 2.0, "{}: 星等指数应不小于 2", s.code);
      // 活动期必须跨过峰值日。
      let y = 2026;
      let start = day_of_year(y, s.activity.0, s.activity.1);
      let end = day_of_year(y, s.activity.2, s.activity.3);
      let peak = day_of_year(y, s.peak.0, s.peak.1);
      if start <= end {
        assert!(
          (start..=end).contains(&peak),
          "{}: 峰值 {peak} 不在活动期 {start}..={end}",
          s.code
        );
      } else {
        assert!(
          peak >= start || peak <= end,
          "{}: 峰值 {peak} 不在跨年活动期",
          s.code
        );
      }
    }
  }

  #[test]
  fn codes_are_unique() {
    let mut codes: Vec<&str> = SHOWERS.iter().map(|s| s.code).collect();
    codes.sort_unstable();
    let n = codes.len();
    codes.dedup();
    assert_eq!(codes.len(), n, "代号重复");
  }

  #[test]
  fn peaks_are_in_calendar_order() {
    // 表里是按峰值先后排的：便于页面直接按季节读。
    let keys: Vec<i64> = SHOWERS
      .iter()
      .map(|s| day_of_year(2026, s.peak.0, s.peak.1))
      .collect();
    assert!(keys.windows(2).all(|w| w[0] <= w[1]), "{keys:?}");
  }

  #[test]
  fn dates_are_validated_including_leap_days() {
    assert!(is_valid_date((2028, 2, 29)));
    assert!(!is_valid_date((2027, 2, 29)));
    assert!(!is_valid_date((1900, 2, 29)), "1900 不是闰年");
    assert!(is_valid_date((2000, 2, 29)));
    assert!(is_valid_date((2026, 12, 31)));
    for bad in [(2026, 0, 1), (2026, 13, 1), (2026, 4, 31), (2026, 1, 0)] {
      assert!(!is_valid_date(bad), "{bad:?}");
    }
  }

  #[test]
  fn days_until_counts_across_the_new_year_and_leap_years() {
    let gem = by_code("GEM").peak;
    assert_eq!(days_until((2026, 12, 1), gem), 13);
    assert_eq!(days_until((2026, 12, 14), gem), 0, "峰值当天算 0 天");
    let qua = by_code("QUA").peak;
    // 2026-11-12 → 2027-01-03：11 月剩 18 天 + 12 月 31 天 + 1 月 3 天 = 52。
    assert_eq!(days_until((2026, 11, 12), qua), 52);
    assert_eq!(days_until((2027, 1, 3), qua), 0);
    assert_eq!(
      days_until((2027, 1, 4), qua),
      364,
      "到 2028-01-03：2027 不是闰年"
    );
    // 2027-12-31 → 2028-03-01：1 月 31 + 2 月 29（2028 闰年）+ 1 = 61 天。
    assert_eq!(days_until((2027, 12, 31), (3, 1)), 61);
    assert_eq!(days_until((2028, 2, 28), (3, 1)), 2, "闰年的 2 月有 29 天");
    assert_eq!(days_until((2027, 2, 28), (3, 1)), 1);
  }

  #[test]
  fn activity_window_includes_edges_and_handles_year_wrap() {
    let qua = by_code("QUA");
    // 活动期 12 月 28 – 1 月 12：年初与年末两段都算。
    assert!(is_active(qua, (2026, 1, 1)));
    assert!(is_active(qua, (2026, 1, 12)));
    assert!(!is_active(qua, (2026, 1, 13)));
    assert!(is_active(qua, (2026, 12, 28)));
    assert!(is_active(qua, (2026, 12, 31)));
    assert!(!is_active(qua, (2026, 12, 27)));
    let gem = by_code("GEM");
    assert!(is_active(gem, (2026, 12, 14)));
    assert!(!is_active(gem, (2026, 12, 21)));
  }

  /// 与 `e2e/tests/meteor_showers.spec.ts` 同一天（2026-12-12）：7 行、顺序、活动标记、「峰值已过」。
  #[test]
  fn timetable_on_2026_12_12_matches_the_hand_computed_rows() {
    let rows = timetable((2026, 12, 12), WINDOW_DAYS);
    let got: Vec<(&str, i64, bool, bool)> = rows
      .iter()
      .map(|e| (e.shower.code, e.days, e.active, e.peak_passed))
      .collect();
    assert_eq!(
      got,
      [
        ("GEM", 2, true, false),   // 12/14，活动期 12/4–12/20
        ("URS", 10, false, false), // 12/22，12/17 才开始
        ("COM", 11, true, false),  // 12/23，活动期跨年 12/4–1/30
        ("QUA", 22, false, false), // 1/3，12/28 才开始
        ("PUP", 360, true, true),  // 12/7 已过、活动期到 12/15
        ("HYD", 362, true, true),  // 12/9 已过、活动期到 12/20
        ("MON", 362, true, true),  // 12/9 已过、活动期到 12/19；同为 362 天按代号排，HYD 在前
      ]
    );
  }

  #[test]
  fn peak_day_itself_is_not_passed_and_the_day_after_is() {
    let at = |today| {
      timetable(today, WINDOW_DAYS)
        .into_iter()
        .find(|e| e.shower.code == "GEM")
        .unwrap()
    };
    let on_peak = at((2026, 12, 14));
    assert_eq!(
      (on_peak.days, on_peak.active, on_peak.peak_passed),
      (0, true, false)
    );
    let after = at((2026, 12, 15));
    assert!(
      after.active && after.peak_passed && after.days > 300,
      "{after:?}"
    );
  }

  #[test]
  fn timetable_is_sorted_and_only_lists_active_or_soon_showers() {
    for today in [
      (2026, 1, 1),
      (2026, 5, 20),
      (2026, 8, 12),
      (2026, 11, 12),
      (2027, 2, 28),
    ] {
      let rows = timetable(today, WINDOW_DAYS);
      assert!(
        rows.iter().all(|e| e.active || e.days <= WINDOW_DAYS),
        "{today:?}"
      );
      let keys: Vec<(i64, &str)> = rows.iter().map(|e| (e.days, e.shower.code)).collect();
      assert!(keys.windows(2).all(|w| w[0] <= w[1]), "{today:?}: {keys:?}");
    }
    // 2026-11-12 是金牛座北支的峰值日：排第一、峰值今天。
    let rows = timetable((2026, 11, 12), WINDOW_DAYS);
    assert_eq!(rows[0].shower.code, "NTA");
    assert_eq!(rows[0].days, 0);
  }

  #[test]
  fn invalid_today_yields_an_empty_table_instead_of_panicking() {
    for bad in [(2026, 0, 12), (2026, 13, 1), (2026, 2, 30), (2026, 6, 0)] {
      assert!(timetable(bad, WINDOW_DAYS).is_empty(), "{bad:?}");
    }
  }
}
