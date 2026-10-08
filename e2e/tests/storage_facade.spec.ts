import { expect, test, type Page } from "@playwright/test";
import { waitSettled } from "./fixtures";

/** 呼号输入框没有 <label> 包裹，用 placeholder 定位。 */
function callInput(page: Page) {
  return page.locator('input[placeholder="BG4XXX"]');
}

/** 往日志里加一条通联。 */
async function addEntry(page: Page, call: string) {
  await callInput(page).fill(call);
  await page.getByRole("button", { name: "添加记录" }).click();
  await expect(page.getByText("共 1 条")).toBeVisible();
}

/**
 * 模拟「快照写成功、IndexedDB 事务被卸载取消」：把权威层换成一份**旧内容**。
 * 裸 payload 没有版本头，门面按版本 0 处理 —— 也就是「比快照旧」。
 */
async function poisonAuthority(page: Page) {
  await page.evaluate(async () => {
    await new Promise<void>((resolve, reject) => {
      const open = indexedDB.open("ham-web", 1);
      open.onupgradeneeded = () => open.result.createObjectStore("kv");
      open.onerror = () => reject(open.error);
      open.onsuccess = () => {
        const tx = open.result.transaction("kv", "readwrite");
        tx.objectStore("kv").put('{"entries":[]}', "logbook");
        tx.oncomplete = () => resolve();
        tx.onerror = () => reject(tx.error);
      };
    });
  });
}

test("本地存储：快照比权威层新时以快照为准，写入后立刻卸载不丢数据", async ({
  page,
}) => {
  await page.goto("/log");
  await addEntry(page, "JA1AA");
  await poisonAuthority(page);

  // 重新加载：版本更高的快照必须赢过旧的权威值。
  // 修好之前这一条会红 —— 列表被旧的权威值回滚成「暂无记录」。
  await page.reload();
  // 等加载占位消失，而不是盲等 500ms：权威层校正（IndexedDB 读 + 版本仲裁）是异步的，
  // 骨架块消失即「这一轮校正已经有结论」，随后两条断言都是 web-first 的。
  await waitSettled(page);
  await expect(page.getByText("共 1 条")).toBeVisible();
  await expect(page.getByText("JA1AA").first()).toBeVisible();

  // 顺带钉住契约：快照带写入版本号（IndexedDB 是异步落盘，靠它判断哪一层更新）。
  const version = await page.evaluate(() =>
    localStorage.getItem("logbook:kv-version"),
  );
  expect(Number(version)).toBeGreaterThan(0);
});

test("本地存储：改完立刻整页刷新，最后一笔改动不丢", async ({ page }) => {
  await page.goto("/log");
  await addEntry(page, "W1AW");

  // 不等待落盘，立刻整页刷新：快照是同步写成功的，首屏与随后的权威层校正都不能把它抹掉。
  await page.reload();
  // 不 sleep：下面的断言是 web-first 的（自动重试到超时），固定等待既拖慢又更脆。
  await expect(page.getByText("共 1 条")).toBeVisible();
  await expect(page.getByText("W1AW").first()).toBeVisible();
});
