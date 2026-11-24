use ham_web_core::picture_tx::{cover_rect, fit_within, rgba_to_rgb};
use leptos::prelude::*;
use leptos::task::spawn_local;
use wasm_bindgen::JsValue;
use web_sys::File;

use super::mode_encoder::{center_field, download_wav, encode_to_wav, params_of};
use crate::codec_modes::{
  MFSK_PICTURE_EDGES, MFSK_PICTURE_MODES, THOR_PICTURE_MODES, THOR_PICTURE_SIZES,
};
use crate::i18n::t;
use crate::photo::{DrawPlan, file_to_rgba, is_image};
use crate::ui::{
  Button, Checkbox, Field, FileInput, SelectOption, SimpleSelect, Size, Variant, push_error,
};
use crate::util::unique_id;

/// 缩放 / 裁剪 / 灰度后的图片：`(宽, 高, RGB)`。
type Prepared = (u32, u32, Vec<u8>);

/// 把图片文件按 `plan` 画好，再转成发端要的 RGB（`gray` 时三个通道都是亮度）。
///
/// 失败返回已本地化的文案（读文件 / 解码失败、像素总量超限）。
async fn prepare(
  file: &File,
  gray: bool,
  plan: impl FnOnce(u32, u32) -> Option<DrawPlan>,
) -> Result<Prepared, String> {
  let (w, h, rgba) = file_to_rgba(file, plan).await.map_err(|e| e.message())?;
  Ok((w, h, rgba_to_rgb(&rgba, gray)))
}

/// worker 编码请求的 `params`：模式、引导文本、可选的中心频率，加上 `picture`。
///
/// `spp` 只有 MFSK 用（THOR 每像素固定 10 个样本，不读这个字段）。
fn picture_params(
  mode: &str,
  lead_in: &str,
  center: &str,
  (w, h, rgb): &Prepared,
  color: bool,
  spp: Option<f64>,
) -> JsValue {
  let mut pic: Vec<(&str, JsValue)> = vec![
    ("width", (*w).into()),
    ("height", (*h).into()),
    ("color", color.into()),
    ("rgb", js_sys::Uint8Array::from(rgb.as_slice()).into()),
  ];
  pic.extend(spp.map(|v| ("spp", v.into())));
  let mut fields: Vec<(&str, JsValue)> = vec![
    ("mode", mode.into()),
    ("text", lead_in.into()),
    ("picture", params_of(&pic)),
  ];
  fields.extend(center_field(center).map(|c| ("centerHz", c)));
  params_of(&fields)
}

/// `"160x120"` → `(160, 120)`（THOR 尺寸下拉的取值）。
fn parse_size(value: &str) -> Option<(usize, usize)> {
  let (w, h) = value.split_once('x')?;
  Some((w.parse().ok()?, h.parse().ok()?))
}

/// 图片发端（MFSK / THOR）：选一张图，缩放后经模拟调频发出，导出 WAV。
///
/// 像素前处理（缩放尺寸、铺满裁剪、透明铺白、灰度）在 `ham_web_core::picture_tx`（有单测），
/// 浏览器只负责解码图片与 canvas；合成在 MFSK / THOR 各自的 GPL worker 里。
/// 子模式与中心频率沿用上面的下拉与输入框（`/tools#mode-encoder` 同一页），引导文本是页面上的「报文内容」，可以为空。
#[component]
pub(super) fn PictureTx(
  /// 报文内容：作为报头前面的引导文本（可为空）。
  text: RwSignal<String>,
  mfsk_mode: RwSignal<String>,
  thor_mode: RwSignal<String>,
  /// 中心频率输入框的内容（留空 = 缺省）。
  center: RwSignal<String>,
) -> impl IntoView {
  // `File` 不是 `Send`，放进本地存储。文件名由 `FileInput` 自己显示，这里不再另画一份：
  // 界面上显示的文件与手里持有的文件必须是同一个，所以「是不是图片」放到发送时再检查，
  // 而不是在选择时拒绝 —— 否则会出现名字留在按钮旁、实际却被拒绝的错位。
  let file = StoredValue::new_local(None::<File>);
  let color = RwSignal::new(true);
  let busy = RwSignal::new(false);
  let mfsk_edge = RwSignal::new(MFSK_PICTURE_EDGES[0].to_string());
  let mfsk_speed = RwSignal::new("8".to_owned());
  let (w0, h0) = THOR_PICTURE_SIZES[0];
  let thor_size = RwSignal::new(format!("{w0}x{h0}"));
  // SSTV：七个模式直接取解码器的模式表（`ham-web-sstv` 是 MIT，直接链进主包，不走 Worker）；
  // 默认 Robot 36（36 秒，最快出结果）。
  let sstv_mode = RwSignal::new("Robot 36".to_owned());
  let sstv_options: Vec<SelectOption> = ham_web_sstv::MODES
    .iter()
    .map(|m| SelectOption::new(m.name, format!("{} · {} × {}", m.name, m.width, m.height)))
    .collect();
  let sstv_id = unique_id("sstv-picture-mode");

  let edge_options: Vec<SelectOption> = MFSK_PICTURE_EDGES
    .iter()
    .map(|e| SelectOption::new(e.to_string(), format!("{e} px")))
    .collect();
  let speed_options = vec![
    SelectOption::new("8", Signal::derive(|| t("tools.picture-tx-speed-standard"))),
    SelectOption::new("4", Signal::derive(|| t("tools.picture-tx-speed-fast"))),
    SelectOption::new("2", Signal::derive(|| t("tools.picture-tx-speed-fastest"))),
  ];
  let size_options: Vec<SelectOption> = THOR_PICTURE_SIZES
    .iter()
    .map(|(w, h)| SelectOption::new(format!("{w}x{h}"), format!("{w} × {h}")))
    .collect();
  let (edge_id, speed_id, size_id) = (
    unique_id("mfsk-picture-edge"),
    unique_id("mfsk-picture-speed"),
    unique_id("thor-picture-size"),
  );

  let on_files = Callback::new(move |files: Vec<File>| {
    if let Some(picked) = files.into_iter().next() {
      file.set_value(Some(picked));
    }
  });

  // 三个发送按钮共用的前置检查之一：选了图、而且是图片。
  let picked_image = move || -> Option<File> {
    let Some(picked) = file.get_value() else {
      push_error(t("tools.picture-tx-no-file"));
      return None;
    };
    if !is_image(&picked) {
      push_error(t("tools.picture-tx-not-image"));
      return None;
    }
    Some(picked)
  };
  // 之二（MFSK / THOR）：子模式有图片模式。
  let ready = move |mode: &RwSignal<String>, supported: &[&str]| -> Option<(File, String)> {
    let mode = mode.get_untracked();
    if !supported.contains(&mode.as_str()) {
      push_error(t("tools.picture-tx-mode-unsupported"));
      return None;
    }
    Some((picked_image()?, mode))
  };

  let send_mfsk = Callback::new(move |_| {
    if busy.get_untracked() {
      return;
    }
    let Some((picked, mode)) = ready(&mfsk_mode, MFSK_PICTURE_MODES) else {
      return;
    };
    let edge = mfsk_edge
      .get_untracked()
      .parse::<usize>()
      .unwrap_or(MFSK_PICTURE_EDGES[0]);
    let spp = mfsk_speed.get_untracked().parse::<f64>().unwrap_or(8.0);
    let (color, lead_in, center) = (
      color.get_untracked(),
      text.get_untracked(),
      center.get_untracked(),
    );
    busy.set(true);
    spawn_local(async move {
      // 等比缩进「最大边长」的方框，整张图都用。
      let prepared = prepare(&picked, !color, |nw, nh| {
        let (w, h) = fit_within(nw as usize, nh as usize, edge)?;
        Some(DrawPlan {
          size: (u32::try_from(w).ok()?, u32::try_from(h).ok()?),
          source: (0.0, 0.0, f64::from(nw), f64::from(nh)),
        })
      })
      .await;
      match prepared {
        Ok(p) => {
          let params = picture_params(&mode, &lead_in, &center, &p, color, Some(spp));
          encode_to_wav("mfsk", params, "mfsk-picture.wav").await;
        }
        Err(message) => push_error(message),
      }
      busy.set(false);
    });
  });

  let send_thor = Callback::new(move |_| {
    if busy.get_untracked() {
      return;
    }
    let Some((picked, mode)) = ready(&thor_mode, THOR_PICTURE_MODES) else {
      return;
    };
    let Some((dw, dh)) = parse_size(&thor_size.get_untracked()) else {
      return;
    };
    let (color, lead_in, center) = (
      color.get_untracked(),
      text.get_untracked(),
      center.get_untracked(),
    );
    busy.set(true);
    spawn_local(async move {
      // THOR 只认固定的几种尺寸：铺满目标比例，多出来的从中间裁掉。
      let prepared = prepare(&picked, !color, |nw, nh| {
        let (x, y, w, h) = cover_rect(nw as usize, nh as usize, dw, dh)?;
        Some(DrawPlan {
          size: (u32::try_from(dw).ok()?, u32::try_from(dh).ok()?),
          source: (x as f64, y as f64, w as f64, h as f64),
        })
      })
      .await;
      match prepared {
        Ok(p) => {
          let params = picture_params(&mode, &lead_in, &center, &p, color, None);
          encode_to_wav("thor", params, "thor-picture.wav").await;
        }
        Err(message) => push_error(message),
      }
      busy.set(false);
    });
  });

  // SSTV：图片铺满裁剪到该模式的尺寸（320 × 256 或 320 × 240），在主线程直接合成 8 kHz 基带
  // （Scottie DX 约 220 万个样本，毫秒级）；不走 Worker，因为 `ham-web-sstv` 是 MIT。
  let send_sstv = Callback::new(move |_| {
    if busy.get_untracked() {
      return;
    }
    let Some(picked) = picked_image() else {
      return;
    };
    let name = sstv_mode.get_untracked();
    let Some(mode) = ham_web_sstv::MODES.iter().find(|m| m.name == name) else {
      return;
    };
    let color = color.get_untracked();
    busy.set(true);
    spawn_local(async move {
      let (dw, dh) = (mode.width, mode.height);
      let prepared = prepare(&picked, !color, |nw, nh| {
        let (x, y, w, h) = cover_rect(nw as usize, nh as usize, dw, dh)?;
        Some(DrawPlan {
          size: (u32::try_from(dw).ok()?, u32::try_from(dh).ok()?),
          source: (x as f64, y as f64, w as f64, h as f64),
        })
      })
      .await;
      match prepared {
        Ok((_, _, rgb)) => match ham_web_sstv::encode_image(mode, &rgb) {
          Ok(samples) => download_wav(samples, ham_web_sstv::WORK_RATE, "sstv.wav"),
          // 模式取自解码器的表、尺寸取自模式本身，正常到不了这里；真到了就如实说失败。
          Err(_) => push_error(t("tools.processing-failed-please-try")),
        },
        Err(message) => push_error(message),
      }
      busy.set(false);
    });
  });

  view! {
    <div class="space-y-3 border-t pt-3">
      <div class="text-xs text-foreground/70">{move || t("tools.picture-tx-heading")}</div>
      <FileInput
        on_files=on_files
        accept="image/png,image/jpeg,image/webp,image/gif,image/bmp"
        label=Signal::derive(|| t("tools.picture-tx-choose"))
        variant=Variant::Outline
        size=Size::Sm
      />
      <label class="flex items-start gap-2 text-sm">
        <Checkbox
          checked=Signal::derive(move || color.get())
          on_change=Callback::new(move |v: bool| color.set(v))
        />
        <span>
          {move || t("tools.decode-picture-color")}
          <span class="block text-xs text-muted-foreground">
            {move || t("tools.picture-tx-color-hint")}
          </span>
        </span>
      </label>
      // 窄屏必须写 `grid-cols-1`（= `minmax(0, 1fr)`）：不写则隐式列是 `auto`，
      // 英文 / 西语下拉里的长文案会把整列、整张卡片乃至整页撑出视口；子项的 `min-w-0` 同理。
      <div class="grid grid-cols-1 gap-3 sm:grid-cols-3">
        <div class="min-w-0 space-y-2">
          <Field label=Signal::derive(|| t("tools.mfsk-picture-edge")) r#for=edge_id.clone()>
            <SimpleSelect
              id=edge_id
              value=mfsk_edge
              on_change=Callback::new(move |v: String| mfsk_edge.set(v))
              options=edge_options
              aria_label=Signal::derive(|| t("tools.mfsk-picture-edge"))
            />
          </Field>
          <Field label=Signal::derive(|| t("tools.mfsk-picture-speed")) r#for=speed_id.clone()>
            <SimpleSelect
              id=speed_id
              value=mfsk_speed
              on_change=Callback::new(move |v: String| mfsk_speed.set(v))
              options=speed_options
              aria_label=Signal::derive(|| t("tools.mfsk-picture-speed"))
            />
          </Field>
          <Button
            variant=Variant::Outline
            size=Size::Sm
            disabled=Signal::derive(move || busy.get())
            on_click=send_mfsk
          >
            {move || t("tools.download-mfsk-picture-wav")}
          </Button>
        </div>
        <div class="min-w-0 space-y-2">
          <Field label=Signal::derive(|| t("tools.thor-picture-size")) r#for=size_id.clone()>
            <SimpleSelect
              id=size_id
              value=thor_size
              on_change=Callback::new(move |v: String| thor_size.set(v))
              options=size_options
              aria_label=Signal::derive(|| t("tools.thor-picture-size"))
            />
          </Field>
          <Button
            variant=Variant::Outline
            size=Size::Sm
            disabled=Signal::derive(move || busy.get())
            on_click=send_thor
          >
            {move || t("tools.download-thor-picture-wav")}
          </Button>
        </div>
        <div class="min-w-0 space-y-2">
          <Field label=Signal::derive(|| t("tools.sstv-picture-mode")) r#for=sstv_id.clone()>
            <SimpleSelect
              id=sstv_id
              value=sstv_mode
              on_change=Callback::new(move |v: String| sstv_mode.set(v))
              options=sstv_options
              aria_label=Signal::derive(|| t("tools.sstv-picture-mode"))
            />
          </Field>
          <Button
            variant=Variant::Outline
            size=Size::Sm
            disabled=Signal::derive(move || busy.get())
            on_click=send_sstv
          >
            {move || t("tools.download-sstv-picture-wav")}
          </Button>
        </div>
      </div>
      <p class="text-xs text-muted-foreground">{move || t("tools.picture-tx-note")}</p>
    </div>
  }
}
