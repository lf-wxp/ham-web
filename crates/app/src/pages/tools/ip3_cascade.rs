use ham_web_core::ip3_cascade::{Cascade, Ip3Error, Referred, Stage, cascade};
use leptos::prelude::*;

use super::{RESULT, fmt_num};
use crate::i18n::{t, tf};
use crate::ui::{
  Button, Field, NativeSelect, NumberField, SelectOption, Size, Table, TableBody, TableCell,
  TableHead, TableHeader, TableRow, Variant,
};
use crate::util::unique_id;

/// 最多几级：再多也放得下，但界面会拖得很长，真有更长的链路分段算。
const MAX_STAGES: usize = 8;

/// 输入框里的一级（原样保留文本：空串 = 没填）。
#[derive(Debug, Clone, PartialEq)]
struct Row {
  gain: String,
  ip3: String,
}

fn row(gain: &str, ip3: &str) -> Row {
  Row {
    gain: gain.to_owned(),
    ip3: ip3.to_owned(),
  }
}

/// 失败原因：带上是第几级，界面才能指给用户看。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Failure {
  Gain(usize),
  Ip3(usize),
  Core(Ip3Error),
}

/// IP3 级联：多级链路的总增益、总 IIP3 / OIP3 与瓶颈级。
#[component]
pub(super) fn Ip3Cascade() -> impl IntoView {
  // 默认是一条典型接收链：衰减器 → LNA → 滤波器 → 混频器 → 中放。
  let rows = RwSignal::new(vec![
    row("-3", ""),
    row("20", "5"),
    row("-6", ""),
    row("15", "15"),
    row("10", "25"),
  ]);
  let by_output = RwSignal::new("input".to_owned());
  let count = Memo::new(move |_| rows.with(Vec::len));
  let referred_id = unique_id("ip3-referred");

  let referred_options = vec![
    SelectOption::new(
      "input",
      Signal::derive(move || t("tools.ip3-referred-input")),
    ),
    SelectOption::new(
      "output",
      Signal::derive(move || t("tools.ip3-referred-output")),
    ),
  ];

  let outcome = Memo::new(move |_| -> Result<Cascade, Failure> {
    let referred = if by_output.with(|v| v == "output") {
      Referred::Output
    } else {
      Referred::Input
    };
    let stages = rows.with(|rows| {
      rows
        .iter()
        .enumerate()
        .map(|(i, r)| {
          let gain_db = r.gain.trim().parse::<f64>().map_err(|_| Failure::Gain(i))?;
          let ip3_dbm = match r.ip3.trim() {
            "" => None,
            s => Some(s.parse::<f64>().map_err(|_| Failure::Ip3(i))?),
          };
          Ok(Stage {
            gain_db,
            ip3_dbm,
            referred,
          })
        })
        .collect::<Result<Vec<_>, Failure>>()
    })?;
    cascade(&stages).map_err(Failure::Core)
  });

  let add = Callback::new(move |()| {
    if rows.with(Vec::len) < MAX_STAGES {
      rows.update(|r| r.push(row("0", "")));
    }
  });
  let remove = Callback::new(move |()| {
    if rows.with(Vec::len) > 1 {
      rows.update(|r| {
        r.pop();
      });
    }
  });

  view! {
    <div class="grid grid-cols-1 gap-3 sm:grid-cols-2">
      <Field
        class="sm:col-span-2"
        label=Signal::derive(move || t("tools.ip3-referred"))
        r#for=referred_id.clone()
      >
        <NativeSelect
          id=referred_id
          value=Signal::derive(move || by_output.get())
          on_change=Callback::new(move |v: String| by_output.set(v))
          options=referred_options
        />
      </Field>
      {move || {
        (0..count.get())
          .map(|i| {
            let (gain_id, ip3_id) = (unique_id("ip3-gain"), unique_id("ip3-ip3"));
            view! {
              <Field
                label=Signal::derive(move || tf("tools.ip3-stage-gain", &[&(i + 1).to_string()]))
                r#for=gain_id.clone()
              >
                <NumberField
                  id=gain_id
                  value=Signal::derive(move || {
                    rows.with(|r| r.get(i).map(|x| x.gain.clone()).unwrap_or_default())
                  })
                  on_change=Callback::new(move |v: String| {
                    rows.update(|r| {
                      if let Some(x) = r.get_mut(i) {
                        x.gain = v;
                      }
                    });
                  })
                  controls=false
                />
              </Field>
              <Field
                label=Signal::derive(move || {
                  let key = if by_output.with(|v| v == "output") {
                    "tools.ip3-stage-oip3"
                  } else {
                    "tools.ip3-stage-iip3"
                  };
                  tf(key, &[&(i + 1).to_string()])
                })
                r#for=ip3_id.clone()
              >
                <NumberField
                  id=ip3_id
                  value=Signal::derive(move || {
                    rows.with(|r| r.get(i).map(|x| x.ip3.clone()).unwrap_or_default())
                  })
                  on_change=Callback::new(move |v: String| {
                    rows.update(|r| {
                      if let Some(x) = r.get_mut(i) {
                        x.ip3 = v;
                      }
                    });
                  })
                  controls=false
                />
              </Field>
            }
          })
          .collect_view()
      }}
      <div class="sm:col-span-2 flex flex-wrap gap-2">
        <Button
          variant=Variant::Outline
          size=Size::Sm
          disabled=Signal::derive(move || count.get() >= MAX_STAGES)
          on_click=Callback::new(move |_| add.run(()))
        >
          {move || t("tools.ip3-add-stage")}
        </Button>
        <Button
          variant=Variant::Outline
          size=Size::Sm
          disabled=Signal::derive(move || count.get() <= 1)
          on_click=Callback::new(move |_| remove.run(()))
        >
          {move || t("tools.ip3-remove-stage")}
        </Button>
      </div>
      <div class=format!("{RESULT} tabular-nums") role="status">
        {move || match outcome.get() {
          Ok(c) => match (c.iip3_dbm, c.oip3_dbm) {
            (Some(iip3), Some(oip3)) => view! {
              <div class="text-foreground">
                {tf(
                  "tools.ip3-total",
                  &[&fmt_num(c.gain_db), &fmt_num(iip3), &fmt_num(oip3)],
                )}
              </div>
              {c.limiting.map(|i| {
                let share = c.stages[i].share * 100.0;
                view! {
                  <div>
                    {tf("tools.ip3-limiting", &[&(i + 1).to_string(), &fmt_num(share)])}
                  </div>
                }
              })}
            }
            .into_any(),
            _ => view! { <div>{tf("tools.ip3-all-linear", &[&fmt_num(c.gain_db)])}</div> }.into_any(),
          },
          Err(e) => view! { <div>{error_text(e)}</div> }.into_any(),
        }}
      </div>
      {move || {
        outcome.get().ok().map(|c| {
          view! {
            <div class="sm:col-span-2">
              <Table aria_label=Signal::derive(move || t("tools.ip3-table")) class="text-sm">
                <TableHead class="text-xs text-muted-foreground">
                  <TableRow>
                    <TableHeader scope="col" class="px-2 py-1.5 font-normal">{move || t("tools.stage")}</TableHeader>
                    <TableHeader scope="col" class="px-2 py-1.5 text-right font-normal">{move || t("tools.ip3-col-gain-before")}</TableHeader>
                    <TableHeader scope="col" class="px-2 py-1.5 text-right font-normal">{move || t("tools.ip3-col-iip3")}</TableHeader>
                    <TableHeader scope="col" class="px-2 py-1.5 text-right font-normal">{move || t("tools.ip3-col-share")}</TableHeader>
                  </TableRow>
                </TableHead>
                <TableBody class="font-mono tabular-nums">
                  {c.stages
                    .iter()
                    .enumerate()
                    .map(|(i, s)| {
                      let iip3 = s.iip3_dbm.map_or_else(|| "—".to_owned(), fmt_num);
                      let share = if s.iip3_dbm.is_some() {
                        format!("{}%", fmt_num(s.share * 100.0))
                      } else {
                        "—".to_owned()
                      };
                      let (gain_before, limiting) = (fmt_num(s.gain_before_db), c.limiting == Some(i));
                      view! {
                        // 瓶颈级用金色底标出（原来靠 `font-semibold`，点阵字体下根本看不出来）。
                        <TableRow class=if limiting { "bg-gold/20" } else { "" }>
                          <TableCell class="px-2 py-1">{i + 1}</TableCell>
                          <TableCell class="px-2 py-1 text-right">{gain_before}</TableCell>
                          <TableCell class="px-2 py-1 text-right">{iip3}</TableCell>
                          <TableCell class="px-2 py-1 text-right">{share}</TableCell>
                        </TableRow>
                      }
                    })
                    .collect_view()}
                </TableBody>
              </Table>
            </div>
          }
        })
      }}
      <p class="sm:col-span-2 text-xs text-muted-foreground">{move || t("tools.ip3-note")}</p>
    </div>
  }
}

fn error_text(e: Failure) -> String {
  match e {
    Failure::Gain(i) => tf("tools.ip3-error-gain", &[&(i + 1).to_string()]),
    Failure::Ip3(i) => tf("tools.ip3-error-ip3", &[&(i + 1).to_string()]),
    Failure::Core(Ip3Error::Empty) => t("tools.ip3-error-empty"),
    Failure::Core(Ip3Error::NotFinite(i)) => tf("tools.ip3-error-finite", &[&(i + 1).to_string()]),
  }
}
