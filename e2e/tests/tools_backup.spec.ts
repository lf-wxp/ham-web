import { readFile } from "node:fs/promises";
import { expect, test } from "./fixtures";

/** 定位某个工具的 section（导航 / 锚点用的 id 与 section 的 id 一致）。 */
function tool(page: import("@playwright/test").Page, id: string) {
  return page.locator(`section#${id}`);
}

test("小工具：频率 ↔ 波长双向换算", async ({ page }) => {
  await page.goto("/tools");
  await expect(page.getByRole("heading", { level: 1, name: "小工具" })).toBeVisible();
  const section = tool(page, "freq-wavelength");
  await expect(section).toBeVisible();

  await section.getByLabel("频率（MHz）").fill("100");
  await expect(section.getByText(/100\.00 MHz ≈ 3\.00 m/)).toBeVisible();

  // 反向：填波长反推频率
  await section.getByLabel("波长（m）").fill("2");
  await expect(section.getByText(/150\.00 MHz ≈ 2\.00 m/)).toBeVisible();
});

test("小工具：驻波比换算反射系数", async ({ page }) => {
  await page.goto("/tools#swr");
  const section = tool(page, "swr");
  await expect(section).toBeVisible();

  await section.getByLabel("驻波比 SWR").fill("3");
  await expect(section.getByText(/反射系数 \|Γ\| = 0\.5000/)).toBeVisible();
  // 文案在 i18n 重构里改成了「回波损耗取正值为 {} dB」，这里只钉数值
  await expect(section.getByText(/回波损耗.*6\.02 dB/)).toBeVisible();
});

test("小工具：欧姆定律填两项算第三项", async ({ page }) => {
  await page.goto("/tools#ohms-law");
  const section = tool(page, "ohms-law");
  await expect(section).toBeVisible();

  // 未填够两项时给出引导
  await expect(section.getByText("填写任意两项（U、I、R）后自动计算。")).toBeVisible();

  await section.getByLabel("电压 U（V）").fill("12");
  await section.getByLabel("电流 I（A）").fill("2");
  await expect(section.getByText(/R = 6\.00/)).toBeVisible();
  await expect(section.getByText(/24\.00/)).toBeVisible();
});

test("小工具：菲涅尔区净空判定随障碍物高度变化", async ({ page }) => {
  await page.goto("/tools#fresnel-zone");
  const section = tool(page, "fresnel-zone");
  await expect(section).toBeVisible();

  // 默认：145 MHz、30 km、两端 100 m、中点无障碍，只剩地球曲率 → 净空达标
  await expect(section.getByText(/净空达标/)).toBeVisible();
  await expect(section.getByText(/地球曲率隆起 13\.24 m/)).toBeVisible();
  await expect(section.getByText(/占第一菲涅尔区半径的 70%/)).toBeVisible();

  // 抬高障碍物：先侵入第一菲涅尔区，再高过视线
  await section.getByLabel("障碍物顶高度（m）").fill("40");
  await expect(section.getByText(/部分遮挡/)).toBeVisible();
  await expect(section.getByText(/还差 .* m/)).toBeVisible();

  await section.getByLabel("障碍物顶高度（m）").fill("200");
  await expect(section.getByText(/视线被挡/)).toBeVisible();

  // 障碍物放到端点上不合法：给出引导而不是算出无穷大
  await section.getByLabel("障碍物距 A 端（km）").fill("30");
  await expect(section.getByText(/障碍物须位于 A、B 之间/)).toBeVisible();
});

test("小工具：路径剖面找出最危险的点并给出抬高建议", async ({ page }) => {
  await page.goto("/tools#path-profile");
  const section = tool(page, "path-profile");
  await expect(section).toBeVisible();
  const points = section.getByLabel("剖面采样点");

  // 示例剖面：310 m 的山脊挡在 15 km 处
  await expect(section.getByText(/视线被挡/)).toBeVisible();
  await expect(section.getByText(/距 A 端 15\.00 km，海拔 310\.00 m/)).toBeVisible();
  await expect(section.getByText(/两端天线同时抬高/)).toBeVisible();
  await expect(section.getByRole("img", { name: "路径剖面图" })).toBeVisible();
  // 剖面表：7 个点 + 表头
  await expect(section.getByRole("row")).toHaveCount(8);

  // 改成平地 + 100 m 天线：与菲涅尔区工具的默认结论一致（净空达标、约 70%）
  await points.fill("0, 0\n15, 0\n30, 0");
  await section.getByLabel("天线 A 高度（m）").fill("100");
  await section.getByLabel("天线 B 高度（m）").fill("100");
  await expect(section.getByText(/净空达标/)).toBeVisible();
  await expect(section.getByRole("cell", { name: "70%", exact: true })).toBeVisible();
  await expect(section.getByRole("row")).toHaveCount(4);
  await expect(section.getByText(/距 60% 净空线仍有 .* m 余量/)).toBeVisible();

  // 输入不合法时给出带行号的引导，而不是算出奇怪的数
  await points.fill("0, 0\n5\n10, 0");
  await expect(section.getByRole("alert")).toContainText("第 2 行");
  await points.fill("0, 0\n10, 5");
  await expect(section.getByRole("alert")).toContainText("至少需要 3 个点");
  await points.fill("0, 0\n10, 0\n10, 0");
  await expect(section.getByRole("alert")).toContainText("第 3 个点的距离必须大于前一个点");
});

test("小工具：磁环绕线由目标电感反推匝数，向上取整并给出公差与窗口提示", async ({ page }) => {
  await page.goto("/tools#toroid-winding");
  const section = tool(page, "toroid-winding");
  await expect(section).toBeVisible();

  // 默认磁环 5943000301（AL 440 nH/匝²，±20%），目标 50 µH：
  // N = √(1000·50/440) = 10.66 → 11 匝；L = 440·11²/1000 = 53.24 µH；±20% → 42.59 – 63.89。
  await expect(section.getByText("5943000301 · 43 材料")).toBeVisible();
  await expect(section.getByRole("status")).toContainText("需要绕 11 匝（计算值 10.66");
  await expect(section.getByText(/实际约 53\.24 µH.*42\.59 – 63\.89 µH/)).toBeVisible();
  // 线径 0.6 mm、内径 7.15 mm → 内圈周长 22.46 mm → 单层约 37 匝：11 匝放得下
  await expect(section.getByText(/内圈单层最多约 37 匝：放得下/)).toBeVisible();

  // 目标 1000 µH → 48 匝 > 37：提示放不下
  await section.getByLabel("目标电感（µH）").fill("1000");
  await expect(section.getByRole("status")).toContainText("需要绕 48 匝");
  await expect(section.getByText(/内圈单层最多约 37 匝：放不下/)).toBeVisible();

  // 换成大磁环（AL 1075，内径 35.55 mm）：同样 1000 µH 只要 31 匝，内圈放得下
  await section.getByRole("combobox", { name: "磁环" }).click();
  await page.getByRole("option", { name: /5943003801/ }).click();
  await expect(section.getByRole("status")).toContainText("需要绕 31 匝");
  await expect(section.getByText(/放得下/)).toBeVisible();

  // 自定义：自己填 AL，不再判断窗口
  await section.getByRole("combobox", { name: "磁环" }).click();
  await page.getByRole("option", { name: /自定义/ }).click();
  await section.getByLabel("AL（nH/匝²）").fill("100");
  await section.getByLabel("目标电感（µH）").fill("1");
  // N = √(1000·1/100) = 3.16 → 4 匝
  await expect(section.getByRole("status")).toContainText("需要绕 4 匝");
  await expect(section.getByText(/自定义磁环没有尺寸/)).toBeVisible();

  // 目标电感不合法：给出引导而不是算出奇怪的数
  await section.getByLabel("目标电感（µH）").fill("0");
  await expect(section.getByText(/请输入有效的 AL/)).toBeVisible();
});

// `/tools` 里有很多带表格 / 图表的块，表格自带的 `min-w-[…]` 放进网格子项里会把整列撑宽（需要 `min-w-0`），
// 手机宽度下整页横向滚动。i18n_layout 只在桌面宽度量，所以这里单独守一次。
for (const [code, name] of [
  ["zh", "中文"],
  ["en", "English"],
  ["es", "Español"],
] as const) {
  test(`小工具：手机宽度下整页不产生横向滚动（${name}）`, async ({ page }) => {
    await page.addInitScript((c) => localStorage.setItem("locale", c), code);
    for (const width of [320, 390]) {
      await page.setViewportSize({ width, height: 900 });
      await page.goto("/tools");
      await expect(page.locator("section#path-profile")).toBeVisible();
      await expect(page.locator("section#toroid-winding")).toBeVisible();
      await expect(page.locator("section#intermod")).toBeVisible();
      await expect(page.locator("section#parabolic-dish")).toBeVisible();
      const overflow = await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth);
      expect(overflow, `${width}px`).toBeLessThanOrEqual(1);
    }
  });
}

test("小工具：互调与镜频列出产物并标出落入业余波段的", async ({ page }) => {
  await page.goto("/tools#intermod");
  const section = tool(page, "intermod");
  await expect(section).toBeVisible();
  const freqs = section.getByLabel("发射频率（MHz）");

  // 默认 14.200 / 14.205：谐波 8 + 2 阶 2 + 3 阶 4 = 14 个产物，其中 5 个在业余波段内
  // （28.4、28.41、28.405 在 10m；14.195、14.21 在 20m）
  await expect(section.getByRole("status")).toContainText("共 14 个产物");
  await expect(section.getByRole("status")).toContainText("其中 5 个落入业余波段");
  await expect(section.getByRole("row")).toHaveCount(6); // 表头 + 5
  const low = section.getByRole("row", { name: /2·f1 − f2/ });
  await expect(low).toContainText("14.195");
  await expect(low).toContainText("20m");
  // 表达式单元格精确匹配：`f1 + f2` 不能同时命中 `2·f1 + f2`
  const rowOf = (expr: string) =>
    section.getByRole("row").filter({ has: page.getByRole("cell", { name: expr, exact: true }) });
  await expect(rowOf("f1 + f2")).toContainText("28.405");
  // 差频 0.005 与 42.6 不在业余波段：默认被过滤掉
  await expect(section.getByRole("cell", { name: "0.005", exact: true })).toHaveCount(0);

  // 取消过滤：全部 14 个都显示，不在业余波段的标 —
  await section.getByRole("checkbox", { name: "只显示落入业余波段的产物" }).click();
  await expect(section.getByRole("row")).toHaveCount(15);
  await expect(rowOf("f2 − f1")).toContainText("0.005");

  // 3.65 MHz 的谐波（7.3 / 10.95 / 14.6 / 18.25）全部落空：过滤时给出说明而不是空表
  await section.getByRole("checkbox", { name: "只显示落入业余波段的产物" }).click();
  await freqs.fill("3.65");
  await expect(section.getByText(/没有产物落入业余波段/)).toBeVisible();
  await expect(section.getByRole("table")).toHaveCount(0);

  // 输入不合法：给出带序号的引导
  await freqs.fill("14.2, abc");
  await expect(section.getByRole("alert")).toContainText("第 2 个值不是正数");
  await freqs.fill("");
  await expect(section.getByRole("alert")).toContainText("请至少输入一个频率");
  await freqs.fill("1 2 3 4 5 6 7 8 9");
  await expect(section.getByRole("alert")).toContainText("最多支持 8 个不同的频率");
  // 重复的频率不占名额
  await freqs.fill("1 2 3 4 5 6 7 8 8 8");
  await expect(section.getByRole("alert")).toHaveCount(0);
});

test("小工具：镜频按高端 / 低端本振给出两侧并判断是否落入业余波段", async ({ page }) => {
  await page.goto("/tools#intermod");
  const section = tool(page, "intermod");
  const rx = section.getByLabel("接收频率（MHz）");
  const ifm = section.getByLabel("中频（MHz）");

  // 默认 145 MHz / 10.7 MHz：166.4 与 123.6，都不在业余波段
  await expect(section.getByText(/高端本振.*镜频 166\.4 MHz，不在业余波段内/)).toBeVisible();
  await expect(section.getByText(/低端本振.*镜频 123\.6 MHz，不在业余波段内/)).toBeVisible();

  // 14.1 MHz、中频 3.5：21.1（15m）与 7.1（40m）
  await rx.fill("14.1");
  await ifm.fill("3.5");
  await expect(section.getByText(/镜频 21\.1 MHz，落入 15m 波段/)).toBeVisible();
  await expect(section.getByText(/镜频 7\.1 MHz，落入 40m 波段/)).toBeVisible();

  // 中频太高：低端镜频为负，没有这一侧
  await rx.fill("7.1");
  await ifm.fill("4");
  await expect(section.getByText(/镜频 15\.1 MHz/)).toBeVisible();
  await expect(section.getByText(/中频过高，镜频为负/)).toBeVisible();

  // 不合法
  await ifm.fill("0");
  await expect(section.getByText(/请输入有效的接收频率与中频/)).toBeVisible();
});

test("小工具：抛物面天线增益、波束与焦点几何", async ({ page }) => {
  await page.goto("/tools#parabolic-dish");
  const section = tool(page, "parabolic-dish");
  await expect(section).toBeVisible();

  // 默认值在输入框里要显示成干净的数字（0.55 × 100 的浮点误差曾显示成 55.00000000000001）
  await expect(section.getByLabel("孔径效率（%）")).toHaveValue("55");
  await expect(section.getByLabel("焦径比 f/D")).toHaveValue("0.4");
  // 默认 10.368 GHz / 0.9 m / 55% / f/D 0.4（预期值由独立脚本算出）：
  // λ = 0.0289 m，口径 31.13 个波长；G = 0.55·(π·D/λ)² = 5258.9 倍 = 37.21 dBi；A_e = 0.3499 m²
  await expect(section.getByText(/波长 0\.0289 m　｜　口径 31\.13 个波长/)).toBeVisible();
  await expect(section.getByRole("status")).toContainText("增益 37.21 dBi（5258.9 倍）");
  await expect(section.getByRole("status")).toContainText("有效面积 0.3499 m²");
  await expect(section.getByText(/波束宽度约 2\.25°/)).toBeVisible();
  // f = 0.36 m，深 0.1406 m，焦点在口径平面前 0.2194 m，张角 128.02°
  await expect(section.getByText(/焦距 0\.3600 m　｜　凹陷深度 0\.1406 m/)).toBeVisible();
  await expect(section.getByText(/口径平面前方 0\.2194 m 处/)).toBeVisible();
  await expect(section.getByText(/全张角 128\.02°/)).toBeVisible();
  await expect(section.getByText(/增益公式在这里不再可靠/)).toHaveCount(0);

  // 口径翻倍：增益 +6.02 dB、波束减半
  await section.getByLabel("口径（m）").fill("1.8");
  await expect(section.getByRole("status")).toContainText("增益 43.23 dBi");
  await expect(section.getByText(/波束宽度约 1\.12°/)).toBeVisible();

  // f/D = 0.25：焦点恰在口径平面上，张角 180°
  await section.getByLabel("口径（m）").fill("0.9");
  await section.getByLabel("焦径比 f/D").fill("0.25");
  await expect(section.getByText(/焦点恰在口径平面上/)).toBeVisible();
  await expect(section.getByText(/全张角 180\.00°/)).toBeVisible();

  // f/D = 0.2：深碗，焦点在碗内 0.1012 m，张角超过 180°
  await section.getByLabel("焦径比 f/D").fill("0.2");
  await expect(section.getByText(/焦点在碗内，距口径平面 0\.1012 m（深碗）/)).toBeVisible();
  await expect(section.getByText(/全张角 205\.36°/)).toBeVisible();

  // 效率翻倍到合法上限之外：给出引导
  await section.getByLabel("孔径效率（%）").fill("120");
  await expect(section.getByText(/请输入有效的频率、口径、焦径比/)).toBeVisible();
  await section.getByLabel("孔径效率（%）").fill("55");

  // 低频小口径：145 MHz、1 m，口径不到半个波长
  await section.getByLabel("频率（MHz）").fill("145");
  await section.getByLabel("口径（m）").fill("1");
  await section.getByLabel("焦径比 f/D").fill("0.4");
  await expect(section.getByText(/增益公式在这里不再可靠/)).toBeVisible();
});

test("小工具：抛物面天线用实测凹陷深度反推焦距", async ({ page }) => {
  await page.goto("/tools#parabolic-dish");
  const section = tool(page, "parabolic-dish");
  const depth = section.getByLabel("实测凹陷深度（m）");

  // 口径 0.9 m、深 0.15 m：f = 0.81/(16·0.15) = 0.3375 m，f/D = 0.375
  await expect(section.getByText(/焦距 0\.3375 m　｜　f\/D = 0\.3750/)).toBeVisible();
  // 深度 = D/4 = 0.225 → f/D = 0.25
  await depth.fill("0.225");
  await expect(section.getByText(/f\/D = 0\.2500/)).toBeVisible();

  await depth.fill("0");
  await expect(section.getByText(/请输入大于 0 的口径与深度/)).toBeVisible();
});

test("数据备份：导出 JSON 含本地数据", async ({ page }) => {
  await page.addInitScript(() => localStorage.setItem("e2e-backup-key", "e2e-value"));
  await page.goto("/tools#backup");
  await expect(page.getByRole("heading", { level: 2, name: "数据备份" })).toBeVisible();
  await expect(page.getByText("本地存储占用（约）")).toBeVisible();

  // 导出成功后有一个「已导出 N 项」提示（全局 Toast，`data-slot="toast"` 定位，
  // `/tools` 各工具的结果框也带 `role="status"`，直接按 role 取会撞 strict mode）。
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
  await page.goto("/tools#backup");
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
  await page.goto("/tools#backup");
  const section = page.locator("section#backup");
  await expect(page.getByRole("heading", { level: 2, name: "数据备份" })).toBeVisible();

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
  await page.goto("/tools#backup");
  const section = page.locator("section#backup");
  await section.getByLabel(/^通联数据/).uncheck();
  await section.getByLabel(/^QSL 影像（扫描件/).uncheck();
  await section.getByLabel(/^其他本地数据/).uncheck();
  await expect(section.getByRole("button", { name: "导出备份" })).toBeDisabled();
  await section.getByLabel(/^其他本地数据/).check();
  await expect(section.getByRole("button", { name: "导出备份" })).toBeEnabled();
});

test("数据备份：导入恢复与合并导入", async ({ page }) => {
  await page.goto("/tools#backup");
  await expect(page.getByRole("heading", { level: 2, name: "数据备份" })).toBeVisible();
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
  await page.goto("/tools#backup");
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
  await page.goto("/tools#backup");
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
  await page.goto("/tools#backup");
  await expect(page.getByRole("heading", { level: 2, name: "数据备份" })).toBeVisible();

  await page.locator('section#backup input[type="file"]').first().setInputFiles({
    name: "broken.json",
    mimeType: "application/json",
    buffer: Buffer.from("这不是 JSON"),
  });
  await expect(page.locator('[data-slot="toast"]')).toContainText(/^导入失败：/);
});
