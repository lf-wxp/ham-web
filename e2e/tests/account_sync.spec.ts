import { createHash } from "node:crypto";
import { expect, test } from "./fixtures";
import type { Browser, Page, Request, Route } from "@playwright/test";

/**
 * 在线账号（`/backup` 页）与「自定义远端同步」并存。
 *
 * 默认用**内存里的假服务**接管 `/api/account/*`（不依赖后端有没有设 `ACCOUNT_DB`），
 * 它按真实服务端的契约实现（会话、按 key 的清单 / 取值 / 带条件的上传、412、清单指纹同口径）；
 * `E2E_ACCOUNT_LIVE=1` 时不拦截，改打真实后端（需 `ACCOUNT_DB` 已设、前端能代理 `/api`），
 * 同一批断言既验前端也验假服务与真服务端没有漂移。
 *
 * 自动同步按 key 增量：每个 key 一个摘要（SHA-256），云端给出清单，前端只上传 / 取回有变化的 key，
 * 两台设备各改各的 key 互不打扰，只有同一项数据两边都改了才停下来问用户。
 *
 * 每个用例在像素风与现代风各跑一遍（两套皮肤的控件与布局互不共享）。
 */

const LIVE = process.env.E2E_ACCOUNT_LIVE === "1";
const PASSWORD = "correct horse";

// ─────────────────────────── 假服务 ───────────────────────────

const sha256 = (text: string) => createHash("sha256").update(text, "utf8").digest("hex");

/** 与 `ham_web_core::remote_sync::fingerprint_pairs` 同口径的 FNV-1a 64 位（逐段加长度前缀，键按字节序）。 */
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

/** 与 `ham_web_core::remote_sync::is_device_local_key` 同口径：只属于这台设备、不参与同步的 key。 */
function isDeviceLocal(key: string): boolean {
  return (
    key.endsWith(":kv-version") ||
    key.startsWith("api-cache:") ||
    key.startsWith("ui:") ||
    key.startsWith("bank-digest:") ||
    [
      "theme", "locale", "fab-position", "cat-settings", "push-subscription", "app:changelog-seen",
      "sat-watch-notified", "sat-apt-notified", "nec.pending-template",
      "remote-sync", "remote-sync-state", "account-sync-state", "account-sync-options", "sync-channel",
    ].includes(key)
  );
}

/** 与 `ham_web_core::delta::in_scope`（服务端按带影像的范围算）同口径。 */
const inScope = (key: string) => !isDeviceLocal(key) && key !== "qsl-image-index" && Buffer.byteLength(key) <= 512;

type Recorded = { method: string; path: string; headers: Record<string, string>; body: string };

class FakeAccountServer {
  /** 每台「设备」（页面）各有各的登录态，像真实的 Cookie 一样互不影响；云端与账号是共享的。 */
  private readonly sessions = new Map<string, { id: number; username: string }>();
  readonly accounts = new Map<string, { id: number; username: string; password: string }>();
  /** 云端备份的内容（单账号即可：用例里只会有一个账号上传）；`null` = 还没有备份。 */
  data: Record<string, string> | null = null;
  private updatedAt = 0;
  enabled = true;
  readonly requests: Recorded[] = [];
  private nextId = 1;

  /** 云端现在每个 key 的摘要（只算参与同步的 key）。 */
  manifest(): Record<string, string> {
    const out: Record<string, string> = {};
    for (const [k, v] of Object.entries(this.data ?? {})) if (inScope(k)) out[k] = sha256(v);
    return out;
  }

  private size() {
    return Object.values(this.data ?? {}).reduce((n, v) => n + Buffer.byteLength(v), 0);
  }

  /** 云端内容的概况（用例里读 `cloud!.body` 比对「有没有被动过」）。 */
  get cloud(): { body: string; fp: string; updatedAt: number } | null {
    return this.data ? { body: JSON.stringify(this.data), fp: fingerprint(this.manifest()), updatedAt: this.updatedAt } : null;
  }

  private touch() {
    this.updatedAt = Math.floor(Date.now() / 1000);
  }

  /** 另一台设备把云端整份换成这些内容（不经过被测页面）。 */
  uploadFromAnotherDevice(map: Record<string, string>) {
    this.data = { ...map };
    this.touch();
  }

  /** 另一台设备只改了这几个 key，其余原样。 */
  anotherDeviceSets(partial: Record<string, string>) {
    this.data = { ...(this.data ?? {}), ...partial };
    this.touch();
  }

  puts(): Recorded[] {
    return this.requests.filter((r) => r.method === "PUT" && r.path === "/api/account/backup");
  }

  /** 自动同步的按 key 上传。 */
  pushes(): Recorded[] {
    return this.requests.filter((r) => r.method === "POST" && r.path === "/api/account/sync/push");
  }

  /** 某次按 key 上传里带的 key 与每条的「我看到的那一版」。 */
  changesOf(push: Recorded): { key: string; base: string | null; value: string | null }[] {
    return JSON.parse(push.body).changes;
  }

  private info() {
    return { size: this.size(), updated_at: this.updatedAt, fingerprint: fingerprint(this.manifest()) };
  }

  private session(dev: string) {
    const user = this.sessions.get(dev);
    return {
      enabled: this.enabled,
      registration_open: true,
      user: this.enabled && user
        ? { id: user.id, username: user.username, created_at: 1_700_000_000, backup: this.data ? this.info() : null }
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
      this.data = null;
      return route.fulfill({ status: 204 });
    }

    // 以下接口都要登录，并核对「我以为的账号」就是当前登录的那个。
    const needsSession = path === "/api/account/backup" || path.startsWith("/api/account/sync/");
    if (!needsSession) return this.err(route, 404, "bad_request");
    const user = me();
    if (!user) return this.err(route, 401, "unauthorized");
    const claimed = req.headers()["x-account-id"];
    if (claimed !== undefined && Number(claimed) !== user.id) return this.err(route, 409, "account_changed");

    if (path === "/api/account/backup") {
      if (method === "GET") {
        if (!this.data) return route.fulfill({ status: 204 });
        return route.fulfill({ status: 200, contentType: "application/json", body: JSON.stringify(this.data) });
      }
      if (method === "DELETE") {
        this.data = null;
        return route.fulfill({ status: 204 });
      }
      if (method === "PUT") {
        const h = req.headers();
        const ifMatch = h["if-match"]?.replace(/"/g, "");
        const ifNone = h["if-none-match"];
        const current = this.data ? fingerprint(this.manifest()) : null;
        if ((ifMatch !== undefined && ifMatch !== current) || (ifNone === "*" && current !== null)) {
          return this.err(route, 412, "precondition_failed");
        }
        let map: Record<string, string>;
        try {
          map = JSON.parse(body);
        } catch {
          return this.err(route, 400, "invalid_backup");
        }
        this.data = map;
        this.touch();
        // 回执：概况 + 写入之后云端的清单（前端据此建立每个 key 的基线）。
        return this.ok(route, 200, { ...this.info(), keys: this.manifest() });
      }
    }

    if (path === "/api/account/sync/manifest" && method === "GET") {
      if (!this.data) return route.fulfill({ status: 204 });
      return this.ok(route, 200, { ...this.info(), keys: this.manifest() });
    }
    if (path === "/api/account/sync/pull" && method === "POST") {
      const keys: string[] = json().keys ?? [];
      const values: Record<string, string> = {};
      for (const k of keys) if (this.data && k in this.data) values[k] = this.data[k];
      return this.ok(route, 200, { values });
    }
    if (path === "/api/account/sync/push" && method === "POST") {
      const changes: { key: string; base: string | null; value: string | null }[] = json().changes ?? [];
      const current = this.manifest();
      // 每个 key 带着「我看到的那一版」：有一个对不上，整批都不生效。
      if (changes.some((c) => (current[c.key] ?? null) !== (c.base ?? null))) {
        return this.err(route, 412, "precondition_failed");
      }
      const next = { ...(this.data ?? {}) };
      for (const c of changes) {
        if (c.value === null) delete next[c.key];
        else next[c.key] = c.value;
      }
      this.data = next;
      this.touch();
      return this.ok(route, 200, { info: this.info(), keys: this.manifest() });
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

/**
 * Toast 的正文节点（外层是 `[data-slot="toast"]`）。
 *
 * 像素风在正文前面画了一枚装饰字符（✓ / ✗ / i，`aria-hidden`，见 `ui/toast.rs`）。断言文案要落在
 * 正文上：取外层节点的 textContent 时，`^已上传到云端…` 这类起锚正则会被装饰字符挡掉。
 */
const toastText = (page: Page) => page.locator('[data-slot="toast-text"]');
const panel = (page: Page) => page.getByTestId("account-panel");
const syncStatus = (page: Page) => panel(page).getByRole("status");

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

async function signIn(page: Page, name: string) {
  await panel(page).getByLabel("账号名").fill(name);
  await panel(page).getByLabel("口令", { exact: true }).fill(PASSWORD);
  await panel(page).getByRole("button", { name: "登录", exact: true }).click();
}

/** 第二台「设备」：独立的浏览器环境（自己的 localStorage 与 Cookie），共用同一个假服务端。 */
async function secondDevice(browser: Browser, fake: FakeAccountServer) {
  const context = await browser.newContext({ locale: "zh-CN", timezoneId: "Asia/Shanghai", serviceWorkers: "block" });
  const page = await context.newPage();
  await useAccountApi(page, fake);
  return { context, page };
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
  await expect(toastText(page).filter({ hasText: /^已合并/ })).toBeVisible();
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
      await expect(toastText(page)).toContainText("已退出登录");
      await expect(panel(page).getByRole("button", { name: "登录", exact: true })).toBeVisible();
      // 退出后偏好回到「没选过」，账号通道的基线与主人作废。
      expect(await ls(page, "sync-channel")).toBeNull();
      const state = await ls(page, "account-sync-state");
      if (state !== null) {
        const parsed = JSON.parse(state);
        expect(Object.keys(parsed.keys ?? {})).toHaveLength(0);
        expect(parsed.owner ?? null).toBeNull();
      }

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
      await expect.poll(() => fake.pushes().length, { timeout: 20_000 }).toBeGreaterThan(0);
      fake.requests.length = 0;

      await page.evaluate(() => {
        localStorage.setItem("bookmarks", JSON.stringify(["q1", "q2", "q3"]));
        localStorage.setItem("e2e-from-cloud", "yes");
      });
      await panel(page).getByRole("button", { name: "上传到云端" }).click();
      await expect(toastText(page)).toContainText(/^已上传到云端 \d+ 项$/);
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
      // 手动上传之后本机与云端一致：自动同步立刻有了基线，不会把刚上传的东西再传一遍。
      const stored = JSON.parse((await ls(page, "account-sync-state"))!);
      expect(Object.keys(stored.keys ?? {}).length).toBeGreaterThan(0);

      // 本机之后变了样：「合并」取并集（云端独有的 key 回来、本机新增的收藏留下）。
      await page.evaluate(() => {
        localStorage.removeItem("e2e-from-cloud");
        localStorage.setItem("bookmarks", JSON.stringify(["only-here"]));
      });
      await panel(page).getByRole("button", { name: "从云端合并" }).click();
      await expect(toastText(page)).toContainText(/^已合并 \d+ 条数据$/);
      expect(await ls(page, "e2e-from-cloud")).toBe("yes");
      expect(JSON.parse((await ls(page, "bookmarks"))!)).toEqual(expect.arrayContaining(["q1", "q3", "only-here"]));

      // 「覆盖」要先确认，之后本机同名数据就是云端的样子。
      await page.evaluate(() => localStorage.setItem("e2e-from-cloud", "local-change"));
      await panel(page).getByRole("button", { name: "从云端恢复（覆盖）" }).click();
      await expect(page.getByRole("dialog")).toContainText("用云端数据覆盖本机？");
      await page.getByRole("dialog").getByRole("button", { name: "从云端恢复（覆盖）" }).click();
      await expect(toastText(page)).toContainText(/^已恢复 \d+ 条数据$/);
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

    test(`在线账号：自动写入只传改动的 key，且每条带着它看到的那一版（${style}）`, async ({ page }) => {
      test.setTimeout(120_000);
      const fake = await useAccountApi(page);
      await openBackup(page, style);
      await signUp(page);
      await expect(syncStatus(page)).toContainText(/已自动同步/);
      const before = fake.pushes().length;

      // 本机改了一个 key → 自动写入；云端还没有这个 key，所以「我看到的那一版」是没有。
      // 登录那一轮刚上传过一次，而两次自动写入至少隔 30 秒（避免频繁上传），所以最多要等到那时候。
      await mergeImport(page, { bookmarks: JSON.stringify(["a"]) });
      await expect.poll(() => fake.pushes().length, { timeout: 45_000 }).toBe(before + 1);
      const first = fake.changesOf(fake.pushes().at(-1)!);
      // 只有改过的那一个 key，其余几十个没动的都不传（旧协议每次整份重传，含 QSL 影像）。
      expect(first.map((c) => c.key)).toEqual(["bookmarks"]);
      expect(first[0].base).toBeNull();
      expect(JSON.parse(first[0].value!)).toEqual(["a"]);
      expect(fake.pushes().at(-1)!.headers["x-account-id"]).toBeDefined();
      await expect(syncStatus(page)).toContainText(/已自动同步/);

      // 再改一次：这次它看到的云端那一版就是上一次写进去的。两次自动写入至少隔 30 秒（避免频繁上传）。
      // 假服务按它自己算的摘要核对每条的 `base`：前端与它对同一份内容算出的 SHA-256 不一致就会 412，
      // 所以这里同时验了「摘要口径跨语言一致」。
      await mergeImport(page, { bookmarks: JSON.stringify(["a", "b"]) });
      await expect.poll(() => fake.pushes().length, { timeout: 60_000 }).toBe(before + 2);
      const second = fake.changesOf(fake.pushes().at(-1)!);
      expect(second.map((c) => c.key)).toEqual(["bookmarks"]);
      expect(second[0].base).toBe(sha256(first[0].value!));
      expect(JSON.parse(second[0].value!)).toEqual(expect.arrayContaining(["a", "b"]));
      await expect(syncStatus(page)).toContainText(/已自动同步/);
    });

    test(`在线账号：两台设备各改各的数据，都成功且不打扰用户（${style}）`, async ({ page, browser }) => {
      test.skip(LIVE, "要在两台设备间对照云端内容，只有假服务能直接读");
      test.setTimeout(180_000);
      const fake = await useAccountApi(page);
      await openBackup(page, style);
      const name = await signUp(page);
      await expect(syncStatus(page)).toContainText(/已自动同步/);

      const other = await secondDevice(browser, fake);
      try {
        await openBackup(other.page, style);
        await signIn(other.page, name);
        await expect(syncStatus(other.page)).toContainText(/已自动同步/);

        // 两台设备都还没同步对方，各自改了不同的数据：A 改收藏，B 另加一项。
        await mergeImport(page, { bookmarks: JSON.stringify(["from-a"]) });
        await mergeImport(other.page, { "e2e-only-on-b": "b-value" });
        await expect.poll(() => fake.data?.bookmarks, { timeout: 60_000 }).toBeDefined();
        await expect.poll(() => fake.data?.["e2e-only-on-b"], { timeout: 60_000 }).toBe("b-value");
        // 旧协议在这里会让后写的那台因为「云端已被更新」被挡下并暂停；现在各传各的，谁也不会挡住谁。
        await expect(syncStatus(page)).not.toContainText("暂停");
        await expect(syncStatus(other.page)).not.toContainText("暂停");

        // 重新打开页面：各自取回对方的改动，没有冲突，两边一致。
        await page.reload();
        await other.page.reload();
        await expect.poll(() => ls(page, "e2e-only-on-b"), { timeout: 20_000 }).toBe("b-value");
        await expect.poll(() => ls(other.page, "bookmarks"), { timeout: 20_000 }).toBe(JSON.stringify(["from-a"]));
        await expect(syncStatus(page)).toContainText(/已自动同步/);
        await expect(syncStatus(other.page)).toContainText(/已自动同步/);
      } finally {
        await other.context.close();
      }
    });

    test(`在线账号：同一项数据两边都改了，能合并的合并，不能合并的停下来问用户（${style}）`, async ({ page }) => {
      test.skip(LIVE, "需要在页面不知情的情况下改云端内容，只有假服务能做到");
      const fake = await useAccountApi(page);
      await openBackup(page, style);
      await signUp(page);
      await expect(syncStatus(page)).toContainText(/已自动同步/);

      // 登录之后另一台设备改了云端的收藏与一项累计数据；本机随后也改了同样两项。
      fake.anotherDeviceSets({
        bookmarks: JSON.stringify(["other-device"]),
        "e2e-counter": "from-cloud",
      });
      await mergeImport(page, { bookmarks: JSON.stringify(["mine"]), "e2e-counter": "from-local" });
      // 没有安全合并规则的那一项：停下来问，说清是「同一项数据」，而不是笼统的「有改动」。
      await expect(syncStatus(page)).toContainText("改动了同一项数据", { timeout: 30_000 });
      // 自动写入不会覆盖云端：云端那两项原样。
      expect(fake.data!["e2e-counter"]).toBe("from-cloud");
      expect(JSON.parse(fake.data!.bookmarks)).toEqual(["other-device"]);

      // 按提示选「从云端恢复（覆盖）」：以云端为准，之后自动同步恢复。
      await panel(page).getByRole("button", { name: "从云端恢复（覆盖）" }).click();
      await page.getByRole("dialog").getByRole("button", { name: "从云端恢复（覆盖）" }).click();
      await expect(toastText(page)).toContainText(/^已恢复 \d+ 条数据$/);
      expect(await ls(page, "e2e-counter")).toBe("from-cloud");
      await expect(syncStatus(page)).toContainText(/已自动同步/);
    });

    test(`在线账号：新设备登录，云端有数据则直接取回；本机已有的收藏与云端自动合并（${style}）`, async ({ page, browser }) => {
      test.skip(LIVE, "需要预置云端内容，只有假服务能做到");
      test.setTimeout(90_000);
      const fake = await useAccountApi(page);
      fake.uploadFromAnotherDevice({ bookmarks: JSON.stringify(["from-cloud"]), "e2e-cloud-only": "1" });
      await openBackup(page, style);
      const name = await signUp(page);
      // 全新浏览器（没有任何用户数据）：自动取回，直接用云端的内容。
      await expect.poll(async () => ls(page, "e2e-cloud-only"), { timeout: 20_000 }).toBe("1");
      await expect(syncStatus(page)).toContainText(/已自动同步/);
      expect(JSON.parse((await ls(page, "bookmarks"))!)).toEqual(["from-cloud"]);

      // 第二台设备（独立的浏览器环境）：本机已有收藏、云端也有、从没同步过。
      // 收藏有安全的合并规则（并集），所以自动合并、不打扰用户；本机原有的收藏不丢。
      const other = await secondDevice(browser, fake);
      try {
        await openBackup(other.page, style, { bookmarks: JSON.stringify(["local-only"]) });
        await signIn(other.page, name);
        await expect(syncStatus(other.page)).toContainText(/已自动同步/, { timeout: 30_000 });
        expect(JSON.parse((await ls(other.page, "bookmarks"))!)).toEqual(expect.arrayContaining(["local-only", "from-cloud"]));
        expect(await ls(other.page, "e2e-cloud-only")).toBe("1");
        // 合并结果也传回了云端。
        await expect
          .poll(() => JSON.parse(fake.data!.bookmarks), { timeout: 30_000 })
          .toEqual(expect.arrayContaining(["local-only", "from-cloud"]));
      } finally {
        await other.context.close();
      }
    });

    test(`在线账号：本机一下子少了很多数据，停下来问用户，不把删除同步到云端（${style}）`, async ({ page }) => {
      test.skip(LIVE, "需要读云端内容并在不经页面的情况下清本机数据，只有假服务能做到");
      test.setTimeout(120_000);
      const fake = await useAccountApi(page);
      await openBackup(page, style);
      await signUp(page);
      await expect(syncStatus(page)).toContainText(/已自动同步/);

      // 本机有 40 项进度，已经同步到云端。
      const keys = Array.from({ length: 40 }, (_, i) => `practice:e2e-${i}`);
      await mergeImport(page, Object.fromEntries(keys.map((k) => [k, "p"])));
      await expect.poll(() => keys.every((k) => fake.data && k in fake.data), { timeout: 30_000 }).toBe(true);
      await expect(syncStatus(page)).toContainText(/已自动同步/);

      // 浏览器清理了站点数据：本机一下子只剩 2 项（不是用户一个个删的）。
      await page.evaluate((ks) => ks.slice(2).forEach((k) => localStorage.removeItem(k)), keys);
      await page.reload();
      await expect(syncStatus(page)).toContainText("少了很多数据", { timeout: 30_000 });
      // 云端一项没少：删除没有被同步出去。
      expect(keys.every((k) => k in fake.data!)).toBe(true);

      // 用户选「从云端恢复」就把数据找回来。
      await panel(page).getByRole("button", { name: "从云端恢复（覆盖）" }).click();
      await page.getByRole("dialog").getByRole("button", { name: "从云端恢复（覆盖）" }).click();
      await expect(toastText(page)).toContainText(/^已恢复 \d+ 条数据$/);
      expect(await ls(page, "practice:e2e-39")).toBe("p");
      await expect(syncStatus(page)).toContainText(/已自动同步/);
    });

    test(`在线账号：退出再登录，本机与云端内容一样就不算冲突（${style}）`, async ({ page }) => {
      test.skip(LIVE, "同上：靠预置账号与云端内容");
      const fake = await useAccountApi(page);
      await openBackup(page, style, { bookmarks: JSON.stringify(["same"]) });
      const name = await signUp(page);
      await panel(page).getByRole("button", { name: "上传到云端" }).click();
      await expect(toastText(page).filter({ hasText: /^已上传到云端/ })).toBeVisible();

      await panel(page).getByRole("button", { name: "退出登录", exact: true }).click();
      await signIn(page, name);
      // 退出作废了基线；但两边其实一样，不该为此打扰用户。
      await expect(syncStatus(page)).toContainText(/已自动同步/);
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
      await expect(toastText(page)).toContainText("口令已修改");
      await expect(dialog).toBeHidden();
      expect(fake.requests.some((r) => r.path === "/api/account/password") || LIVE).toBe(true);

      await panel(page).getByRole("button", { name: "注销账号" }).click();
      await dialog.getByLabel("口令", { exact: true }).fill("wrong password");
      await dialog.getByRole("button", { name: "注销账号" }).click();
      await expect(dialog.getByRole("alert")).toContainText("账号名或口令不对");
      await dialog.getByLabel("口令", { exact: true }).fill("a brand new pass");
      await dialog.getByRole("button", { name: "注销账号" }).click();
      await expect(toastText(page)).toContainText("账号已注销");
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
