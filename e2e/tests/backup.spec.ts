import { readFile } from "node:fs/promises";
import { expect, test } from "./fixtures";

/**
 * 数据备份（`/backup`）。
 *
 * 原先是小工具页最底部的 `/tools#backup`，埋在几十个计算器之后；现在独立成页，
 * 入口在顶栏「设置」对话框、页面页脚与移动菜单里。
 */
test("数据备份：导出 JSON 含本地数据", async ({ page }) => {
  await page.addInitScript(() => localStorage.setItem("e2e-backup-key", "e2e-value"));
  await page.goto("/backup");
  await expect(page.getByRole("heading", { level: 1, name: "数据备份" })).toBeVisible();
  await expect(page.getByText("本地存储占用（约）")).toBeVisible();

  // 导出成功后有一个「已导出 N 项」提示（全局 Toast，`data-slot="toast"` 定位）。
  const dl = page.waitForEvent("download");
  await page.getByRole("button", { name: "导出备份" }).click();
  const file = await dl;
  expect(file.suggestedFilename()).toMatch(/^ham-backup-\d{8}\.json$/);
  await expect(page.locator('[data-slot="toast"]')).toContainText(/^已导出 \d+ 项$/);

  const data = JSON.parse(await readFile(await file.path(), "utf8"));
  expect(data["e2e-backup-key"]).toBe("e2e-value");
});

test("数据备份：按模块勾选导出（不勾选的通联数据不进备份）", async ({ page }) => {
  await page.addInitScript(() => {
    localStorage.setItem("logbook", JSON.stringify({ entries: [] }));
    localStorage.setItem("e2e-backup-key", "e2e-value");
  });
  await page.goto("/backup");
  const section = page.locator("section#backup");
  await expect(section.getByText("导出模块（按需勾选，减小备份体积）")).toBeVisible();

  // 只保留「其他本地数据」，取消通联数据与 QSL 影像。
  await section.getByLabel(/^通联数据/).uncheck();
  await section.getByLabel(/^QSL 影像（扫描件/).uncheck();
  const dl = page.waitForEvent("download");
  await section.getByRole("button", { name: "导出备份" }).click();
  const file = await dl;
  await expect(page.locator('[data-slot="toast"]')).toContainText(/^已导出 \d+ 项$/);

  const data = JSON.parse(await readFile(await file.path(), "utf8"));
  expect(data["logbook"]).toBeUndefined();
  expect(data["e2e-backup-key"]).toBe("e2e-value");
});

test("数据备份：只勾选 QSL 影像也要导出影像与索引；导入拒绝不合格影像", async ({ page }) => {
  await page.goto("/backup");
  const section = page.locator("section#backup");
  await expect(page.getByRole("heading", { level: 1, name: "数据备份" })).toBeVisible();

  // 经导入路径把影像写进 IndexedDB（导入是外部输入：SVG 与非法 key 必须被拒绝，不计入条数）。
  const png = "data:image/png;base64,iVBORw0KGgo=";
  const backup = {
    "qsl-image-index": JSON.stringify([7]),
    "qsl-image:7": png,
    "qsl-image:8": "data:image/svg+xml;base64,PHN2Zz4=",
    "qsl-image:x": png,
  };
  await section.locator('input[type="file"]').first().setInputFiles({
    name: "qsl.json",
    mimeType: "application/json",
    buffer: Buffer.from(JSON.stringify(backup)),
  });
  // 索引（localStorage）+ 合格的 1 张影像 = 2 条（结果走全局 Toast）。
  await expect(page.locator('[data-slot="toast"]')).toContainText("已恢复 2 条数据");

  // 取消「通联数据」与「其他」：索引不在被勾选的模块里，影像仍要导出。
  await section.getByLabel(/^通联数据/).uncheck();
  await section.getByLabel(/^其他本地数据/).uncheck();
  const dl = page.waitForEvent("download");
  await section.getByRole("button", { name: "导出备份" }).click();
  const file = await dl;

  const data = JSON.parse(await readFile(await file.path(), "utf8"));
  expect(data["qsl-image:7"]).toBe(png);
  expect(data["qsl-image-index"]).toBe(JSON.stringify([7]));
  expect(data["qsl-image:8"]).toBeUndefined();
  expect(data["logbook"]).toBeUndefined();
});

test("数据备份：三个模块全不勾选时不能导出", async ({ page }) => {
  await page.goto("/backup");
  const section = page.locator("section#backup");
  await section.getByLabel(/^通联数据/).uncheck();
  await section.getByLabel(/^QSL 影像（扫描件/).uncheck();
  await section.getByLabel(/^其他本地数据/).uncheck();
  await expect(section.getByRole("button", { name: "导出备份" })).toBeDisabled();
  await section.getByLabel(/^其他本地数据/).check();
  await expect(section.getByRole("button", { name: "导出备份" })).toBeEnabled();
});

test("数据备份：导入恢复与合并导入", async ({ page }) => {
  await page.goto("/backup");
  await expect(page.getByRole("heading", { level: 1, name: "数据备份" })).toBeVisible();
  const inputs = page.locator('section#backup input[type="file"]');

  // 覆盖导入
  const restore = {
    logbook: JSON.stringify({ entries: [] }),
    "e2e-imported": "cover",
  };
  await inputs.nth(0).setInputFiles({
    name: "backup.json",
    mimeType: "application/json",
    buffer: Buffer.from(JSON.stringify(restore)),
  });
  await expect(page.locator('[data-slot="toast"]')).toContainText(/^已恢复 \d+ 条数据$/);
  expect(await page.evaluate(() => localStorage.getItem("e2e-imported"))).toBe("cover");

  // 合并导入（保留本机已有的 key，并入新的）
  await inputs.nth(1).setInputFiles({
    name: "backup2.json",
    mimeType: "application/json",
    buffer: Buffer.from(JSON.stringify({ "e2e-merged": "yes" })),
  });
  await expect(page.locator('[data-slot="toast"]')).toContainText(/^已合并 \d+ 条数据$/);
  expect(await page.evaluate(() => localStorage.getItem("e2e-imported"))).toBe("cover");
  expect(await page.evaluate(() => localStorage.getItem("e2e-merged"))).toBe("yes");
});

test("数据备份：跨设备合并重映射影像 ID，保留本机扫描件", async ({ page }) => {
  await page.goto("/backup");
  const inputs = page.locator('section#backup input[type="file"]');
  const entry = (id: number, callsign: string, date: string) => ({
    id, date, time: "12:00", freq: "14.074", mode: "FT8", callsign, remark: "",
  });
  const a = "data:image/png;base64,QUFB";
  const b = "data:image/png;base64,QkJC";
  const importFile = async (merge: boolean, data: Record<string, string>) => {
    await expect(page.locator('[data-slot="toast"]')).toHaveCount(0);
    await inputs.nth(merge ? 1 : 0).setInputFiles({
      name: "images.json", mimeType: "application/json", buffer: Buffer.from(JSON.stringify(data)),
    });
    await expect(page.locator('[data-slot="toast"]')).toContainText(merge ? /^已合并/ : /^已恢复/);
    await expect(page.locator('section#backup input[type="file"]').first()).toBeEnabled();
    await expect(page.locator('[data-slot="toast"]')).toHaveCount(0);
  };
  await importFile(false, {
    logbook: JSON.stringify({ entries: [entry(1, "JA1AAA", "2026-01-01")] }),
    "qsl-image-index": "[1]", "qsl-image:1": a,
  });
  const backup = {
    logbook: JSON.stringify({ entries: [entry(1, "W1AW", "2026-01-02")] }),
    "qsl-image-index": "[1]", "qsl-image:1": b,
  };
  await importFile(true, backup);
  await importFile(true, backup);
  const book = await page.evaluate(() => JSON.parse(localStorage.getItem("logbook") ?? "null"));
  expect(book.entries.map((e: { id: number }) => e.id)).toEqual([1, 2]);
  expect(await page.evaluate(() => JSON.parse(localStorage.getItem("qsl-image-index") ?? "null"))).toEqual([1, 2]);
  const download = page.waitForEvent("download");
  await page.getByRole("button", { name: "导出备份" }).click();
  const file = await download;
  const data = JSON.parse(await readFile(await file.path(), "utf8"));
  expect(data["qsl-image:1"]).toBe(a);
  expect(data["qsl-image:2"]).toBe(b);
  await page.locator("[data-nav]").getByRole("button", { name: "工具", exact: true }).click();
  await page.locator("[data-nav]").getByRole("button", { name: "日志 · 竞赛", exact: true }).click();
  await page.locator('[data-nav] a[href="/log"]').first().click();
  await expect(page.getByRole("row").filter({ hasText: "W1AW" })).toHaveCount(1);
  await page.locator('input[placeholder="BG4XXX"]').fill("DL1AAA");
  await page.getByRole("button", { name: "添加记录" }).click();
  const after = await page.evaluate(() => JSON.parse(localStorage.getItem("logbook") ?? "null"));
  expect(after.entries.map((e: { callsign: string }) => e.callsign)).toEqual(["JA1AAA", "W1AW", "DL1AAA"]);
});

test("覆盖新日志会删除同 ID 的旧影像，随后合并正确影像不误认领", async ({ page }) => {
  await page.goto("/backup");
  const inputs = page.locator('section#backup input[type="file"]');
  const log = (callsign: string) => JSON.stringify({ entries: [{
    id: 1, date: "2026-01-01", time: "12:00", freq: "14.074", mode: "FT8", callsign, remark: "",
  }] });
  const a = "data:image/png;base64,QUFB";
  const b = "data:image/png;base64,QkJC";
  const restore = async (merge: boolean, data: Record<string, string>) => {
    await expect(page.locator('[data-slot="toast"]')).toHaveCount(0);
    await inputs.nth(merge ? 1 : 0).setInputFiles({
      name: "cover.json", mimeType: "application/json", buffer: Buffer.from(JSON.stringify(data)),
    });
    await expect(page.locator('[data-slot="toast"]')).toContainText(merge ? /^已合并/ : /^已恢复/);
    await expect(page.locator('section#backup input[type="file"]').first()).toBeEnabled();
    await expect(page.locator('[data-slot="toast"]')).toHaveCount(0);
  };
  await restore(false, { logbook: log("JA1AAA"), "qsl-image-index": "[1]", "qsl-image:1": a });
  await restore(false, { logbook: log("W1AW"), "qsl-image-index": "[]" });
  await restore(true, { logbook: log("W1AW"), "qsl-image-index": "[1]", "qsl-image:1": b });
  const download = page.waitForEvent("download");
  await page.getByRole("button", { name: "导出备份" }).click();
  const file = await download;
  const data = JSON.parse(await readFile(await file.path(), "utf8"));
  expect(JSON.parse(data.logbook).entries[0].callsign).toBe("W1AW");
  expect(data["qsl-image:1"]).toBe(b);
});

test("数据备份：非法 JSON 给出失败提示而不是静默", async ({ page }) => {
  await page.goto("/backup");
  await expect(page.getByRole("heading", { level: 1, name: "数据备份" })).toBeVisible();

  await page.locator('section#backup input[type="file"]').first().setInputFiles({
    name: "broken.json",
    mimeType: "application/json",
    buffer: Buffer.from("这不是 JSON"),
  });
  await expect(page.locator('[data-slot="toast"]')).toContainText(/^导入失败：/);
});

/**
 * 自定义远端同步：用户配「获取」与「写入」两个接口，自己管理备份 JSON。
 * 远端用 `page.route` 模拟（Playwright 会替跨域的 fulfill 响应处理 CORS 预检）。
 */
test.describe("自定义远端同步", () => {
  const REMOTE = "https://remote.test";

  test("写入远端：按所选方法提交备份 JSON，带上自定义请求头", async ({ page }) => {
    await page.addInitScript(() => localStorage.setItem("e2e-backup-key", "e2e-value"));
    let seen: { method: string; auth: string | undefined; body: string } | undefined;
    await page.route(`${REMOTE}/put`, async (route) => {
      const req = route.request();
      if (req.method() !== "OPTIONS") {
        seen = { method: req.method(), auth: req.headers()["authorization"], body: req.postData() ?? "" };
      }
      await route.fulfill({ status: 204 });
    });
    await page.goto("/backup");
    await page.locator("#remote-put-url").fill(`${REMOTE}/put`);
    await page.locator("#remote-put-headers").fill("Authorization: Bearer secret");
    await page.getByRole("button", { name: "写入远端" }).click();
    await expect(page.locator('[data-slot="toast"]')).toContainText(/^已写入远端 \d+ 项$/);

    expect(seen?.method).toBe("PUT");
    expect(seen?.auth).toBe("Bearer secret");
    const body = JSON.parse(seen?.body ?? "{}");
    expect(body["e2e-backup-key"]).toBe("e2e-value");
    // 配置含鉴权头，只属于这台设备，不能随备份外流。
    expect(body["remote-sync"]).toBeUndefined();
  });

  test("读取和写入各用各自的令牌，互不串用", async ({ page }) => {
    const auth: Record<string, string | undefined> = {};
    await page.route(`${REMOTE}/get`, (route) => {
      const req = route.request();
      if (req.method() !== "OPTIONS") auth.get = req.headers()["authorization"];
      return route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({ "e2e-from-remote": "yes" }),
      });
    });
    await page.route(`${REMOTE}/put`, (route) => {
      const req = route.request();
      if (req.method() !== "OPTIONS") auth.put = req.headers()["authorization"];
      return route.fulfill({ status: 204 });
    });
    await page.goto("/backup");
    await page.locator("#remote-get-url").fill(`${REMOTE}/get`);
    await page.locator("#remote-get-headers").fill("Authorization: Bearer read-token");
    await page.locator("#remote-put-url").fill(`${REMOTE}/put`);
    await page.locator("#remote-put-headers").fill("Authorization: Bearer write-token");

    await page.getByRole("button", { name: "从远端合并" }).click();
    await expect(page.locator('[data-slot="toast"]')).toContainText(/^已合并 \d+ 条数据$/);
    await expect(page.locator('[data-slot="toast"]')).toHaveCount(0);
    await page.getByRole("button", { name: "写入远端" }).click();
    await expect(page.locator('[data-slot="toast"]')).toContainText(/^已写入远端 \d+ 项$/);

    expect(auth.get).toBe("Bearer read-token");
    expect(auth.put).toBe("Bearer write-token");

    // 两份请求头各自持久化。
    await page.reload();
    await expect(page.locator("#remote-get-headers")).toHaveValue("Authorization: Bearer read-token");
    await expect(page.locator("#remote-put-headers")).toHaveValue("Authorization: Bearer write-token");
  });

  test("从远端合并：拉回 JSON 并并入本机；配置刷新后仍在", async ({ page }) => {
    await page.route(`${REMOTE}/get`, (route) =>
      route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({ "e2e-from-remote": "yes" }),
      }),
    );
    await page.goto("/backup");
    await page.locator("#remote-get-url").fill(`${REMOTE}/get`);
    await page.getByRole("button", { name: "从远端合并" }).click();
    await expect(page.locator('[data-slot="toast"]')).toContainText(/^已合并 \d+ 条数据$/);
    expect(await page.evaluate(() => localStorage.getItem("e2e-from-remote"))).toBe("yes");

    await page.reload();
    await expect(page.locator("#remote-get-url")).toHaveValue(`${REMOTE}/get`);
  });

  test("从远端恢复（覆盖）：先确认，远端 404 视为还没有数据", async ({ page }) => {
    await page.route(`${REMOTE}/get`, (route) => route.fulfill({ status: 404, body: "nope" }));
    await page.goto("/backup");
    await page.locator("#remote-get-url").fill(`${REMOTE}/get`);
    await page.getByRole("button", { name: "从远端恢复（覆盖）" }).click();
    await page.getByRole("dialog").getByRole("button", { name: "从远端恢复（覆盖）" }).click();
    await expect(page.locator('[data-slot="toast"]')).toHaveText("远端还没有数据");
  });

  test("从远端获取：HTTP 错误给出失败提示", async ({ page }) => {
    await page.route(`${REMOTE}/get`, (route) => route.fulfill({ status: 401, body: "" }));
    await page.goto("/backup");
    await page.locator("#remote-get-url").fill(`${REMOTE}/get`);
    await page.getByRole("button", { name: "从远端合并" }).click();
    await expect(page.locator('[data-slot="toast"]')).toContainText("HTTP 401");
  });

  test("地址或请求头不合格时就地报错并禁用对应按钮", async ({ page }) => {
    await page.goto("/backup");
    const pull = page.getByRole("button", { name: "从远端合并" });
    const push = page.getByRole("button", { name: "写入远端" });
    // 两个接口都没填：动作不可用，但也不报错（可以只配一个）。
    await expect(pull).toBeDisabled();
    await expect(push).toBeDisabled();

    await page.locator("#remote-get-url").fill("example.com/data");
    await expect(page.getByText("请填写以 http:// 或 https:// 开头的完整地址。")).toBeVisible();
    await expect(pull).toBeDisabled();

    await page.locator("#remote-get-url").fill(`${REMOTE}/get`);
    await expect(pull).toBeEnabled();

    await page.locator("#remote-get-headers").fill("A: 1\nnot a header");
    await expect(page.getByText("第 2 行的请求头格式不对", { exact: false })).toBeVisible();
    await expect(pull).toBeDisabled();

    // 请求头各管各的：获取请求头有错，不应连累写入。
    await page.locator("#remote-put-url").fill(`${REMOTE}/put`);
    await expect(push).toBeEnabled();
    await page.locator("#remote-put-headers").fill("not a header");
    await expect(push).toBeDisabled();
    await page.locator("#remote-get-headers").fill("");
    await expect(pull).toBeEnabled();
    await expect(push).toBeDisabled();
  });

  test("写入远端总是完整快照，不跟「导出备份」的模块勾选走", async ({ page }) => {
    await page.addInitScript(() => {
      localStorage.setItem("logbook", JSON.stringify({ entries: [] }));
      localStorage.setItem("e2e-backup-key", "e2e-value");
    });
    let body = "";
    await page.route(`${REMOTE}/put`, async (route) => {
      const req = route.request();
      if (req.method() !== "OPTIONS") body = req.postData() ?? "";
      await route.fulfill({ status: 204 });
    });
    await page.goto("/backup");
    const section = page.locator("section#backup");
    // 导出区只留「其他」：这只管本地「导出备份」，远端写入不能被它连累而漏掉日志。
    await section.getByLabel(/^通联数据/).uncheck();
    await section.getByLabel(/^QSL 影像（扫描件/).uncheck();
    await page.locator("#remote-put-url").fill(`${REMOTE}/put`);
    await page.getByRole("button", { name: "写入远端" }).click();
    await expect(page.locator('[data-slot="toast"]')).toContainText(/^已写入远端 \d+ 项$/);

    const data = JSON.parse(body);
    expect(data["logbook"]).toBeDefined();
    expect(data["e2e-backup-key"]).toBe("e2e-value");
  });

  test("同步时不含 QSL 影像：勾选后写出的内容不再带影像，取消勾选又带上", async ({ page }) => {
    const bodies: Record<string, string>[] = [];
    await page.route(`${REMOTE}/put`, async (route) => {
      const req = route.request();
      if (req.method() !== "OPTIONS") bodies.push(JSON.parse(req.postData() ?? "{}"));
      await route.fulfill({ status: 204 });
    });
    await page.goto("/backup");
    const toast = page.locator('[data-slot="toast"]');

    // 经导入路径把一张合格的影像写进 IndexedDB。
    const png = "data:image/png;base64,iVBORw0KGgo=";
    await page.locator('section#backup input[type="file"]').first().setInputFiles({
      name: "qsl.json",
      mimeType: "application/json",
      buffer: Buffer.from(JSON.stringify({ "qsl-image-index": JSON.stringify([7]), "qsl-image:7": png })),
    });
    await expect(toast).toContainText("已恢复 2 条数据");
    await expect(toast).toHaveCount(0);

    await page.locator("#remote-put-url").fill(`${REMOTE}/put`);
    await page.getByRole("button", { name: "写入远端" }).click();
    await expect(toast).toContainText(/^已写入远端/);
    expect(bodies).toHaveLength(1);
    expect(bodies[0]["qsl-image:7"]).toBe(png);
    await expect(toast).toHaveCount(0);

    const exclude = page.getByRole("checkbox", { name: "同步时不含 QSL 影像" });
    await exclude.click();
    await expect(exclude).toHaveAttribute("aria-checked", "true");
    await page.getByRole("button", { name: "写入远端" }).click();
    await expect(toast).toContainText(/^已写入远端/);
    expect(bodies).toHaveLength(2);
    expect(bodies[1]["qsl-image:7"]).toBeUndefined();
    await expect(toast).toHaveCount(0);

    // 开关随配置保存，刷新后还在。
    await page.reload();
    await expect(page.getByRole("checkbox", { name: "同步时不含 QSL 影像" })).toHaveAttribute("aria-checked", "true");
  });

  test("导出的备份不含同步配置，导入的备份也改不了它", async ({ page }) => {
    await page.goto("/backup");
    await page.locator("#remote-get-url").fill(`${REMOTE}/get`);

    const dl = page.waitForEvent("download");
    await page.getByRole("button", { name: "导出备份" }).click();
    const data = JSON.parse(await readFile(await (await dl).path(), "utf8"));
    expect(data["remote-sync"]).toBeUndefined();

    await page.locator('section#backup input[type="file"]').first().setInputFiles({
      name: "evil.json",
      mimeType: "application/json",
      buffer: Buffer.from(JSON.stringify({ "remote-sync": '{"put_url":"https://evil.test/"}' })),
    });
    await expect(page.locator('[data-slot="toast"]')).toContainText(/^已恢复 \d+ 条数据$/);
    const stored = await page.evaluate(() => JSON.parse(localStorage.getItem("remote-sync") ?? "{}"));
    expect(stored.get_url).toBe(`${REMOTE}/get`);
    expect(stored.put_url ?? "").toBe("");
  });
});

/**
 * 自动同步：打开页面自动获取，数据变化后自动写入。
 *
 * 种子用 `sessionStorage` 标记「只种一次」，而不是 localStorage：后者本身会被算成「本机有数据」，
 * 影响「全新浏览器」这类判断；用 sessionStorage 则刷新后不会重复覆盖应用写下的状态。
 */
test.describe("自动同步", () => {
  const REMOTE = "https://remote.test";
  const GET = `${REMOTE}/get`;
  const PUT = `${REMOTE}/put`;

  type Seed = { config?: object; state?: object; items?: Record<string, string> };
  const seed = (page: import("@playwright/test").Page, s: Seed) =>
    page.addInitScript((s) => {
      if (window !== window.top || sessionStorage.getItem("e2e-seeded")) return;
      sessionStorage.setItem("e2e-seeded", "1");
      if (s.config) localStorage.setItem("remote-sync", JSON.stringify(s.config));
      if (s.state) localStorage.setItem("remote-sync-state", JSON.stringify(s.state));
      for (const [k, v] of Object.entries(s.items ?? {})) localStorage.setItem(k, v);
    }, s);

  const serveRemote = (page: import("@playwright/test").Page, data: Record<string, string>) =>
    page.route(GET, (route) =>
      route.fulfill({ status: 200, contentType: "application/json", body: JSON.stringify(data) }),
    );
  const ls = (page: import("@playwright/test").Page, key: string) =>
    page.evaluate((k) => localStorage.getItem(k), key);
  const toast = (page: import("@playwright/test").Page) => page.locator('[data-slot="toast"]');

  test("打开页面自动获取：远端有更新、本机没有未同步改动 → 用远端覆盖本机", async ({ page }) => {
    await seed(page, { config: { get_url: GET }, state: { base: 1, dirty: false } });
    await serveRemote(page, { "e2e-from-remote": "yes" });
    await page.goto("/backup");
    await expect(toast(page)).toContainText("已从远端更新本机数据");
    expect(await ls(page, "e2e-from-remote")).toBe("yes");
  });

  test("全新浏览器（没有任何用户数据）第一次连上有数据的远端 → 直接获取", async ({ page }) => {
    await seed(page, { config: { get_url: GET } });
    await serveRemote(page, { "e2e-from-remote": "yes" });
    await page.goto("/backup");
    await expect(toast(page)).toContainText("已从远端更新本机数据");
    expect(await ls(page, "e2e-from-remote")).toBe("yes");
  });

  test("用过的浏览器第一次连上有数据的远端 → 不替用户选，暂停并提示", async ({ page }) => {
    await seed(page, { config: { get_url: GET }, items: { bookmarks: "[]" } });
    await serveRemote(page, { "e2e-from-remote": "yes" });
    await page.goto("/backup");
    await expect(toast(page)).toContainText("已暂停自动同步");
    await expect(page.locator("section#backup").getByRole("status")).toContainText("已暂停自动同步");
    expect(await ls(page, "e2e-from-remote")).toBeNull();
  });

  test("远端更新了，本机也有没写出去的改动 → 冲突：不覆盖本机", async ({ page }) => {
    await seed(page, {
      config: { get_url: GET },
      state: { base: 1, dirty: true },
      items: { "e2e-local": "mine" },
    });
    await serveRemote(page, { "e2e-from-remote": "yes", "e2e-local": "theirs" });
    await page.goto("/backup");
    await expect(page.locator("section#backup").getByRole("status")).toContainText("已暂停自动同步");
    expect(await ls(page, "e2e-local")).toBe("mine");
    expect(await ls(page, "e2e-from-remote")).toBeNull();
  });

  test("远端没变 → 不再覆盖本机（自动获取是覆盖，必须只在远端变了时才做）", async ({ page }) => {
    await seed(page, { config: { get_url: GET }, state: { base: 1, dirty: false } });
    await serveRemote(page, { "e2e-from-remote": "yes" });
    await page.goto("/backup");
    await expect(toast(page)).toContainText("已从远端更新本机数据");

    // 本机改成别的值；刷新后远端内容与上次同步时一致，不应再被覆盖回去。
    await page.evaluate(() => localStorage.setItem("e2e-from-remote", "local"));
    const fetched = page.waitForResponse((r) => r.url() === GET);
    await page.reload();
    await fetched;
    await expect(page.locator("section#backup").getByRole("status")).toContainText(/已自动同步（\d{2}:\d{2}）/);
    expect(await ls(page, "e2e-from-remote")).toBe("local");
  });

  test("启动时获取失败 → 暂停自动写入并说明原因", async ({ page }) => {
    await seed(page, { config: { get_url: GET, put_url: PUT }, state: { base: 1, dirty: true } });
    await page.route(GET, (route) => route.fulfill({ status: 500, body: "" }));
    await page.goto("/backup");
    await expect(toast(page)).toContainText("HTTP 500");
    await expect(page.locator("section#backup").getByRole("status")).toContainText("已暂停自动写入");
  });

  test("数据变化后自动写入：只写同步内容，不含设备专属的 key", async ({ page }) => {
    await seed(page, { config: { put_url: PUT }, items: { theme: "dark" } });
    await page.route(PUT, (route) => route.fulfill({ status: 204 }));
    await page.goto("/log");
    await page.locator('input[placeholder="BG4XXX"]').fill("BG4AUTO");
    // 自动写入有约 5 秒的攒批延时。
    const sent = page.waitForRequest((r) => r.url() === PUT && r.method() === "PUT", { timeout: 20_000 });
    await page.getByRole("button", { name: "添加记录" }).click();
    const body = JSON.parse((await sent).postData() ?? "{}");
    expect(JSON.parse(body.logbook).entries.map((e: { callsign: string }) => e.callsign)).toContain("BG4AUTO");
    for (const deviceOnly of ["remote-sync", "remote-sync-state", "theme", "locale"]) {
      expect(body[deviceOnly], deviceOnly).toBeUndefined();
    }
  });

  test("没「上岗」前不自动写：刚填的地址可能还是半截，手动同步成功后才开始自动写", async ({ page }) => {
    const puts: string[] = [];
    await page.route(PUT, (route) => {
      if (route.request().method() === "PUT") puts.push(route.request().postData() ?? "");
      return route.fulfill({ status: 204 });
    });
    await page.goto("/backup");
    await page.locator("#remote-put-url").fill(PUT);
    const importFile = async (data: Record<string, string>) =>
      page.locator('section#backup input[type="file"]').first().setInputFiles({
        name: "d.json",
        mimeType: "application/json",
        buffer: Buffer.from(JSON.stringify(data)),
      });

    // 本次会话里还没成功同步过：数据变了也不写。
    await importFile({ "e2e-a": "1" });
    await expect(toast(page)).toContainText(/^已恢复/);
    await page.waitForTimeout(7_000);
    expect(puts).toHaveLength(0);

    // 手动写入成功 = 这组地址验证过了，之后的变化才自动写。
    await expect(toast(page)).toHaveCount(0);
    await page.getByRole("button", { name: "写入远端" }).click();
    await expect(toast(page)).toContainText(/^已写入远端/);
    expect(puts).toHaveLength(1);

    await expect(toast(page)).toHaveCount(0);
    await importFile({ "e2e-b": "2" });
    await expect.poll(() => puts.length, { timeout: 20_000 }).toBe(2);
    expect(JSON.parse(puts[1])["e2e-b"]).toBe("2");
  });

  const section = (page: import("@playwright/test").Page) => page.locator("section#backup");
  const okOrPreflight = (status: number) => (route: import("@playwright/test").Route) =>
    route.fulfill({ status: route.request().method() === "OPTIONS" ? 204 : status, body: "" });

  test("首次连接：远端是空的（404）而本机有数据 → 不自动上传，提示手动写入一次", async ({ page }) => {
    const puts: string[] = [];
    await seed(page, { config: { get_url: GET, put_url: PUT }, items: { bookmarks: "[]" } });
    await page.route(GET, (route) => route.fulfill({ status: 404, body: "nope" }));
    await page.route(PUT, (route) => {
      if (route.request().method() === "PUT") puts.push(route.request().postData() ?? "");
      return route.fulfill({ status: 204 });
    });
    await page.goto("/backup");
    // 404 可能只是获取地址写错了：没人确认过这组接口，不能替用户把本机推上去。
    await expect(toast(page)).toContainText("请先手动点一次「写入远端」");
    await expect(section(page).getByRole("status")).toContainText("请先手动点一次「写入远端」");
    await page.waitForTimeout(7_000);
    expect(puts).toHaveLength(0);

    // 手动写入成功 = 这组地址验证过了。
    await expect(toast(page)).toHaveCount(0);
    await page.getByRole("button", { name: "写入远端" }).click();
    await expect(toast(page)).toContainText(/^已写入远端/);
    expect(puts).toHaveLength(1);
  });

  test("同步成功过的接口，远端被清空 → 自动补传本机数据", async ({ page }) => {
    await seed(page, {
      config: { get_url: GET, put_url: PUT },
      state: { base: 1, dirty: false },
      items: { bookmarks: "[1]" },
    });
    await page.route(GET, (route) => route.fulfill({ status: 404, body: "" }));
    await page.route(PUT, (route) => route.fulfill({ status: 204 }));
    const sent = page.waitForRequest((r) => r.url() === PUT && r.method() === "PUT", { timeout: 20_000 });
    await page.goto("/backup");
    const body = JSON.parse((await sent).postData() ?? "{}");
    expect(body["bookmarks"]).toBe("[1]");
  });

  test("取回远端期间用户又改了数据 → 当作冲突，不覆盖这些改动", async ({ page }) => {
    await seed(page, { config: { get_url: GET }, state: { base: 1, dirty: false } });
    // 让远端迟迟不回：用户在这段时间里照常操作。
    let release!: () => void;
    const gate = new Promise<void>((resolve) => (release = resolve));
    await page.route(GET, async (route) => {
      await gate;
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({ "e2e-from-remote": "yes" }),
      });
    });
    await page.goto("/log");
    await page.locator('input[placeholder="BG4XXX"]').fill("BG4RACE");
    await page.getByRole("button", { name: "添加记录" }).click();
    await expect.poll(() => ls(page, "logbook")).toContain("BG4RACE");

    release();
    await expect(toast(page)).toContainText("已暂停自动同步");
    expect(await ls(page, "e2e-from-remote")).toBeNull();
    expect(await ls(page, "logbook")).toContain("BG4RACE");
  });

  test("写入遇到永久性错误（HTTP 401）→ 暂停自动写入并说明原因", async ({ page }) => {
    await seed(page, { config: { put_url: PUT } });
    await page.route(PUT, okOrPreflight(401));
    await page.goto("/backup");
    await section(page).locator('input[type="file"]').first().setInputFiles({
      name: "d.json",
      mimeType: "application/json",
      buffer: Buffer.from(JSON.stringify({ "e2e-a": "1" })),
    });
    // 令牌不对重试多少次都一样：不进入「稍后重试」，而是暂停并让用户处理。
    const status = section(page).getByRole("status");
    await expect(status).toContainText("HTTP 401", { timeout: 20_000 });
    await expect(status).toContainText("处理好后手动点一次「写入远端」");
    await expect(status).not.toContainText("稍后会自动重试");
  });

  test("写入遇到暂时性错误（HTTP 503）→ 保持上岗，稍后自动重试", async ({ page }) => {
    await seed(page, { config: { put_url: PUT } });
    await page.route(PUT, okOrPreflight(503));
    await page.goto("/backup");
    await section(page).locator('input[type="file"]').first().setInputFiles({
      name: "d.json",
      mimeType: "application/json",
      buffer: Buffer.from(JSON.stringify({ "e2e-a": "1" })),
    });
    const status = section(page).getByRole("status");
    await expect(status).toContainText("HTTP 503", { timeout: 20_000 });
    await expect(status).toContainText("稍后会自动重试");
  });

  test("手动从远端恢复后，本机独有的数据会自动补写回远端", async ({ page }) => {
    await page.route(GET, (route) =>
      route.fulfill({ status: 200, contentType: "application/json", body: JSON.stringify({ "e2e-remote": "1" }) }),
    );
    await page.route(PUT, (route) => route.fulfill({ status: 204 }));
    await page.goto("/backup");
    await page.evaluate(() => localStorage.setItem("e2e-local", "mine"));
    await page.locator("#remote-get-url").fill(GET);
    await page.locator("#remote-put-url").fill(PUT);

    const sent = page.waitForRequest((r) => r.url() === PUT && r.method() === "PUT", { timeout: 20_000 });
    await page.getByRole("button", { name: "从远端恢复（覆盖）" }).click();
    await page.getByRole("dialog").getByRole("button", { name: "从远端恢复（覆盖）" }).click();
    await expect(toast(page)).toContainText(/^已恢复/);
    // 覆盖导入不会删掉远端没有的 key：远端因此还缺这一份，自动同步要把它补回去。
    const body = JSON.parse((await sent).postData() ?? "{}");
    expect(body["e2e-local"]).toBe("mine");
    expect(body["e2e-remote"]).toBe("1");
  });
});
