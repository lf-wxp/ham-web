import { readFile } from "node:fs/promises";
import { expect, settleFrames, stubApi, test, type Page } from "./fixtures";

async function seedStation(page: Page, entries: Record<string, unknown>[] = []) {
  await page.addInitScript((rows) => {
    if (sessionStorage.getItem("review-seeded")) return;
    sessionStorage.setItem("review-seeded", "1");
    localStorage.setItem("station-book", JSON.stringify({
      next_id: 2, active_id: 1, profiles: [{ id: 1, callsign: "BG4ABC" }],
    }));
    localStorage.setItem("logbook", JSON.stringify({ entries: rows.map(row => ({ remark: "", ...row })) }));
    localStorage.setItem("contest-session", JSON.stringify({
      contest_id: "CQ-WW-CW", start: "2026-10-31 00:00", my_exch: "24",
      freq: "14.025", mode: "CW", power: "LOW",
    }));
  }, entries);
}

async function importCabrillo(page: Page, text: string) {
  await page.locator('input[type="file"][accept*=".cbr"]').setInputFiles({
    name: "review.cbr", mimeType: "text/plain", buffer: Buffer.from(text),
  });
  await expect(page.locator('[data-slot="toast"]')).toContainText("已导入");
}

async function exportCabrillo(page: Page) {
  const download = page.waitForEvent("download");
  await page.getByRole("button", { name: "导出 Cabrillo", exact: true }).click();
  const file = await download;
  return readFile((await file.path())!, "utf8");
}

test("审查修复：Cabrillo 文件读取期间离开路由，不读取已卸载场次", async ({ page }) => {
  await seedStation(page);
  await page.addInitScript(() => {
    const original = Blob.prototype.text;
    Blob.prototype.text = function () {
      return original.call(this).then((text) => new Promise<string>((resolve) => {
        document.documentElement.dataset.reviewReading = "1";
        document.addEventListener("review-release-file", () => {
          resolve(text);
          document.documentElement.dataset.reviewReleased = "1";
        }, { once: true });
      }));
    };
  });
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));
  await page.goto("/contest-log");
  await page.locator('input[type="file"][accept*=".cbr"]').setInputFiles({
    name: "slow.cbr", mimeType: "text/plain",
    buffer: Buffer.from("CONTEST: CQ-WW-CW\nQSO: 14021 CW 2025-11-29 0010 BG4ABC 599 24 W1AW 599 5\n"),
  });
  await page.waitForFunction(() => document.documentElement.dataset.reviewReading === "1");
  await page.locator('main a[href="/log"]').first().click();
  await expect(page).toHaveURL(/\/log$/);
  await page.evaluate(() => document.dispatchEvent(new Event("review-release-file")));
  await page.waitForFunction(() => document.documentElement.dataset.reviewReleased === "1");
  await settleFrames(page);
  expect(errors).toEqual([]);
  await expect(page.getByRole("heading", { level: 1 })).toBeVisible();
  expect(await page.evaluate(() => JSON.parse(localStorage.getItem("logbook")!).entries.length)).toBe(0);
});

test("审查修复：导入绑定文件本台，评分与再导出不改署当前台站", async ({ page }) => {
  await seedStation(page);
  await page.goto("/contest-log");
  await importCabrillo(page, "CONTEST: CQ-WW-CW\nCALLSIGN: W1AW\nQSO: 14020 CW 2025-11-29 0010 W1AW 599 5 JA1ABC 599 25\n");
  const text = await exportCabrillo(page);
  expect(text).toContain("CALLSIGN: W1AW");
  expect(text).toContain("CLAIMED-SCORE: 6");
  expect(text).toMatch(/QSO:.*W1AW\s+599\s+5\s+JA1ABC/);
  expect(await page.evaluate(() => JSON.parse(localStorage.getItem("station-book")!).active_id)).toBe(1);
});

test("审查修复：历史同名比赛隔离年度，不混合导出", async ({ page }) => {
  await seedStation(page, [{
    id: 1, station_id: 1, date: "2026-10-31", time: "00:10", freq: "14.020",
    mode: "CW", callsign: "JA1ABC", contest_id: "CQ-WW-CW", stx: "24", srx: "25",
  }]);
  await page.goto("/contest-log");
  await importCabrillo(page, "CONTEST: CQ-WW-CW\nQSO: 14020 CW 2025-11-29 0010 BG4ABC 599 24 JA1ABC 599 25\n");
  const text = await exportCabrillo(page);
  expect(text.match(/^QSO:/gm)).toHaveLength(1);
  expect(text).toContain("2025-11-29");
  expect(text).not.toContain("2026-10-31");
  expect(await page.evaluate(() => JSON.parse(localStorage.getItem("logbook")!).entries.length)).toBe(2);
});

test("审查修复：迁移部分 WPX 日志后发送序号接续最大值", async ({ page }) => {
  await page.clock.setFixedTime(new Date("2026-10-31T00:12:00Z"));
  await seedStation(page);
  await page.goto("/contest-log");
  await importCabrillo(page, "CONTEST: CQ-WPX-CW\nQSO: 14020 CW 2026-10-31 0010 BG4ABC 599 100 W1AW 599 1\nQSO: 14020 CW 2026-10-31 0011 BG4ABC 599 101 W2AW 599 2\n");
  await page.getByLabel("对方呼号").fill("DL1ABC");
  await page.getByLabel(/^收到的/).fill("9");
  await page.getByRole("button", { name: "记录", exact: true }).click();
  await expect(page.getByRole("row").filter({ hasText: "DL1ABC" })).toBeVisible();
  const serial = await page.evaluate(() => JSON.parse(localStorage.getItem("logbook")!).entries.find((e: { callsign: string }) => e.callsign === "DL1ABC").stx);
  expect(serial).toBe("102");
});

test("审查修复：SCP 删除失败不清内存、不误报成功，刷新仍可重试", async ({ page }) => {
  await seedStation(page);
  await page.goto("/contest-log");
  await page.locator('[data-slot="scp-source"] summary').click();
  await page.locator('input[type="file"][accept*=".scp"]').setInputFiles({
    name: "calls.scp", mimeType: "text/plain", buffer: Buffer.from("W1AW\n"),
  });
  await expect(page.locator('[data-slot="scp-source"]')).toContainText("来自载入的 SCP 文件：1 个呼号");
  await expect(page.locator('[data-slot="toast"]')).toHaveCount(0);
  await page.evaluate(() => {
    IDBObjectStore.prototype.delete = function () {
      throw new DOMException("review delete failure", "InvalidStateError");
    };
  });
  const clear = page.getByRole("button", { name: "清除已载入的文件" });
  await clear.click();
  await expect(page.locator('[data-slot="toast"]')).toContainText("无法从存储中删除 SCP 文件");
  await expect(clear).toBeEnabled();
  await page.reload();
  await page.locator('[data-slot="scp-source"] summary').click();
  await expect(page.locator('[data-slot="scp-source"]')).toContainText("来自载入的 SCP 文件：1 个呼号");
});

for (const route of ["/tools", "/browse", "/glossary"]) {
  test(`审查修复：${route} 隐藏侧栏滚动后跨断点，吸顶保持在页头下`, async ({ page }) => {
    await page.setViewportSize({ width: 390, height: 844 });
    await page.goto(route);
    await expect(page.locator("[data-page-header]")).toBeVisible();
    await page.evaluate(() => window.scrollTo(0, 500));
    await page.setViewportSize({ width: 1280, height: 844 });
    const panel = page.locator("main aside .sticky").first();
    await expect(panel).toBeVisible();
    await expect.poll(() => panel.evaluate((el) => {
      const header = document.querySelector("[data-page-header]")!;
      return parseFloat((el as HTMLElement).style.top) - header.getBoundingClientRect().bottom;
    })).toBeGreaterThanOrEqual(0);
  });
}

test("审查修复：热图提示有描述关联，聚焦首行气泡不被滚动裁剪", async ({ page }) => {
  await seedStation(page, [{ id: 1, date: "2026-10-01", time: "12:00", freq: "14.074", mode: "FT8", callsign: "W1AW" }]);
  await page.goto("/stats");
  const trigger = page.locator("main .pxl-tip [tabindex='0']").first();
  await expect(trigger).toHaveAttribute("aria-describedby", /tooltip-/);
  const id = await trigger.getAttribute("aria-describedby");
  await expect(page.locator(`[id="${id}"]`)).not.toHaveText("");
  await trigger.focus();
  const popup = page.locator('[data-slot="tooltip-popup"]');
  await expect(popup).toBeVisible();
  const rect = await popup.boundingBox();
  expect(rect!.x).toBeGreaterThanOrEqual(8);
  expect(rect!.y).toBeGreaterThanOrEqual(8);
  expect(rect!.x + rect!.width).toBeLessThanOrEqual(1280 - 8);
  expect(await popup.evaluate((el) => el.closest("main") === null)).toBe(true);
  await trigger.press("Escape");
  await expect(popup).toBeHidden();
});

test("审查修复：Toast 队尾入队不重置首条计时，模态打开后仍可关闭反馈", async ({ page }) => {
  await page.clock.install();
  await page.goto("/contest-calendar");
  const add = page.getByRole("button", { name: "加入提醒", exact: true }).first();
  await add.click();
  const toast = page.locator('[data-slot="toast"]');
  await expect(toast).toBeVisible();
  const first = await toast.getAttribute("data-toast-id");
  await page.clock.runFor(2000);
  await add.click();
  await page.clock.runFor(1800);
  await expect(toast).not.toHaveAttribute("data-toast-id", first!);
  await page.getByRole("button", { name: "搜索", exact: true }).click();
  const dialog = page.getByRole("dialog");
  await expect(dialog).toBeVisible();
  await expect(dialog.locator('[data-slot="toast-live-region"]')).not.toHaveText("");
  await toast.locator("button").click();
  await page.clock.runFor(200);
  await expect(toast).toHaveCount(0);
  await expect(dialog).toBeVisible();
});

test("审查修复：Kp 失败后页面刷新会重试卡片", async ({ page }) => {
  await page.clock.setFixedTime(new Date("2026-10-16T00:00:00Z"));
  await stubApi(page, {});
  let calls = 0;
  await page.route("**/api/kp-forecast", (route) => {
    calls += 1;
    return route.fulfill(calls === 1 ? { status: 503, body: "unavailable" } : {
      status: 200, contentType: "application/json", body: JSON.stringify({ source: "NOAA SWPC", updated: null,
        points: [{ time: "2026-10-16T03:00:00", kp: 3, kind: "predicted" }] }),
    });
  });
  await page.goto("/solar");
  await expect(page.getByText("预报暂时取不到（可能因网络受限），稍后重试。")).toBeVisible();
  await page.getByRole("button", { name: "刷新", exact: true }).click();
  await expect(page.getByRole("table", { name: "Kp 三日预报表" })).toBeVisible();
  expect(calls).toBe(2);
});

test.describe("opaque-origin 初始化隔离", () => {
  // Playwright 的 serviceWorkers:block 注入本身会访问 sandbox 禁止的 serviceWorker；本组只隔离存储种子。
  test.use({ serviceWorkers: "allow" });
  test("审查修复：测试种子不向 opaque-origin iframe 访问 localStorage", async ({ page }) => {
  const errors: string[] = [];
  page.on("pageerror", error => errors.push(error.message));
  await page.goto("/");
  await expect(page.getByRole("button", { name: "搜索", exact: true })).toBeVisible();
  await page.evaluate(() => new Promise<void>(resolve => {
    const frame = document.createElement("iframe");
    frame.setAttribute("sandbox", "allow-scripts");
    frame.srcdoc = "<!doctype html><html><body>isolated</body></html>";
    frame.onload = () => resolve();
    document.body.appendChild(frame);
  }));
  await settleFrames(page);
  expect(errors).toEqual([]);
  expect(await page.evaluate(() => localStorage.getItem("ui:shortcutsHelpSeen:practice"))).toBe("1");
  });
});
