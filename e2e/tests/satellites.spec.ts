import { expect, stubApi, test } from "./fixtures";

const NOW = new Date("2026-10-14T10:00:00+08:00");
const T0 = Math.floor(NOW.getTime() / 1000);

/** 两颗星各过境一次：ISS 一小时后、SO-50 两小时后；时长都是 10 分钟。 */
const PASSES = {
  passes: [
    { name: "ISS (ZARYA)", norad: 25544, aos: T0 + 3600, los: T0 + 4200, max_elev: 45.4, max_elev_time: T0 + 3900, azimuth: 225.2 },
    { name: "SO-50", norad: 27607, aos: T0 + 7200, los: T0 + 7800, max_elev: 22.6, max_elev_time: T0 + 7500, azimuth: 10.1 },
  ],
};

test("业余卫星：ISS 实时位置、过境查询带参数、收藏与只看收藏，刷新后自动查询", async ({ page }) => {
  await page.clock.setFixedTime(NOW);
  const asked: URLSearchParams[] = [];
  await stubApi(page, {
    "/api/iss": { latitude: 39.9123, longitude: 116.4049, altitude: 418.6, velocity: 7.66, visibility: "daylight" },
    "/api/passes": (url: URL) => {
      asked.push(url.searchParams);
      return PASSES;
    },
  });
  await page.goto("/satellites");

  // ISS 卡：按 2 位小数与整数公里显示，日照状态翻成中文。
  await expect(page.getByText("39.91°")).toBeVisible();
  await expect(page.getByText("419 km")).toBeVisible();
  await expect(page.getByText(/日照中/)).toBeVisible();

  // 没有收藏时不自动查询；改最小仰角后点查询，参数原样带上。
  await expect(page.getByText("输入经纬度后点击「查询过境」，默认位置为北京。")).toBeVisible();
  expect(asked).toHaveLength(0);
  await page.getByLabel("最小仰角（°）").fill("20");
  await page.getByRole("button", { name: "查询过境" }).click();
  const table = page.getByRole("table").filter({ hasText: "最大仰角" });
  await expect(table.getByRole("row")).toHaveCount(3); // 表头 + 两次过境
  const q = asked.at(-1)!;
  expect([q.get("lat"), q.get("lon"), q.get("min_elev"), q.get("group")]).toEqual(["39.9", "116.4", "20", "amateur"]);
  const iss = table.getByRole("row", { name: /ISS \(ZARYA\)/ });
  await expect(iss).toContainText("45°");
  await expect(iss).toContainText("225°");
  await expect(iss).toContainText("10 分钟");

  // 收藏 ISS → 只看收藏后剩一行；收藏摘要列出下一次过境。
  const star = page.getByRole("button", { name: "收藏 ISS (ZARYA)" });
  await star.click();
  await expect(star).toHaveAttribute("aria-pressed", "true");
  await expect(page.getByText("已收藏：ISS (ZARYA)")).toBeVisible();
  // 括号里是查询到的过境总数（不是收藏数）。
  await page.getByRole("checkbox", { name: "只看收藏（共 2 次过境）" }).click();
  await expect(table.getByRole("row")).toHaveCount(2);
  await expect(table.getByRole("row", { name: /SO-50/ })).toHaveCount(0);

  // 收藏持久化：刷新后挂载即查询，不必再点。
  const before = asked.length;
  await page.reload();
  await expect(page.getByRole("button", { name: "收藏 ISS (ZARYA)" })).toHaveAttribute("aria-pressed", "true");
  expect(asked.length).toBeGreaterThan(before);
  const stored = await page.evaluate(() => localStorage.getItem("sat-watch") ?? "");
  expect(stored).toContain("25544");
});

test("业余卫星：过境与 ISS 接口失败时分别降级", async ({ page }) => {
  await stubApi(page, {});
  await page.goto("/satellites");
  await expect(page.getByText("ISS 数据暂不可用（可能因网络受限）。")).toBeVisible();
  await page.getByRole("button", { name: "查询过境" }).click();
  await expect(page.getByText("过境数据暂不可用（可能因网络受限），请稍后重试。")).toBeVisible();
  // 静态的频率表不受影响。
  await expect(page.getByRole("region", { name: "卫星频率表" })).toBeVisible();
});
