use leptos::prelude::*;

use crate::cn::cn;
use crate::ui::TextValue;

/// 内容卡片（L1 平卡）：2px 墨框 + 像素标题栏（`pxl-section-title`），正文由 `children` 提供。
///
/// 全站内容卡片的唯一外壳。首页下半部分原先有三种外壳（窗口级 `card_class`、平卡
/// `flat_card_class`、整张手写的 `<a>`），标题体、内边距、悬停反馈各不相同；统一到这里后，
/// 标题栏、边框、间距只有一份定义。
///
/// - `meta`：标题栏右侧的小字（待复习数、连续天数、「查看详情 →」这类元信息）。
///   它放在标题 `h2` **外面**，标题的无障碍名因此只有标题本身。
/// - `href`：整卡可点。标题成为铺满整张卡的链接（stretched link），卡片带 `motion-lift`
///   悬停反馈（上抬 2px + 硬投影 + 淡强调底）和整卡的虚线焦点环（见 `controls.css` 的
///   `.pxl-card-link`）。**悬停有反馈 = 整卡可点**，不可点的卡片不要加。卡片里还有
///   别的按钮 / 链接时，给它们 `relative z-10` 浮到铺满的链接之上，否则点不到。
/// - 正文自己带内边距（`p-4`），表格 / `divide-y` 列表可以贴边。
#[component]
pub fn SectionCard(
  #[prop(into)] title: TextValue,
  /// 标题栏右侧的元信息。
  #[prop(optional, into)]
  meta: Option<ViewFn>,
  /// 整卡可点的目标地址。
  #[prop(optional, into)]
  href: Option<String>,
  /// 锚点 id（`/progress#plan` 这类页内跳转用）。
  #[prop(optional, into)]
  id: Option<String>,
  #[prop(optional, into)] class: String,
  children: Children,
) -> impl IntoView {
  let root = cn(&[
    "pxl-card border bg-card",
    if href.is_some() {
      "pxl-card-link group relative motion-lift hover:bg-accent/40"
    } else {
      ""
    },
    class.as_str(),
  ]);
  let heading = match href {
    Some(href) => view! {
      <a
        href=href
        data-card-link
        class="underline-offset-4 after:absolute after:inset-0"
      >
        {move || title.get()}
      </a>
    }
    .into_any(),
    None => view! { {move || title.get()} }.into_any(),
  };
  view! {
    <section id=id class=root>
      <div class="pxl-section-title">
        <h2 class="min-w-0">{heading}</h2>
        {meta
          .map(|m| {
            view! {
              // `move ||` 不能省：`m.run()` 里读的信号（异步到的传播数据、语言）只有在响应式闭包里
              // 才会被追踪；直接写 `{m.run()}` 只执行一次，数据晚到 / 切语言后元信息就停在旧值。
              <span class="ml-auto font-sans text-xs normal-case text-muted-foreground">{move || m.run()}</span>
            }
          })}
      </div>
      {children()}
    </section>
  }
}
