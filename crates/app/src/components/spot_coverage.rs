//! 接收报告的方位覆盖：把「谁收到了我」落到 8 个扇区上。
//!
//! 只做**本台视角**的统计（接收台相对本台的方位），不画世界地图：地图回答
//! 「他们在哪」，扇区回答「我的天线往哪个方向打得出去」，后者才是调天线时要看的。

use ham_web_core::bands::band_rank;
use ham_web_core::spot_coverage::{self, SECTORS, Spot};
use leptos::prelude::*;

use crate::i18n::{t, tf, tp};
use crate::ui::{Chip, ChipGroup, Table, TableBody, TableCell, TableHead, TableHeader, TableRow};

/// 「全部波段」在 chip 组里的取值。
const ALL_BANDS: &str = "-";

fn sector_key(sector: usize) -> &'static str {
  match sector {
    0 => "coverage.n",
    1 => "coverage.ne",
    2 => "coverage.e",
    3 => "coverage.se",
    4 => "coverage.s",
    5 => "coverage.sw",
    6 => "coverage.w",
    _ => "coverage.nw",
  }
}

#[component]
pub fn SpotCoverage(
  /// 本台网格（Maidenhead）。
  #[prop(into)]
  home: Signal<String>,
  #[prop(into)] spots: Signal<Vec<Spot>>,
  /// 是否画时段分布：没有时间戳的数据源（如 RBN）传 `false`。
  #[prop(optional, default = true)]
  show_hourly: bool,
  /// 数据源的接收台位置是「按 DXCC 实体中心换算」的粗定位。
  #[prop(optional, default = false)]
  coarse: bool,
) -> impl IntoView {
  let band = RwSignal::new(ALL_BANDS.to_string());

  // 下面三个都是 `Memo`：`Signal::derive` 每次**读取**都重算，而这里的读取点很多——每个 chip
  // 读一次 `effective`，`effective` 又读一次 `bands`，几十个 chip 就是几十遍「扫全部报告并
  // 排序」。`Memo` 只在上游真的变了的时候算一次。
  //
  // 用 `with` 读 `spots`：它是几十到几千条的报告数组，没必要每次都整份 clone；
  // `aggregate` 直接吃切片，全波段那条路径连过滤都不用做。
  let bands = Memo::new(move |_| {
    let mut list: Vec<String> = spots.with(|list| list.iter().map(|s| s.band.clone()).collect());
    // 按频率（`AMATEUR_BAND_EDGES` 的顺序）排，而不是字符串字典序：后者会把 10m 排在
    // 160m 前面，与站内其它波段列表的方向相反。
    list.sort_by(|a, b| band_rank(a).cmp(&band_rank(b)).then_with(|| a.cmp(b)));
    list.dedup();
    list
  });

  // 选中的波段在数据刷新后可能消失（换了呼号 / 换了查询）：这时回落到「全部」。否则
  // 过滤结果是空表，界面会显示「暂无报告」，而其实只是选中的那个波段没数据了。
  let effective = Memo::new(move |_| {
    let selected = band.get();
    if selected == ALL_BANDS || bands.with(|list| list.contains(&selected)) {
      selected
    } else {
      ALL_BANDS.to_string()
    }
  });

  let coverage = Memo::new(move |_| {
    let selected = effective.get();
    let home = home.get();
    spots.with(|all| {
      if selected == ALL_BANDS {
        return spot_coverage::aggregate(&home, all);
      }
      let filtered: Vec<Spot> = all.iter().filter(|s| s.band == selected).cloned().collect();
      spot_coverage::aggregate(&home, &filtered)
    })
  });

  view! {
    <section class="pxl-card border bg-card" aria-labelledby="spot-coverage-heading">
      <h2 id="spot-coverage-heading" class="pxl-section-title">
        {move || t("coverage.title")}
      </h2>
      <div class="space-y-3 p-4">
        <p class="text-xs text-muted-foreground">{move || t("coverage.desc")}</p>
        {coarse
          .then(|| {
            view! {
              <p class="text-xs text-muted-foreground">{move || t("coverage.coarse-hint")}</p>
            }
          })}

        {move || {
          let list = bands.get();
          (list.len() > 1)
            .then(|| {
              view! {
                <ChipGroup
                  value=Signal::derive(move || effective.get())
                  on_change=Callback::new(move |v: String| band.set(v))
                  aria_label=Signal::derive(|| t("coverage.band-filter"))
                >
                  <Chip value=ALL_BANDS>{move || t("coverage.all-bands")}</Chip>
                  {list
                    .into_iter()
                    .map(|b| {
                      let value = b.clone();
                      view! { <Chip value=value>{b}</Chip> }
                    })
                    .collect_view()}
                </ChipGroup>
              }
            })
        }}

        {move || {
          match coverage.get() {
            None => {
              view! {
                <p class="bg-inset px-3 py-2 text-sm text-muted-foreground">
                  {move || t("coverage.home-missing")}
                </p>
              }
                .into_any()
            }
            Some(cov) if cov.total == 0 => {
              view! {
                <p class="bg-inset px-3 py-2 text-sm text-muted-foreground">
                  {move || t("coverage.empty")}
                </p>
              }
                .into_any()
            }
            Some(cov) => {
              let peak = cov.sectors.iter().map(|s| s.count).max().unwrap_or(0);
              let peak_hour = cov.hourly.iter().copied().max().unwrap_or(0);
              let rows = (0..SECTORS)
                .map(|i| {
                  let stat = cov.sectors[i];
                  let pct = if peak == 0 {
                    0.0
                  } else {
                    stat.count as f64 / peak as f64 * 100.0
                  };
                  let snr = if stat.count == 0 {
                    "—".to_string()
                  } else {
                    format!("{:.1} dB", stat.mean_snr)
                  };
                  view! {
                    <TableRow>
                      <TableCell class="border px-3 py-2">{t(sector_key(i))}</TableCell>
                      <TableCell class="border px-3 py-2 text-right tabular-nums">
                        {stat.count}
                      </TableCell>
                      <TableCell class="border px-3 py-2 text-right tabular-nums text-muted-foreground">
                        {snr}
                      </TableCell>
                      <TableCell class="border px-3 py-2">
                        <div class="h-2 w-full bg-muted" role="presentation">
                          <div class="h-2 bg-win/70" style=format!("width: {pct:.0}%")></div>
                        </div>
                      </TableCell>
                    </TableRow>
                  }
                })
                .collect_view();
              view! {
                <div class="space-y-2">
                  <p class="text-xs text-muted-foreground">
                    {tp("coverage.total", cov.total, &[&cov.total.to_string()])}
                  </p>
                  {show_hourly
                    .then(|| {
                      view! {
                        <div class="space-y-1">
                          <p class="text-xs text-foreground/70">
                            {move || t("coverage.hourly")}
                          </p>
                    <div class="flex h-8 items-end gap-0.5">
                      {(0..24)
                        .map(|h| {
                          let count = cov.hourly[h];
                          let pct = if peak_hour == 0 {
                            0.0
                          } else {
                            count as f64 / peak_hour as f64 * 100.0
                          };
                          let label = tp(
                            "coverage.hourly-tip",
                            count,
                            &[&format!("{h:02}"), &count.to_string()],
                          );
                          view! {
                            // `role="img"`：普通 div 上的 `aria-label` 不会被读屏暴露，
                            // 加了这个角色整根柱子才能被念成「14 时，3 条」。
                            <div
                              class="flex h-8 w-full items-end bg-muted"
                              role="img"
                              aria-label=label
                            >
                              <div
                                class="w-full bg-win/70"
                                style=format!("height: {pct:.0}%")
                              ></div>
                            </div>
                          }
                        })
                        .collect_view()}
                    </div>
                    {cov
                      .busiest_hour()
                      .map(|h| {
                        view! {
                          <p class="text-xs text-muted-foreground">
                            {tf("coverage.busiest", &[&format!("{h:02}")])}
                          </p>
                        }
                      })}
                        </div>
                      }
                    })}
                  <Table
                    class="min-w-[420px] text-sm"
                    aria_label=Signal::derive(|| t("coverage.title"))
                    tabindex="0"
                    role="region"
                  >
                    <TableHead>
                      <TableRow>
                        <TableHeader class="border px-3 py-2">
                          {move || t("coverage.sector")}
                        </TableHeader>
                        <TableHeader class="border px-3 py-2 text-right">
                          {move || t("coverage.reports")}
                        </TableHeader>
                        <TableHeader class="border px-3 py-2 text-right">
                          {move || t("coverage.mean-snr")}
                        </TableHeader>
                        <TableHeader class="border px-3 py-2">
                          {move || t("coverage.distribution")}
                        </TableHeader>
                      </TableRow>
                    </TableHead>
                    <TableBody>{rows}</TableBody>
                  </Table>
                </div>
              }
                .into_any()
            }
          }
        }}
      </div>
    </section>
  }
}
