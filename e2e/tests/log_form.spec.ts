import { confirmDialog, expect, test } from "./fixtures";

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

test("通联日志：实体名随界面语言本地化", async ({ page }) => {
  await page.addInitScript(() => localStorage.setItem("locale", "en"));
  await page.goto("/log");
  await callInput(page).fill("JA1AA");
  // 先等呼号查询返回（CQ/ITU 分区），再断言实体名用英文名 Japan、不再漏出中文「日本」。
  await expect(page.getByText(/CQ 25 · ITU 45/)).toBeVisible();
  await expect(page.getByText("Japan")).toBeVisible();
  await expect(page.getByText("日本")).toHaveCount(0);
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

  // 页内按钮限定在 `main`：确认框已传送到 body，退场动画期间它的「清空全部」还在 DOM 里。
  const clear = page.locator("main").getByRole("button", { name: "清空", exact: true });

  // 取消：确认框关掉，数据不动。
  await clear.click();
  await page.getByRole("dialog").getByRole("button", { name: "取消" }).click();
  await expect(page.getByText("共 1 条")).toBeVisible();

  // 确认：走 `ui::confirm` 的「清空全部」。
  await clear.click();
  await confirmDialog(page, "清空全部");
  await expect(page.getByText("暂无记录，添加第一条通联日志吧。")).toBeVisible();
});

test("通联日志：单条删除需确认，连点「删除」只执行一次", async ({ page }) => {
  await page.goto("/log");
  await expect(page.getByRole("heading", { level: 1, name: "通联日志" })).toBeVisible();
  await addEntry(page, "JA1AA");
  await addEntry(page, "DL1AA");
  await expect(page.getByText("共 2 条")).toBeVisible();

  const row = page.getByRole("row").filter({ hasText: "JA1AA" });
  const dialog = page.getByRole("dialog");

  // 取消：什么都不删。
  await row.getByRole("button", { name: "删除", exact: true }).click();
  await expect(dialog).toContainText("删除这条通联记录？");
  await dialog.getByRole("button", { name: "取消" }).click();
  await expect(page.getByText("共 2 条")).toBeVisible();

  // 确认时手抖连点两下：删除只发生一次；成功提示若跑了两遍，会被合并成「×2」。
  await row.getByRole("button", { name: "删除", exact: true }).click();
  await dialog.getByRole("button", { name: "删除", exact: true }).dblclick();
  await expect(page.getByText("共 1 条")).toBeVisible();
  const toast = page.locator('[data-slot="toast"]');
  await expect(toast).toContainText("已删除该条通联记录");
  await expect(toast).not.toContainText("×");
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

// 布局回归（i18n_layout.spec.ts）只在桌面宽度量；录入表单里那个列出了全部折叠字段的长按钮，
// 在手机宽度下不换行会把单列网格撑到 500px 以上，整页出现横向滚动。
for (const [code, more] of [
  ["zh", "更多字段"],
  ["en", "More fields"],
  ["es", "Más campos"],
] as const) {
  test(`通联日志：手机宽度下录入表单不产生横向滚动（${code}）`, async ({ page }) => {
    await page.addInitScript((c) => localStorage.setItem("locale", c), code);
    for (const width of [320, 390]) {
      await page.setViewportSize({ width, height: 900 });
      await page.goto("/log");
      await expect(page.getByRole("button", { name: new RegExp(`^${more}`) })).toBeVisible();
      const overflow = () =>
        page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth);
      expect(await overflow(), `${width}px 折叠时`).toBeLessThanOrEqual(1);

      await page.getByRole("button", { name: new RegExp(`^${more}`) }).click();
      expect(await overflow(), `${width}px 展开后`).toBeLessThanOrEqual(1);
    }
  });
}
