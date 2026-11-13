//! 台站链路预算：发射机 → 馈线 → 天线 → 路径 → 天线 → 馈线 → 接收机 → 噪声 → 所需 SNR。

use ham_web_core::feedline::FEEDLINE_SPECS;
use ham_web_core::noise_model::{self, Environment};
use ham_web_core::station_link::{
  self, FAR_FIELD_WAVELENGTHS, LinkInput, Stage, cable_loss_db, format_power_w, format_reach_km,
};
use leptos::prelude::*;

use super::atmosphere_fields::AtmosphereFields;
use super::{num, result_class};
use crate::i18n::{t, tf};
use crate::ui::{
  Field, NumberField, SelectOption, SimpleSelect, Table, TableBody, TableCell, TableDensity,
  TableHead, TableHeader, TableRow,
};
use crate::util::unique_id;

/// 环节的词条 key。
fn stage_key(stage: Stage) -> &'static str {
  match stage {
    Stage::TxLine => "tools.station-link-stage-tx-line",
    Stage::RxLine => "tools.station-link-stage-rx-line",
    Stage::Misc => "tools.station-link-stage-misc",
    Stage::RxNoiseFigure => "tools.station-link-stage-rx-nf",
    Stage::ExternalNoise => "tools.station-link-stage-ext",
  }
}

/// P.372 环境类别的词条 key（下拉选项与说明文字共用，名字里带曲线字母）。
fn env_key(env: Environment) -> &'static str {
  match env {
    Environment::City => "tools.station-link-env-city",
    Environment::Residential => "tools.station-link-env-residential",
    Environment::Rural => "tools.station-link-env-rural",
    Environment::QuietRural => "tools.station-link-env-quiet-rural",
    Environment::Galactic => "tools.station-link-env-galactic",
  }
}

/// dB / dBm 读数：两位小数，符号照实显示（接收电平、噪声几乎总是负的）。
/// 余量的「够 / 不够」由调用方选 `verdict-ok` / `verdict-short` 词条表达，那里传的是非负的绝对值。
fn fmt_db(v: f64) -> String {
  format!("{v:.2}")
}

#[component]
pub(super) fn StationLink() -> impl IntoView {
  let rg213 = FEEDLINE_SPECS
    .iter()
    .position(|s| s.name == "RG-213")
    .unwrap_or(0);

  let freq = RwSignal::new(String::from("145.5"));
  let dist = RwSignal::new(String::from("200"));
  let path_extra = RwSignal::new(String::from("0"));
  let power = RwSignal::new(String::from("5"));
  let tx_kind = RwSignal::new(rg213);
  let tx_len = RwSignal::new(String::from("10"));
  let tx_gain = RwSignal::new(String::from("3"));
  let rx_gain = RwSignal::new(String::from("3"));
  let rx_kind = RwSignal::new(rg213);
  let rx_len = RwSignal::new(String::from("10"));
  let misc = RwSignal::new(String::from("1"));
  let nf = RwSignal::new(String::from("2"));
  let ext = RwSignal::new(String::from("0"));
  // 外部噪声来源：`None` = 自填（下面的 Fa 栏），`Some(i)` = `Environment::ALL[i]`，
  // `Some(Environment::ALL.len())` = 大气噪声（P.372 第 5 部分，需要月 / 时 / 坐标）。
  // 默认自填，与加预设之前的行为逐位一致。
  let env = RwSignal::new(0usize);
  // 下拉里「大气噪声」的序号（排在五条环境曲线之后）。
  //
  // 选项值是 `Environment::ALL` 的下标 + 1（1..=5），所以这一项必须从 `len + 1` 起，
  // 否则会和最后一条曲线（银河噪声）撞值 —— 撞了之后 `preset()` 会把「大气噪声」
  // 解析成银河噪声，页面显示的是银河的范围提示。
  let atmos_index = Environment::ALL.len() + 1;
  // 大气噪声（P.372 第 5 部分）需要的四个量：月、UTC 小时、纬度、经度。
  // 默认取「7 月、本地傍晚、北京」，与 `atmospheric_noise` 的示例口径一致。
  let atm_month = RwSignal::new(String::from("7"));
  let atm_hour = RwSignal::new(String::from("12"));
  let atm_lat = RwSignal::new(String::from("40"));
  let atm_lon = RwSignal::new(String::from("116"));
  let bw = RwSignal::new(String::from("12500"));
  let req = RwSignal::new(String::from("12"));

  let options = || -> Vec<SelectOption> {
    FEEDLINE_SPECS
      .iter()
      .enumerate()
      .map(|(i, s)| SelectOption::new(i.to_string(), s.name))
      .collect()
  };
  let tx_options = options();
  let rx_options = options();
  let env_options: Vec<SelectOption> = std::iter::once(SelectOption::new(
    "0",
    Signal::derive(move || t("tools.station-link-env-custom")),
  ))
  .chain(
    Environment::ALL
      .iter()
      .enumerate()
      .map(|(i, &e)| SelectOption::new((i + 1).to_string(), Signal::derive(move || t(env_key(e))))),
  )
  // 大气噪声：需要月 / 时 / 位置，选它之后页面上多出这几个栏。
  .chain(std::iter::once(SelectOption::new(
    atmos_index.to_string(),
    Signal::derive(move || t("tools.station-link-env-atmos")),
  )))
  .collect();

  let freq_id = unique_id("slink-freq");
  let dist_id = unique_id("slink-dist");
  let extra_id = unique_id("slink-extra");
  let power_id = unique_id("slink-power");
  let tx_kind_id = unique_id("slink-tx-kind");
  let tx_len_id = unique_id("slink-tx-len");
  let tx_gain_id = unique_id("slink-tx-gain");
  let rx_gain_id = unique_id("slink-rx-gain");
  let rx_kind_id = unique_id("slink-rx-kind");
  let rx_len_id = unique_id("slink-rx-len");
  let misc_id = unique_id("slink-misc");
  let nf_id = unique_id("slink-nf");
  let ext_id = unique_id("slink-ext");
  let env_id = unique_id("slink-env");
  let bw_id = unique_id("slink-bw");
  let req_id = unique_id("slink-req");

  // 选了 P.372 预设时的 `(环境, 频率)`；自填、「大气噪声」或频率不是数则 `None`
  // （大气噪声走 `atmos_fa()`，不能让这里再解析成某条环境曲线）。
  let preset = move || -> Option<(Environment, f64)> {
    let i = env.get().checked_sub(1)?;
    if i >= Environment::ALL.len() {
      return None;
    }
    let e = *Environment::ALL.get(i)?;
    Some((e, num(freq)?))
  };
  // 预设给出的 Fa：`None` = 没选预设；`Some(None)` = 频率在该曲线的适用频段之外（不外推）。
  let preset_fa = move || preset().map(|(e, f)| noise_model::fam_db(e, f));

  // 大气噪声那一项：`Some(None)` = 输入越界（月 / 小时 / 坐标 / 频率），页面给出原因。
  let is_atmos = move || env.get() == atmos_index;
  // 返回 `None` = 不是大气噪声这一项；`Some(None)` = 是它但输入还不合法
  // （月不是 1–12、坐标或频率越界）—— 后者要落到「输入无效」，不能悄悄退回自填。
  let atmos_fa = move || -> Option<Option<f64>> {
    if !is_atmos() {
      return None;
    }
    let input = (|| {
      Some(ham_web_core::atmospheric_noise::AtmNoiseInput {
        month: ham_web_core::atmospheric_noise::month_from_f64(num(atm_month)?)?,
        hour_utc: num(atm_hour)?,
        lat_deg: num(atm_lat)?,
        lon_deg: num(atm_lon)?,
        freq_mhz: num(freq)?,
      })
    })();
    Some(input.and_then(|i| ham_web_core::atmospheric_noise::atmospheric(&i).map(|n| n.fa_db)))
  };
  // 预设在范围内：说明取到的值（显示在下拉的提示行）。
  let preset_result = move || -> Option<String> {
    let (e, f) = preset()?;
    let fa = noise_model::fam_db(e, f)?;
    let shown = freq.get().trim().to_owned();
    Some(tf(
      "tools.station-link-env-result",
      &[&t(env_key(e)), &shown, &fmt_db(fa)],
    ))
  };
  // 预设在范围外：说明为什么没有值（显示在结果框里，替代泛泛的「输入无效」）。
  // 两句话分开放，是为了同一句不会同时出现在提示行与结果框、被读屏读两遍。
  let preset_range = move || -> Option<String> {
    let (e, f) = preset()?;
    if noise_model::fam_db(e, f).is_some() {
      return None;
    }
    let c = e.curve();
    let shown = freq.get().trim().to_owned();
    Some(tf(
      "tools.station-link-env-range",
      &[
        &t(env_key(e)),
        &c.lo_mhz.to_string(),
        &c.hi_mhz.to_string(),
        &shown,
      ],
    ))
  };

  // 输入 → (馈线损耗, 结果)。纯派生，不进 Effect。
  // 任何一栏不是有限数就整条链路「输入无效」，不拿默认值悄悄顶上；
  // 取值范围（负长度、非正频率……）由内核拒绝，同样落到「输入无效」。
  // 选了预设而频率在曲线范围之外，同样不出结果（不外推），页面另给原因。
  let outcome = move || {
    let f = num(freq)?;
    let (tx_line_db, rx_line_db) = (
      cable_loss_db(tx_kind.get(), num(tx_len)?, f)?,
      cable_loss_db(rx_kind.get(), num(rx_len)?, f)?,
    );
    let input = LinkInput {
      freq_mhz: f,
      dist_km: num(dist)?,
      path_extra_db: num(path_extra)?,
      tx_power_w: num(power)?,
      tx_line_db,
      tx_gain_dbi: num(tx_gain)?,
      rx_gain_dbi: num(rx_gain)?,
      rx_line_db,
      misc_loss_db: num(misc)?,
      rx_nf_db: num(nf)?,
      ext_noise_db: if is_atmos() {
        atmos_fa()??
      } else {
        match preset_fa() {
          None => num(ext)?,
          Some(fa) => fa?,
        }
      },
      bandwidth_hz: num(bw)?,
      required_snr_db: num(req)?,
    };
    station_link::evaluate(&input).map(|r| (tx_line_db, rx_line_db, r))
  };

  view! {
    // `[&>*]:min-w-0`：网格子项默认 `min-width:auto`。「外部噪声来源」下拉里是一整行不换行的文字
    // （如「银河噪声（P.372 曲线 E，理论下限）」），不加这个会把整列撑宽，手机宽度下整页横向滚动
    // （西班牙语 320 px 实测超出 38 px）。同 `ToroidWinding` / `CommonModeChoke`。
    <div class="grid gap-3 sm:grid-cols-2 [&>*]:min-w-0">
      <Field label=Signal::derive(move || t("log.frequency-mhz")) r#for=freq_id.clone()>
        <NumberField id=freq_id value=Signal::derive(move || freq.get()) on_change=Callback::new(move |v: String| freq.set(v)) controls=false />
      </Field>
      <Field label=Signal::derive(move || t("tools.distance-km")) r#for=dist_id.clone()>
        <NumberField id=dist_id value=Signal::derive(move || dist.get()) on_change=Callback::new(move |v: String| dist.set(v)) controls=false />
      </Field>
      <Field
        label=Signal::derive(move || t("tools.station-link-path-extra"))
        hint=Signal::derive(move || t("tools.station-link-path-extra-hint"))
        r#for=extra_id.clone()
      >
        <NumberField id=extra_id value=Signal::derive(move || path_extra.get()) on_change=Callback::new(move |v: String| path_extra.set(v)) controls=false />
      </Field>
      <Field label=Signal::derive(move || t("tools.transmit-power-w")) r#for=power_id.clone()>
        <NumberField id=power_id value=Signal::derive(move || power.get()) on_change=Callback::new(move |v: String| power.set(v)) controls=false />
      </Field>

      <Field label=Signal::derive(move || t("tools.station-link-tx-cable")) r#for=tx_kind_id.clone()>
        <SimpleSelect
          id=tx_kind_id
          value=Signal::derive(move || tx_kind.get().to_string())
          on_change=Callback::new(move |v: String| {
            if let Ok(n) = v.parse::<usize>() {
              tx_kind.set(n);
            }
          })
          options=tx_options
          aria_label=Signal::derive(move || t("tools.station-link-tx-cable"))
        />
      </Field>
      <Field label=Signal::derive(move || t("tools.station-link-tx-length")) r#for=tx_len_id.clone()>
        <NumberField id=tx_len_id value=Signal::derive(move || tx_len.get()) on_change=Callback::new(move |v: String| tx_len.set(v)) controls=false />
      </Field>
      <Field label=Signal::derive(move || t("tools.transmit-antenna-gain-dbi")) r#for=tx_gain_id.clone()>
        <NumberField id=tx_gain_id value=Signal::derive(move || tx_gain.get()) on_change=Callback::new(move |v: String| tx_gain.set(v)) controls=false />
      </Field>
      <Field label=Signal::derive(move || t("tools.receive-antenna-gain-dbi")) r#for=rx_gain_id.clone()>
        <NumberField id=rx_gain_id value=Signal::derive(move || rx_gain.get()) on_change=Callback::new(move |v: String| rx_gain.set(v)) controls=false />
      </Field>

      <Field label=Signal::derive(move || t("tools.station-link-rx-cable")) r#for=rx_kind_id.clone()>
        <SimpleSelect
          id=rx_kind_id
          value=Signal::derive(move || rx_kind.get().to_string())
          on_change=Callback::new(move |v: String| {
            if let Ok(n) = v.parse::<usize>() {
              rx_kind.set(n);
            }
          })
          options=rx_options
          aria_label=Signal::derive(move || t("tools.station-link-rx-cable"))
        />
      </Field>
      <Field label=Signal::derive(move || t("tools.station-link-rx-length")) r#for=rx_len_id.clone()>
        <NumberField id=rx_len_id value=Signal::derive(move || rx_len.get()) on_change=Callback::new(move |v: String| rx_len.set(v)) controls=false />
      </Field>
      <Field
        label=Signal::derive(move || t("tools.station-link-misc"))
        hint=Signal::derive(move || t("tools.station-link-misc-hint"))
        r#for=misc_id.clone()
      >
        <NumberField id=misc_id value=Signal::derive(move || misc.get()) on_change=Callback::new(move |v: String| misc.set(v)) controls=false />
      </Field>
      <Field label=Signal::derive(move || t("tools.noise-figure-nf-db")) r#for=nf_id.clone()>
        <NumberField id=nf_id value=Signal::derive(move || nf.get()) on_change=Callback::new(move |v: String| nf.set(v)) controls=false />
      </Field>

      <Field
        label=Signal::derive(move || t("tools.station-link-env"))
        hint=Signal::derive(move || preset_result().unwrap_or_else(|| t("tools.station-link-env-hint")))
        r#for=env_id.clone()
      >
        <SimpleSelect
          id=env_id
          value=Signal::derive(move || env.get().to_string())
          on_change=Callback::new(move |v: String| {
            if let Ok(n) = v.parse::<usize>() {
              env.set(n);
            }
          })
          options=env_options
          aria_label=Signal::derive(move || t("tools.station-link-env"))
        />
      </Field>
      // 大气噪声：月 / UTC 小时 / 纬度 / 经度。只在选中那一项时出现。
      {move || is_atmos().then(|| view! {
        <AtmosphereFields month=atm_month hour=atm_hour lat=atm_lat lon=atm_lon id_prefix="slink" />
      })}
      // 手填的 Fa 只在「自填」时用；选了预设就不显示，免得出现一个填了却不生效的栏。
      {move || {
        (env.get() == 0)
          .then(|| {
            let ext_id = ext_id.clone();
            view! {
              <Field
                label=Signal::derive(move || t("tools.station-link-ext-noise"))
                hint=Signal::derive(move || t("tools.station-link-ext-noise-hint"))
                r#for=ext_id.clone()
              >
                <NumberField id=ext_id value=Signal::derive(move || ext.get()) on_change=Callback::new(move |v: String| ext.set(v)) controls=false />
              </Field>
            }
          })
      }}
      <Field label=Signal::derive(move || t("tools.station-link-bandwidth")) r#for=bw_id.clone()>
        <NumberField id=bw_id value=Signal::derive(move || bw.get()) on_change=Callback::new(move |v: String| bw.set(v)) controls=false />
      </Field>
      <Field
        label=Signal::derive(move || t("tools.station-link-required-snr"))
        hint=Signal::derive(move || t("tools.station-link-required-snr-hint"))
        r#for=req_id.clone()
      >
        <NumberField id=req_id value=Signal::derive(move || req.get()) on_change=Callback::new(move |v: String| req.set(v)) controls=false />
      </Field>

      {move || match outcome() {
        None => {
          // 预设的频率在曲线范围之外：原因是「不外推」，不是「输入无效」，分开说。
          let text = preset_range().unwrap_or_else(|| t("tools.station-link-invalid"));
          view! {
            <div class=result_class()>
              <p>{text}</p>
            </div>
          }
            .into_any()
        }
        Some((tx_line_db, rx_line_db, r)) => {
          let verdict = if r.margin_db >= 0.0 {
            tf(
              "tools.station-link-verdict-ok",
              &[&fmt_db(r.margin_db), &fmt_db(r.snr_db), &fmt_db(r.snr_db - r.margin_db)],
            )
          } else {
            tf(
              "tools.station-link-verdict-short",
              &[&fmt_db(-r.margin_db), &fmt_db(r.snr_db), &fmt_db(r.snr_db - r.margin_db)],
            )
          };
          let levels = tf(
            "tools.station-link-levels",
            &[
              &fmt_db(r.eirp_dbm),
              &fmt_db(r.path_loss_db),
              &fmt_db(r.signal_dbm),
              &fmt_db(r.noise_dbm),
            ],
          );
          let share = match r.ext_share {
            Some(s) => tf(
              "tools.station-link-noise-share",
              &[&format!("{:.1}%", s * 100.0), &format!("{:.1}%", (1.0 - s) * 100.0)],
            ),
            None => t("tools.station-link-noise-thermal"),
          };
          let lines = tf(
            "tools.station-link-lines",
            &[&fmt_db(tx_line_db), &fmt_db(rx_line_db)],
          );
          let reach = tf(
            "tools.station-link-reach",
            &[&format_reach_km(r.max_dist_km), &format_power_w(r.required_tx_w)],
          );
          // 距离不到十个波长：自由空间公式失效，结果只能当量级参考。
          let near_field = r.near_field.then(|| {
            tf(
              "tools.station-link-near-field",
              &[&FAR_FIELD_WAVELENGTHS.to_string(), &format_reach_km(r.far_field_km)],
            )
          });
          // 第一名回收不到 0.005 dB（显示精度之下）就当「都接近理想」，不拿浮点噪声排名。
          let first = r.reclaim[0];
          let first_line = if first.gain_db < 0.005 {
            t("tools.station-link-first-none")
          } else {
            tf(
              "tools.station-link-first",
              &[&t(stage_key(first.stage)), &fmt_db(first.gain_db)],
            )
          };
          let rows = r.reclaim;
          view! {
            <div class=format!("{} space-y-1 tabular-nums", result_class())>
              {near_field.map(|text| view! { <p role="alert">{text}</p> })}
              <p>{verdict}</p>
              <p>{levels}</p>
              <p>{share}</p>
              <p>{lines}</p>
              <p>{reach}</p>
              <p>{first_line}</p>
            </div>
            // 不要给 `Table` 再套一层 `div.sm:col-span-2`（会把整页撑出手机宽度，见
            // `atu_synth.rs` 的同一条注释）：跨列写进 `wrapper_class`。
            <Table
              aria_label=Signal::derive(move || t("tools.station-link-table"))
              class="min-w-[320px]"
              wrapper_class="border sm:col-span-2"
              density=TableDensity::Compact
            >
              <TableHead>
                <TableRow>
                  <TableHeader class=crate::kit::table_cell_class("")>{move || t("tools.station-link-col-stage")}</TableHeader>
                  <TableHeader class=crate::kit::table_cell_class("")>{move || t("tools.station-link-col-gain")}</TableHeader>
                </TableRow>
              </TableHead>
              <TableBody>
                {rows
                  .into_iter()
                  .map(|x| {
                    view! {
                      <TableRow>
                        <TableCell class=crate::kit::table_cell_class("")>{move || t(stage_key(x.stage))}</TableCell>
                        <TableCell class=crate::kit::table_cell_class("font-mono tabular-nums")>
                          {fmt_db(x.gain_db)}
                        </TableCell>
                      </TableRow>
                    }
                  })
                  .collect_view()}
              </TableBody>
            </Table>
          }
            .into_any()
        }
      }}

      <p class="sm:col-span-2 text-xs text-muted-foreground">{move || t("tools.station-link-notes")}</p>
    </div>
  }
}
