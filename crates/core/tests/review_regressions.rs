use ham_web_core::cabrillo_import::{drop_logged, parse_cabrillo};
use ham_web_core::logbook::{LogEntry, Logbook};
use ham_web_core::station::StationBook;

fn book() -> StationBook {
  let mut book = StationBook::default();
  book.profiles[0].callsign = "BG4ABC".into();
  book
}

#[test]
fn microwave_qsos_keep_separate_bands() {
  let parsed = parse_cabrillo(
    "CONTEST: CQ-WW-CW\nQSO: 3400100 CW 2026-10-31 0010 BG4ABC 599 24 W1AW 599 5\nQSO: 5760100 CW 2026-10-31 0010 BG4ABC 599 24 W1AW 599 5\n",
  );
  assert_eq!(parsed.records[0].band, "9cm");
  assert_eq!(parsed.records[1].band, "6cm");
  assert_eq!(
    Logbook::default().import(parsed.records, &mut book()),
    (2, 0)
  );
}

#[test]
fn fm_and_ssb_are_not_the_same_import() {
  let existing = LogEntry {
    callsign: "W1AW".into(),
    date: "2026-10-31".into(),
    time: "00:10".into(),
    freq: "144.300".into(),
    mode: "SSB".into(),
    ..Default::default()
  };
  let parsed = parse_cabrillo("QSO: 144 FM 2026-10-31 0010 BG4ABC 59 24 W1AW 59 5\n");
  let (kept, dropped) = drop_logged(parsed.records, &[existing], &book());
  assert_eq!((kept.len(), dropped), (1, 0));
}

#[test]
fn both_import_filters_preserve_different_stations() {
  let existing = LogEntry {
    callsign: "W1AW".into(),
    date: "2026-10-31".into(),
    time: "00:10".into(),
    freq: "14.021".into(),
    mode: "CW".into(),
    ..Default::default()
  };
  let parsed = parse_cabrillo("QSO: 14021 CW 2026-10-31 0010 BG4XYZ 599 24 W1AW 599 5\n");
  let mut stations = book();
  let (kept, dropped) = drop_logged(parsed.records, std::slice::from_ref(&existing), &stations);
  assert_eq!((kept.len(), dropped), (1, 0));
  let mut log = Logbook {
    entries: vec![existing],
  };
  assert_eq!(log.import(kept, &mut stations), (1, 0));
  assert_ne!(
    stations.of_entry(&log.entries[0]).id,
    log.entries[1].station_id
  );
}

#[test]
fn special_call_import_and_single_session_scope() {
  let parsed =
    parse_cabrillo("CONTEST: CQ-WW-CW\nQSO: 14021 CW 2026-10-31 0010 BG4ABC 599 24 RAEM 599 18\n");
  assert_eq!(parsed.records.len(), 1);
  let scope = parsed.scope().unwrap();
  assert_eq!(scope.start, "2026-10-31 00:10");
  assert_eq!(scope.end, "2026-10-31 00:11");
  let mixed = parse_cabrillo(
    "QSO: 14021 CW 2025-10-31 0010 BG4ABC 599 24 W1AW 599 5\nQSO: 14021 CW 2026-10-31 0010 BG4ABC 599 24 W1AW 599 5\n",
  );
  assert!(mixed.scope().is_none());
}

#[test]
fn hostile_numeric_inputs_do_not_succeed_with_infinity() {
  use ham_web_core::{duty_power, ip3_cascade, pagination, rain_fade, sat_doppler, waveguide};
  assert!(
    pagination::page_window(u32::MAX, u32::MAX).contains(&pagination::PageItem::Page(u32::MAX - 1))
  );
  assert_eq!(duty_power::worst_on_time(1e308, 1e308, 360.0), 360.0);
  let geometry = waveguide::Geometry {
    a_m: 1e-200,
    b_m: 5e-201,
    er: 1.0,
  };
  assert!(waveguide::cutoff_hz(&geometry, 1, 0).unwrap().is_finite());
  assert!(
    ip3_cascade::cascade(&[ip3_cascade::Stage {
      gain_db: 0.0,
      ip3_dbm: Some(4000.0),
      referred: ip3_cascade::Referred::Input
    }])
    .is_err()
  );
  assert!(
    rain_fade::estimate(&rain_fade::Input {
      freq_ghz: 10.0,
      dist_km: 20.0,
      rain_rate_001: 1e308,
      polarization: rain_fade::Polarization::Horizontal,
      percent: 0.01
    })
    .is_err()
  );
  let sample = |t| sat_doppler::Sample {
    t,
    az: 0.0,
    el: 10.0,
    range_km: 1.0,
    range_rate_km_s: 0.0,
  };
  assert!(sat_doppler::Track::new(vec![sample(i64::MIN), sample(i64::MAX)]).is_err());
}

#[test]
fn expired_kp_forecast_is_not_current() {
  use ham_web_core::kp_forecast::{KpKind, KpPoint, live_outlook};
  let points = [KpPoint {
    time: "2026-10-16T00:00:00".into(),
    kp: 3.0,
    kind: KpKind::Predicted,
  }];
  let at = |h| ham_web_core::logbook::minutes_of("2026-10-16", h).unwrap();
  assert!(live_outlook(&points, at("02:59")).is_some());
  assert!(live_outlook(&points, at("03:00")).is_none());
}
