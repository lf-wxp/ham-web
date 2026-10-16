import { expect, stubApi, test, type Page } from "./fixtures";

/**
 * `/solar` 的 Kp 三日预报卡片。桩数据是 2026-10-15 从 NOAA SWPC 取回的真实原文（10-14 至 10-18，33 个时段，
 * 一行没改）；固定在样本有效期内，避免实时过期保护让历史夹具失效。
 * 期望值来自独立脚本（不复用页面或内核的任何逻辑）。
 */
test.beforeEach(async ({ page }) => {
  await page.clock.setFixedTime(new Date("2026-10-15T06:30:00Z"));
});

const POINTS = [
  {
    "time": "2026-10-14T00:00:00",
    "kp": 2.33,
    "kind": "observed"
  },
  {
    "time": "2026-10-14T03:00:00",
    "kp": 3.33,
    "kind": "observed"
  },
  {
    "time": "2026-10-14T06:00:00",
    "kp": 1.67,
    "kind": "observed"
  },
  {
    "time": "2026-10-14T09:00:00",
    "kp": 2.33,
    "kind": "observed"
  },
  {
    "time": "2026-10-14T12:00:00",
    "kp": 2.0,
    "kind": "observed"
  },
  {
    "time": "2026-10-14T15:00:00",
    "kp": 2.0,
    "kind": "observed"
  },
  {
    "time": "2026-10-14T18:00:00",
    "kp": 2.67,
    "kind": "observed"
  },
  {
    "time": "2026-10-14T21:00:00",
    "kp": 2.67,
    "kind": "observed"
  },
  {
    "time": "2026-10-15T00:00:00",
    "kp": 2.0,
    "kind": "observed"
  },
  {
    "time": "2026-10-15T03:00:00",
    "kp": 0.67,
    "kind": "observed"
  },
  {
    "time": "2026-10-15T06:00:00",
    "kp": 1.33,
    "kind": "observed"
  },
  {
    "time": "2026-10-15T09:00:00",
    "kp": 2.33,
    "kind": "estimated"
  },
  {
    "time": "2026-10-15T12:00:00",
    "kp": 1.67,
    "kind": "estimated"
  },
  {
    "time": "2026-10-15T15:00:00",
    "kp": 1.0,
    "kind": "estimated"
  },
  {
    "time": "2026-10-15T18:00:00",
    "kp": 1.33,
    "kind": "estimated"
  },
  {
    "time": "2026-10-15T21:00:00",
    "kp": 2.33,
    "kind": "estimated"
  },
  {
    "time": "2026-10-16T00:00:00",
    "kp": 2.33,
    "kind": "predicted"
  },
  {
    "time": "2026-10-16T03:00:00",
    "kp": 2.67,
    "kind": "predicted"
  },
  {
    "time": "2026-10-16T06:00:00",
    "kp": 2.0,
    "kind": "predicted"
  },
  {
    "time": "2026-10-16T09:00:00",
    "kp": 1.67,
    "kind": "predicted"
  },
  {
    "time": "2026-10-16T12:00:00",
    "kp": 1.67,
    "kind": "predicted"
  },
  {
    "time": "2026-10-16T15:00:00",
    "kp": 2.0,
    "kind": "predicted"
  },
  {
    "time": "2026-10-16T18:00:00",
    "kp": 2.0,
    "kind": "predicted"
  },
  {
    "time": "2026-10-16T21:00:00",
    "kp": 2.67,
    "kind": "predicted"
  },
  {
    "time": "2026-10-17T00:00:00",
    "kp": 1.67,
    "kind": "predicted"
  },
  {
    "time": "2026-10-17T03:00:00",
    "kp": 1.67,
    "kind": "predicted"
  },
  {
    "time": "2026-10-17T06:00:00",
    "kp": 1.33,
    "kind": "predicted"
  },
  {
    "time": "2026-10-17T09:00:00",
    "kp": 1.33,
    "kind": "predicted"
  },
  {
    "time": "2026-10-17T12:00:00",
    "kp": 1.33,
    "kind": "predicted"
  },
  {
    "time": "2026-10-17T15:00:00",
    "kp": 1.33,
    "kind": "predicted"
  },
  {
    "time": "2026-10-17T18:00:00",
    "kp": 1.33,
    "kind": "predicted"
  },
  {
    "time": "2026-10-17T21:00:00",
    "kp": 1.67,
    "kind": "predicted"
  },
  {
    "time": "2026-10-18T00:00:00",
    "kp": 1.67,
    "kind": "predicted"
  }
] as const;

const forecast = (points: readonly object[]) => ({
  source: "NOAA SWPC",
  updated: "2026-10-15T06:00:00",
  points,
});

function card(page: Page) {
  return page.locator("section").filter({ has: page.getByRole("heading", { name: "Kp 三日预报" }) });
}

/** 一行的各格文本（行标题 + 四天）；格内含给读屏的「实测 / 估计 / 预报」说明。 */
function cells(page: Page, slot: string) {
  return card(page)
    .getByRole("row")
    .filter({ has: page.getByRole("rowheader", { name: slot }) })
    .getByRole("cell");
}

test("太阳数据：Kp 三日预报按日 × 时段排好，区分实测 / 估计 / 预报，并给出预报期最高值", async ({ page }) => {
  await stubApi(page, { "/api/kp-forecast": forecast(POINTS) });
  await page.goto("/solar");
  const sec = card(page);
  await expect(sec).toBeVisible();

  // 「今天」是第一个非实测时段所在的天（10-15）：展示昨天到后天四列，10-18 那条孤零零的 00:00 不展示。
  const table = sec.getByRole("table", { name: "Kp 三日预报表" });
  await expect(table.getByRole("columnheader")).toHaveText(["时段（UTC）", "10-14", "10-15", "10-16", "10-17"]);
  await expect(sec.getByText("10-18")).toHaveCount(0);
  await expect(table.getByRole("row")).toHaveCount(9); // 表头 + 8 个时段

  // 03–06 一行：10-14 实测 3.33、10-15 实测 0.67、10-16 预报 2.67、10-17 预报 1.67。
  await expect(cells(page, "03–06")).toHaveText(["3.33 实测", "0.67 实测", "2.67 预报", "1.67 预报"]);
  // 09–12 一行：10-15 起这一格还是「估计」（当天尚未定稿）。
  await expect(cells(page, "09–12")).toHaveText(["2.33 实测", "2.33 估计", "1.67 预报", "1.33 预报"]);
  // 21–00 一行（上游 time_tag 是时段起点 21:00）。
  await expect(cells(page, "21–00")).toHaveText(["2.67 实测", "2.33 估计", "2.67 预报", "1.67 预报"]);

  // 预报期最高：10-16 的 03–06 与 21–00 都是 2.67，取最早的；10-14 实测的 3.33 更高，但那是历史。
  await expect(sec.getByText("预报期内最高 Kp 2.67（10-16 03–06 UTC），低于 NOAA 磁暴标度（G1 起算）。")).toBeVisible();
  await expect(sec.getByText("Kp 更新于 2026-10-15 06:00 UTC")).toBeVisible();
  await expect(table.getByText(/G\d/)).toHaveCount(0); // 表格里没有任何磁暴格（摘要与图例里的「G1」是文字说明）
});

test("太阳数据：预报里出现磁暴时标出 G 级与名称，实测里的磁暴不当成预报", async ({ page }) => {
  // 把 10-17 的 06–09 改成 6.67（= 7-，NOAA 归 G3），再把 10-14 的一格实测改成 7.0。
  const edited = POINTS.map((p) =>
    p.time === "2026-10-17T06:00:00" ? { ...p, kp: 6.67 } : p.time === "2026-10-14T03:00:00" ? { ...p, kp: 7.0 } : p,
  );
  await stubApi(page, { "/api/kp-forecast": forecast(edited) });
  await page.goto("/solar");
  const sec = card(page);

  await expect(sec.getByText("预报期内最高 Kp 6.67（10-17 06–09 UTC），达到 G3 级（强）。")).toBeVisible();
  // 磁暴格自带 G 字样（不只靠颜色）；两格：预报的 G3 和实测的 G3。
  await expect(cells(page, "06–09")).toHaveText(["1.67 实测", "1.33 实测", "2.00 预报", "6.67 G3 预报"]);
  await expect(cells(page, "03–06").first()).toHaveText("7.00 G3 实测");
  await expect(sec.getByRole("table").getByText(/G3/)).toHaveCount(2); // 表格里恰好两格：预报的与实测的
  await expect(sec.getByText(/达到 G3 级（强）/)).toHaveCount(1); // 摘要一处
});

test("太阳数据：只有实测、没有预报时说明情况，不编造预报期最高值", async ({ page }) => {
  await stubApi(page, {
    "/api/kp-forecast": forecast(POINTS.filter((p) => p.kind === "observed")),
  });
  await page.goto("/solar");
  const sec = card(page);
  await expect(sec.getByText("数据里没有预报时段，下面只有实测值。")).toBeVisible();
  await expect(sec.getByText(/预报期内最高/)).toHaveCount(0);
  await expect(sec.getByRole("table")).toBeVisible();
});

test("太阳数据：Kp 预报接口不可用时给出降级提示，其余内容照常", async ({ page }) => {
  await stubApi(page, { "/api/solar": { solar_flux: 123, a_index: 5, k_index: 1,
    sunspots: 42, xray: "A1.0", updated: "2026-10-15T06:00:00", conditions: [] } });
  await page.goto("/solar");
  await expect(card(page).getByText("预报暂时取不到（可能因网络受限），稍后重试。")).toBeVisible();
  await expect(page.getByRole("heading", { name: "太阳活动指数" })).toBeVisible();
  await expect(page.getByText("123.0", { exact: true })).toBeVisible();
});
