import { expect, stubApi, test, type Page } from "./fixtures";

/** `/api/solar` 的最小合法响应：`conditions` 是必填数组，其余字段可省。 */
const SOLAR_OK = {
  updated: "2026-11-27 10:00",
  solar_flux: 123,
  a_index: 5,
  k_index: 2,
  sunspots: 42,
  xray: "B1.2",
  conditions: [],
};

/** NOAA 跨源降级路径在测试环境没有外网，直接掐掉，别让用例白等 5 秒超时。 */
async function blockNoaa(page: Page) {
  await page.route("**/services.swpc.noaa.gov/**", (route) => route.abort());
}

/** 实时接口全断：纯静态托管 / 断网时的表现。 */
function failApi(page: Page) {
  return page.route("**/api/**", (route) =>
    route.fulfill({ status: 503, contentType: "text/plain", body: "offline" }),
  );
}

/**
 * 站内导航（不刷新页面）：触发路由器自己的点击委托，与用户点链接走同一条路径。
 * 用 `page.goto` 会整页重载，把「状态要随路由清空」这类要点直接洗掉，断言就空了。
 */
async function spaNavigate(page: Page, href: string) {
  await page.evaluate((target) => {
    const a = document.querySelector<HTMLAnchorElement>(`a[href="${target}"]`);
    if (!a) throw new Error(`页面里没有指向 ${target} 的链接`);
    a.click();
  }, href);
  await expect(page).toHaveURL(new RegExp(`${href}$`));
}

/**
 * 观察期间 `[data-data-status]` 一旦出现就返回 `true`，窗口结束仍没有则返回 `false`。
 * 用 MutationObserver 而不是「睡一会儿再查一次」：缺陷写入落在固定等待之后时，睡后查会漏掉；
 * 探针只要挂得够早，晚落地也一样被记下（约定见 `docs/engineering/testing.md`）。
 * 只在当前 document 有效：不要跨整页跳转使用（新 document 会带走旧探针）。
 * 注意：只对**当前页面**有效，换页前老页上合法出现的提示也会被记下——要盯新页就换页后再挂。
 */
function watchForNotice(page: Page, windowMs: number) {
  return page.evaluate(
    (ms) =>
      new Promise<boolean>((resolve) => {
        if (document.querySelector("[data-data-status]")) {
          resolve(true);
          return;
        }
        const mo = new MutationObserver(() => {
          if (document.querySelector("[data-data-status]")) {
            mo.disconnect();
            resolve(true);
          }
        });
        mo.observe(document.body, { childList: true, subtree: true });
        setTimeout(() => {
          mo.disconnect();
          resolve(false);
        }, ms);
      }),
    windowMs,
  );
}

test("离线快照：接口再次失败时给出快照时间，页面内容不消失", async ({ page }) => {
  await blockNoaa(page);
  await stubApi(page, { "/api/solar": SOLAR_OK });
  await page.goto("/solar");

  // 有的接口成功、有的失败时只说「局部」，不夸大成整页不可用。
  await expect(page.locator('[data-data-status="partial"]')).toBeVisible();

  // 同一 URL 失败后回退到刚才成功时写入的快照。
  await page.unroute("**/api/**");
  await failApi(page);
  await page.reload();

  const notice = page.locator('[data-data-status="snapshot"]');
  await expect(notice).toBeVisible();
  await expect(notice).toContainText("离线快照");
  await expect(notice).toContainText(/\d{4}-\d{2}-\d{2} \d{2}:\d{2}/);
  await expect(page.locator("#fatal")).toHaveCount(0);
  await expect(page.getByRole("heading", { name: "实时太阳活动" })).toBeVisible();
  await expect(page.getByText("太阳活动指数").first()).toBeVisible();
});

test("没有快照时才说整页实时数据不可用，静态资料仍在", async ({ page }) => {
  await blockNoaa(page);
  // 每次导航前都清一次：确保这一档真的没有快照可用。
  await page.addInitScript(() => {
    for (const key of Object.keys(localStorage)) {
      if (key.startsWith("api-cache:")) localStorage.removeItem(key);
    }
  });
  await failApi(page);
  await page.goto("/solar");

  const notice = page.locator('[data-data-status="unavailable"]');
  await expect(notice).toBeVisible();
  await expect(notice).toContainText("实时数据暂不可用");
  await expect(page.locator("#fatal")).toHaveCount(0);
  await expect(page.getByText("太阳活动指数").first()).toBeVisible();

  // 换页后旧页的失败状态不能跟过来（站内导航，不是整页重载）。
  await spaNavigate(page, "/tools");
  await expect(page.locator("[data-data-status]")).toHaveCount(0);
});

test("在途请求晚于换页失败时，不记到新页头上", async ({ page }) => {
  await blockNoaa(page);
  await page.route("**/api/**", async (route) => {
    await new Promise((resolve) => setTimeout(resolve, 1500));
    await route.fulfill({ status: 503, contentType: "text/plain", body: "slow offline" });
  });
  await page.goto("/solar");
  // 请求还在路上就切走。
  await spaNavigate(page, "/tools");
  // 探针只盯新页：换页前老页上合法出现的提示（请求在切走前就失败）不算数。
  const sawNotice = watchForNotice(page, 3000);
  await page.waitForResponse((r) => new URL(r.url()).pathname.startsWith("/api/"));
  expect(await sawNotice).toBe(false);
});

test("后台过境提醒的轮询失败不计入页头提示，静态页保持干净", async ({ page }) => {
  await page.addInitScript(() => {
    Object.defineProperty(Notification, "permission", { value: "granted", configurable: true });
    // APT 提醒不需要收藏卫星，开了就会在启动时立刻去取 /api/passes。
    localStorage.setItem("sat-watch", JSON.stringify({ apt_alert: true }));
  });
  await blockNoaa(page);
  await failApi(page);
  const passes = page.waitForResponse((r) => new URL(r.url()).pathname === "/api/passes");
  await page.goto("/tools");
  await passes;
  // 响应 503 是立刻回的，但记录发生在响应之后的异步链里：用探针盯住这一小段，
  // 而不是固定睡一会儿（初始检查兜住「已经写了」的情形，观察兜住「稍后写」的情形）。
  expect(await watchForNotice(page, 1000)).toBe(false);
});

test("200 但内容无法解析的响应不会顶掉上一份可用快照", async ({ page }) => {
  await blockNoaa(page);
  await stubApi(page, { "/api/solar": SOLAR_OK });
  await page.goto("/solar");
  await expect(page.locator('[data-data-status="partial"]')).toBeVisible();

  // 网关错误页：状态码 200、内容不是 JSON。
  await page.unroute("**/api/**");
  await page.route("**/api/**", (route) =>
    route.fulfill({ status: 200, contentType: "text/html", body: "<html>bad gateway</html>" }),
  );
  await page.reload();
  await expect(page.locator('[data-data-status="unavailable"]')).toBeVisible();

  // 之后真正断网：快照必须还在。
  await page.unroute("**/api/**");
  await failApi(page);
  await page.reload();
  await expect(page.locator('[data-data-status="snapshot"]')).toBeVisible();
});

test("后台推送不可用时说明页内每日提醒仍生效", async ({ page }) => {
  await stubApi(page, {});
  await page.goto("/notifications");

  await expect(page.getByRole("heading", { level: 2, name: "后台推送" })).toBeVisible();
  await expect(page.getByText(/页面打开期间的每日提醒仍然生效/)).toBeVisible();
  // 推送 / 账号接口不是页面数据，不该让通知中心冒出「实时数据不可用」。
  await expect(page.locator("[data-data-status]")).toHaveCount(0);
});
