use ham_web_core::weak_exam::CategoryDelta;
use leptos::prelude::*;

/// 正确率格式化：无数据时显示占位符。
fn pct(rate: Option<f64>) -> String {
  rate.map_or_else(|| "—".to_owned(), |r| format!("{:.0}%", r * 100.0))
}

/// 本次各一级分类正确率与以往对比。
#[component]
pub(crate) fn CategoryCompare(deltas: Vec<CategoryDelta>) -> impl IntoView {
  view! {
    <div class="mt-3 border-t pt-3">
      <div class="text-sm font-medium">"分类对比（本次 / 以往）"</div>
      <table class="mt-2 w-full text-xs tabular-nums">
        <thead class="text-muted-foreground">
          <tr>
            <th scope="col" class="py-1 text-left font-normal">"分类"</th>
            <th scope="col" class="py-1 text-right font-normal">"本次"</th>
            <th scope="col" class="py-1 text-right font-normal">"以往"</th>
            <th scope="col" class="py-1 text-right font-normal">"变化"</th>
          </tr>
        </thead>
        <tbody>
          {deltas
            .into_iter()
            .map(|d| {
              let change = d.change();
              let (text, class) = match change {
                Some(c) if c >= 0.5 => (format!("+{c:.0}"), "text-emerald-700 dark:text-emerald-400"),
                Some(c) if c <= -0.5 => (format!("{c:.0}"), "text-red-700 dark:text-red-400"),
                Some(_) => ("持平".to_owned(), "text-muted-foreground"),
                None => ("—".to_owned(), "text-muted-foreground"),
              };
              view! {
                <tr class="border-t">
                  <td class="py-1">{d.name}</td>
                  <td class="py-1 text-right">
                    {format!("{}/{} · {}", d.exam.correct, d.exam.answered, pct(d.exam.rate()))}
                  </td>
                  <td class="py-1 text-right">{pct(d.before)}</td>
                  <td class=format!("py-1 text-right font-medium {class}")>{text}</td>
                </tr>
              }
            })
            .collect_view()}
        </tbody>
      </table>
    </div>
  }
}
