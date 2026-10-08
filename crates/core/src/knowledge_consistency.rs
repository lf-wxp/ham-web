//! 知识库跨模块一致性回归测试。
//!
//! 针对历史上出现过的「同一事实在多文件中不一致」问题，防止修复后被再次改坏。
//! 断言以第一性原理（能量守恒、正割定律、Friis、香农信息论、ITU RR / 47 CFR 等）
//! 逐条复算库中数值与公式，把结论固化为不变量。

use crate::amplifier;
use crate::bandplan;
use crate::bands;
use crate::coordination;
use crate::feedline;
use crate::filter_design;
use crate::frequencies;
use crate::license_classes;
use crate::mode_encoder;
use crate::propagation;
use crate::reference;
use crate::rf_exposure;
use crate::spectrum;
use crate::sstv;
use crate::voacap;

/// 2 米波段的区际划分不应把第三区写成 144–146。
#[test]
fn two_meter_region_range_consistent() {
  let row = coordination::IARU_BAND_DIFFS
    .iter()
    .find(|(b, _, _)| *b == "2m")
    .expect("IARU 区际差异应包含 2m");
  assert!(row.2.contains("148"), "三区应为 144–148：{}", row.2);
  assert!(row.1.contains("146"), "一区应为 144–146：{}", row.1);
}

/// 10m 属短波，不应出现在「30MHz 以上」的 A 类权限行。
#[test]
fn ten_meter_is_below_30mhz() {
  for (band, class) in license_classes::BAND_PERMISSIONS {
    if band.contains("10m") {
      assert!(
        !class.contains('A'),
        "10m 属短波（<30MHz），A 类不得使用：{band} -> {class}"
      );
    }
  }
}

/// D/E/F1/F2 层高度应逐层递增。
#[test]
fn ionosphere_layers_are_ordered() {
  let order = ["D 层", "E 层", "F1 层", "F2 层"];
  let nums: Vec<f64> = order
    .iter()
    .map(|name| {
      let (_, range, _) = propagation::LAYERS
        .iter()
        .find(|(n, _, _)| n == name)
        .unwrap_or_else(|| panic!("缺少 {name}"));
      range
        .trim_start_matches('约')
        .trim()
        .split('–')
        .next()
        .unwrap()
        .trim()
        .trim_end_matches("km")
        .trim()
        .parse::<f64>()
        .unwrap_or_else(|_| panic!("无法解析层高 {range}"))
    })
    .collect();
  for w in nums.windows(2) {
    assert!(w[0] < w[1], "电离层高度应递增：{nums:?}");
  }
}

/// ISS SSTV 下行必须是 145.800MHz，而非跨段中继下行的 437.800MHz。
#[test]
fn iss_sstv_uses_145_800() {
  assert!(
    sstv::SSTV_TIPS.iter().any(|t| t.contains("145.800")),
    "SSTV 提示应指出 145.800MHz"
  );
  assert!(
    !sstv::SSTV_TIPS.iter().any(|t| t.contains("用 437.800")),
    "SSTV 不应使用 437.800MHz"
  );
}

/// 频谱幅度标定：幅度 0.5 的实正弦应读到约 −6dB（已补偿 Hann 窗相干增益）。
#[test]
fn spectrum_amplitude_is_calibrated() {
  let samples = spectrum::synthesize(8000, 1.0, &[(1000.0, 0.5)], 0.0);
  let db = spectrum::spectrum_db(&samples);
  let peak = db.iter().copied().fold(f32::MIN, f32::max);
  assert!(
    (-8.0..=-4.0).contains(&peak),
    "0.5 幅度单音应约 −6dB，实际 {peak} dB（未补偿窗增益时会偏低约 6dB）"
  );
}

/// `band_of` 的波段边界必须与 `bands.rs`（中国 / ITU 三区划分）一致。
///
/// 历史上 `band_of` 用的是二区值（80m 到 4.0、40m 到 7.3），而 `bands.rs` /
/// `bandplan.rs` 是三区值；只靠「内容非空」的测试抓不到这种运行期函数的偏差。
#[test]
fn band_of_matches_region_three_limits() {
  // 80m 上界 3.9（不是二区的 4.0）。
  assert_eq!(frequencies::band_of(3.8), "80m");
  assert_ne!(frequencies::band_of(3.95), "80m", "80m 上界应为 3.9MHz");
  // 40m 上界 7.2（不是二区的 7.3）。
  assert_eq!(frequencies::band_of(7.1), "40m");
  assert_ne!(frequencies::band_of(7.25), "40m", "40m 上界应为 7.2MHz");
  // 17m 下界 18.068（不是 18.0）。
  assert_eq!(frequencies::band_of(18.1), "17m");
  assert_ne!(frequencies::band_of(18.05), "17m", "17m 下界应为 18.068MHz");
  // 60m 上界 5.3665，与 bands.rs 一致。
  assert_eq!(frequencies::band_of(5.36), "60m");
  assert_ne!(frequencies::band_of(5.3666), "60m");
  // 10m 属短波（<30MHz），不得归到 VHF。
  assert_eq!(frequencies::band_of(29.0), "10m");
}

/// B 类在 30MHz 以下的功率上限必须低于 A 类 —— 否则「A 类功率上限最低」的说法
/// 会让考生把两类的功率大小记反。
#[test]
fn class_b_hf_power_is_below_class_a() {
  // 数值层面：B 类 30MHz 以下 ≤15W，A 类 ≤25W —— 最低限值属于 B 类（编译期即可确定）。
  // 引用全站唯一事实来源，避免各处硬编码漂移。
  const A_HF_MAX: f64 = license_classes::CLASS_A_MAX_W;
  const B_HF_MAX: f64 = license_classes::CLASS_B_HF_MAX_W;
  const {
    assert!(B_HF_MAX < A_HF_MAX, "最低功率限值应属于 B 类");
  }
  // 文本层面：不得再出现「A 类…功率上限最低」这类表述。
  for tip in license_classes::CLASS_TIPS {
    assert!(
      !tip.contains("A 类") || !tip.contains("功率上限最低"),
      "A 类并非功率上限最低（B 类 HF ≤15W 更低）：{tip}"
    );
  }
}

/// 电离层分层高度的具体数值：只校验「逐层递增」抓不到 F1/F2 边界写偏。
#[test]
fn ionosphere_layer_bounds_match_textbook() {
  let height_of = |name: &str| -> (f64, f64) {
    let (_, range, _) = propagation::LAYERS
      .iter()
      .find(|(n, _, _)| *n == name)
      .unwrap_or_else(|| panic!("缺少 {name}"));
    let nums: Vec<f64> = range
      .trim_start_matches('约')
      .trim()
      .split('–')
      .map(|p| {
        p.trim()
          .trim_end_matches("km")
          .trim()
          .parse::<f64>()
          .unwrap_or_else(|_| panic!("无法解析层高 {range}"))
      })
      .collect();
    (nums[0], nums[1])
  };
  assert_eq!(height_of("D 层"), (60.0, 90.0));
  assert_eq!(height_of("E 层"), (90.0, 150.0));
  // 国际主流口径：F1 150–250、F2 250–400（中文教材另有 150–200 / 200–400 的写法，
  // 修改此处前请先核对题库标准答案）。
  let f1 = height_of("F1 层");
  let f2 = height_of("F2 层");
  assert!(f1.0 <= 150.0 && f1.1 >= 200.0, "F1 {f1:?}");
  assert!(f2.0 >= 200.0 && f2.1 >= 400.0, "F2 {f2:?}");
  assert!(f1.1 <= f2.1, "F1 上限不应超过 F2 上限");
}

/// FCC 非受控环境在 1.34MHz 处换用 `180/f²`，受控环境则到 3.0MHz 为止。
///
/// 两条曲线共用 180/f² 会让 160m 波段的公众限值放宽最多 5 倍、安全距离被低估约 2 倍。
#[test]
fn rf_exposure_uncontrolled_mf_corner_differs_from_controlled() {
  assert!((rf_exposure::mpe_limit_mw_cm2(1.0, false) - 100.0).abs() < 1e-9);
  assert!((rf_exposure::mpe_limit_mw_cm2(1.0, true) - 100.0).abs() < 1e-9);
  // 1.34MHz 是非受控的分界：以下 100、以上 180/f²。
  assert!(rf_exposure::mpe_limit_mw_cm2(1.5, false) < 100.0);
  assert!((rf_exposure::mpe_limit_mw_cm2(1.5, true) - 100.0).abs() < 1e-9);
  // 160m 波段内（1.8MHz）非受控不应还是 100。
  assert!(rf_exposure::mpe_limit_mw_cm2(1.8, false) < 60.0);
  assert!((rf_exposure::mpe_limit_mw_cm2(3.0, false) - 20.0).abs() < 1e-9);
  assert!((rf_exposure::mpe_limit_mw_cm2(3.0, true) - 100.0).abs() < 1e-9);
}

/// 带阻必须在带边给出归一化值 = gₖ；只看 L·C 乘积（=1/ω0²）抓不到 gₖ 与 1/gₖ 用反。
#[test]
fn bandstop_band_edge_immittance_equals_g() {
  let (f0, bw, z0): (f64, f64, f64) = (7.1e6, 0.5e6, 50.0);
  let f_upper = (bw + (bw * bw + 4.0 * f0 * f0).sqrt()) / 2.0;
  let w = std::f64::consts::TAU * f_upper;
  let stages = filter_design::design_bandstop(3, f0, bw, z0);
  let g = filter_design::butterworth_g(3).expect("g 表");
  for (stage, &gk) in stages.iter().zip(g.iter()) {
    let l = stage.l_uh.expect("L") * 1e-6;
    let c = stage.c_pf.expect("C") * 1e-12;
    // 串联臂要 |z|/Z₀，并联臂要 |y|·Z₀；并联 LC 天然给 y、串联 LC 天然给 z。
    let got = match (stage.series, stage.topology) {
      (true, filter_design::ResonatorTopology::SeriesLc) => (w * l - 1.0 / (w * c)).abs() / z0,
      (true, filter_design::ResonatorTopology::ParallelLc) => {
        1.0 / ((w * c - 1.0 / (w * l)).abs() * z0)
      }
      (false, filter_design::ResonatorTopology::ParallelLc) => (w * c - 1.0 / (w * l)).abs() * z0,
      (false, filter_design::ResonatorTopology::SeriesLc) => z0 / (w * l - 1.0 / (w * c)).abs(),
      (_, filter_design::ResonatorTopology::Single) => f64::NAN,
    };
    assert!(
      (got - gk).abs() / gk < 1e-6,
      "带阻带边归一化值应等于 gₖ={gk}，实际 {got}（gₖ 与 1/gₖ 用反时会差 gₖ² 倍）"
    );
  }
}

/// ITA2 的 CR / LF 不得互换（标准低位在先写法：CR = 01000、LF = 00010）。
#[test]
fn baudot_cr_and_lf_are_not_swapped() {
  assert_eq!(mode_encoder::letter_code('\r'), Some(0b01000));
  assert_eq!(mode_encoder::letter_code('\n'), Some(0b00010));
}

/// MUF 因子必须随单跳距离**增大**（正割定律），不能写成「仰角越低因子越小」。
#[test]
fn muf_factor_increases_with_hop_distance() {
  let f500 = voacap::muf_factor_for_hop(500.0);
  let f3000 = voacap::muf_factor_for_hop(3000.0);
  assert!(f500 < f3000, "仰角越低（跨距越远）因子越大：{f500} {f3000}");
  assert!(f500 < 1.5, "500km 因子约 1.3，不能当成 3.0：{f500}");
  assert!((3.0..=3.4).contains(&f3000), "M(3000) 约 3.3：{f3000}");
}

/// 馈线损耗表必须给出可用数值，且短路径不能被高估到不切实际的水平。
#[test]
fn feedline_loss_table_is_usable() {
  for spec in feedline::FEEDLINE_SPECS {
    assert!(!spec.loss.is_empty(), "{} 缺损耗数据", spec.name);
    assert!(
      (0.0..=1.0).contains(&spec.velocity_factor),
      "{} 速度因子 {:.2}",
      spec.name,
      spec.velocity_factor
    );
    // 损耗必须随频率单调上升。
    for w in spec.loss.windows(2) {
      assert!(w[1].1 > w[0].1, "{} 损耗未随频率上升", spec.name);
    }
  }
  let rg58 = feedline::feedline_spec("RG-58").expect("RG-58");
  let d14 = feedline::feedline_loss_db(rg58, 14.0, 100.0);
  let d28 = feedline::feedline_loss_db(rg58, 28.0, 100.0);
  assert!(
    (5.0..=7.5).contains(&d14),
    "RG-58 @14MHz 约 6dB/100m：{d14}"
  );
  assert!(d28 > d14, "损耗应随频率上升");
}

/// 12m 与 17m 同属含卫星业余业务的 WARC 段，标注必须一致。
#[test]
fn warc_bands_share_satellite_allocation() {
  assert!(
    bands::satellite_count() >= 19,
    "12m 与 17m 均含卫星业余业务，数量应 ≥19：{}",
    bands::satellite_count()
  );
}

// ---------------------------------------------------------------------------
// 2026-10 知识库审查新增回归：把已修正的硬错误固化为不变量，防止被再次改坏。
// ---------------------------------------------------------------------------

/// A 类功放的**阻性负载**效率上限是 25%，不是 50%（只有变压器 / 电感耦合才到 50%）。
#[test]
fn power_amp_class_a_resistive_efficiency() {
  let class = amplifier::PA_CLASSES
    .iter()
    .find(|c| c.0 == "A 类")
    .expect("应有 A 类功放");
  assert!(class.1.contains("25%"), "A 类效率区间应含 25%：{}", class.1);
  assert!(
    !class.2.contains("阻性负载理论上限 50%"),
    "A 类阻性负载上限应为 25%（50% 仅变压器 / 电感耦合可达）：{}",
    class.2
  );
  assert!(class.2.contains("25%"), "A 类说明应写明 25%：{}", class.2);
}

/// 同一标称分辨率下，Scottie DX 的时长约为 Scottie 1 的 2.5 倍（而非 4 倍）。
#[test]
fn sstv_scottie_dx_duration_ratio() {
  fn seconds(name: &str) -> f64 {
    let desc = sstv::SSTV_MODES
      .iter()
      .find(|m| m.0 == name)
      .unwrap_or_else(|| panic!("缺少 SSTV 模式 {name}"))
      .1;
    let tail = desc.split('约').nth(1).unwrap_or(desc);
    tail
      .trim_start()
      .chars()
      .take_while(|c| c.is_ascii_digit())
      .collect::<String>()
      .parse()
      .unwrap_or_else(|_| panic!("无法解析时长：{desc}"))
  }
  let ratio = seconds("Scottie DX") / seconds("Scottie 1");
  assert!(
    (2.2..=2.8).contains(&ratio),
    "Scottie DX / Scottie 1 时长比应约 2.5，实际 {ratio:.2}"
  );
  let note = sstv::SSTV_MODES
    .iter()
    .find(|m| m.0 == "Scottie DX")
    .unwrap()
    .2;
  assert!(
    !note.contains("约 4 倍"),
    "Scottie DX 不应再写「约 4 倍」：{note}"
  );
}

/// 卫星业余业务标志由 `bands::AMATEUR_SATELLITE_RANGES_MHZ`（ITU 表内划分 + 脚注 5.282）
/// 唯一派生：引用 5.282 的划分必须判为卫星业务，且派生值与频率范围判断函数一致。
#[test]
fn satellite_flag_derives_from_itu_ranges() {
  for band in bands::BANDS {
    for i in 0..band.allocations.len() {
      let alloc = &band.allocations[i];
      let cites = band
        .remark_for(i)
        .iter()
        .any(|n| matches!(n, bands::Note::Ref("5.282")));
      if cites {
        assert!(
          alloc.is_satellite(),
          "{} {} 引用 5.282 却未判为卫星业务",
          band.number,
          alloc.range
        );
      }
      // 派生结果必须与独立调用频率范围判断函数一致（已无手工字段可漂移）。
      assert_eq!(alloc.is_satellite(), bands::is_satellite_range(alloc.range));
    }
  }
  // 脚注 5.282 的五个子段都应落在卫星业务判定内。
  for r in [
    "435-438MHz",
    "1260-1270MHz",
    "2400-2450MHz",
    "3400-3410MHz",
    "5650-5670MHz",
  ] {
    assert!(
      bands::is_satellite_range(r),
      "5.282 子段 {r} 应判为卫星业务"
    );
  }
  assert_eq!(bands::satellite_count(), 24, "卫星业余业务划分应为 24 条");
}

/// B 类 30MHz 以下功率限值是「不大于 15W」（含边界），不得写成严格不等号。
#[test]
fn class_b_power_limit_is_inclusive() {
  for (name, text) in license_classes::CLASS_USAGE {
    assert!(!text.contains("<15W"), "{name} 不应使用严格不等号：{text}");
  }
  for tip in license_classes::CLASS_TIPS {
    assert!(!tip.contains("<15W"), "备考要点不应使用严格不等号：{tip}");
  }
  let b = reference::LICENSE_CLASSES
    .iter()
    .find(|c| c.class == "B")
    .expect("应有 B 类");
  assert!(
    !b.power.contains("< 15W") && !b.power.contains("<15W"),
    "B 类功率应为「不大于 15W」：{}",
    b.power
  );
}

/// PSK31 的 20m 呼叫频率是 14.070MHz（不是偏门的 14.071）。
#[test]
fn psk31_calling_frequency() {
  let found = frequencies::FREQ_GROUPS
    .iter()
    .flat_map(|g| g.freqs)
    .any(|(f, usage)| usage.contains("PSK31") && f.starts_with("14.070"));
  assert!(found, "PSK31 呼叫频率应为 14.070 MHz");
}

/// 波段规划应包含 60m，且 2m 子段覆盖到 148MHz。
#[test]
fn bandplan_covers_registered_bands() {
  assert!(
    bandplan::BAND_PLANS.iter().any(|b| b.band == "60m"),
    "波段规划应包含 60m"
  );
  let two = bandplan::BAND_PLANS
    .iter()
    .find(|b| b.band == "2m")
    .expect("应有 2m 规划");
  assert!(
    two.segments.iter().any(|seg| seg.range.contains("148")),
    "2m 规划子段应覆盖到 148MHz"
  );
}

/// `band_of` 的边界必须与 `bands::AMATEUR_BAND_EDGES`（唯一事实来源）一致，
/// 且每个波段都在 `bandplan::BAND_PLANS` 中有规划。
#[test]
fn amateur_band_edges_are_single_source() {
  for &(name, lo, hi) in bands::AMATEUR_BAND_EDGES {
    // 半开区间：含下限、不含上限。
    assert_eq!(frequencies::band_of(lo), name, "{name} 应含下限 {lo}MHz");
    assert_eq!(
      frequencies::band_of((lo + hi) / 2.0),
      name,
      "{name} 区间中点应归属该波段"
    );
    assert_ne!(frequencies::band_of(hi), name, "{name} 不应含上限 {hi}MHz");
    assert!(
      bandplan::BAND_PLANS.iter().any(|b| b.band == name),
      "波段 {name} 缺少对应的波段规划"
    );
  }
}

/// 75Ω 同轴的速度因子须按介质区分，不能把 RG-59 与泡沫 RG-6 混为 0.82。
#[test]
fn feedline_75ohm_velocity_factor() {
  let entry = feedline::FEEDLINES
    .iter()
    .find(|e| e.0.contains("RG-59"))
    .expect("应有 75Ω 同轴条目");
  assert!(
    entry.2.contains("0.66") && entry.2.contains("0.82"),
    "75Ω 同轴应区分 RG-59（0.66）与 RG-6（0.82）：{}",
    entry.2
  );
}
