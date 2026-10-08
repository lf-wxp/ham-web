import { readFile } from "node:fs/promises";

import { expect, test, type Page } from "./fixtures";

/** 本台信息面板（台站档案列表）。 */
function panel(page: Page) {
  return page.locator("section", { hasText: "本台信息" });
}

/** 展开本台信息面板（「新增台站」只在展开后才有，用它判断展开成功）。 */
async function openPanel(page: Page) {
  await page.getByRole("button", { name: /本台信息/ }).click();
  const p = panel(page);
  await expect(p.getByRole("button", { name: "新增台站" })).toBeVisible();
  return p;
}

/**
 * 在表单里加一条通联（只填呼号，其余用默认值）。
 *
 * 台站面板展开时也有一个同样 placeholder 的「本台呼号」输入框，所以必须限定在
 * 表单那一节里 —— 否则 `input[placeholder="BG4XXX"]` 会同时命中两个输入框。
 */
async function addQso(page: Page, call: string) {
  await page
    .locator("section", { hasText: "添加通联记录" })
    .locator('input[placeholder="BG4XXX"]')
    .fill(call);
  await page.getByRole("button", { name: "添加记录" }).click();
}

test("台站档案：多台站的日志各自归属，ADIF 逐条写本台字段", async ({ page }) => {
  page.on("dialog", (d) => d.accept());
  await page.goto("/log");

  // 第一条档案（迁移出来的默认档案）填成家里的固定台；不填档案名，
  // 展示名回落为呼号（档案名 → 呼号 → 默认名）。
  let p = await openPanel(page);
  await p.getByPlaceholder("BG4XXX").fill("BG4XXX");
  await p.getByPlaceholder("OM89EW").fill("OM89EW");
  await p.getByRole("button", { name: "保存台站档案" }).click();
  await addQso(page, "JA1AA");

  // 新增一条野外台站并切过去：同一个呼号、不同网格。
  await p.getByRole("button", { name: "新增台站" }).click();
  await p.getByPlaceholder("固定台 / 车载 / POTA").fill("POTA");
  await p.getByPlaceholder("BG4XXX").fill("BG4XXX");
  await p.getByPlaceholder("OM89EW").fill("OL99AA");
  await p.getByRole("button", { name: "保存台站档案" }).click();
  await addQso(page, "JA2AA");

  // 两条通联归属不同的台站 —— 列表在多台站时才标出来。
  const ja1 = page.locator("main").getByRole("row").filter({ hasText: "JA1AA" });
  const ja2 = page.locator("main").getByRole("row").filter({ hasText: "JA2AA" });
  await expect(ja1).toContainText("BG4XXX");
  await expect(ja2).toContainText("POTA");
  await expect(p.getByText(/当前台站名下 1 条通联/)).toBeVisible();

  // 导出 ADIF：本台字段是**逐条**写的 —— 两个网格都要在文件里。
  const dl = page.waitForEvent("download");
  await page.getByRole("button", { name: "导出 ADIF" }).click();
  const text = await readFile(await (await dl).path(), "utf8");
  expect(text).toContain("<MY_GRIDSQUARE:6>OM89EW");
  expect(text).toContain("<MY_GRIDSQUARE:6>OL99AA");
  expect(text.match(/<STATION_CALLSIGN:6>BG4XXX/g)?.length).toBe(2);

  // 切回第一台站：新记录跟过去，老记录不动。
  // 台站切换是 `ChipGroup` 输出的 `role=radio`，不是普通按钮。
  await p.getByRole("radio", { name: "BG4XXX" }).click();
  await addQso(page, "JA3AA");
  await expect(ja2).toContainText("POTA");
  await expect(page.locator("main").getByRole("row").filter({ hasText: "JA3AA" })).toContainText(
    "BG4XXX",
  );
});

test("台站档案：最后一个不许删", async ({ page }) => {
  page.on("dialog", (d) => d.accept());
  await page.goto("/log");
  const p = await openPanel(page);

  // 只有一个档案时删除按钮是灰的。
  const remove = p.getByRole("button", { name: "删除台站" });
  await expect(remove).toBeDisabled();

  // 加一条之后可删，删掉又只剩一条，按钮重新变灰。
  await p.getByRole("button", { name: "新增台站" }).click();
  await expect(remove).toBeEnabled();
  await remove.click();
  await expect(remove).toBeDisabled();
});

test("台站档案：老的本台信息自动迁成第一条档案", async ({ page }) => {
  // 只有老 key（`station-info`）的旧版本用户。
  await page.addInitScript(() => {
    localStorage.setItem(
      "station-info",
      JSON.stringify({
        callsign: "BG4XXX",
        operator: "WXP",
        gridsquare: "OM89EW",
        rig: "FT-710",
        antenna: "",
      }),
    );
  });
  await page.goto("/log");
  await expect(page.getByRole("button", { name: /本台信息/ })).toContainText("BG4XXX");

  // 迁移结果落进新的档案册，且 `station-info` 仍作为当前台站的镜像在写。
  const book = await page.evaluate(() => localStorage.getItem("station-book"));
  expect(book).toContain("OM89EW");
  expect(book).toContain("FT-710");
  const mirror = await page.evaluate(() => localStorage.getItem("station-info"));
  expect(mirror).toContain("OM89EW");
});
