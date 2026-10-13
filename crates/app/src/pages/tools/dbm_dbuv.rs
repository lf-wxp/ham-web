use ham_web_core::decibel::{
  dbm_to_dbuv, dbm_to_uv, dbuv_to_dbm, dbuv_to_uv, uv_to_dbm, uv_to_dbuv,
};
use leptos::prelude::*;

use super::RESULT;
use crate::i18n::t;
use crate::ui::{Field, NumberField};
use crate::util::unique_id;

/// 换算结果的排版：按 6 位有效数字舍入，抹掉 `223872.1109238346` 这类浮点尾数。
///
/// 舍入放在**写进信号之前**，而不是显示时再格式化：`NumberField` 的显示就是 `to_string()`，
/// 若显示层格式化，用户正在敲的 `13` 会立刻被改写为 `13.00`。
///
/// `±1e-5` 以内的残差归零：`223872.1 μV` 换回 dBm 会得到 `−3.6e-8`，那是 107 取整值的
/// 固有误差（μV 显示到 6 位有效数字时换算误差本就到 1e-5 dB 量级），不该露给用户。
fn tidy(v: f64) -> f64 {
  if !v.is_finite() {
    return v;
  }
  let rounded: f64 = format!("{v:.5e}").parse().unwrap_or(v);
  if rounded.abs() < 1e-5 { 0.0 } else { rounded }
}

/// dBm ↔ dBμV ↔ μV 换算（50Ω 阻抗）：三个量互为输入，改一个联动另外两个。
#[component]
pub(super) fn DbmDbuv() -> impl IntoView {
  let dbm = RwSignal::new(0.0);
  let dbuv = RwSignal::new(107.0);
  let uv = RwSignal::new(tidy(dbm_to_uv(0.0)));

  let dbm_id = unique_id("dbm-dbuv-dbm");
  let dbuv_id = unique_id("dbm-dbuv-dbuv");
  let uv_id = unique_id("dbm-dbuv-uv");

  view! {
    <div class="space-y-4">
      <div class="grid gap-3 sm:grid-cols-3">
        <Field label=Signal::derive(move || t("tools.power-level-dbm")) r#for=dbm_id.clone()>
          <NumberField
            id=dbm_id
            value=Signal::derive(move || dbm.get().to_string())
            on_change=Callback::new(move |v: String| {
              // 只接受有限的换算结果：`1e9 dBm` 会让 μV 溢出成 `inf`，三个框再回不来。
              if let Ok(d) = v.trim().parse::<f64>()
                && d.is_finite()
              {
                let u = dbm_to_uv(d);
                if u.is_finite() && u > 0.0 {
                  dbm.set(d);
                  dbuv.set(tidy(dbm_to_dbuv(d)));
                  uv.set(tidy(u));
                }
              }
            })
            controls=false
          />
        </Field>
        <Field label=Signal::derive(move || t("tools.power-level-dbuv")) r#for=dbuv_id.clone()>
          <NumberField
            id=dbuv_id
            value=Signal::derive(move || dbuv.get().to_string())
            on_change=Callback::new(move |v: String| {
              if let Ok(dv) = v.trim().parse::<f64>()
                && dv.is_finite()
              {
                let u = dbuv_to_uv(dv);
                if u.is_finite() && u > 0.0 {
                  dbuv.set(dv);
                  dbm.set(tidy(dbuv_to_dbm(dv)));
                  uv.set(tidy(u));
                }
              }
            })
            controls=false
          />
        </Field>
        <Field label=Signal::derive(move || t("tools.voltage-uv")) r#for=uv_id.clone()>
          <NumberField
            id=uv_id
            value=Signal::derive(move || uv.get().to_string())
            on_change=Callback::new(move |v: String| {
              if let Ok(u) = v.trim().parse::<f64>()
                && u.is_finite()
                && u > 0.0
              {
                uv.set(u);
                dbuv.set(tidy(uv_to_dbuv(u)));
                dbm.set(tidy(uv_to_dbm(u)));
              }
            })
            controls=false
          />
        </Field>
      </div>

      <div class=RESULT>
        {move || format!("{} dBm = {} dBμV = {} μV", dbm.get(), dbuv.get(), uv.get())}
      </div>
    </div>
  }
}
