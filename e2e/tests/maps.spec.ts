import { expect, seedLog, stubApi, test } from "./fixtures";

/** 地图类页面。地图本体是 `<svg role="img">`，世界内容横向平铺三份，标记计数要按「至少」断言。 */

test("网格地图：查询网格中心、反查地名、统计日志里的网格", async ({ page }) => {
  await seedLog(page, [
    { callsign: "JA1AA", gridsquare: "PM95" },
    { callsign: "DL1AA", gridsquare: "JO62" },
    { callsign: "DL2AA", gridsquare: "JO62" },
  ]);
  const geocoded: string[] = [];
  await stubApi(page, {
    "/api/geocode": (url: URL) => {
      geocoded.push(url.search);
      return { country: "中国", city: "北京" };
    },
  });
  await page.goto("/grid-map");
  // 两条 JO62 去重后是 2 个网格。
  await expect(page.getByText("已通联 2 个网格")).toBeVisible();

  const input = page.getByRole("textbox", { name: "输入 Maidenhead 网格码" });
  await expect(page.getByText("输入网格码（≥4 位，如 OM89）查询其中心经纬度。")).toBeVisible();
  await input.fill("om8");
  await expect(page.getByText("无效网格码：需 4 或 6 位（如 OM89 / OM89EW）。")).toBeVisible();
  // 小写自动转大写；中心坐标是网格的几何中心，反查结果拼在后面。
  await input.fill("om89");
  await expect(input).toHaveValue("OM89");
  await expect(page.getByText(/OM89 中心：纬度 39\.5000°，经度 117\.0000°.*中国 · 北京/)).toBeVisible();
  expect(geocoded.at(-1)).toBe("?lat=39.5&lon=117");
});

test("灰线地图：时间偏移联动时钟、滑块与晨昏圈", async ({ page }) => {
  await page.clock.setFixedTime(new Date("2026-03-20T12:00:00Z"));
  await page.goto("/grayline");
  const map = page.getByRole("img", { name: /^灰线地图/ });
  await expect(map).toBeVisible();
  await expect(page.getByText("12:00:00 UTC")).toBeVisible();
  await expect(page.getByText("当前时间")).toBeVisible();
  const before = await map.innerHTML();

  await page.getByRole("radio", { name: "+6h" }).click();
  await expect(page.getByRole("radio", { name: "+6h" })).toHaveAttribute("aria-checked", "true");
  await expect(page.getByText("18:00:00 UTC")).toBeVisible();
  await expect(page.getByText("偏移 +6.0h")).toBeVisible();
  const slider = page.getByRole("slider", { name: "时间偏移（小时）" });
  await expect(slider).toHaveAttribute("aria-valuenow", "6");
  // 晨昏圈与太阳直射点跟着时间走：地图内容必须变化。
  await expect.poll(() => map.innerHTML()).not.toBe(before);

  // 回到现在：时钟与地图都回到原样。
  await page.getByRole("radio", { name: "现在" }).click();
  await expect(page.getByText("12:00:00 UTC")).toBeVisible();
  await expect.poll(() => map.innerHTML()).toBe(before);
});

test("分区号地图：按分区号着色，选一个分区只留下它", async ({ page }) => {
  await page.goto("/reference");
  const map = page.getByRole("img", { name: /^中国业余电台分区号地图/ });
  await expect(map).toBeVisible();
  const all = page.getByRole("radio", { name: "全部" });
  const zone3 = page.getByRole("radio", { name: "3", exact: true });

  // 31 个省级行政区各有一块（多环的省份会画成多个 polygon，所以按「至少」断言）；
  // 每个省两个文本：省名 + 分区号。
  await expect.poll(() => map.locator("polygon").count()).toBeGreaterThanOrEqual(31);
  await expect(map.locator("text")).toHaveCount(62);
  await expect(map.getByText("内蒙古", { exact: true })).toHaveCount(1);
  await expect(map.getByText("北京", { exact: true })).toHaveCount(1);
  // 默认「全部」：没有选区，每块都是同一个状态；每块都带自己的分区号。
  await expect(all).toHaveAttribute("aria-checked", "true");
  await expect(map.locator('polygon:not([data-state="all"])')).toHaveCount(0);
  await expect(map.locator("polygon:not([data-area])")).toHaveCount(0);

  // 选 3 区：天津 · 河北 · 内蒙古 · 山西 亮起，其余淡出。
  await zone3.click();
  await expect(zone3).toHaveAttribute("aria-checked", "true");
  await expect(map.locator('polygon[data-state="selected"]').first()).toBeAttached();
  await expect(map.locator('polygon[data-state="dimmed"]').first()).toBeAttached();
  await expect(map.locator('polygon[data-state="all"]')).toHaveCount(0);
  // 亮起的全部属于 3 区，3 区的全部都亮起：不多不少。
  await expect(map.locator('polygon[data-state="selected"]:not([data-area="3"])')).toHaveCount(0);
  await expect(map.locator('polygon[data-area="3"]:not([data-state="selected"])')).toHaveCount(0);

  // 「全部」清掉选区，回到全图。
  await all.click();
  await expect(all).toHaveAttribute("aria-checked", "true");
  await expect(map.locator('polygon:not([data-state="all"])')).toHaveCount(0);
});

test("分区号地图：点省份选中它所属的分区，再点取消", async ({ page }) => {
  await page.goto("/reference");
  const map = page.getByRole("img", { name: /^中国业余电台分区号地图/ });
  await expect(map).toBeVisible();
  const all = page.getByRole("radio", { name: "全部" });
  const zone3 = page.getByRole("radio", { name: "3", exact: true });
  await expect(all).toHaveAttribute("aria-checked", "true");

  // 多边形互相相邻，真实鼠标点会落到最上层的邻省上：直接对目标元素派发 click。
  await map.locator('polygon[data-area="3"]').first().dispatchEvent("click");
  await expect(zone3).toHaveAttribute("aria-checked", "true");
  await expect(map.locator('polygon[data-state="selected"]').first()).toBeAttached();

  await map.locator('polygon[data-area="3"]').first().dispatchEvent("click");
  await expect(all).toHaveAttribute("aria-checked", "true");
  await expect(map.locator('polygon:not([data-state="all"])')).toHaveCount(0);

  // 悬停提示（`<title>`）是「省名（N 区）」。
  await expect(map.locator("polygon title", { hasText: "（3 区）" }).first()).toBeAttached();
});

test("分区号地图：选中的省画在淡出省的标签之上", async ({ page }) => {
  await page.goto("/reference");
  const map = page.getByRole("img", { name: /^中国业余电台分区号地图/ });
  await expect(map).toBeVisible();
  await expect.poll(() => map.locator("polygon").count()).toBeGreaterThanOrEqual(31);

  // 北京（1 区）的标注点落在河北（3 区）上：选 3 区后，淡出的「北京」标签必须在文档里排在
  // 所有选中多边形之前（SVG 后画的盖住先画的），否则它会浮在被点亮的河北之上、读起来像河北的字。
  await page.getByRole("radio", { name: "3", exact: true }).click();
  const order = await map.evaluate((svg) => {
    const nodes = [...svg.querySelectorAll("polygon, text[data-label]")];
    const firstSelected = nodes.findIndex(
      (n) => n.tagName.toLowerCase() === "polygon" && (n as SVGElement).dataset.state === "selected",
    );
    const beijing = nodes.findIndex((n) => (n as SVGElement).dataset.label === "BJ");
    const hebei = nodes.findIndex((n) => (n as SVGElement).dataset.label === "HE");
    return { firstSelected, beijing, hebei };
  });
  expect(order.firstSelected).toBeGreaterThan(-1);
  expect(order.beijing).toBeGreaterThan(-1);
  expect(order.beijing).toBeLessThan(order.firstSelected);
  // 选中省自己的标签仍在多边形之后，读得出来。
  expect(order.hebei).toBeGreaterThan(order.firstSelected);
});

test("SOTA / POTA 地图：按日志里的编号逐个定位，查不到的跳过", async ({ page }) => {
  await seedLog(page, [
    { callsign: "W6AA", my_sota_ref: "w6/ct-001" },
    { callsign: "K1AA", my_pota_ref: "K-0001" },
    // 重复编号只查一次；查不到的编号不计入。
    { callsign: "W6BB", my_sota_ref: "W6/CT-001" },
    { callsign: "K2AA", my_pota_ref: "K-9999" },
  ]);
  const asked: string[] = [];
  await stubApi(page, {
    "/api/sota": (url: URL) => {
      asked.push(`sota ${url.searchParams.get("ref")}`);
      return { reference: "W6/CT-001", name: "San Jacinto", latitude: 33.81, longitude: -116.68 };
    },
    "/api/pota": (url: URL) => {
      const r = url.searchParams.get("ref");
      asked.push(`pota ${r}`);
      if (r !== "K-0001") {
        // 打桩函数只能返回 200；用缺必填字段的响应模拟「查不到」（页面按解析失败跳过）。
        return { reference: r };
      }
      return { reference: "K-0001", name: "Acadia", latitude: 44.35, longitude: -68.21 };
    },
  });
  await page.goto("/portable-map");
  await expect(page.getByText("已激活 2 个")).toBeVisible();
  expect(asked.sort()).toEqual(["pota K-0001", "pota K-9999", "sota W6/CT-001"]);
  await expect(page.locator("circle title", { hasText: "W6/CT-001 · San Jacinto" }).first()).toBeAttached();
  await expect(page.locator("circle title", { hasText: "K-0001 · Acadia" }).first()).toBeAttached();
});

test("SOTA / POTA 地图：日志里没有编号时给出指引", async ({ page }) => {
  await stubApi(page, {});
  await page.goto("/portable-map");
  await expect(page.getByText(/暂无 SOTA \/ POTA 激活记录/)).toBeVisible();
});

test("SDR 在线接收站地图：定位到接收站并给出外链", async ({ page }) => {
  await page.goto("/sdr-map");
  const map = page.getByRole("img", { name: /^世界地图/ });
  await expect(map).toBeVisible();
  await expect(page.locator("circle title", { hasText: "特温特大学 WebSDR" }).first()).toBeAttached();

  const locate = page.getByRole("button", { name: "定位" });
  await expect(locate).toHaveCount(2);
  const view0 = await map.getAttribute("viewBox");
  await locate.first().click();
  await expect.poll(() => map.getAttribute("viewBox")).not.toBe(view0);
  const view1 = await map.getAttribute("viewBox");
  // 换一个站：视图再变（两站一个在荷兰、一个在加州）。
  await locate.nth(1).click();
  await expect.poll(() => map.getAttribute("viewBox")).not.toBe(view1);

  // 外链在新标签页打开且不泄露 opener。
  const open = page.getByRole("link", { name: "打开接收机" }).first();
  await expect(open).toHaveAttribute("target", "_blank");
  await expect(open).toHaveAttribute("rel", /noopener/);
});
