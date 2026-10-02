//! 通联记录表单状态：新增 / 编辑共用的字段信号与读写方法，供表单组件与页面共享。

use leptos::prelude::*;

use super::{LogEntry, utc_now_time, utc_today};

/// 通联记录表单字段（26 项 + 「更多字段」展开开关），按值传递（信号均为 `Copy`）。
#[derive(Clone, Copy)]
pub(super) struct LogFormState {
  pub(super) date: RwSignal<String>,
  pub(super) time: RwSignal<String>,
  pub(super) time_off: RwSignal<String>,
  pub(super) freq: RwSignal<String>,
  pub(super) band: RwSignal<String>,
  pub(super) mode: RwSignal<String>,
  pub(super) callsign: RwSignal<String>,
  pub(super) rst_sent: RwSignal<String>,
  pub(super) rst_rcvd: RwSignal<String>,
  pub(super) tx_pwr: RwSignal<String>,
  pub(super) gridsquare: RwSignal<String>,
  pub(super) name: RwSignal<String>,
  pub(super) qth: RwSignal<String>,
  pub(super) prop_mode: RwSignal<String>,
  pub(super) sat_name: RwSignal<String>,
  pub(super) sota_ref: RwSignal<String>,
  pub(super) pota_ref: RwSignal<String>,
  pub(super) cqz: RwSignal<String>,
  pub(super) ituz: RwSignal<String>,
  pub(super) remark: RwSignal<String>,
  pub(super) qsl_sent: RwSignal<bool>,
  pub(super) qsl_rcvd: RwSignal<bool>,
  pub(super) lotw_sent: RwSignal<bool>,
  pub(super) lotw_rcvd: RwSignal<bool>,
  pub(super) eqsl_sent: RwSignal<bool>,
  pub(super) eqsl_rcvd: RwSignal<bool>,
  pub(super) show_more: RwSignal<bool>,
}

impl LogFormState {
  pub(super) fn new() -> Self {
    Self {
      date: RwSignal::new(utc_today()),
      time: RwSignal::new(utc_now_time()),
      time_off: RwSignal::new(String::new()),
      freq: RwSignal::new(String::new()),
      band: RwSignal::new(String::new()),
      mode: RwSignal::new("SSB".to_owned()),
      callsign: RwSignal::new(String::new()),
      rst_sent: RwSignal::new("59".to_owned()),
      rst_rcvd: RwSignal::new(String::new()),
      tx_pwr: RwSignal::new(String::new()),
      gridsquare: RwSignal::new(String::new()),
      name: RwSignal::new(String::new()),
      qth: RwSignal::new(String::new()),
      prop_mode: RwSignal::new(String::new()),
      sat_name: RwSignal::new(String::new()),
      sota_ref: RwSignal::new(String::new()),
      pota_ref: RwSignal::new(String::new()),
      cqz: RwSignal::new(String::new()),
      ituz: RwSignal::new(String::new()),
      remark: RwSignal::new(String::new()),
      qsl_sent: RwSignal::new(false),
      qsl_rcvd: RwSignal::new(false),
      lotw_sent: RwSignal::new(false),
      lotw_rcvd: RwSignal::new(false),
      eqsl_sent: RwSignal::new(false),
      eqsl_rcvd: RwSignal::new(false),
      show_more: RwSignal::new(false),
    }
  }

  /// 清空表单为「新增」默认值。
  pub(super) fn reset(self) {
    self.date.set(utc_today());
    self.time.set(utc_now_time());
    self.time_off.set(String::new());
    self.freq.set(String::new());
    self.band.set(String::new());
    self.mode.set("SSB".to_owned());
    self.callsign.set(String::new());
    self.rst_sent.set("59".to_owned());
    self.rst_rcvd.set(String::new());
    self.tx_pwr.set(String::new());
    self.gridsquare.set(String::new());
    self.name.set(String::new());
    self.qth.set(String::new());
    self.prop_mode.set(String::new());
    self.sat_name.set(String::new());
    self.sota_ref.set(String::new());
    self.pota_ref.set(String::new());
    self.cqz.set(String::new());
    self.ituz.set(String::new());
    self.remark.set(String::new());
    self.qsl_sent.set(false);
    self.qsl_rcvd.set(false);
    self.lotw_sent.set(false);
    self.lotw_rcvd.set(false);
    self.eqsl_sent.set(false);
    self.eqsl_rcvd.set(false);
  }

  /// 从记录载入表单，返回该记录是否含「更多字段」（供自动展开）。
  pub(super) fn load(self, e: &LogEntry) -> bool {
    let has_more = !(e.time_off.is_empty()
      && e.tx_pwr.is_empty()
      && e.prop_mode.is_empty()
      && e.sat_name.is_empty()
      && e.sota_ref.is_empty()
      && e.pota_ref.is_empty());
    self.date.set(e.date.clone());
    self.time.set(e.time.clone());
    self.time_off.set(e.time_off.clone());
    self.freq.set(e.freq.clone());
    self.band.set(e.band.clone());
    self.mode.set(e.mode.clone());
    self.callsign.set(e.callsign.clone());
    self.rst_sent.set(e.rst_sent.clone());
    self.rst_rcvd.set(e.rst_rcvd.clone());
    self.tx_pwr.set(e.tx_pwr.clone());
    self.gridsquare.set(e.gridsquare.clone());
    self.name.set(e.name.clone());
    self.qth.set(e.qth.clone());
    self.prop_mode.set(e.prop_mode.clone());
    self.sat_name.set(e.sat_name.clone());
    self.sota_ref.set(e.sota_ref.clone());
    self.pota_ref.set(e.pota_ref.clone());
    self.cqz.set(e.cqz.clone());
    self.ituz.set(e.ituz.clone());
    self.remark.set(e.remark.clone());
    self.qsl_sent.set(e.qsl_sent);
    self.qsl_rcvd.set(e.qsl_rcvd);
    self.lotw_sent.set(e.lotw_sent);
    self.lotw_rcvd.set(e.lotw_rcvd);
    self.eqsl_sent.set(e.eqsl_sent);
    self.eqsl_rcvd.set(e.eqsl_rcvd);
    has_more
  }

  /// 依当前表单构建一条记录：编辑时呼号未变则保留原 DXCC（可能来自导入的 ADIF，
  /// 比前缀推断更准）；呼号改了则重新推断 DXCC，未手动改过的分区也随之重新推断。
  pub(super) fn build(self, id: u64, old: Option<&LogEntry>) -> LogEntry {
    let call = self.callsign.get().trim().to_uppercase();
    let same_call = old.filter(|o| o.callsign.eq_ignore_ascii_case(&call));
    let renamed = old.filter(|o| !o.callsign.eq_ignore_ascii_case(&call));
    let zone = |sig: RwSignal<String>, prev: Option<&String>| {
      let v = sig.get().trim().to_owned();
      if prev.is_some_and(|p| *p == v) {
        String::new()
      } else {
        v
      }
    };
    LogEntry {
      id,
      dxcc: same_call.map(|o| o.dxcc.clone()).unwrap_or_default(),
      cqz: zone(self.cqz, renamed.map(|o| &o.cqz)),
      ituz: zone(self.ituz, renamed.map(|o| &o.ituz)),
      date: self.date.get().trim().to_owned(),
      time: self.time.get().trim().to_owned(),
      time_off: self.time_off.get().trim().to_owned(),
      freq: self.freq.get().trim().to_owned(),
      band: self.band.get(),
      mode: self.mode.get(),
      callsign: call,
      rst_sent: self.rst_sent.get().trim().to_owned(),
      rst_rcvd: self.rst_rcvd.get().trim().to_owned(),
      tx_pwr: self.tx_pwr.get().trim().to_owned(),
      gridsquare: self.gridsquare.get().trim().to_uppercase(),
      name: self.name.get().trim().to_owned(),
      qth: self.qth.get().trim().to_owned(),
      prop_mode: self.prop_mode.get(),
      sat_name: self.sat_name.get().trim().to_uppercase(),
      sota_ref: self.sota_ref.get().trim().to_uppercase(),
      pota_ref: self.pota_ref.get().trim().to_uppercase(),
      remark: self.remark.get().trim().to_owned(),
      qsl_sent: self.qsl_sent.get(),
      qsl_rcvd: self.qsl_rcvd.get(),
      lotw_sent: self.lotw_sent.get(),
      lotw_rcvd: self.lotw_rcvd.get(),
      eqsl_sent: self.eqsl_sent.get(),
      eqsl_rcvd: self.eqsl_rcvd.get(),
      ..Default::default()
    }
  }
}
