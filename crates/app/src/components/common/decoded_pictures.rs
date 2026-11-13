//! 解码页的「录音里解出的图片」列表（MFSK / THOR 的图片模式共用）。

use leptos::prelude::*;

use crate::decode_worker::PicturePayload;
use crate::i18n::{t, tf};
use crate::icons::{Icon, IconKind};
use crate::ui::ButtonLink;

/// 小图放大显示时最多放到多少个像素宽：再大就没有「看清像素」的意义，只会把窄屏撑出横向滚动。
const MAX_DISPLAY_PX: u32 = 1024;

/// 每张图一个 `<figure>`：图、PNG 下载链接、说明（序号 / 尺寸 / 彩色或灰度）。
/// `file_stem` 是下载文件名的前缀（`mfsk-picture` → `mfsk-picture-1.png`）；
/// `skipped` 为真表示正文里有图片报头却没解出图（采样率不符等），给一行说明而不是悄悄少一张图。
///
/// PNG 编码在 [`PicturePayload::url`] 里惰性做一次并缓存，这里只摆放：重渲染、切语言重新挂载都不重新编码。
#[component]
pub fn DecodedPictures(
  pictures: Vec<PicturePayload>,
  file_stem: &'static str,
  skipped: bool,
) -> impl IntoView {
  let figures = pictures
    .into_iter()
    .enumerate()
    .map(|(i, pic)| {
      // 图注随界面语言变：组件体只跑一次，必须写成闭包交给 `view!` 订阅。
      let (index, width, height, color) = (i + 1, pic.width, pic.height, pic.color);
      let caption = move || {
        tf(
          "tools.decode-picture-caption",
          &[
            &index.to_string(),
            &width.to_string(),
            &height.to_string(),
            &t(if color {
              "tools.decode-picture-color"
            } else {
              "tools.decode-picture-gray"
            }),
          ],
        )
      };
      // 小图（32 × 24 之类）放大显示时保持像素边界，不要被浏览器插值糊掉。
      let max_width = width.saturating_mul(8).min(MAX_DISPLAY_PX);
      let url = pic.url().map(str::to_owned);
      let failed = url.is_none();
      let image = url.map(|url| {
        view! {
          <img
            src=url.clone()
            alt=caption
            style:image-rendering="pixelated"
            style:width="100%"
            style:max-width=format!("{max_width}px")
            class="mx-auto border"
          />
          <div class="flex justify-center">
            <ButtonLink href=url download=format!("{file_stem}-{index}.png")>
              <Icon kind=IconKind::Download class="size-6" />
              {move || t("tools.download-png")}
            </ButtonLink>
          </div>
        }
      });
      view! {
        <figure class="space-y-2" data-testid="decode-picture">
          {image}
          {failed
            .then(|| {
              view! {
                <p
                  class="text-center text-xs text-hp-text"
                  data-testid="decode-picture-failed"
                >
                  {move || t("tools.decoding-failed-generic")}
                </p>
              }
            })}
          <figcaption class="text-center text-xs text-muted-foreground">{caption}</figcaption>
        </figure>
      }
    })
    .collect_view();
  view! {
    {figures}
    {skipped
      .then(|| {
        view! {
          <p class="text-xs text-muted-foreground" data-testid="decode-picture-skipped">
            {move || t("tools.decode-picture-skipped")}
          </p>
        }
      })}
  }
}
