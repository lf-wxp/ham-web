//! 麦克风录音：开始、结束、拿到一个能回放的 `blob:` 地址。
//!
//! 只服务于「话音通联练习」：录下来给用户回放自己的声音，**不识别、不上传、不存盘**。
//! 授权弹窗开着的时候用户可能已经离开或点了结束，所以 [`start`] 返回后调用方要自己判断这次录音
//! 还要不要（不要就 [`Recording::cancel`]，麦克风指示灯随之熄灭）。

use std::cell::RefCell;
use std::rc::Rc;

use js_sys::{Array, Promise};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{
  Blob, BlobEvent, BlobPropertyBag, MediaRecorder, MediaStream, MediaStreamConstraints,
  MediaStreamTrack, RecordingState, Url,
};

/// 进行中的一次录音。
pub struct Recording {
  recorder: MediaRecorder,
  stream: MediaStream,
  chunks: Rc<RefCell<Vec<Blob>>>,
  /// 回调要活到录音结束，所以放在结构体里。
  _on_data: Closure<dyn FnMut(BlobEvent)>,
}

/// 停掉麦克风的所有轨道（指示灯随之熄灭）。
fn stop_tracks(stream: &MediaStream) {
  for t in stream.get_tracks() {
    if let Ok(t) = t.dyn_into::<MediaStreamTrack>() {
      t.stop();
    }
  }
}

/// 请求麦克风并开始录音。没有麦克风、用户拒绝授权、浏览器不支持 `MediaRecorder` 都返回 `Err`。
pub async fn start() -> Result<Recording, JsValue> {
  let devices = crate::util::window().navigator().media_devices()?;
  let constraints = MediaStreamConstraints::new();
  constraints.set_audio(&JsValue::TRUE);
  let stream: MediaStream = JsFuture::from(devices.get_user_media_with_constraints(&constraints)?)
    .await?
    .dyn_into()?;
  let recorder = match MediaRecorder::new_with_media_stream(&stream) {
    Ok(r) => r,
    Err(e) => {
      // 麦克风已经打开了：失败时必须放掉，否则指示灯一直亮着。
      stop_tracks(&stream);
      return Err(e);
    }
  };
  let chunks = Rc::new(RefCell::new(Vec::<Blob>::new()));
  let sink = Rc::clone(&chunks);
  let on_data = Closure::<dyn FnMut(BlobEvent)>::new(move |e: BlobEvent| {
    if let Some(b) = e.data()
      && b.size() > 0.0
    {
      sink.borrow_mut().push(b);
    }
  });
  recorder.set_ondataavailable(Some(on_data.as_ref().unchecked_ref()));
  if let Err(e) = recorder.start() {
    stop_tracks(&stream);
    return Err(e);
  }
  Ok(Recording {
    recorder,
    stream,
    chunks,
    _on_data: on_data,
  })
}

impl Recording {
  /// 不要这次录音了：放掉麦克风，什么都不留。
  pub fn cancel(self) {
    if self.recorder.state() != RecordingState::Inactive {
      let _ = self.recorder.stop();
    }
    stop_tracks(&self.stream);
  }

  /// 结束录音，返回录到的整段音频；没录到任何数据时为 `None`。
  pub async fn stop_blob(self) -> Option<Blob> {
    // `stop` 事件在最后一块数据（`dataavailable`）之后触发：等它，才能拿全。
    let stopped = Promise::new(&mut |resolve, _| {
      self.recorder.set_onstop(Some(&resolve));
    });
    // `stop` / 等 `stop` 事件失败也要放掉麦克风（指示灯不能一直亮着），所以成败都走到 `stop_tracks`。
    let ok = if self.recorder.state() != RecordingState::Inactive {
      self.recorder.stop().is_ok() && JsFuture::from(stopped).await.is_ok()
    } else {
      true
    };
    stop_tracks(&self.stream);
    if !ok {
      return None;
    }
    let chunks = self.chunks.borrow();
    if chunks.is_empty() {
      return None;
    }
    let parts = Array::new();
    for b in chunks.iter() {
      parts.push(b);
    }
    let opts = BlobPropertyBag::new();
    opts.set_type(&self.recorder.mime_type());
    Blob::new_with_blob_sequence_and_options(&parts, &opts).ok()
  }

  /// 结束录音，返回可回放的 `blob:` 地址；没录到任何数据时为 `None`。
  ///
  /// 地址用完要 [`revoke`]，否则这段录音会一直占着内存。
  pub async fn stop(self) -> Option<String> {
    let blob = self.stop_blob().await?;
    Url::create_object_url_with_blob(&blob).ok()
  }
}

/// 释放 [`Recording::stop`] 给出的地址。
pub fn revoke(url: &str) {
  let _ = Url::revoke_object_url(url);
}
