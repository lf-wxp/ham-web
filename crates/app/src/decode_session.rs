//! 解码页的会话状态机：一份实现，替掉各页逐字重复的「五个信号 + 四个 `StoredValue` +
//! 取消 / 选文件 / 重置」约 100 行样板。
//!
//! 生命周期约定与 `docs/decode-worker-protocol.md` 一致：每次选新文件**先 terminate 旧
//! Worker 再 spawn 新的**（Worker 无法取消在途解码），用代际计数丢弃过期回复；文件读取
//! `await` 之后先查代际 / 是否已卸载，再碰任何信号。
//!
//! **必须在组件体（有 Owner）内创建**：内部的消息闭包、超时槽位与卸载清理都挂在当前 Owner 上。

use leptos::leptos_dom::helpers::{IntervalHandle, TimeoutHandle, set_interval_with_handle};
use leptos::prelude::*;
use leptos::task::spawn_local;
use wasm_bindgen::JsValue;
use wasm_bindgen::closure::Closure;
use web_sys::{File, MessageEvent, Worker};

use crate::decode_worker::{
  arm_decode_timeout_with, decode_timeout_slot, error_handler, parse_outcome, post_decode,
  post_decode_transfer, read_file_buffer, read_file_bytes, retain_error_handler,
  retain_message_handler, shutdown_worker, spawn_worker,
};
use crate::i18n::{t, tf};
use crate::util::{js_error_message, now_ms};

/// 各解码页共用的上传体积上限（字节）。
///
/// 页面只把「读到的字节」交给 Worker，8 MiB 对一段几十秒到几分钟的录音足够；更大的
/// 录音应该先自己截取。上限写在一处，避免三个页面各写一份后漂移。
pub const MAX_UPLOAD_BYTES: u64 = 8 * 1024 * 1024;

/// 会话配置。回调都用函数指针：词条字面量留在各页的调用点（`check-i18n` 只采集直接实参）。
pub struct DecodeConfig<T> {
  /// 统一信封里的 codec 名（`rtty` / `olivia` / …）。
  pub codec: &'static str,
  /// 允许的最大文件体积（字节）。
  pub max_bytes: u64,
  /// `true`：把读到的 `ArrayBuffer` 直接转移给 Worker（128 MB 级录音，主线程不留副本）；
  /// `false`：先拷成 `Vec<u8>` 再发。
  pub transfer: bool,
  /// 解析统一信封里的 `result`（可以顺带做渲染，失败就返回面向用户的文案）。
  pub parse: fn(&JsValue) -> Result<T, String>,
  /// 文件过大的提示文案，参数是 `(实际 MB, 上限 MB)`。
  pub too_large: fn(&str, &str) -> String,
  /// 超时提示文案。
  pub timed_out: fn() -> String,
  /// 协议损坏（Worker 回了空错误串）时的兜底文案。
  pub failed: fn() -> String,
}

/// 一个解码页的全部会话状态。`Copy`：可以随意 move 进各个闭包。
pub struct DecodeSession<T: Send + Sync + 'static> {
  /// 正在读文件或解码。
  pub processing: RwSignal<bool>,
  /// 仍在读文件（`processing` 的子阶段）。
  pub reading: RwSignal<bool>,
  /// 本次解码已用时（毫秒）。
  ///
  /// 为什么是「已用时」而不是百分比：Worker 是同步的一次性计算，协议里没有中间进度
  /// （`decode-worker-protocol.md`），要拿到百分比得改 11 个 worker 的返回。大文件解码
  /// 时用户最缺的其实只是「它还在动 / 已经跑了多久」，这个前端自己就能给。
  ///
  /// 读数是 `现在 - 起点` 的时间戳差，**不是**按 tick 次数累加：解码恰好是用户会切走
  /// 等待的场景，后台标签页的定时器会被浏览器降频到约 1 次 / 秒，数 tick 会越走越慢。
  pub elapsed_ms: RwSignal<u64>,
  /// 面向用户的错误文案。
  pub error: RwSignal<Option<String>>,
  /// 当前文件名。
  pub file_name: RwSignal<String>,
  /// 解码结果。
  pub result: RwSignal<Option<T>>,
  codec: &'static str,
  max_bytes: u64,
  transfer: bool,
  too_large: fn(&str, &str) -> String,
  timed_out: fn() -> String,
  timer: StoredValue<Option<TimeoutHandle>>,
  /// 计时器句柄：由 `ticker_slot` 保证 `processing` 一旦回到 `false`（成功 / 失败 / 取消 /
  /// 超时的任何出口）或页面卸载就停掉，不靠每条出口各自记得调用。
  ticker: StoredValue<Option<IntervalHandle>>,
  generation: StoredValue<u32>,
  onmessage: StoredValue<js_sys::Function>,
  /// `Worker.onerror` 的转发函数：与 `onmessage` 同生命周期，重建 Worker 时复用。
  onerror: StoredValue<js_sys::Function>,
  worker: StoredValue<Option<Worker>>,
}

// 手写而不是 derive：derive 会要求 `T: Clone / Copy`，而这里所有字段本身都是 `Copy`。
impl<T: Send + Sync + 'static> Clone for DecodeSession<T> {
  fn clone(&self) -> Self {
    *self
  }
}
impl<T: Send + Sync + 'static> Copy for DecodeSession<T> {}

impl<T: Send + Sync + 'static> DecodeSession<T> {
  /// 创建会话（**不**创建 Worker：第一个 Worker 在首次 `start` 时才 spawn）。
  pub fn new(config: DecodeConfig<T>) -> Self {
    let processing = RwSignal::new(false);
    let result = RwSignal::new(None::<T>);
    let error = RwSignal::new(None::<String>);
    let elapsed_ms = RwSignal::new(0u64);
    let ticker = ticker_slot(processing);
    let (parse, failed) = (config.parse, config.failed);
    let handler = Closure::<dyn FnMut(MessageEvent)>::new(move |event: MessageEvent| {
      match parse_outcome(&event.data()).and_then(|raw| parse(&raw)) {
        Ok(p) => result.set(Some(p)),
        // 空串 = 协议损坏（缺 error 字段）：回退页面给的兜底文案，不硬编码界面文案。
        Err(e) => error.set(Some(if e.is_empty() { failed() } else { e })),
      }
      processing.set(false);
    });
    let onmessage = StoredValue::new(retain_message_handler(handler));
    // `onerror` 转发 Worker 的加载 / 运行期异常；与 `onmessage` 一样挂在当前 Owner 上，
    // 重建 Worker 时复用同一份，不必每次新建闭包。
    let onerror = StoredValue::new(retain_error_handler(error_handler(&onmessage.get_value())));
    // Worker 惰性创建：第一次选文件时 `restart_worker` 才会 spawn。挂载即建的话，
    // 每次进页面白加载一份 wasm（约 1–2 MB），随后首文件又 terminate 掉重建。
    let worker = StoredValue::new(None::<Worker>);
    let session = DecodeSession {
      processing,
      reading: RwSignal::new(false),
      elapsed_ms,
      error,
      file_name: RwSignal::new(String::new()),
      result,
      codec: config.codec,
      max_bytes: config.max_bytes,
      transfer: config.transfer,
      too_large: config.too_large,
      timed_out: config.timed_out,
      timer: decode_timeout_slot(processing),
      ticker,
      generation: StoredValue::new(0u32),
      onmessage,
      onerror,
      worker,
    };
    on_cleanup(move || session.stop_worker());
    session
  }

  /// 终止当前 Worker 并清空槽位（已卸载的槽位直接忽略）。
  fn stop_worker(self) {
    if let Some(Some(w)) = self.worker.try_get_value() {
      shutdown_worker(&w);
      // 清掉：已经 terminate 的 Worker 留在槽里，后面的 `start` 虽然总会先重建，但别的
      // 路径若误用它只会静默丢消息。
      self.worker.try_set_value(None);
    }
  }

  /// 让当前这一批作废：终止 Worker、代际递增，**不**新建 Worker。返回新批次号。
  ///
  /// 取消 / 超时只需要这一步。Worker 在下次 `start` 时才创建（见 `new` 里「惰性创建」的
  /// 注释），取消后立刻新建只会白加载一份 wasm，随后又被 `start` 终止重建。
  fn invalidate(self) -> u32 {
    self.stop_worker();
    let batch = self.generation.get_value().wrapping_add(1);
    self.generation.set_value(batch);
    batch
  }

  /// 作废旧批次并起新 Worker，返回新批次号。
  fn restart_worker(self) -> u32 {
    let batch = self.invalidate();
    self.worker.set_value(spawn_worker(
      &self.onmessage.get_value(),
      &self.onerror.get_value(),
    ));
    batch
  }

  /// 重置并启动「已用时」计时器（先停旧的，连续选文件不会叠出两个 interval）。
  fn start_ticker(self) {
    stop_ticker(self.ticker);
    self.elapsed_ms.set(0);
    let started = now_ms();
    let elapsed = self.elapsed_ms;
    self.ticker.set_value(
      set_interval_with_handle(
        // `try_set`：页面卸载后信号已释放，静默忽略而不是 panic。
        move || {
          elapsed.try_set((now_ms() - started).max(0) as u64);
        },
        std::time::Duration::from_millis(200),
      )
      .ok(),
    );
  }

  /// 取消当前解码：terminate + 代际递增（解码是同步的，只能整体丢弃）。
  pub fn cancel(self) {
    self.invalidate();
    self.processing.set(false);
    self.reading.set(false);
    self.error.set(None);
    self.file_name.set(String::new());
  }

  /// 清掉结果与错误，回到「选文件」状态。
  pub fn reset(self) {
    self.result.set(None);
    self.error.set(None);
    self.file_name.set(String::new());
  }

  /// 开始解码一个文件。
  ///
  /// `params` 在**文件读完之后**才求值（此时才快照输入框的值，读取期间用户改的也算）；
  /// 返回 `Err(文案)` 会直接报错并结束，不发给 Worker。
  pub fn start(self, file: File, params: impl FnOnce() -> Result<JsValue, String> + 'static) {
    let size = file.size() as u64;
    if size > self.max_bytes {
      let mb = size as f64 / 1024.0 / 1024.0;
      self.error.set(Some((self.too_large)(
        &format!("{mb:.0}"),
        &format!("{}", self.max_bytes / 1024 / 1024),
      )));
      return;
    }
    let batch = self.restart_worker();
    self.processing.set(true);
    self.reading.set(true);
    self.start_ticker();
    self.error.set(None);
    self.result.set(None);
    self.file_name.set(file.name());
    spawn_local(async move {
      // 读文件期间页面可能已卸载或用户换了文件：先查代际，不能读已释放的值。
      let stale = move || self.generation.try_get_value() != Some(batch);
      let payload = if self.transfer {
        read_file_buffer(&file).await.map(Payload::Buffer)
      } else {
        read_file_bytes(&file).await.map(Payload::Bytes)
      };
      if stale() {
        return;
      }
      self.reading.set(false);
      let payload = match payload {
        Ok(p) => p,
        // 读文件失败拿到的 `e` 是英文 DOMException 原文，包一层词条再展示，
        // 中文界面不至于冒出英文。
        Err(e) => {
          self.error.set(Some(tf("tools.decode-io-failed", &[&e])));
          self.processing.set(false);
          return;
        }
      };
      let params = match params() {
        Ok(p) => p,
        Err(e) => {
          self.error.set(Some(e));
          self.processing.set(false);
          return;
        }
      };
      let Some(Some(worker)) = self.worker.try_get_value() else {
        self.error.set(Some(t("radio.could-not-create-the")));
        self.processing.set(false);
        return;
      };
      let sent = match &payload {
        Payload::Buffer(b) => post_decode_transfer(&worker, self.codec, b, &params),
        Payload::Bytes(b) => post_decode(&worker, self.codec, b, &params),
      };
      match sent {
        Ok(()) => arm_decode_timeout_with(
          self.timer,
          self.generation,
          batch,
          self.processing,
          self.error,
          move |()| self.cancel(),
          self.timed_out,
        ),
        Err(e) => {
          self
            .error
            .set(Some(tf("tools.decode-io-failed", &[&js_error_message(&e)])));
          self.processing.set(false);
        }
      }
    });
  }
}

/// 计时器槽位：与 `decode_timeout_slot` 同一模式 —— `processing` 回到 `false` 时停表，
/// 卸载时也停表。
///
/// 解码的出口不止一条（成功 / Worker 报错 / 读文件失败 / 参数校验失败 / 取消 / 超时），
/// 它们的共同点只有「把 `processing` 置回 `false`」，所以挂在这个信号上，而不是让每条
/// 出口各自记得停表（漏一条就是一个永不停的 `setInterval`）。
fn ticker_slot(processing: RwSignal<bool>) -> StoredValue<Option<IntervalHandle>> {
  let ticker = StoredValue::new(None::<IntervalHandle>);
  Effect::new(move |_| {
    if !processing.get() {
      stop_ticker(ticker);
    }
  });
  on_cleanup(move || stop_ticker(ticker));
  ticker
}

/// 停掉计时器并清空槽位（重复调用安全；已卸载的槽位直接忽略）。
fn stop_ticker(ticker: StoredValue<Option<IntervalHandle>>) {
  if let Some(Some(handle)) = ticker.try_get_value() {
    handle.clear();
    ticker.try_set_value(None);
  }
}

/// 读到的文件内容：转移路径持有 `ArrayBuffer`，拷贝路径持有 `Vec<u8>`。
enum Payload {
  Buffer(js_sys::ArrayBuffer),
  Bytes(Vec<u8>),
}
