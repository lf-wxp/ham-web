//! 现代风的页面头部：标题 + 副标题 + 可选操作区 + 可选多行区。
//!
//! 与像素 `PageHeader` **结构同构**（现行规则见 `docs/ui/modern.md` 的 `ModernPageHead` 一条）：
//! 三件套（标题 / 副标题 / 操作区）一个不少，另加一个多行子区（筛选条、工具条）。
//! 差异只在呈现：现代风不做吸顶（标题进正文流），标题用展示字体、下沿一条 1px 墨线。
//!
//! `data-page-head` 是结构与 e2e 的定位契约；`md-article-head` 保留是为了让既有的
//! `::after` 短线、阅读签名等样式继续命中（一次迁移不重写两套钩子）。

use leptos::prelude::*;

#[component]
pub fn ModernPageHead(
  #[prop(into)] title: Signal<String>,
  #[prop(into)] subtitle: Signal<String>,
  /// 右侧操作区（搜索 / 筛选 / 新建）。与像素 `PageHeader` 的 `actions` 同义。
  #[prop(optional)]
  actions: Option<ViewFn>,
  /// 标题行下方的多行区（工具条、确认条），与像素 `PageHeader` 的 `children` 同义。
  #[prop(optional)]
  children: Option<Children>,
) -> impl IntoView {
  view! {
    <header data-page-head class="md-article-head md-page-head">
      <div class="md-page-head-row">
        <div class="md-page-head-copy">
          <h1 class="md-article-title">{move || title.get()}</h1>
          <p class="md-lede">{move || subtitle.get()}</p>
        </div>
        {actions
          .map(|actions| view! { <div class="md-page-head-actions">{actions.run()}</div> })}
      </div>
      {children.map(|children| children())}
    </header>
  }
}
