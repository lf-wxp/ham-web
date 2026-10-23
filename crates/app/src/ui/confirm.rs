//! 全局确认弹窗：破坏性操作（删除 / 合并 / 清空）的唯一确认入口。
//!
//! 为什么不是 `window.confirm`：原生弹窗不能本地化按钮、不能换样式，在像素风界面里等于
//! 突然换了一套系统 UI；而且每个调用点都要自己拼一段同步阻塞逻辑，确认文案散落各处。
//! 这里把「问一句」收敛成一个服务：调用点给文案与回调，组件挂在 `App` 根部，
//! 全站同时只有一个确认框（与 [`crate::ui::ToastRoot`] 同一套挂载方式）。
//!
//! 与 [`crate::components::common::MessageDialog`] 的分工：`MessageDialog` 是「告知」
//! （只有确定按钮、开合由页面自己持有）；这里是「二选一」（取消 / 执行），
//! 状态由服务持有，适合表格行、卡片图标这类不适合各自带弹窗的调用点。

use leptos::prelude::*;
use leptos_router::hooks::use_location;

use crate::i18n::t;
use crate::ui::{
  Button, Dialog, DialogDescription, DialogFooter, DialogHeader, DialogTitle, Size, Variant,
};

/// 确认框的语气：删除类用 [`ConfirmTone::Danger`]（执行按钮是 destructive 红），
/// 其余不可逆动作（批量标记、合并）用 [`ConfirmTone::Normal`]。
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ConfirmTone {
  Danger,
  Normal,
}

#[derive(Clone)]
struct Request {
  title: String,
  description: String,
  confirm_text: String,
  tone: ConfirmTone,
  on_confirm: Callback<()>,
}

thread_local! {
  static REQUEST: ArcRwSignal<Option<Request>> = ArcRwSignal::new(None);
}

/// 请求一次确认；`on_confirm` 只在用户点下执行按钮后运行。
///
/// `title` / `description` / `confirm_text` 由调用点用 `t(…)` 取好（与 Toast 同一约定：
/// 服务不持有词条 key，切语言时不会重算，确认框本身是瞬时交互，可以接受）。
pub fn confirm(
  title: impl Into<String>,
  description: impl Into<String>,
  confirm_text: impl Into<String>,
  tone: ConfirmTone,
  on_confirm: Callback<()>,
) {
  REQUEST.with(|r| {
    r.set(Some(Request {
      title: title.into(),
      description: description.into(),
      confirm_text: confirm_text.into(),
      tone,
      on_confirm,
    }))
  });
}

/// 全局确认框挂载点（`App` 根部，与 `ToastRoot` 并列）。
#[component]
pub fn ConfirmRoot() -> impl IntoView {
  let request = REQUEST.with(|r| r.clone());
  // `ArcRwSignal` 不是 `Copy`：Effect 与 `close` 各拿一份，别让前者把后者要用的那份移走。
  let incoming = request.clone();
  // 展示内容与开合分开存：`Dialog` 退场动画期间 `open` 已经是 false，
  // 内容若跟着清空会先闪一块空白再消失。
  let shown = RwSignal::new(None::<Request>);
  let open = RwSignal::new(false);

  Effect::new(move |_| {
    if let Some(req) = incoming.with(|r| r.clone()) {
      shown.set(Some(req));
      open.set(true);
    }
  });

  // 关掉之后清空待办：下次 `confirm` 重新置位，避免旧回调被重复触发。
  let close = Callback::new(move |()| {
    open.set(false);
    request.set(None);
  });

  // 路由一变就收起：确认框是模态的，但浏览器后退 / 全局快捷键仍能把页面换掉。
  // 回调里捕获的是旧页面的信号，页面卸载后它们已被释放，这时再点「执行」就是在操作
  // 一个不存在的页面 —— 与其事后防御，不如不让这个窗口存在。
  let location = use_location();
  Effect::new(move |prev: Option<String>| {
    let path = location.pathname.get();
    if prev.as_ref().is_some_and(|p| *p != path) && open.get_untracked() {
      close.run(());
    }
    path
  });

  view! {
    <Dialog open=open>
      {move || {
        shown
          .get()
          .map(|req| {
            let description = req.description;
            let variant = match req.tone {
              ConfirmTone::Danger => Variant::Destructive,
              ConfirmTone::Normal => Variant::Default,
            };
            let on_confirm = req.on_confirm;
            view! {
              <DialogHeader>
                <DialogTitle>{req.title}</DialogTitle>
                {(!description.is_empty())
                  .then(|| view! { <DialogDescription>{description}</DialogDescription> })}
              </DialogHeader>
              <DialogFooter>
                <Button
                  variant=Variant::Outline
                  size=Size::Default
                  on_click=Callback::new(move |_| close.run(()))
                >
                  {move || t("exam.cancel")}
                </Button>
                <Button
                  variant=variant
                  size=Size::Default
                  on_click=Callback::new(move |_| {
                    // 退场动画期间按钮还在 DOM 里：已经关了就不再响应，否则连点两下
                    // 「执行」会把删除 / 批量标记跑两遍（连带弹两条成功提示）。
                    if !open.get_untracked() {
                      return;
                    }
                    close.run(());
                    // `try_run`：回调属于发起确认的页面，万一那一页已卸载，静默放弃而不是 panic。
                    on_confirm.try_run(());
                  })
                >
                  {req.confirm_text}
                </Button>
              </DialogFooter>
            }
          })
      }}
    </Dialog>
  }
}
