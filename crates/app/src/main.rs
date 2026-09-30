//! 业余无线电执照考试模拟：Leptos CSR 前端入口。

mod app;
mod cn;
mod components;
mod data;
mod exam_history;
mod icons;
mod morse_audio;
mod pages;
mod photo;
mod pwa;
mod shortcuts;
mod speech;
mod store;
mod theme;
mod ui;
mod util;

fn main() {
  console_error_panic_hook::set_once();
  pwa::register_service_worker();
  leptos::mount::mount_to_body(app::App);
}
