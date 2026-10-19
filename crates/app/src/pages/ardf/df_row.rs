//! 一行测向输入：坐标、指向目标的方位角与角度误差。

use leptos::prelude::*;

use crate::i18n::tf;
use crate::ui::{Field, NumberField, NumberKind};

#[component]
pub(super) fn DfRow(
  /// 行号（从 0 开始，界面显示时 +1）。
  index: usize,
  lat: RwSignal<String>,
  lon: RwSignal<String>,
  azimuth: RwSignal<String>,
  sigma: RwSignal<String>,
) -> impl IntoView {
  let number = index + 1;
  let lat_id = format!("df-{index}-lat");
  let lon_id = format!("df-{index}-lon");
  let az_id = format!("df-{index}-az");
  let sigma_id = format!("df-{index}-sigma");

  view! {
    <fieldset class="space-y-3 border p-3">
      <legend class="px-1 text-xs font-semibold text-muted-foreground">
        {move || tf("ardf.df-row", &[&number.to_string()])}
      </legend>
      <div class="grid gap-3 sm:grid-cols-2">
        <Field
          label=Signal::derive(move || tf("ardf.df-lat-n", &[&number.to_string()]))
          r#for=lat_id.clone()
        >
          <NumberField
            id=lat_id.clone()
            value=Signal::derive(move || lat.get())
            on_change=Callback::new(move |v: String| lat.set(v))
            kind=NumberKind::Decimal
            step=0.0001
            controls=false
          />
        </Field>
        <Field
          label=Signal::derive(move || tf("ardf.df-lon-n", &[&number.to_string()]))
          r#for=lon_id.clone()
        >
          <NumberField
            id=lon_id.clone()
            value=Signal::derive(move || lon.get())
            on_change=Callback::new(move |v: String| lon.set(v))
            kind=NumberKind::Decimal
            step=0.0001
            controls=false
          />
        </Field>
        <Field
          label=Signal::derive(move || tf("ardf.df-azimuth-n", &[&number.to_string()]))
          r#for=az_id.clone()
        >
          <NumberField
            id=az_id.clone()
            value=Signal::derive(move || azimuth.get())
            on_change=Callback::new(move |v: String| azimuth.set(v))
            step=1.0
            controls=false
          />
        </Field>
        <Field
          label=Signal::derive(move || tf("ardf.df-sigma-n", &[&number.to_string()]))
          r#for=sigma_id.clone()
        >
          <NumberField
            id=sigma_id.clone()
            value=Signal::derive(move || sigma.get())
            on_change=Callback::new(move |v: String| sigma.set(v))
            kind=NumberKind::Decimal
            step=0.5
            max=45.0
            controls=false
            invalid=Signal::derive(move || {
              sigma
                .get()
                .trim()
                .parse::<f64>()
                .is_ok_and(|v| !v.is_finite() || v <= 0.0 || v > 45.0)
            })
          />
        </Field>
      </div>
    </fieldset>
  }
}
