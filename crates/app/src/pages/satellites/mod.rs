//! 业余卫星入门：常用卫星与操作要点，并接入真实过境预报。

mod iss_tracker;
mod pass_predictor;
mod satellites_page;

pub use satellites_page::SatellitesPage;

const CELL: &str = "border px-3 py-2 text-left align-top";
