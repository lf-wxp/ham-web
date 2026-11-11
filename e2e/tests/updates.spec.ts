import type { Page } from "@playwright/test";

import { expect, test } from "./fixtures";

/** 页面里挂的假 Service Worker 句柄（`seedWaitingWorker` 注入）。 */
type FakeSw = {
  worker: EventTarget & { state: string };
  container: EventTarget;
  messages: { type: string }[];
};

/**
 * 给页面喂一个「有 waiting worker」的假 Service Worker 容器。
 *
 * `UpdateNotices` 的 Service Worker 接线只在 release 构建里跑（`!cfg!(debug_assertions)`），
 * 而 Playwright 默认 `serviceWorkers: "block"`，真 worker 起不来；这里用最小的 `EventTarget`
 * 替身顶上，页面看到的就是「注册表里有一个 `installed` 的 waiting worker」。
 * 所以这组用例只在 release 产物（`cargo make e2e`）上成立，`trunk serve` 的 dev 产物里提示永远不会出现。
 */
async function seedWaitingWorker(page: Page) {
  await page.addInitScript(() => {
    // addInitScript 也会进入 axe / 第三方 iframe，替身只属于被测主文档。
    if (window !== window.top) return;
    const messages: { type: string }[] = [];
    const worker = Object.assign(new EventTarget(), {
      state: "installed",
      postMessage: (msg: { type: string }) => {
        messages.push(msg);
      },
    });
    const reg = Object.assign(new EventTarget(), {
      waiting: worker,
      installing: null,
      active: null,
      update: () => Promise.resolve(),
    });
    const container = Object.assign(new EventTarget(), {
      ready: Promise.resolve(reg),
      controller: {},
      register: () => Promise.resolve(reg),
      getRegistrations: () => Promise.resolve([]),
    });
    Object.defineProperty(navigator, "serviceWorker", { value: container, configurable: true });
    (window as unknown as { __fakeSw: FakeSw }).__fakeSw = { worker, container, messages };
  });
}

/**
 * 让假 worker 换一个状态并派发 `statechange`（和浏览器激活 / 被顶替时一样）。
 * 返回前再等两帧：「提示还在」这类负向断言要等页面真有机会把它关掉，不然会空过。
 */
async function settleWorker(page: Page, state: string) {
  await page.evaluate(async (next) => {
    const { worker } = (window as unknown as { __fakeSw: FakeSw }).__fakeSw;
    worker.state = next;
    worker.dispatchEvent(new Event("statechange"));
    await new Promise<void>((done) => requestAnimationFrame(() => requestAnimationFrame(() => done())));
  }, state);
}

test("更新提示：版本更新后展示本次更新内容，只展示一次", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByRole("heading", { level: 1 })).toBeAttached();
  // 首次访问只记录版本，不打扰新用户
  await expect(page.getByText("已更新到新版本")).toHaveCount(0);

  await page.evaluate(() => localStorage.setItem("app:changelog-seen", "2000-01-01"));
  await page.reload();
  const notice = page.getByRole("status").filter({ hasText: "已更新到新版本" });
  await expect(notice).toBeVisible();
  await expect(notice.getByRole("listitem").first()).toBeVisible();
  await notice.getByRole("button", { name: "知道了" }).click();
  await expect(notice).toHaveCount(0);

  await page.reload();
  await expect(page.getByRole("heading", { level: 1 })).toBeAttached();
  await expect(page.getByText("已更新到新版本")).toHaveCount(0);
});

test("题库更新提示：内容与上次记录不同时列出改动", async ({ page }) => {
  // 必须在页面脚本运行前预置旧摘要：首页会预取 A 类题库，若先 goto 再写入，
  // 首页的加载会抢先把「当前内容」当成新摘要写回，等进练习页时已无差异。
  // 用哨兵键保证只播种一次，reload 时不会把旧摘要又塞回去。
  await page.addInitScript(() => {
    if (localStorage.getItem("e2e:bank-digest-seeded")) return;
    localStorage.setItem(
      "bank-digest:A",
      JSON.stringify({ rev: "", digest: { hashes: [1, 2, 3], explained: 0 } }),
    );
    localStorage.setItem("e2e:bank-digest-seeded", "1");
  });

  await page.goto("/practice?bank=A");
  const notice = page.getByRole("status").filter({ hasText: "题库已更新" });
  await expect(notice).toBeVisible();
  await expect(notice).toContainText(/A 类：修改 3 题，新增 \d+ 题.*现共 \d+ 题/);
  await notice.getByRole("button", { name: "知道了" }).click();

  // 记录已更新为当前内容，再次进入不再提示
  await page.reload();
  await expect(page.getByText(/第 \d+ \/ \d+ 题/).first()).toBeVisible();
  await expect(page.getByText("题库已更新")).toHaveCount(0);
});

test("更新提示：点「刷新以更新」后按钮转圈、提示留着，新版接管后才刷新", async ({ page }) => {
  await seedWaitingWorker(page);
  await page.goto("/");
  const notice = page.getByRole("status").filter({ hasText: "发现新版本" });
  await expect(notice).toBeVisible();
  const update = notice.getByRole("button", { name: "刷新以更新" });
  const later = notice.getByRole("button", { name: "稍后" });
  await expect(update).toBeEnabled();
  await expect(later).toBeEnabled();

  await update.click();
  // 等新版接管的这几秒：按钮忙态、不能重复点，「稍后」置灰 —— 用户看得到「正在更新」。
  await expect(update).toHaveAttribute("aria-busy", "true");
  await expect(update).toBeDisabled();
  await expect(later).toBeDisabled();
  // 新旧两种消息格式都发了（旧 worker 脚本认的是 `skip-waiting`）。
  const sent = await page.evaluate(() =>
    (window as unknown as { __fakeSw: FakeSw }).__fakeSw.messages.map((m) => m.type),
  );
  expect(sent).toEqual(["SKIP_WAITING", "skip-waiting"]);

  // worker 开始激活：这是预期的，提示不能因此被关掉（否则用户在等待期间看不到任何反馈）。
  await settleWorker(page, "activating");
  await expect(notice).toBeVisible();
  await expect(update).toHaveAttribute("aria-busy", "true");

  // 新版接管页面 → 刷新。
  await Promise.all([
    page.waitForEvent("framenavigated", (frame) => frame === page.mainFrame()),
    page.evaluate(() =>
      (window as unknown as { __fakeSw: FakeSw }).__fakeSw.container.dispatchEvent(
        new Event("controllerchange"),
      ),
    ),
  ]);
});

test("更新提示：用户没点更新时，等待中的 worker 自己离开 installed 提示就关掉", async ({ page }) => {
  await seedWaitingWorker(page);
  await page.goto("/");
  const notice = page.getByRole("status").filter({ hasText: "发现新版本" });
  await expect(notice).toBeVisible();

  // 被更新的版本顶替（redundant）：提示指向的东西已经没了，点了也没有任何效果，该自动关。
  await settleWorker(page, "redundant");
  await expect(notice).toHaveCount(0);
});
