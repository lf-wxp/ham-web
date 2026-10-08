import { expect, pickOption, test, type Page } from "./fixtures";

/** 本台信息面板（台站档案列表）。 */
function panel(page: Page) {
  return page.locator("section", { hasText: "本台信息" });
}

/** 表单所在的那一节（面板里也有同样 placeholder 的输入框，必须限定范围）。 */
function form(page: Page) {
  return page.locator("section", { hasText: "添加通联记录" });
}

async function openPanel(page: Page) {
  await page.getByRole("button", { name: /本台信息/ }).click();
  const p = panel(page);
  await expect(p.getByRole("button", { name: "新增台站" })).toBeVisible();
  return p;
}

/** 加一条通联；`operator` 非空时展开「更多字段」填上值机员。 */
async function addQso(page: Page, call: string, operator?: string) {
  const f = form(page);
  await f.locator('input[placeholder="BG4XXX"]').fill(call);
  if (operator) {
    const op = f.locator('input[placeholder="BD1ABC"]');
    if (!(await op.isVisible())) {
      await page.getByRole("button", { name: /更多字段/ }).click();
    }
    await op.fill(operator);
  }
  await page.getByRole("button", { name: "添加记录" }).click();
}

/** 填一条台站档案（档案名 / 呼号 / 网格 / 操作员）。 */
async function fillStation(page: Page, label: string, call: string, grid: string, operator: string) {
  const p = panel(page);
  const labelInput = p.getByPlaceholder("固定台 / 车载 / POTA");
  if (label !== "") {
    await labelInput.fill(label);
  }
  await p.getByPlaceholder("BG4XXX").fill(call);
  await p.getByPlaceholder("OM89EW").fill(grid);
  await p.getByPlaceholder("同呼号").fill(operator);
  await p.getByRole("button", { name: "保存台站档案" }).click();
}

test("统计范围：按台站与操作员分账，切页也记着", async ({ page }) => {
  page.on("dialog", (d) => d.accept());
  await page.goto("/log");
  const p = await openPanel(page);

  // 台站一：家里固定台，操作员 WXP。
  await fillStation(page, "本台", "BG4XXX", "OM89EW", "WXP");
  await addQso(page, "JA1AA");

  // 台站二：POTA 野外，操作员 LZ。
  await p.getByRole("button", { name: "新增台站" }).click();
  await fillStation(page, "POTA", "BG4XXX", "OL99AA", "LZ");
  await addQso(page, "DL1AA");

  // 回到家里台站，第三条显式换人值机（俱乐部台的常态）。
  // 台站切换是 `ChipGroup` 输出的 `role=radio`，不是普通按钮。
  await p.getByRole("radio", { name: "本台" }).click();
  await addQso(page, "JA3AA", "BD1ABC");

  const scope = page.getByLabel("统计范围");
  await expect(scope).toBeVisible();

  // 按操作员分账：WXP 只算家里那次，BD1ABC 只算显式填的那次。
  await pickOption(page, "统计范围", "WXP");
  await expect(page.getByText("该范围 1 条")).toBeVisible();
  await pickOption(page, "统计范围", "BD1ABC");
  await expect(page.getByText("该范围 1 条")).toBeVisible();

  // 按台站分账：家里 2 条（含换人值机那条），野外 1 条。
  await pickOption(page, "统计范围", "本台");
  await expect(page.getByText("该范围 2 条")).toBeVisible();
  await pickOption(page, "统计范围", "POTA");
  await expect(page.getByText("该范围 1 条")).toBeVisible();

  // 回到「全部」：提示消失。
  await pickOption(page, "统计范围", "全部");
  await expect(page.getByText(/该范围 \d+ 条/)).toBeHidden();

  // 切页也记着（口径存 localStorage）：重载后触发器仍显示所选台站。
  await pickOption(page, "统计范围", "POTA");
  await page.reload();
  await expect(page.getByRole("combobox", { name: "统计范围" })).toContainText("POTA");
  await expect(page.getByText("该范围 1 条")).toBeVisible();
});

test("统计范围：只有单台站单操作员时不显示选择器", async ({ page }) => {
  page.on("dialog", (d) => d.accept());
  await page.goto("/log");
  const p = await openPanel(page);
  await fillStation(page, "本台", "BG4XXX", "OM89EW", "WXP");
  await addQso(page, "JA1AA");

  // 一个台站、一个操作员 → 没有可分的面，不占地方。
  await expect(page.getByLabel("统计范围")).toBeHidden();
  await expect(page.getByText(/该范围 \d+ 条/)).toBeHidden();
});
