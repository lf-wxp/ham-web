import { expect, seedLog, stubApi, test } from "./fixtures";

/**
 * 依赖 `/api/*` 实时数据的页面：接口全部打桩成固定数据，断言页面对数据的**处理**
 * （筛选、补全、格式化、降级），而不是上游此刻有什么。
 */

const SPOTS = [
  { spotter: "DL1AA", freq_khz: 14025, dx: "JA1ABC", country: "", comment: "CW 599", time: "1230Z" },
  { spotter: "W1AW", freq_khz: 7074, dx: "VK2XX", country: "Australia", comment: "FT8 -10", time: "1231Z" },
  { spotter: "G4ABC", freq_khz: 21200, dx: "PY2AA", country: "Brazil", comment: "SSB 59", time: "1232Z" },
];

test("DX 热点：按波段 / 模式筛选、对照日志标出需要的实体、点呼号记入日志", async ({ page }) => {
  // 日志里已经在 20m 联过日本：JA1ABC 应标「已联」，其余两个是新实体。
  await seedLog(page, [{ callsign: "JA1XYZ", band: "20m", freq: "14.074" }]);
  await stubApi(page, { "/api/spots": SPOTS });
  await page.goto("/dx-spots");

  // 呼号按钮的可访问名 = 呼号 + 国家（如「JA1ABC 日本」）。
  const rows = page.getByRole("button", { name: /^(JA1ABC|VK2XX|PY2AA) / });
  await expect(rows).toHaveCount(3);
  await expect(page.getByText("3 条", { exact: true })).toBeVisible();
  // 上游没给国家的那条，按呼号前缀从内置 DXCC 表补上。
  await expect(page.getByRole("button", { name: "JA1ABC 日本" })).toBeVisible();

  // 波段筛选：只剩 20m 那条；总数不随筛选变化。
  await page.getByRole("button", { name: "20m", exact: true }).click();
  await expect(page.getByRole("button", { name: "20m", exact: true })).toHaveAttribute("aria-pressed", "true");
  await expect(rows).toHaveCount(1);
  await expect(rows.first()).toContainText("JA1ABC");
  // 20m + FT8：没有这样的报告，给出空态而不是空白。
  await page.getByRole("button", { name: "FT8", exact: true }).click();
  await expect(page.getByText("当前筛选下暂无报告，试试其他波段或模式。")).toBeVisible();
  // 模式从注释推断：波段放开后 FT8 只命中 VK2XX。
  await page.getByRole("button", { name: "全部", exact: true }).first().click();
  await expect(rows).toHaveCount(1);
  await expect(rows.first()).toContainText("VK2XX");
  await page.getByRole("button", { name: "全部", exact: true }).nth(1).click();
  await expect(rows).toHaveCount(3);

  // 只看需要的：已联的 JA1ABC 被滤掉。
  await page.getByRole("button", { name: "只看需要的（2）" }).click();
  await expect(rows).toHaveCount(2);
  await expect(page.getByRole("button", { name: /^JA1ABC / })).toHaveCount(0);

  // 点呼号记入日志：之后 VK2XX 的实体也算联过了，「需要的」只剩 PY2AA。
  // 记入结果走全局 Toast（`role="status"`）。
  await page.getByRole("button", { name: /^VK2XX / }).click();
  await expect(page.getByRole("status")).toContainText(/已加入日志：VK2XX/);
  await expect(page.getByRole("button", { name: "只看需要的（1）" })).toBeVisible();
  await expect(rows).toHaveCount(1);
  await expect(rows.first()).toContainText("PY2AA");
});

test("DX 热点：接口不可用时给出降级提示", async ({ page }) => {
  await stubApi(page, {});
  await page.goto("/dx-spots");
  await expect(page.getByText("实时热点暂不可用（可能因网络受限），稍后重试。")).toBeVisible();
});

/** `YYYY-MM-DD HH:MM UTC`，与页面同一口径（秒级时间戳按 UTC 格式化）。 */
function utc(seconds: number): string {
  const iso = new Date(seconds * 1000).toISOString();
  return `${iso.slice(0, 10)} ${iso.slice(11, 16)} UTC`;
}

test("PSK Reporter：呼号规整后查询、报告表格式化、空结果与失败分开提示", async ({ page }) => {
  const asked: string[] = [];
  const t0 = 1_790_000_000;
  await stubApi(page, {
    "/api/psk-reporter": (url: URL) => {
      const call = url.searchParams.get("callsign") ?? "";
      asked.push(call);
      if (call === "BG4XXX") {
        return {
          reports: [
            { receiver: "JA1RX", freq_khz: 14074.5, band: "20m", mode: "FT8", snr: -12, flow_start_seconds: t0, receiver_locator: "PM95" },
            { receiver: "VK2RX", freq_khz: 7074.1, band: "40m", mode: "FT8", snr: 3, flow_start_seconds: t0 + 600, receiver_locator: "QF56" },
          ],
        };
      }
      return { reports: [] };
    },
  });
  await page.goto("/psk-reporter");

  // 空呼号：不发请求，直接 Toast 提示（`role="status"`）。
  await page.getByRole("textbox", { name: "呼号" }).fill("");
  await page.getByRole("button", { name: "查询" }).click();
  await expect(page.getByRole("status")).toContainText("请输入呼号");
  expect(asked).toHaveLength(0);

  // 小写 + 空白 → 大写去空格后再查。
  await page.getByRole("textbox", { name: "呼号" }).fill(" bg4xxx ");
  await page.getByRole("button", { name: "查询" }).click();
  await expect(page.getByRole("heading", { name: "接收报告（2 条）" })).toBeVisible();
  expect(asked).toEqual(["BG4XXX"]);
  const table = page.getByRole("table");
  await expect(table.getByText("14074.5 kHz")).toBeVisible();
  await expect(table.getByText("-12 dB")).toBeVisible();
  await expect(table.getByText("3 dB", { exact: true })).toBeVisible();
  await expect(table.getByText(utc(t0))).toBeVisible();
  await expect(table.getByText("PM95")).toBeVisible();

  // 查无记录与接口失败是两种提示。
  await page.getByRole("textbox", { name: "呼号" }).fill("BA1ZZZ");
  await page.getByRole("button", { name: "查询" }).click();
  await expect(page.getByText(/最近没有查询到该呼号的接收报告/)).toBeVisible();
  // 失败路径换一个**没查过**的呼号：成功的响应会进离线缓存（`api-cache:<url>`），
  // 同一个 URL 再失败时页面会用缓存兜底 —— 那是设计行为，不是这里要测的。
  await page.unroute("**/api/**");
  await stubApi(page, {});
  await page.getByRole("textbox", { name: "呼号" }).fill("BA2NEW");
  await page.getByRole("button", { name: "查询" }).click();
  await expect(page.getByText(/查询暂不可用/)).toBeVisible();
});

test("RBN：信标报告带 SNR 与速率，空结果给出说明", async ({ page }) => {
  await stubApi(page, {
    "/api/rbn": (url: URL) =>
      url.searchParams.get("callsign") === "BG4XXX"
        ? { spots: [{ spotter: "DK8NE", freq_khz: 14023.9, band: "20m", mode: "CW", snr: 18, wpm: 24 }] }
        : { spots: [] },
  });
  await page.goto("/rbn");
  await page.getByRole("textbox", { name: "呼号" }).fill("bg4xxx");
  await page.getByRole("button", { name: "查询" }).click();
  await expect(page.getByRole("heading", { name: "信标报告（1 条）" })).toBeVisible();
  const row = page.getByRole("row", { name: /DK8NE/ });
  await expect(row).toContainText("14023.9 kHz");
  await expect(row).toContainText("18 dB");
  await expect(row).toContainText("24");

  await page.getByRole("textbox", { name: "呼号" }).fill("BA1ZZZ");
  await page.getByRole("button", { name: "查询" }).click();
  await expect(page.getByText(/监听期间没有收到该呼号的信标报告/)).toBeVisible();
});

test("实时仪表盘：五张卡各自解析自己的接口，并链到详情页", async ({ page }) => {
  await stubApi(page, {
    "/api/solar": { k_index: 3, solar_flux: 152, sunspots: 98 },
    "/api/alerts": [{ level: "WATCH", product_id: "A20F" }],
    "/api/spots": SPOTS,
    "/api/iss": { latitude: 39.9123, longitude: 116.4049, altitude: 420, velocity: 7.66, visibility: "daylight" },
    "/api/most-wanted": { entries: [{ rank: 1, adif: 246 }, { rank: 2, adif: 24 }] },
  });
  await page.goto("/dashboard");

  const card = (href: string, title: string) => page.locator(`a[href="${href}"]`, { hasText: title });
  const solar = card("/solar", "太阳活动");
  await expect(solar).toContainText("152");
  await expect(solar).toContainText("98");
  await expect(card("/solar", "空间天气警报")).toContainText("WATCH");
  const spots = card("/dx-spots", "DX 实时热点");
  await expect(spots).toContainText("JA1ABC");
  await expect(spots).toContainText("14.025 MHz");
  await expect(card("/satellites", "ISS")).toContainText("39.91°, 116.40°");
  const wanted = card("/most-wanted", "DXCC 最稀有");
  await expect(wanted).toContainText("#1");
  await expect(wanted).toContainText("DXCC 246");
});

test("实时仪表盘：接口全挂时各卡降级而不是整页报错", async ({ page }) => {
  await stubApi(page, {});
  await page.goto("/dashboard");
  await expect(page.locator('a[href="/dx-spots"]')).toContainText("暂不可用");
  await expect(page.locator('a[href="/satellites"]')).toContainText("暂不可用");
  await expect(page.locator("#fatal")).toHaveCount(0);
});
