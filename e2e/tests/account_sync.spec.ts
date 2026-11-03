import { expect, test } from "./fixtures";
import type { Page, Request, Route } from "@playwright/test";

/**
 * 在线账号（`/backup` 页）与「自定义远端同步」并存。
 *
 * 默认用**内存里的假服务**接管 `/api/account/*`（不依赖后端有没有设 `ACCOUNT_DB`），
 * 它按真实服务端的契约实现（条件写入 `If-Match` / `If-None-Match`、412、会话、指纹同口径）；
 * `E2E_ACCOUNT_LIVE=1` 时不拦截，改打真实后端（需 `ACCOUNT_DB` 已设、前端能代理 `/api`），
 * 同一批断言既验前端也验假服务与真服务端没有漂移。
 *
 * 每个用例在像素风与现代风各跑一遍（两套皮肤的控件与布局互不共享）。
 */

const LIVE = process.env.E2E_ACCOUNT_LIVE === "1";
const PASSWORD = "correct horse";

// ─────────────────────────── 假服务 ───────────────────────────

/** 与 `ham_web_core::remote_sync::fingerprint` 同口径的 FNV-1a 64 位（逐段加长度前缀，键按字节序）。 */
function fingerprint(map: Record<string, string>): string {
  const MASK = (1n << 64n) - 1n;
  const enc = new TextEncoder();
  let h = 0xcbf29ce484222325n;
  const feed = (bytes: Uint8Array) => {
    const len = new Uint8Array(8);
    new DataView(len.buffer).setBigUint64(0, BigInt(bytes.length), true);
    for (const b of [...len, ...bytes]) {
      h ^= BigInt(b);
      h = (h * 0x100000001b3n) & MASK;
    }
  };
  for (const k of Object.keys(map).sort()) {
    feed(enc.encode(k));
    feed(enc.encode(map[k]));
  }
  return h.toString();
}

type Recorded = { method: string; path: string; headers: Record<string, string>; body: string };

class FakeAccountServer {
  /** 每台「设备」（页面）各有各的登录态，像真实的 Cookie 一样互不影响；云端与账号是共享的。 */
  private readonly sessions = new Map<string, { id: number; username: string }>();
  readonly accounts = new Map<string, { id: number; username: string; password: string }>();
  /** 云端备份（单账号即可：用例里只会有一个账号上传）。 */
  cloud: { body: string; fp: string; updatedAt: number } | null = null;
  enabled = true;
  readonly requests: Recorded[] = [];
  private nextId = 1;

  /** 另一台设备往云端传了东西（不经过被测页面）。 */
  uploadFromAnotherDevice(map: Record<string, string>) {
    this.cloud = { body: JSON.stringify(map), fp: fingerprint(map), updatedAt: Math.floor(Date.now() / 1000) };
  }

  puts(): Recorded[] {
    return this.requests.filter((r) => r.method === "PUT" && r.path === "/api/account/backup");
  }

  private session(dev: string) {
    const user = this.sessions.get(dev);
    return {
      enabled: this.enabled,
      registration_open: true,
      user: this.enabled && user
        ? {
            id: user.id,
            username: user.username,
            created_at: 1_700_000_000,
            backup: this.cloud
              ? { size: this.cloud.body.length, updated_at: this.cloud.updatedAt, fingerprint: this.cloud.fp }
              : null,
          }
        : null,
    };
  }

  private err(route: Route, status: number, code: string) {
    return route.fulfill({ status, contentType: "application/json", body: JSON.stringify({ error: { code, message: code } }) });
  }

  private ok(route: Route, status: number, body: unknown) {
    // 指纹是 u64：JSON.stringify 会把 BigInt 数字串当成字符串输出，这里手工保持为数字。
    const text = JSON.stringify(body).replace(/"fingerprint":"(\d+)"/g, '"fingerprint":$1');
    return route.fulfill({ status, contentType: "application/json", body: text });
  }

  async handle(route: Route, req: Request, dev: string) {
    const path = new URL(req.url()).pathname;
    const method = req.method();
    const body = req.postData() ?? "";
    const json = () => {
      try {
        return JSON.parse(body);
      } catch {
        return {};
      }
    };

    const me = () => this.sessions.get(dev) ?? null;
    if (path === "/api/account/session") return this.ok(route, 200, this.session(dev));
    if (!this.enabled) return this.err(route, 503, "unavailable");

    if (path === "/api/account/register" || path === "/api/account/login") {
      const { username, password } = json();
      if (path.endsWith("register")) {
        if (this.accounts.has(String(username).toLowerCase())) return this.err(route, 409, "username_taken");
        const account = { id: this.nextId++, username, password };
        this.accounts.set(String(username).toLowerCase(), account);
        this.sessions.set(dev, { id: account.id, username });
        return this.ok(route, 201, this.session(dev));
      }
      const account = this.accounts.get(String(username).toLowerCase());
      if (!account || account.password !== password) return this.err(route, 401, "bad_credentials");
      this.sessions.set(dev, { id: account.id, username: account.username });
      return this.ok(route, 200, this.session(dev));
    }
    if (path === "/api/account/logout") {
      this.sessions.delete(dev);
      return route.fulfill({ status: 204 });
    }
    if (path === "/api/account/password") {
      const { current, new: next } = json();
      const account = me() && [...this.accounts.values()].find((a) => a.id === me()!.id);
      if (!account) return this.err(route, 401, "unauthorized");
      if (account.password !== current) return this.err(route, 401, "bad_credentials");
      account.password = next;
      return route.fulfill({ status: 204 });
    }
    if (path === "/api/account/delete") {
      const account = me() && [...this.accounts.values()].find((a) => a.id === me()!.id);
      if (!account) return this.err(route, 401, "unauthorized");
      if (account.password !== json().password) return this.err(route, 401, "bad_credentials");
      this.accounts.delete(account.username.toLowerCase());
      this.sessions.delete(dev);
      this.cloud = null;
      return route.fulfill({ status: 204 });
    }
    if (path === "/api/account/backup") {
      const user = me();
      if (!user) return this.err(route, 401, "unauthorized");
      const claimed = req.headers()["x-account-id"];
      if (claimed !== undefined && Number(claimed) !== user.id) return this.err(route, 409, "account_changed");
      if (method === "GET") {
        if (!this.cloud) return route.fulfill({ status: 204 });
        return route.fulfill({ status: 200, contentType: "application/json", body: this.cloud.body });
      }
      if (method === "DELETE") {
        this.cloud = null;
        return route.fulfill({ status: 204 });
      }
      if (method === "PUT") {
        const h = req.headers();
        const ifMatch = h["if-match"]?.replace(/"/g, "");
        const ifNone = h["if-none-match"];
        const current = this.cloud?.fp ?? null;
        if ((ifMatch !== undefined && ifMatch !== current) || (ifNone === "*" && current !== null)) {
          return this.err(route, 412, "precondition_failed");
        }
        let map: Record<string, string>;
        try {
          map = JSON.parse(body);
        } catch {
          return this.err(route, 400, "invalid_backup");
        }
        this.cloud = { body, fp: fingerprint(map), updatedAt: Math.floor(Date.now() / 1000) };
        return this.ok(route, 200, { size: body.length, updated_at: this.cloud.updatedAt, fingerprint: this.cloud.fp });
      }
    }
    return this.err(route, 404, "bad_request");
  }
}

/**
 * 装上假服务；`E2E_ACCOUNT_LIVE=1` 时什么都不拦（返回的实例只用于读，不会有数据）。
 * 传入已有的 `fake` 就让两个页面（两台「设备」）共用同一个服务端。
 */
let devices = 0;
async function useAccountApi(page: Page, fake = new FakeAccountServer()): Promise<FakeAccountServer> {
  const dev = `device-${devices++}`;
  // 记录页面发出的账号请求：假服务与真后端两种模式都要能按「发了什么请求」断言。
  page.on("request", (req) => {
    const path = new URL(req.url()).pathname;
    if (!path.startsWith("/api/account/")) return;
    fake.requests.push({ method: req.method(), path, headers: req.headers(), body: req.postData() ?? "" });
  });
  if (!LIVE) await page.route("**/api/account/**", (route, req) => fake.handle(route, req, dev));
  return fake;
}

// ─────────────────────────── 辅助 ───────────────────────────

const toast = (page: Page) => page.locator('[data-slot="toast"]');
const panel = (page: Page) => page.getByTestId("account-panel");

let counter = 0;
/** 每个用例一个新账号名（真服务端里账号名全局唯一，不能在用例间复用）。 */
const uniqueName = () => `e2e${Date.now().toString(36)}${counter++}`;

async function openBackup(page: Page, style: string, seed: Record<string, string> = {}) {
  await page.addInitScript(
    ([s, data]) => {
      if (window !== window.top) return;
      localStorage.setItem("ui:uiStyle", s as string);
      for (const [k, v] of Object.entries(data as Record<string, string>)) localStorage.setItem(k, v);
    },
    [style, seed] as const,
  );
  await page.goto("/backup");
  await expect(page.getByRole("heading", { level: 1, name: "数据备份" })).toBeVisible();
}

async function signUp(page: Page, name = uniqueName()) {
  await panel(page).getByLabel("账号名").fill(name);
  await panel(page).getByLabel("口令", { exact: true }).fill(PASSWORD);
  await panel(page).getByRole("button", { name: "注册新账号" }).click();
  await expect(panel(page).getByTestId("account-signed-in-as")).toContainText(name);
  return name;
}

/**
 * 用「合并导入」改本机数据。
 *
 * 测试里直接写 `localStorage` 绕过了应用的存储钩子（自动同步感知不到）；导入是应用自己的路径，
 * 与用户在页面上改数据一样会触发「本机有改动」。
 */
async function mergeImport(page: Page, data: Record<string, string>) {
  await page.locator('section#backup input[type="file"]').nth(1).setInputFiles({
    name: "change.json",
    mimeType: "application/json",
    buffer: Buffer.from(JSON.stringify(data)),
  });
  await expect(toast(page).filter({ hasText: /^已合并/ })).toBeVisible();
}

const ls = (page: Page, key: string) => page.evaluate((k) => localStorage.getItem(k), key);
const accountSwitch = (page: Page) => page.getByRole("switch", { name: "用在线账号自动同步" });
const customSwitch = (page: Page) => page.getByRole("switch", { name: "用自定义远端自动同步" });

for (const style of ["pixel", "modern"] as const) {
  test.describe(`在线账号（${style}）`, () => {
    test(`在线账号：注册、登录态、退出（${style}）`, async ({ page }) => {
      await useAccountApi(page);
      await openBackup(page, style);

      // 没登录：只有表单；注册规则在本地就拦下，不发请求。
      await expect(panel(page).getByRole("heading", { name: "在线账号" })).toBeVisible();
      await panel(page).getByLabel("账号名").fill("a b");
      await panel(page).getByLabel("口令", { exact: true }).fill(PASSWORD);
      await panel(page).getByRole("button", { name: "注册新账号" }).click();
      await expect(panel(page).getByRole("alert")).toContainText("账号名要 3–32 位");
      await panel(page).getByLabel("账号名").fill(uniqueName());
      await panel(page).getByLabel("口令", { exact: true }).fill("short");
      await panel(page).getByRole("button", { name: "注册新账号" }).click();
      await expect(panel(page).getByRole("alert")).toContainText("口令要 8–128 个字符");

      const name = await signUp(page);
      await expect(panel(page).getByTestId("account-cloud-summary")).toContainText("云端还没有备份");
      // 没配自定义接口、也没选过通道：登录就是想用账号同步，自动选上。
      await expect(accountSwitch(page)).toHaveAttribute("aria-checked", "true");
      expect(await ls(page, "sync-channel")).toBe("account");

      // 重新打开页面：登录态来自服务端（Cookie），不是前端自己存的。
      await page.reload();
      await expect(panel(page).getByTestId("account-signed-in-as")).toContainText(name);

      await panel(page).getByRole("button", { name: "退出登录", exact: true }).click();
      await expect(toast(page)).toContainText("已退出登录");
      await expect(panel(page).getByRole("button", { name: "登录", exact: true })).toBeVisible();
      // 退出后偏好回到「没选过」，账号通道的基线作废。
      expect(await ls(page, "sync-channel")).toBeNull();
      const state = await ls(page, "account-sync-state");
      expect(state === null || JSON.parse(state).base === null).toBe(true);

      // 登录失败不泄露是哪一项错了。
      await panel(page).getByLabel("账号名").fill(name);
      await panel(page).getByLabel("口令", { exact: true }).fill("wrong password");
      await panel(page).getByRole("button", { name: "登录", exact: true }).click();
      await expect(panel(page).getByRole("alert")).toContainText("账号名或口令不对");
      await panel(page).getByLabel("口令", { exact: true }).fill(PASSWORD);
      await panel(page).getByLabel("口令", { exact: true }).press("Enter");
      await expect(panel(page).getByTestId("account-signed-in-as")).toContainText(name);
    });

    test(`在线账号：手动上传 / 从云端恢复 / 合并（${style}）`, async ({ page }) => {
      const fake = await useAccountApi(page);
      await openBackup(page, style, { bookmarks: JSON.stringify(["q1", "q2"]), theme: "dark" });
      await signUp(page);
      // 本机有数据、云端空、从没同步过：账号通道直接传（去处是登录态，不必像自定义地址那样先确认）。
      await expect.poll(() => fake.puts().length, { timeout: 20_000 }).toBeGreaterThan(0);
      fake.requests.length = 0;

      await page.evaluate(() => {
        localStorage.setItem("bookmarks", JSON.stringify(["q1", "q2", "q3"]));
        localStorage.setItem("e2e-from-cloud", "yes");
      });
      await panel(page).getByRole("button", { name: "上传到云端" }).click();
      await expect(toast(page)).toContainText(/^已上传到云端 \d+ 项$/);
      const put = fake.puts().at(-1)!;
      // 手动上传不带条件（用户明确要覆盖云端）；内容是备份 JSON，不含设备专属 key。
      expect(put.headers["if-match"]).toBeUndefined();
      expect(put.headers["if-none-match"]).toBeUndefined();
      expect(put.headers["content-type"]).toBe("application/json");
      const sent = JSON.parse(put.body) as Record<string, string>;
      expect(JSON.parse(sent.bookmarks)).toEqual(["q1", "q2", "q3"]);
      expect(sent["e2e-from-cloud"]).toBe("yes");
      for (const deviceKey of ["theme", "sync-channel", "account-sync-state", "remote-sync", "ui:uiStyle"]) {
        expect(sent[deviceKey]).toBeUndefined();
      }
      await expect(panel(page).getByTestId("account-cloud-summary")).toContainText(/^云端备份 .+ · 更新于 \d{4}-\d{2}-\d{2} \d{2}:\d{2}$/);

      // 本机之后变了样：「合并」取并集（云端独有的 key 回来、本机新增的收藏留下）。
      await page.evaluate(() => {
        localStorage.removeItem("e2e-from-cloud");
        localStorage.setItem("bookmarks", JSON.stringify(["only-here"]));
      });
      await panel(page).getByRole("button", { name: "从云端合并" }).click();
      await expect(toast(page)).toContainText(/^已合并 \d+ 条数据$/);
      expect(await ls(page, "e2e-from-cloud")).toBe("yes");
      expect(JSON.parse((await ls(page, "bookmarks"))!)).toEqual(expect.arrayContaining(["q1", "q3", "only-here"]));

      // 「覆盖」要先确认，之后本机同名数据就是云端的样子。
      await page.evaluate(() => localStorage.setItem("e2e-from-cloud", "local-change"));
      await panel(page).getByRole("button", { name: "从云端恢复（覆盖）" }).click();
      await expect(page.getByRole("dialog")).toContainText("用云端数据覆盖本机？");
      await page.getByRole("dialog").getByRole("button", { name: "从云端恢复（覆盖）" }).click();
      await expect(toast(page)).toContainText(/^已恢复 \d+ 条数据$/);
      expect(await ls(page, "e2e-from-cloud")).toBe("yes");
    });

    test(`在线账号：与自定义远端并存，两个自动同步开关互斥（${style}）`, async ({ page }) => {
      await useAccountApi(page);
      await openBackup(page, style);

      // 自定义远端没配接口：它的开关用不了；账号没登录时，账号面板里根本没有开关。
      await expect(customSwitch(page)).toBeDisabled();
      await expect(accountSwitch(page)).toHaveCount(0);

      // 先配自定义接口（它自动成为自动同步通道，行为与升级前一致）……
      await page.locator("#remote-get-url").fill("https://example.invalid/backup.json");
      await expect(customSwitch(page)).toBeEnabled();
      await expect(customSwitch(page)).toHaveAttribute("aria-checked", "true");

      // ……再登录：不抢它的自动同步，账号开关是关的。
      await signUp(page);
      await expect(accountSwitch(page)).toHaveAttribute("aria-checked", "false");
      await expect(customSwitch(page)).toHaveAttribute("aria-checked", "true");
      expect(await ls(page, "sync-channel")).toBeNull();

      // 打开账号：自定义自然关掉，同一时间只有一个。
      await accountSwitch(page).click();
      await expect(accountSwitch(page)).toHaveAttribute("aria-checked", "true");
      await expect(customSwitch(page)).toHaveAttribute("aria-checked", "false");
      expect(await ls(page, "sync-channel")).toBe("account");

      // 反过来。
      await customSwitch(page).click();
      await expect(customSwitch(page)).toHaveAttribute("aria-checked", "true");
      await expect(accountSwitch(page)).toHaveAttribute("aria-checked", "false");
      expect(await ls(page, "sync-channel")).toBe("custom");

      // 关掉正在跑的这个 = 都不自动同步（不是悄悄交给另一个），手动按钮仍可用。
      await customSwitch(page).click();
      await expect(customSwitch(page)).toHaveAttribute("aria-checked", "false");
      await expect(accountSwitch(page)).toHaveAttribute("aria-checked", "false");
      expect(await ls(page, "sync-channel")).toBe("off");
      await expect(panel(page).getByRole("button", { name: "上传到云端" })).toBeEnabled();

      // 两条通道各存各的状态：同步状态与选项都不进备份。
      const raw = await page.evaluate(() => Object.keys(localStorage));
      expect(raw).toEqual(expect.arrayContaining(["sync-channel", "remote-sync"]));
    });

    test(`在线账号：自动写入带条件，第二次写入要求云端还停在上一次的内容（${style}）`, async ({ page }) => {
      test.setTimeout(120_000);
      const fake = await useAccountApi(page);
      await openBackup(page, style);
      await signUp(page);
      await expect(panel(page).getByRole("status")).toContainText(/已自动同步/);

      // 本机改动 → 约 5 秒后自动写入；云端为空且没有基线：要求「云端还是空的」。
      await mergeImport(page, { bookmarks: JSON.stringify(["a"]) });
      await expect.poll(() => fake.puts().length, { timeout: 20_000 }).toBe(1);
      const first = fake.puts()[0];
      expect(first.headers["if-none-match"]).toBe("*");
      expect(first.headers["if-match"]).toBeUndefined();
      expect(first.headers["x-account-id"]).toBeDefined();
      await expect(panel(page).getByRole("status")).toContainText(/已自动同步/);
      // 第一次写出的内容的指纹（用本文件的 TS 实现算：真后端模式下它与前端 / 服务端的 Rust 实现对拍）。
      const afterFirst = fingerprint(JSON.parse(first.body));

      // 再改一次：基线就是上一次写出去的内容。两次自动写入至少隔 30 秒（整份快照可能很大）。
      // 假服务按它自己算出的指纹核对 `If-Match`：前端与它对同一份内容算出的指纹不一致就会 412，
      // 所以这里同时验了「指纹口径跨语言一致」。
      await mergeImport(page, { bookmarks: JSON.stringify(["a", "b"]) });
      await expect.poll(() => fake.puts().length, { timeout: 60_000 }).toBe(2);
      expect(fake.puts()[1].headers["if-match"]).toBe(afterFirst);
      await expect(panel(page).getByRole("status")).toContainText(/已自动同步/);
      expect(JSON.parse(JSON.parse(fake.puts()[1].body).bookmarks)).toEqual(expect.arrayContaining(["a", "b"]));
    });

    test(`在线账号：另一台设备抢先写过云端，本机自动写入被挡下并暂停，不覆盖（${style}）`, async ({ page }) => {
      test.skip(LIVE, "需要在页面不知情的情况下改云端内容，只有假服务能做到");
      const fake = await useAccountApi(page);
      await openBackup(page, style);
      await signUp(page);
      await expect(panel(page).getByRole("status")).toContainText(/已自动同步/);

      // 登录时云端还是空的；之后另一台设备传了东西，本机随后才有改动。
      fake.uploadFromAnotherDevice({ bookmarks: JSON.stringify(["other-device"]) });
      const before = fake.cloud!.body;
      await mergeImport(page, { bookmarks: JSON.stringify(["mine"]) });
      await expect(panel(page).getByRole("status")).toContainText("云端数据已被其他设备更新", { timeout: 30_000 });
      // 服务端按条件挡下，云端内容没被盖掉；自动同步暂停而不是重试。
      expect(fake.cloud!.body).toBe(before);
      expect(fake.puts()).toHaveLength(1);
      expect(fake.puts()[0].headers["if-none-match"]).toBe("*");
    });

    test(`在线账号：新设备登录，云端有数据则直接取回；本机已有数据则交给用户（${style}）`, async ({ page, browser }) => {
      test.skip(LIVE, "需要预置云端内容，只有假服务能做到");
      const fake = await useAccountApi(page);
      fake.uploadFromAnotherDevice({ bookmarks: JSON.stringify(["from-cloud"]), "e2e-cloud-only": "1" });
      await openBackup(page, style);
      const name = await signUp(page);
      // 全新浏览器（没有任何用户数据）：自动快进，直接用云端覆盖本机。
      await expect.poll(async () => ls(page, "e2e-cloud-only"), { timeout: 20_000 }).toBe("1");
      await expect(panel(page).getByRole("status")).toContainText(/已自动同步/);
      // 取回之后本机比云端多出应用自己的默认数据，第一台设备会把差异补写回云端（约 5 秒后）。
      // 等它写完再开第二台：否则第二台拿到的云端内容随后就被这次补写改掉，
      // 它带着旧基线的写入会被服务端以 412 挡下——那是对的，但不是这条用例要验的。
      await expect.poll(() => fake.puts().length, { timeout: 20_000 }).toBe(1);
      const putsBefore = fake.puts().length;

      // 第二台设备（独立的浏览器环境）：本机已有数据、云端也有、从没同步过，
      // 不知道谁新，暂停并交给用户，不发任何写入，本机数据原样保留。
      const device = await browser.newContext({ locale: "zh-CN", timezoneId: "Asia/Shanghai", serviceWorkers: "block" });
      try {
        const other = await device.newPage();
        const shared = await useAccountApi(other, fake);
        await openBackup(other, style, { bookmarks: JSON.stringify(["local-only"]) });
        await panel(other).getByLabel("账号名").fill(name);
        await panel(other).getByLabel("口令", { exact: true }).fill(PASSWORD);
        await panel(other).getByRole("button", { name: "登录", exact: true }).click();
        await expect(panel(other).getByRole("status")).toContainText("都有未同步的改动");
        expect(shared.puts()).toHaveLength(putsBefore);
        expect(JSON.parse((await ls(other, "bookmarks"))!)).toEqual(["local-only"]);

        // 按提示「从云端合并」：两边并集，之后自动同步恢复（状态行不再是冲突）。
        await panel(other).getByRole("button", { name: "从云端合并" }).click();
        await expect(toast(other).filter({ hasText: /^已合并/ })).toBeVisible();
        await expect(panel(other).getByRole("status")).toContainText(/已自动同步/);
        expect(await ls(other, "e2e-cloud-only")).toBe("1");
      } finally {
        await device.close();
      }
    });

    test(`在线账号：退出再登录，本机与云端内容一样就不算冲突（${style}）`, async ({ page }) => {
      test.skip(LIVE, "同上：靠预置账号与云端内容");
      const fake = await useAccountApi(page);
      await openBackup(page, style, { bookmarks: JSON.stringify(["same"]) });
      const name = await signUp(page);
      await panel(page).getByRole("button", { name: "上传到云端" }).click();
      await expect(toast(page).filter({ hasText: /^已上传到云端/ })).toBeVisible();

      await panel(page).getByRole("button", { name: "退出登录", exact: true }).click();
      await panel(page).getByLabel("账号名").fill(name);
      await panel(page).getByLabel("口令", { exact: true }).fill(PASSWORD);
      await panel(page).getByRole("button", { name: "登录", exact: true }).click();
      // 退出作废了基线；但两边其实一样，不该为此打扰用户。
      await expect(panel(page).getByRole("status")).toContainText(/已自动同步/);
      expect(fake.puts().length).toBeGreaterThan(0);
    });

    test(`在线账号：修改口令与注销账号（${style}）`, async ({ page }) => {
      const fake = await useAccountApi(page);
      await openBackup(page, style);
      const name = await signUp(page);

      await panel(page).getByRole("button", { name: "修改口令" }).click();
      const dialog = page.getByRole("dialog");
      await dialog.getByLabel("当前口令").fill(PASSWORD);
      await dialog.getByLabel("新口令", { exact: true }).fill("a brand new pass");
      await dialog.getByLabel("再输一次新口令").fill("a different one");
      await dialog.getByRole("button", { name: "修改口令" }).click();
      await expect(dialog.getByRole("alert")).toContainText("两次输入的新口令不一致");
      await dialog.getByLabel("再输一次新口令").fill("a brand new pass");
      await dialog.getByRole("button", { name: "修改口令" }).click();
      await expect(toast(page)).toContainText("口令已修改");
      await expect(dialog).toBeHidden();
      expect(fake.requests.some((r) => r.path === "/api/account/password") || LIVE).toBe(true);

      await panel(page).getByRole("button", { name: "注销账号" }).click();
      await dialog.getByLabel("口令", { exact: true }).fill("wrong password");
      await dialog.getByRole("button", { name: "注销账号" }).click();
      await expect(dialog.getByRole("alert")).toContainText("账号名或口令不对");
      await dialog.getByLabel("口令", { exact: true }).fill("a brand new pass");
      await dialog.getByRole("button", { name: "注销账号" }).click();
      await expect(toast(page)).toContainText("账号已注销");
      await expect(panel(page).getByRole("button", { name: "登录", exact: true })).toBeVisible();
      // 注销只删云端，本机数据不动；登录同名账号的口令已失效。
      await panel(page).getByLabel("账号名").fill(name);
      await panel(page).getByLabel("口令", { exact: true }).fill("a brand new pass");
      await panel(page).getByRole("button", { name: "登录", exact: true }).click();
      await expect(panel(page).getByRole("alert")).toContainText("账号名或口令不对");
    });

    test(`在线账号：服务器没启用账号时给出说明，自定义远端不受影响（${style}）`, async ({ page }) => {
      test.skip(LIVE, "需要让服务器「没启用账号」，只有假服务能做到");
      const fake = await useAccountApi(page);
      fake.enabled = false;
      await openBackup(page, style);
      await expect(panel(page).getByRole("status")).toContainText("没有启用在线账号");
      await expect(panel(page).getByLabel("账号名")).toHaveCount(0);
      // 自定义远端的表单照常可用。
      await page.locator("#remote-get-url").fill("https://example.invalid/backup.json");
      await expect(page.getByRole("button", { name: "从远端合并" })).toBeEnabled();

      // 服务恢复后点「重试」就出现登录表单。
      fake.enabled = true;
      await panel(page).getByRole("button", { name: "重试" }).click();
      await expect(panel(page).getByLabel("账号名")).toBeVisible();
    });
  });
}
