//! 信号听辨向导：把「我听到的是什么」从听感特征收窄到具体模式与站内解码器。
//!
//! 三组条件各自可「不限」，收窄顺序按现场最容易判断的先后（听感 → 节奏 → 带宽）。
//! 候选为空的可能来自条件互相矛盾（如「人声 + 周期突发 + 窄带」），页面如实提示
//! 放宽某一项，不去猜一个最接近的答案。

use ham_web_core::signal_id::{self, Bandwidth, Rhythm, SignalQuery, Sound};
use leptos::prelude::*;

use crate::components::common::{PageContainer, PageHeader};
use crate::i18n::{t, tf};
use crate::ui::{ButtonLink, Chip, ChipGroup, Size};
use crate::util::set_title;

/// chip 组里「不限」的取值。
const ANY: &str = "-";

fn parse_sound(v: &str) -> Option<Sound> {
  match v {
    "speech" => Some(Sound::Speech),
    "single" => Some(Sound::Single),
    "dual" => Some(Sound::Dual),
    "multi" => Some(Sound::Multi),
    "noise" => Some(Sound::Noise),
    "chirp" => Some(Sound::Chirp),
    _ => None,
  }
}

fn parse_rhythm(v: &str) -> Option<Rhythm> {
  match v {
    "continuous" => Some(Rhythm::Continuous),
    "keyed" => Some(Rhythm::Keyed),
    "burst" => Some(Rhythm::Burst),
    "packet" => Some(Rhythm::Packet),
    "sweep" => Some(Rhythm::Sweep),
    _ => None,
  }
}

fn parse_bandwidth(v: &str) -> Option<Bandwidth> {
  match v {
    "narrow" => Some(Bandwidth::Narrow),
    "medium" => Some(Bandwidth::Medium),
    "wide" => Some(Bandwidth::Wide),
    "broad" => Some(Bandwidth::Broad),
    _ => None,
  }
}

/// 模式名与听感提示的 i18n key 必须在调用点写字面量：**不要**改成
/// `format!("signal.{key}-name")` —— 检查器看不到拼接出来的词条，会当成死条目。
fn name_key(key: &str) -> &'static str {
  match key {
    "cw" => "signal.cw-name",
    "rtty" => "signal.rtty-name",
    "psk31" => "signal.psk31-name",
    "ft8" => "signal.ft8-name",
    "ft4" => "signal.ft4-name",
    "wspr" => "signal.wspr-name",
    "olivia" => "signal.olivia-name",
    "dominoex" => "signal.dominoex-name",
    "hell" => "signal.hell-name",
    "navtex" => "signal.navtex-name",
    "packet" => "signal.packet-name",
    "sstv" => "signal.sstv-name",
    "wefax" => "signal.wefax-name",
    "apt" => "signal.apt-name",
    "ssb" => "signal.ssb-name",
    "fm" => "signal.fm-name",
    "lora" => "signal.lora-name",
    _ => "signal.unknown-name",
  }
}

fn hint_key(key: &str) -> &'static str {
  match key {
    "cw" => "signal.cw-hint",
    "rtty" => "signal.rtty-hint",
    "psk31" => "signal.psk31-hint",
    "ft8" => "signal.ft8-hint",
    "ft4" => "signal.ft4-hint",
    "wspr" => "signal.wspr-hint",
    "olivia" => "signal.olivia-hint",
    "dominoex" => "signal.dominoex-hint",
    "hell" => "signal.hell-hint",
    "navtex" => "signal.navtex-hint",
    "packet" => "signal.packet-hint",
    "sstv" => "signal.sstv-hint",
    "wefax" => "signal.wefax-hint",
    "apt" => "signal.apt-hint",
    "ssb" => "signal.ssb-hint",
    "fm" => "signal.fm-hint",
    "lora" => "signal.lora-hint",
    _ => "signal.unknown-hint",
  }
}

/// 带宽区间的人类可读写法（数值 + 单位，不翻译）。
fn bandwidth_text(lo: f64, hi: f64) -> String {
  let hi = if hi.is_finite() {
    format!("{hi:.0}")
  } else {
    "∞".into()
  };
  format!("{lo:.0}–{hi} Hz")
}

#[component]
pub fn SignalIdPage() -> impl IntoView {
  set_title("shell.signal-id-title");
  let sound = RwSignal::new(ANY.to_string());
  let rhythm = RwSignal::new(ANY.to_string());
  let bandwidth = RwSignal::new(ANY.to_string());

  let candidates = Signal::derive(move || {
    let query = SignalQuery {
      sound: parse_sound(&sound.get()),
      rhythm: parse_rhythm(&rhythm.get()),
      bandwidth: parse_bandwidth(&bandwidth.get()),
    };
    signal_id::narrow(&query)
  });

  view! {
    <PageHeader
      title=Signal::derive(|| t("shell.signal-id-title"))
      subtitle=Signal::derive(|| t("shell.signal-id-sub"))
    />
    <PageContainer>
      <section class="space-y-4" aria-label=move || t("signal.filters")>
        <div class="space-y-2">
          <div class="text-xs font-semibold text-muted-foreground">{move || t("signal.group-sound")}</div>
          <ChipGroup
            value=Signal::derive(move || sound.get())
            on_change=Callback::new(move |v: String| sound.set(v))
            aria_label=Signal::derive(|| t("signal.group-sound"))
          >
            <Chip value=ANY>{move || t("signal.any")}</Chip>
            <Chip value="speech">{move || t("signal.sound-speech")}</Chip>
            <Chip value="single">{move || t("signal.sound-single")}</Chip>
            <Chip value="dual">{move || t("signal.sound-dual")}</Chip>
            <Chip value="multi">{move || t("signal.sound-multi")}</Chip>
            <Chip value="noise">{move || t("signal.sound-noise")}</Chip>
            <Chip value="chirp">{move || t("signal.sound-chirp")}</Chip>
          </ChipGroup>
        </div>

        <div class="space-y-2">
          <div class="text-xs font-semibold text-muted-foreground">{move || t("signal.group-rhythm")}</div>
          <ChipGroup
            value=Signal::derive(move || rhythm.get())
            on_change=Callback::new(move |v: String| rhythm.set(v))
            aria_label=Signal::derive(|| t("signal.group-rhythm"))
          >
            <Chip value=ANY>{move || t("signal.any")}</Chip>
            <Chip value="continuous">{move || t("signal.rhythm-continuous")}</Chip>
            <Chip value="keyed">{move || t("signal.rhythm-keyed")}</Chip>
            <Chip value="burst">{move || t("signal.rhythm-burst")}</Chip>
            <Chip value="packet">{move || t("signal.rhythm-packet")}</Chip>
            <Chip value="sweep">{move || t("signal.rhythm-sweep")}</Chip>
          </ChipGroup>
        </div>

        <div class="space-y-2">
          <div class="text-xs font-semibold text-muted-foreground">{move || t("signal.group-bandwidth")}</div>
          <ChipGroup
            value=Signal::derive(move || bandwidth.get())
            on_change=Callback::new(move |v: String| bandwidth.set(v))
            aria_label=Signal::derive(|| t("signal.group-bandwidth"))
          >
            <Chip value=ANY>{move || t("signal.any")}</Chip>
            <Chip value="narrow">{move || t("signal.bandwidth-narrow")}</Chip>
            <Chip value="medium">{move || t("signal.bandwidth-medium")}</Chip>
            <Chip value="wide">{move || t("signal.bandwidth-wide")}</Chip>
            <Chip value="broad">{move || t("signal.bandwidth-broad")}</Chip>
          </ChipGroup>
        </div>
      </section>

      // live region 只圈住计数行：挂在整个结果区上，每点一个 chip 都会把全部候选卡重播一遍。
      <section class="space-y-3">
        // live region 落在 `role="status"` 的 span 上，而不是 heading 上：`aria-live`
        // 挂在标题角色上时部分读屏不播报，而把 heading 改成 status 又会丢掉标题语义。
        <h2 class="pxl-section-head">
          <span role="status">{move || tf("signal.results", &[&candidates.get().len().to_string()])}</span>
        </h2>
        {move || {
          let list = candidates.get();
          if list.is_empty() {
            return view! {
              <p class="bg-muted/40 px-3 py-2 text-sm text-muted-foreground">{move || t("signal.empty")}</p>
            }
              .into_any();
          }
          list
            .into_iter()
            .map(|p| {
              let key = p.key;
              view! {
                <article class="pxl-card border bg-card">
                  <div class="flex flex-wrap items-baseline gap-x-3 gap-y-1 px-4 py-3">
                    <h3 class="text-sm font-semibold">{move || t(name_key(p.key))}</h3>
                    <span class="text-xs tabular-nums text-muted-foreground">
                      {bandwidth_text(p.bandwidth_hz.0, p.bandwidth_hz.1)}
                    </span>
                  </div>
                  <p class="px-4 pb-3 text-sm text-muted-foreground">{move || t(hint_key(p.key))}</p>
                  {p
                    .page
                    .map(|href| {
                      view! {
                        <div class="border-t px-4 py-3">
                          // 十几张卡的链接文字都是同一句，读屏的链接列表里分不出谁是谁：
                          // 可访问名补上模式名。名称里要包含可见文字（WCAG 2.5.3），所以是
                          // 「打开对应页面（FT8）」而不是单独一个「打开 FT8」。
                          <ButtonLink
                            href=href
                            size=Size::Sm
                            aria_label=Signal::derive(move || {
                              format!("{} ({})", t("signal.open"), t(name_key(key)))
                            })
                          >
                            {move || t("signal.open")}
                          </ButtonLink>
                        </div>
                      }
                    })}
                </article>
              }
            })
            .collect_view()
            .into_any()
        }}
      </section>
    </PageContainer>
  }
}
