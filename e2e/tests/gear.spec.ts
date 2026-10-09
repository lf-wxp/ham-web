import { expect, test } from "./fixtures";

/** 参数对比那一节（含雷达图、得分、录入区、对比表）。 */
function panel(page: import("@playwright/test").Page) {
  return page.locator("section", { hasText: "参数对比" });
}

/** 读取本机实测值（只存 localStorage）。 */
function stored(page: import("@playwright/test").Page) {
  return page.evaluate(() => window.localStorage.getItem("gear-measurements") ?? "");
}

/**
 * 实测值只存本地，用例之间不该互相污染：失败在「录了 62」那一步时，残留会让后面用例
 * 的「对比表是原表值」类断言直接变红，掩盖真正的原因。
 */
test.afterEach(async ({ page }) => {
  await page
    .evaluate(() => window.localStorage.removeItem("gear-measurements"))
    .catch(() => undefined);
});

test("机型库：接收机实测值带出处、差异高亮标出该行最好", async ({ page }) => {
  await page.goto("/gear");
  const p = panel(page);

  // 默认类别（HF / 全段收发信机）默认选中前 3 台，其中 FT-710 在 Sherwood 表里有实测值。
  await expect(p.getByRole("rowheader", { name: "20 kHz 间隔三阶互调动态范围" })).toBeVisible();
  const imd = p.getByRole("row").filter({ hasText: "窄间隔（2–3 kHz）三阶互调动态范围" });
  await expect(imd).toContainText("107 dB（2 kHz）");

  // 差异高亮：该行最好的一格加重（表下注释里写明「加粗 = 该行最好」）。
  const best = imd.locator("td").filter({ hasText: "107 dB（2 kHz）" }).first();
  await expect(best).toHaveClass(/font-semibold/);
  await expect(p.getByText(/加粗 = 该行最好/)).toBeVisible();

  // 出处与核对日期必须写出来，否则这些数字没有来源。
  await expect(p.getByText(/Rob Sherwood \(NC0B\).*2026-10-08/)).toBeVisible();
  await expect(p.getByRole("link", { name: "http://sherweng.com/table.html" })).toBeVisible();
});

test("机型库：按用途加权的雷达图与得分，依据不足时不给分", async ({ page }) => {
  await page.goto("/gear");
  const p = panel(page);

  // 雷达图：轴名 + 4 圈网格 + 每台一条多边形；图例条数应与多边形条数（减去网格）一致。
  await expect(p.getByRole("heading", { name: "六轴雷达图" })).toBeVisible();
  await expect(p.locator("svg title")).toHaveText("六轴雷达图");
  const series = await p.locator("svg + ul li").count();
  expect(series).toBeGreaterThanOrEqual(2);
  await expect(p.locator("svg polygon")).toHaveCount(series + 4);

  // 图例色块必须真的有色（`fill-*` 对 HTML `span` 无效，这里钉住回归）。
  const swatch = p.locator("svg + ul li span").first();
  await expect(swatch).not.toHaveCSS("background-color", "rgba(0, 0, 0, 0)");

  // 竞赛预设：HF 这三台都有实测值 → 得分行给数字，不出现「依据不足」。
  // 预设是互斥选择（`ChipGroup` 里的 `Chip`），角色是 radio 而不是 button。
  await p.getByRole("radio", { name: "竞赛强台" }).click();
  // 得分标题与表头同名，所以按角色限定（getByText 会撞严格模式）。
  await expect(p.getByRole("heading", { name: "加权得分（按用途）" })).toBeVisible();
  const scoreRow = p.getByRole("row").filter({ hasText: "加权得分（按用途）" });
  await expect(scoreRow).toBeVisible();
  await expect(scoreRow).not.toContainText("依据不足");
  await expect(scoreRow).toContainText(/\d/);

  // 手持台不在 Sherwood 表里：实测行标「未收录」；竞赛权重下只剩功率一条轴 → 依据不足。
  await p.getByRole("radio", { name: "手持对讲机" }).click();
  await expect(p.getByRole("row").filter({ hasText: "接收机噪声底" })).toContainText("未收录");
  await expect(p.getByText("依据不足").first()).toBeVisible();
  await expect(p.getByText(/有数据的不足两条/).first()).toBeVisible();
});

test("机型库：自己测的值以虚线叠加、打分跟着改，刷新后还在", async ({ page }) => {
  await page.goto("/gear");
  const p = panel(page);

  // 给默认三台里的 FT-710 录一项有基准的实测值。
  const rmdr = page.getByLabel(/FT-710.*RMDR/);
  await rmdr.fill("62");
  await expect(rmdr).toHaveValue("62");

  // 实测值以「同色虚线 + 图例（我的实测）」叠加到雷达图上：
  // 原表 3 条 + 自己 1 条 + 4 圈网格。
  await expect(p.getByText("Yaesu FT-710（我的实测）")).toBeVisible();
  await expect(p.locator("svg polygon")).toHaveCount(3 + 1 + 4);

  // 对比表里那一格换成自己测的值（间隔是字段定义里固定的 2 kHz）。
  const imd = p.getByRole("row").filter({ hasText: "窄间隔（2–3 kHz）三阶互调动态范围" });
  await expect(imd).toContainText("62 dB（2 kHz）");
  // 打分也改用自己的值 —— 界面必须说清楚，否则分数变了没人知道为什么。
  await expect(p.getByText("含你的实测值").first()).toBeVisible();

  // 只存本地，但存得住。
  await page.reload();
  await expect(page.getByLabel(/FT-710.*RMDR/)).toHaveValue("62");
  await expect(p.getByText("Yaesu FT-710（我的实测）")).toBeVisible();

  // 清除之后虚线消失。
  await page.getByRole("button", { name: "FT-710 清除" }).click();
  await expect(page.getByText("Yaesu FT-710（我的实测）")).toHaveCount(0);
  await expect(page.getByLabel(/FT-710.*RMDR/)).toHaveValue("");
});

/**
 * 逐字符输入：`fill` 是一次性设值 + 一次 `input` 事件，掩盖了「每次按键都把输入框
 * 回写成存储里的格式化文本」这类缺陷（曾实测出「想输 22 会存下 2.02」）。
 */
test("机型库：逐字符输入不丢字、不写错数，越界有提示", async ({ page }) => {
  await page.goto("/gear");
  const p = panel(page);

  // 整数位：敲 `2` 时值合法（2.0），但输入框不能被立刻改写成 `2.0` —— 否则接着敲 `2`
  // 会拼出 `2.02`。
  const current = page.getByLabel(/FT-710.*工作电流/);
  await current.pressSequentially("22");
  await expect(current).toHaveValue("22");
  expect(await stored(page)).toContain('"current_a":22');

  // 负数：`-` 与 `-1` 都是半截输入，不能被「逐键钳上限」吃掉（`max = -80`）。
  const noise = page.getByLabel(/FT-710.*噪声底/);
  await noise.pressSequentially("-126.5");
  await expect(noise).toHaveValue("-126.5");
  expect(await stored(page)).toContain('"noise_floor_dbm":-126.5');

  // 下界之外：值留在输入框里（让用户接着改），但不能存下来，而且要说明原因。
  const rmdr = page.getByLabel(/FT-710.*RMDR/);
  await rmdr.pressSequentially("20");
  await expect(rmdr).toHaveValue("20");
  await expect(page.getByText(/请填 40–130 之间的数值，未保存/)).toBeVisible();
  // 没保存：序列化里那一格仍是 null（`serde` 会把所有字段都写出来）。
  expect(await stored(page)).toContain('"rmdr_db":null');
  // 对比表仍是原表值（没被这半截输入改掉）。
  await expect(p.getByRole("row").filter({ hasText: "窄间隔（2–3 kHz）三阶互调动态范围" }))
    .toContainText("107 dB（2 kHz）");

  // RMDR 的首字符（6）同样低于下界：校验失败时输入框不能被清空，否则永远敲不出 62。
  const rmdrOk = page.getByLabel(/FT-710.*RMDR/);
  await rmdrOk.fill("");
  await rmdrOk.pressSequentially("62");
  await expect(rmdrOk).toHaveValue("62");
  await expect(p.getByRole("row").filter({ hasText: "窄间隔（2–3 kHz）三阶互调动态范围" }))
    .toContainText("62 dB（2 kHz）");
});
