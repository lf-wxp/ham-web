use ham_web_core::sat_transponder::{PassbandError, TRANSPONDERS, Transponder};
use leptos::prelude::*;

use crate::i18n::{t, tf};
use crate::ui::{
  Button, Checkbox, Field, Label, NativeSelect, NumberField, SelectOption, Size, Variant,
};
use crate::util::unique_id;

/// 下拉里「自定义」一项的值（预置转发器的名字里不会出现它）。
const CUSTOM: &str = "custom";

/// 步进按钮的步长（Hz）：线性转发器上 SSB 的细调是百赫兹量级，粗调到千赫兹。
const STEPS_HZ: [i64; 4] = [-1000, -100, 100, 1000];

/// MHz，保留到 1 Hz（6 位小数）。
fn fmt_mhz(hz: u64) -> String {
  format!("{:.6}", hz as f64 / 1e6)
}

/// 把输入框里的 MHz 文本变成 Hz；空、乱码、非正、非有限数为 `None`。
fn parse_mhz(text: &str) -> Option<u64> {
  let v: f64 = text.trim().parse().ok()?;
  (v.is_finite() && v > 0.0 && v < 1.0e6).then(|| (v * 1e6).round() as u64)
}

/// 自定义通带不合法的原因 → 词条（`t()` 的字面量写在调用点，静态扫描才认得）。
fn passband_error_text(e: PassbandError) -> String {
  match e {
    PassbandError::NotPositive => t("radio.linear-custom-error-positive"),
    PassbandError::Inverted => t("radio.linear-custom-error-inverted"),
    PassbandError::WidthMismatch => t("radio.linear-custom-error-width-mismatch"),
    PassbandError::WidthOutOfRange => t("radio.linear-custom-error-width-range"),
  }
}

/// 线性转发器的调谐面板：选转发器，设「卫星坐标系下的下行调谐点」，上行按通带映射跟着动。
///
/// 这里的频率都是卫星自己收发的频率（不含多普勒）；地面实际要调的收发频率由外面再做多普勒修正。
/// 调谐点始终夹在通带内：步进按钮不会把它推出通带；手填越界时调谐点被拉回通带边界、
/// 输入框保持原文，并提示「已拉回」。手输半截（如 `4`、`435.`）不会被夹紧重排，可以继续敲。
///
/// 本面板**不跟电台旋钮**：调谐只在这里做（页面步进 / 手填），电台上转旋钮仍然会触发「手动改频即停」。
#[component]
pub(super) fn LinearTuner(
  /// 当前选中的转发器（`None` = 没选，线性模式不起作用）。
  active: RwSignal<Option<Transponder>>,
  /// 卫星坐标系的下行调谐点（Hz）。
  tuned_hz: RwSignal<u64>,
  /// 用户选了一个转发器：外层据此带出它的 NORAD 编号并作废已加载的过境。
  /// 选转发器**必须**连带换卫星，否则会拿 A 卫星的轨迹去算 B 卫星的多普勒，页面看起来一切正常、频率却全是错的。
  on_pick: Callback<Transponder>,
) -> impl IntoView {
  let tp_id = unique_id("linear-transponder");
  let custom_inverting_id = StoredValue::new(unique_id("linear-custom-inverting"));
  let ul_lo_id = StoredValue::new(unique_id("linear-custom-ul-lo"));
  let ul_hi_id = StoredValue::new(unique_id("linear-custom-ul-hi"));
  let dl_lo_id = StoredValue::new(unique_id("linear-custom-dl-lo"));
  let dl_hi_id = StoredValue::new(unique_id("linear-custom-dl-hi"));
  // 自定义通带的四个端点（输入框原文）与反相开关；解析与校验在 `apply_custom` 里做。
  let custom_on = RwSignal::new(false);
  // 自定义通带是否已**真正生效**（合法并写进了 `active`）。选了「自定义」但还没填合法时为 false：
  // 此时下方不显示通带摘要，免得下拉写着「自定义」、摘要却是别的转发器的通带，让人误以为在用自定义的。
  let custom_applied = RwSignal::new(false);
  let ul_lo = RwSignal::new(String::new());
  let ul_hi = RwSignal::new(String::new());
  let dl_lo = RwSignal::new(String::new());
  let dl_hi = RwSignal::new(String::new());
  let custom_inverting = RwSignal::new(true);
  let custom_error = RwSignal::new(None::<PassbandError>);
  let dl_id = unique_id("linear-downlink");

  // 只显示转发器名：正反相在选中后的「通带」一行里用文字写明（走词条，三语都有）。
  let options: Vec<SelectOption> = TRANSPONDERS
    .iter()
    .map(|t| SelectOption::new(t.name, t.name))
    .chain(std::iter::once(SelectOption::new(
      CUSTOM,
      Signal::derive(move || t("radio.linear-custom")),
    )))
    .collect();

  // 手填的下行 MHz 文本；解析不出（空 / 乱码 / 半截）就保持原值不动，而不是回落成 0。
  let typed = RwSignal::new(String::new());
  // 手填的值越界被拉回通带时置真，用来显示「已拉回」提示；合法输入或外部改变时清掉。
  let was_clamped = RwSignal::new(false);

  // 把 `hz` 夹进当前转发器的通带并写回 `tuned_hz`；返回是否发生了夹紧（即输入越界）。
  let set_tuned = move |hz: u64| -> bool {
    match active.get_untracked() {
      Some(tp) => {
        let clamped = tp.clamp_downlink(hz);
        tuned_hz.set(clamped);
        clamped != hz
      }
      None => false,
    }
  };
  // 校验自定义通带：合法才换进 `active`；不合法**不动当前通带**并显示原因。
  // 若一边填、一边就把 `active` 清空，正在进行的自动调谐会被悄悄打断，用户还不知道为什么。
  // 四个端点没填全时不报错（还在输入中），只是不生效。
  let apply_custom = move || {
    let parsed = (
      parse_mhz(&ul_lo.get_untracked()),
      parse_mhz(&ul_hi.get_untracked()),
      parse_mhz(&dl_lo.get_untracked()),
      parse_mhz(&dl_hi.get_untracked()),
    );
    let (Some(a), Some(b), Some(c), Some(d)) = parsed else {
      custom_error.set(None);
      return;
    };
    match Transponder::custom((a, b), (c, d), custom_inverting.get_untracked()) {
      Ok(tp) => {
        custom_error.set(None);
        custom_applied.set(true);
        active.set(Some(tp));
        let center = tp.downlink_center();
        tuned_hz.set(center);
        typed.set(fmt_mhz(center));
        was_clamped.set(false);
        // 自定义转发器没有 NORAD：不调 `on_pick`，不动卫星选择，用户自己在上方填 NORAD。
      }
      Err(e) => custom_error.set(Some(e)),
    }
  };
  // 外部改变调谐点（步进、切转发器、回中心）后，把文本同步成夹紧后的值。
  let sync_text = move |hz: u64| typed.set(fmt_mhz(hz));

  view! {
    <div class="space-y-3 border border-dashed px-3 py-3" data-slot="linear-tuner">
      <div class="grid gap-3 sm:grid-cols-2 [&>*]:min-w-0">
        <Field label=Signal::derive(move || t("radio.linear-transponder")) r#for=tp_id.clone()>
          <NativeSelect
            id=tp_id
            value=Signal::derive(move || {
              if custom_on.get() {
                CUSTOM.to_owned()
              } else {
                active.get().map_or_else(String::new, |t| t.name.to_owned())
              }
            })
            on_change=Callback::new(move |name: String| {
              if name == CUSTOM {
                // 进入自定义：通带要等 `apply_custom` 校验通过才生效。
                custom_on.set(true);
                // 刚切到自定义、还没有合法的自定义通带：旧通带不再展示，所以也**不能继续按它联动**。
                // 置空后 `target` / `tx_plan` 都为 None，自动调谐停止写电台，直到填出合法通带。
                // （自定义通带生效之后再被改成非法，走的是 `apply_custom` 的 Err 分支，沿用上一个自定义通带，
                // 免得敲错一位就打断联动；那条路径不经过这里。）
                active.set(None);
                custom_applied.set(false);
                return;
              }
              if let Some(tp) = TRANSPONDERS.iter().find(|t| t.name == name) {
                custom_on.set(false);
                custom_applied.set(false);
                custom_error.set(None);
                active.set(Some(*tp));
                let center = tp.downlink_center();
                tuned_hz.set(center);
                sync_text(center);
                was_clamped.set(false);
                on_pick.run(*tp);
              }
            })
            options=options
            aria_label=Signal::derive(move || t("radio.linear-transponder"))
          />
        </Field>
        <Field label=Signal::derive(move || t("radio.linear-downlink-mhz")) r#for=dl_id.clone()>
          <NumberField
            id=dl_id
            value=typed
            on_change=Callback::new(move |v: String| {
              // 手输期间保持原文，不重写输入框：只有完整解析出的值才去更新调谐点；
              // 半截输入（如 `4`、`435.`）不会被夹到边界再重排，光标后面可以继续敲。
              typed.set(v.clone());
              if let Ok(mhz) = v.trim().parse::<f64>()
                && mhz.is_finite()
                && mhz > 0.0
              {
                was_clamped.set(set_tuned((mhz * 1e6).round() as u64));
              }
            })
            controls=false
          />
        </Field>
      </div>

      <Show when=move || custom_on.get()>
        <div class="space-y-2 border-l-2 border-primary/60 pl-3" data-slot="linear-custom">
          <p class="text-xs text-muted-foreground">{move || t("radio.linear-custom-help")}</p>
          <div class="grid grid-cols-1 gap-3 sm:grid-cols-2 [&>*]:min-w-0">
            <Field label=Signal::derive(move || t("radio.linear-custom-ul-lo")) r#for=ul_lo_id.get_value()>
              <NumberField id=ul_lo_id.get_value() value=ul_lo on_change=Callback::new(move |v: String| { ul_lo.set(v); apply_custom(); }) controls=false />
            </Field>
            <Field label=Signal::derive(move || t("radio.linear-custom-ul-hi")) r#for=ul_hi_id.get_value()>
              <NumberField id=ul_hi_id.get_value() value=ul_hi on_change=Callback::new(move |v: String| { ul_hi.set(v); apply_custom(); }) controls=false />
            </Field>
            <Field label=Signal::derive(move || t("radio.linear-custom-dl-lo")) r#for=dl_lo_id.get_value()>
              <NumberField id=dl_lo_id.get_value() value=dl_lo on_change=Callback::new(move |v: String| { dl_lo.set(v); apply_custom(); }) controls=false />
            </Field>
            <Field label=Signal::derive(move || t("radio.linear-custom-dl-hi")) r#for=dl_hi_id.get_value()>
              <NumberField id=dl_hi_id.get_value() value=dl_hi on_change=Callback::new(move |v: String| { dl_hi.set(v); apply_custom(); }) controls=false />
            </Field>
          </div>
          <div class="flex items-center gap-2">
            <Checkbox
              id=custom_inverting_id.get_value()
              checked=custom_inverting
              on_change=Callback::new(move |on: bool| { custom_inverting.set(on); apply_custom(); })
            />
            <Label r#for=custom_inverting_id.get_value()>{move || t("radio.linear-custom-inverting")}</Label>
          </div>
          {move || custom_error.get().map(|e| view! {
            <div class="text-xs text-destructive" role="alert">{passband_error_text(e)}</div>
          })}
          <p class="text-xs text-muted-foreground">{move || t("radio.linear-custom-note")}</p>
        </div>
      </Show>

      {move || {
        let hidden = custom_on.get() && !custom_applied.get();
        (!hidden).then(|| active.get()).flatten().map(|tp| {
          let dl = tuned_hz.get();
          let ul = tp.uplink_for(dl);
          let (dl_lo, dl_hi) = (fmt_mhz(tp.downlink_lo), fmt_mhz(tp.downlink_hi));
          let (ul_lo, ul_hi) = (fmt_mhz(tp.uplink_lo), fmt_mhz(tp.uplink_hi));
          let kind = if tp.inverting { t("radio.linear-inverting") } else { t("radio.linear-non-inverting") };
          view! {
            <div class="space-y-1 text-xs text-muted-foreground tabular-nums">
              <div>{tf("radio.linear-passband", &[&ul_lo, &ul_hi, &dl_lo, &dl_hi, &kind])}</div>
              <div class="font-medium text-foreground" role="status">
                {match ul {
                  Some(ul) => tf("radio.linear-mapped", &[&fmt_mhz(dl), &fmt_mhz(ul)]),
                  // 防御：`tuned_hz` 恒被 clamp 在通带内，此分支正常不可达。
                  None => t("radio.linear-outside"),
                }}
              </div>
              {was_clamped.get().then(|| view! {
                <div class="text-destructive" role="alert">{t("radio.linear-clamped")}</div>
              })}
            </div>
          }
        })
      }}

      <div class="flex flex-wrap items-center gap-2" role="group" aria-label=move || t("radio.linear-steps")>
        {STEPS_HZ
          .iter()
          .map(|&step| {
            let label = if step.abs() >= 1000 {
              format!("{:+} kHz", step / 1000)
            } else {
              format!("{step:+} Hz")
            };
            view! {
              <Button
                variant=Variant::Outline
                size=Size::Sm
                disabled=Signal::derive(move || active.with(Option::is_none))
                on_click=Callback::new(move |_| {
                  if let Some(tp) = active.get_untracked() {
                    let hz = tp.step_downlink(tuned_hz.get_untracked(), step);
                    tuned_hz.set(hz);
                    sync_text(hz);
                    was_clamped.set(false);
                  }
                })
              >
                {label}
              </Button>
            }
          })
          .collect_view()}
        <Button
          variant=Variant::Outline
          size=Size::Sm
          disabled=Signal::derive(move || active.with(Option::is_none))
          on_click=Callback::new(move |_| {
            if let Some(tp) = active.get_untracked() {
              let center = tp.downlink_center();
              tuned_hz.set(center);
              sync_text(center);
              was_clamped.set(false);
            }
          })
        >
          {move || t("radio.linear-center")}
        </Button>
      </div>
    </div>
  }
}
