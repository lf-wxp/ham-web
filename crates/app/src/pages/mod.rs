//! 页面。

mod amplifier;
mod analog_modes;
mod antenna_analyzer;
mod antenna_array;
mod antenna_diy;
mod antenna_farm;
mod antenna_installation;
mod antenna_modeling;
mod antenna_tuning;
mod antennas;
mod aprs;
mod ardf;
mod atv;
mod awards;
mod bandplan;
mod bands;
mod beginner;
mod bookmarks;
mod browse;
mod cabrillo;
mod cards;
mod contest;
mod countdown;
mod cw_op;
mod dashboard;
mod dv_network;
mod dx;
mod dx_spots;
mod dxpedition;
mod electronics;
mod emcomm;
mod eme;
mod eqsl;
mod exam;
mod feedline;
mod filters;
mod flashcards;
mod frequencies;
mod ft8;
mod glossary;
mod gnuradio;
mod grayline;
mod grid_map;
mod grid_system;
mod grounding;
mod history;
mod home;
mod iota;
mod license;
mod license_classes;
mod listen;
pub(crate) mod log;
mod logging_software;
mod map;
mod meters;
mod microwave;
mod mistakes;
mod mobile;
mod modes;
mod morse;
mod most_wanted;
mod muf;
mod not_found;
mod nvis;
mod operating;
mod organizations;
mod packet;
mod phonetic;
mod photo;
mod polarization;
mod portable;
mod power;
mod power_supply;
mod practice;
mod prefixes;
mod print_sheet;
mod progress;
mod propagation;
mod qcode;
mod qrp;
mod qsl_card;
mod qsl_labels;
mod receiver;
mod reference;
mod regulations;
mod remote;
mod repeater;
mod repeater_build;
mod rfi;
mod rst;
mod rtty;
mod safety;
mod satellites;
mod sdr;
mod solar;
mod special_prop;
mod sstv;
mod stats;
mod swl;
mod tools;
mod transceiver;
mod weather_sat;
mod wspr;

pub use amplifier::AmplifierPage;
pub use analog_modes::AnalogModesPage;
pub use antenna_analyzer::AntennaAnalyzerPage;
pub use antenna_array::AntennaArrayPage;
pub use antenna_diy::AntennaDiyPage;
pub use antenna_farm::AntennaFarmPage;
pub use antenna_installation::AntennaInstallationPage;
pub use antenna_modeling::AntennaModelingPage;
pub use antenna_tuning::AntennaTuningPage;
pub use antennas::AntennasPage;
pub use aprs::AprsPage;
pub use ardf::ArdfPage;
pub use atv::AtvPage;
pub use awards::AwardsPage;
pub use bandplan::BandPlanPage;
pub use bands::BandsPage;
pub use beginner::BeginnerPage;
pub use bookmarks::BookmarksPage;
pub use browse::BrowsePage;
pub use cabrillo::CabrilloPage;
pub use cards::{CardsPage, load_schedule as load_card_schedule};
pub use contest::ContestPage;
pub use countdown::CountdownPage;
pub use cw_op::CwOpPage;
pub use dashboard::DashboardPage;
pub use dv_network::DvNetworkPage;
pub use dx::DxPage;
pub use dx_spots::DxSpotsPage;
pub use dxpedition::DxpeditionPage;
pub use electronics::ElectronicsPage;
pub use emcomm::EmcommPage;
pub use eme::EmePage;
pub use eqsl::EqslPage;
pub use exam::ExamPage;
pub use feedline::FeedlinePage;
pub use filters::FiltersPage;
pub use flashcards::FlashcardsPage;
pub use frequencies::FrequenciesPage;
pub use ft8::Ft8Page;
pub use glossary::GlossaryPage;
pub use gnuradio::GnuradioPage;
pub use grayline::GraylinePage;
pub use grid_map::GridMapPage;
pub use grid_system::GridSystemPage;
pub use grounding::GroundingPage;
pub use history::HistoryPage;
pub use home::HomePage;
pub use iota::IotaPage;
pub use license::LicensePage;
pub use license_classes::LicenseClassesPage;
pub use listen::ListenPage;
pub use log::{ContestLogPage, LogPage};
pub use logging_software::LoggingSoftwarePage;
pub use meters::MetersPage;
pub use microwave::MicrowavePage;
pub use mistakes::MistakesPage;
pub use mobile::MobilePage;
pub use modes::ModesPage;
pub use morse::MorsePage;
pub use most_wanted::MostWantedPage;
pub use muf::MufPage;
pub use not_found::NotFoundPage;
pub use nvis::NvisPage;
pub use operating::OperatingPage;
pub use organizations::OrganizationsPage;
pub use packet::PacketPage;
pub use phonetic::PhoneticPage;
pub use photo::PhotoProcessorPage;
pub use polarization::PolarizationPage;
pub use portable::PortablePage;
pub use power::PowerPage;
pub use power_supply::PowerSupplyPage;
pub use practice::PracticePage;
pub use prefixes::PrefixesPage;
pub use print_sheet::PrintPage;
pub use progress::ProgressPage;
pub use propagation::PropagationPage;
pub use qcode::QCodePage;
pub use qrp::QrpPage;
pub use qsl_card::QslCardPage;
pub use qsl_labels::QslLabelsPage;
pub use receiver::ReceiverPage;
pub use reference::ReferencePage;
pub use regulations::RegulationsPage;
pub use remote::RemotePage;
pub use repeater::RepeaterPage;
pub use repeater_build::RepeaterBuildPage;
pub use rfi::RfiPage;
pub use rst::RstPage;
pub use rtty::RttyPage;
pub use safety::SafetyPage;
pub use satellites::SatellitesPage;
pub use sdr::SdrPage;
pub use solar::SolarPage;
pub use special_prop::SpecialPropPage;
pub use sstv::SstvPage;
pub use stats::StatsPage;
pub use swl::SwlPage;
pub use tools::ToolsPage;
pub use transceiver::TransceiverPage;
pub use weather_sat::WeatherSatPage;
pub use wspr::WsprPage;

/// 属于「简语」的一级分类 key：术语表与简语速查页据此拆分展示。
pub const SLANG_CATEGORY: &str = "用语";

use ham_web_core::Bank;
use leptos::prelude::*;
use leptos_router::hooks::use_query_map;

use crate::util::body_class;

/// 默认页面标题。
pub const DEFAULT_TITLE: &str = "业余无线电｜题库 · 知识 · 工具";

/// 从 URL 读取 `version` 与 `bank` 参数。
pub fn use_bank_query() -> (Memo<Option<String>>, Memo<Bank>) {
  let query = use_query_map();
  let version = Memo::new(move |_| query.with(|q| q.get("version")).filter(|v| !v.is_empty()));
  let bank = Memo::new(move |_| Bank::from_param(query.with(|q| q.get("bank")).as_deref()));
  (version, bank)
}

/// 答题页隐藏站点页脚。
pub fn use_no_site_footer() {
  body_class("no-site-footer", true);
  on_cleanup(|| body_class("no-site-footer", false));
}

/// 构造带题库参数的链接。
pub fn bank_href(path: &str, version: Option<&str>, bank: Bank) -> String {
  match version {
    Some(v) => format!(
      "{path}?version={}&bank={bank}",
      String::from(js_sys::encode_uri_component(v))
    ),
    None => format!("{path}?bank={bank}"),
  }
}
