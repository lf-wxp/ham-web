import { expect, test } from "./fixtures";

/** 呼号输入框没有 <label> 包裹，用 placeholder 定位。 */
function callInput(page: import("@playwright/test").Page) {
  return page.locator('input[placeholder="BG4XXX"]');
}

async function addEntry(page: import("@playwright/test").Page, call: string) {
  await callInput(page).fill(call);
  await page.getByRole("button", { name: "添加记录" }).click();
}

test("通联日志：呼号为空时提交无效，列表保持空态", async ({ page }) => {
  await page.goto("/log");
  await expect(page.getByRole("heading", { level: 1, name: "通联日志" })).toBeVisible();
  await expect(page.getByText("暂无记录，添加第一条通联日志吧。")).toBeVisible();

  await page.getByRole("button", { name: "添加记录" }).click();
  await expect(page.getByText("暂无记录，添加第一条通联日志吧。")).toBeVisible();
});

test("通联日志：填写呼号后入库并识别实体", async ({ page }) => {
  await page.goto("/log");
  await expect(page.getByRole("heading", { level: 1, name: "通联日志" })).toBeVisible();

  await callInput(page).fill("JA1AA");
  // 呼号提示条给出 DXCC 实体与分区
  await expect(page.getByText(/CQ \d+ · ITU \d+/)).toBeVisible();
  await expect(page.getByText("首次通联该呼号")).toBeVisible();

  await addEntry(page, "JA1AA");
  await expect(page.getByText("共 1 条")).toBeVisible();
  await expect(page.getByRole("row").filter({ hasText: "JA1AA" })).toHaveCount(1);

  // 再次添加同一呼号：提示已通联过
  await callInput(page).fill("JA1AA");
  await expect(page.getByText(/已通联 1 次/)).toBeVisible();
  await addEntry(page, "JA1AA");
  await expect(page.getByText("共 2 条")).toBeVisible();
});

test("通联日志：搜索与筛选", async ({ page }) => {
  await page.goto("/log");
  await expect(page.getByRole("heading", { level: 1, name: "通联日志" })).toBeVisible();
  await addEntry(page, "JA1AA");
  await addEntry(page, "W1AW");
  await expect(page.getByText("共 2 条")).toBeVisible();

  await page.getByPlaceholder("搜索呼号 / 姓名 / QTH / 网格 / 备注").fill("JA1");
  await expect(page.getByText(/筛选出 1 \/ 2 条/)).toBeVisible();

  await page.getByPlaceholder("搜索呼号 / 姓名 / QTH / 网格 / 备注").fill("不存在");
  await expect(page.getByText(/筛选出 0 \/ 2 条/)).toBeVisible();
});

test("通联日志：编辑与取消编辑", async ({ page }) => {
  await page.goto("/log");
  await expect(page.getByRole("heading", { level: 1, name: "通联日志" })).toBeVisible();
  await addEntry(page, "JA1AA");
  await expect(page.getByText("共 1 条")).toBeVisible();

  await page.getByRole("button", { name: "编辑" }).first().click();
  await expect(page.getByRole("heading", { level: 2, name: "编辑通联记录" })).toBeVisible();
  await expect(page.getByRole("button", { name: "保存修改" })).toBeVisible();

  await page.getByRole("button", { name: "取消" }).click();
  await expect(page.getByRole("heading", { level: 2, name: "添加通联记录" })).toBeVisible();
});

test("通联日志：清空需二次确认，取消不丢数据", async ({ page }) => {
  await page.goto("/log");
  await expect(page.getByRole("heading", { level: 1, name: "通联日志" })).toBeVisible();
  await addEntry(page, "JA1AA");

  page.once("dialog", (d) => d.dismiss());
  await page.getByRole("button", { name: "清空" }).click();
  await expect(page.getByText("共 1 条")).toBeVisible();

  page.once("dialog", (d) => d.accept());
  await page.getByRole("button", { name: "清空" }).click();
  await expect(page.getByText("暂无记录，添加第一条通联日志吧。")).toBeVisible();
});

test("通联日志：同步 QSL 空报告给出提示", async ({ page }) => {
  await page.goto("/log");
  await expect(page.getByRole("heading", { level: 1, name: "通联日志" })).toBeVisible();
  await page.getByRole("button", { name: "同步 QSL" }).click();
  const dialog = page.getByRole("dialog", { name: "同步 QSL 确认" });
  await expect(dialog).toBeVisible();

  page.once("dialog", (d) => {
    expect(d.message()).toBe("请先粘贴或上传确认报告（ADIF）。");
    return d.accept();
  });
  await dialog.getByRole("button", { name: "应用确认" }).click();
});

test("通联日志：呼号过短时查询给出提示", async ({ page }) => {
  await page.goto("/log");
  await expect(page.getByRole("heading", { level: 1, name: "通联日志" })).toBeVisible();
  await callInput(page).fill("J");

  page.once("dialog", (d) => {
    expect(d.message()).toBe("请输入完整呼号后再查询");
    return d.accept();
  });
  await page.getByRole("button", { name: "查询" }).click();
});

test("通联日志：本台信息可保存", async ({ page }) => {
  await page.goto("/log");
  await expect(page.getByRole("heading", { level: 1, name: "通联日志" })).toBeVisible();
  await page.getByRole("button", { name: /本台信息/ }).first().click();
  await page.getByLabel("本台呼号").fill("BG4XXX");
  await page.getByLabel("本台网格").fill("OM89EW");

  page.once("dialog", (d) => {
    expect(d.message()).toBe("本台信息已保存");
    return d.accept();
  });
  await page.getByRole("button", { name: "保存台站档案" }).click();

  // 档案册是真相来源，`station-info` 是当前台站的镜像（RBN / PSK Reporter 还在读它）。
  await expect
    .poll(() => page.evaluate(() => localStorage.getItem("station-book")))
    .toContain("BG4XXX");
  await expect
    .poll(() => page.evaluate(() => localStorage.getItem("station-info")))
    .toContain("BG4XXX");
});
