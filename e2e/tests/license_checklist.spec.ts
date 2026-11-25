import { expect, test, type Page } from "./fixtures";

/**
 * 设台申请核对清单（`/license`）：每一项指到《业余无线电台管理办法》的条文，能核对的再指到工具。
 * 出处原话是否真的在条文里由核心单测逐条核对；这里钉住页面：渲染、勾选与记忆、锚点与工具链接可点、
 * 不编办法里没有的材料项、三语、320 px 溢出。
 */

const TOTAL = 23;
const row = (page: Page, id: string) => page.locator(`[data-checklist-item="${id}"]`);
const progress = (page: Page) => page.getByTestId("checklist-progress");

test("核对清单：六组、23 项，没有漏词条而露出的裸 id", async ({ page }) => {
  await page.goto("/license");
  const card = page.getByTestId("license-checklist");
  await expect(card).toBeVisible();
  await expect(progress(page)).toHaveText(`已核对 0 / ${TOTAL} 项`);
  for (const g of ["设台条件", "向谁申请", "提交的材料", "频率、功率与执照载明事项", "受理与审批", "领证之后"]) {
    await expect(card.getByRole("heading", { name: g, exact: true })).toBeVisible();
  }
  await expect(card.locator("[data-checklist-item]")).toHaveCount(TOTAL);
  // 漏了词条会显示 id 本身（形如 know-rules）：任何一项都不该是纯 ASCII 的 kebab-case。
  const labels = await card.locator("[data-checklist-item] label").allTextContents();
  for (const l of labels) expect(l.trim(), "说明文字缺词条").not.toMatch(/^[a-z]+(-[a-z]+)*$/);
});

test("核对清单：勾选计数、刷新后还在、清除勾选", async ({ page }) => {
  await page.goto("/license");
  const form = row(page, "form").getByRole("checkbox");
  const ident = row(page, "identity").getByRole("checkbox");
  const reset = page.getByRole("button", { name: "清除勾选" });
  await expect(reset).toBeDisabled();

  await form.click();
  await ident.click();
  await expect(progress(page)).toHaveText(`已核对 2 / ${TOTAL} 项`);
  await expect(form).toHaveAttribute("aria-checked", "true");
  // 取消一项：计数回落。
  await ident.click();
  await expect(progress(page)).toHaveText(`已核对 1 / ${TOTAL} 项`);

  // 刷新：勾选还在，且只记勾选的那一项。
  await expect.poll(() => page.evaluate(() => localStorage.getItem("ui:license-checklist"))).toBe('["form"]');
  await page.reload();
  await expect(progress(page)).toHaveText(`已核对 1 / ${TOTAL} 项`);
  await expect(row(page, "form").getByRole("checkbox")).toHaveAttribute("aria-checked", "true");
  await expect(row(page, "identity").getByRole("checkbox")).toHaveAttribute("aria-checked", "false");

  await page.getByRole("button", { name: "清除勾选" }).click();
  await expect(progress(page)).toHaveText(`已核对 0 / ${TOTAL} 项`);
  await expect.poll(() => page.evaluate(() => localStorage.getItem("ui:license-checklist"))).toBe("[]");
  await page.reload();
  await expect(progress(page)).toHaveText(`已核对 0 / ${TOTAL} 项`);
});

test("核对清单：本机存的旧 id（清单改过之后不存在的项）不占计数", async ({ page }) => {
  await page.addInitScript(() =>
    localStorage.setItem("ui:license-checklist", JSON.stringify(["form", "no-such-item", "removed-in-a-later-version"])),
  );
  await page.goto("/license");
  await expect(progress(page)).toHaveText(`已核对 1 / ${TOTAL} 项`);
});

test("核对清单：点条号跳到法规原文库的那一条，原文里确有这项的出处", async ({ page }) => {
  await page.goto("/license");
  // 申请表：办法第十一条。
  await row(page, "form").getByRole("link", { name: "第11条" }).click();
  await expect.poll(() => decodeURIComponent(page.url())).toMatch(/\/radio-law#办法-11$/);
  const article = page.locator('[id="办法-11"]');
  await expect(article).toBeVisible();
  await expect(article).toContainText("申请表（样式见附件1）");
  await expect(article).toContainText("身份证明复印件");
});

test("核对清单：每个条号链接都落在法规库里真实存在的锚点上", async ({ page }) => {
  await page.goto("/license");
  const numbers = new Set(
    await page.locator("[data-law-article]").evaluateAll((els) => els.map((e) => e.getAttribute("data-law-article")!)),
  );
  // 清单引用了办法的这些条（独立数一遍：7、8、10–22、25、30、31、34 共 22 条里去掉没引的）。
  for (const n of [7, 8, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 25, 30, 31, 34]) {
    expect(numbers.has(String(n)), `清单里应引用第 ${n} 条`).toBe(true);
  }
  await page.goto("/radio-law");
  for (const n of numbers) {
    await expect(page.locator(`[id="办法-${n}"]`), `办法第 ${n} 条的锚点`).toHaveCount(1);
  }
});

test("核对清单：工具链接可点——频率功率合规", async ({ page }) => {
  await page.goto("/license");
  await row(page, "class-limits").getByRole("link", { name: "频率功率合规" }).click();
  await expect(page).toHaveURL(/\/compliance$/);
  await expect(page.getByRole("heading", { level: 1 })).toContainText("频率功率合规");
});

test("核对清单：电磁辐射一项链到射频暴露评估，执照更换一项链到本页的到期提醒", async ({ page }) => {
  await page.goto("/license");
  await expect(row(page, "radiation").locator("[data-checklist-tool]")).toHaveAttribute("href", "/tools#rf-exposure");
  await row(page, "radiation").getByRole("link", { name: "射频暴露评估" }).click();
  await expect(page).toHaveURL(/\/tools#rf-exposure$/);

  await page.goto("/license");
  await row(page, "renewal").locator("[data-checklist-tool]").click();
  await expect(page).toHaveURL(/\/license#expiry-reminder$/);
  await expect(page.locator("#expiry-reminder")).toBeVisible();
});

test("核对清单：只列办法写明的材料，没有天线形式等办法里没有的项；未成年人 / 单位项带标记", async ({ page }) => {
  await page.goto("/license");
  const materials = page
    .getByTestId("license-checklist")
    .locator("div", { has: page.getByRole("heading", { name: "提交的材料", exact: true }) })
    .last();
  const items = materials.locator("[data-checklist-item]");
  await expect(items).toHaveCount(6);
  const text = (await items.allTextContents()).join("\n");
  // 办法第十一、十二条没有要求提交天线形式 / 台址图 / 购买凭证。
  for (const forbidden of ["天线", "台址图", "平面图", "购买", "发票"]) {
    expect(text, `材料里不该出现「${forbidden}」`).not.toContain(forbidden);
  }
  // 材料的出处只有第 11、12 条。
  const cited = await materials
    .locator("[data-law-article]")
    .evaluateAll((els) => [...new Set(els.map((e) => e.getAttribute("data-law-article")))].sort());
  expect(cited).toEqual(["11", "12"]);

  await expect(row(page, "guardian")).toContainText("仅未成年人");
  await expect(row(page, "minor-limits")).toContainText("仅未成年人");
  await expect(row(page, "unit-documents")).toContainText("仅单位申请");
  await expect(row(page, "form")).not.toContainText("仅");
});

test("核对清单：英文 / 西语界面都有译文，不留中文", async ({ page }) => {
  for (const [code, title, article] of [
    ["en", "Station licence application checklist", "Article 11"],
    ["es", "Lista de verificación para solicitar una licencia de estación", "Artículo 11"],
  ] as const) {
    await page.addInitScript((c) => localStorage.setItem("locale", c), code);
    await page.goto("/license");
    const card = page.getByTestId("license-checklist");
    await expect(card.getByRole("heading", { name: title })).toBeVisible();
    await expect(card.getByRole("link", { name: article }).first()).toBeVisible();
    await expect(card).not.toContainText(/[\u4e00-\u9fff]/);
  }
});

/** 整页与清单卡片各自的横向溢出（像素，≤ 1 视为没有溢出）。 */
async function overflowOf(page: Page) {
  // 现代风点按钮 / 勾选会放一圈 560 ms 的按压涟漪（`.md-ping`），动画放大时短暂伸出视口，之后自清理。
  await expect(page.locator(".md-ping")).toHaveCount(0);
  return page.evaluate(() => {
    const el = document.querySelector('[data-testid="license-checklist"]') as HTMLElement;
    return {
      page: document.documentElement.scrollWidth - window.innerWidth,
      card: el.scrollWidth - el.clientWidth,
    };
  });
}

test("核对清单：溢出检测器自测——塞进 900px 宽的元素时必须量得出溢出", async ({ page }) => {
  await page.setViewportSize({ width: 320, height: 800 });
  await page.goto("/license");
  await expect(page.getByTestId("license-checklist")).toBeVisible();
  const before = await overflowOf(page);
  expect(before.page).toBeLessThanOrEqual(1);
  expect(before.card).toBeLessThanOrEqual(1);
  await page.evaluate(() => {
    const wide = document.createElement("div");
    wide.style.cssText = "width:900px;height:4px";
    document.querySelector('[data-testid="license-checklist"]')!.appendChild(wide);
  });
  const after = await overflowOf(page);
  expect(after.card, "卡片里放 900px 宽的元素，量到的溢出应远大于 1").toBeGreaterThan(100);
  expect(after.page, "整页同理").toBeGreaterThan(100);
});

// 三语里最长的说明（西语的第十条与第三十条）在 320 px 下不能把整页或卡片撑出横向滚动；两套皮肤各量一次。
for (const lang of ["zh", "en", "es"]) {
  for (const style of ["pixel", "modern"]) {
    test(`核对清单：320px 下不横向溢出（${lang} · ${style}）`, async ({ page }) => {
      await page.setViewportSize({ width: 320, height: 800 });
      await page.addInitScript(
        ([code, ui]) => {
          localStorage.setItem("locale", code);
          localStorage.setItem("ui:uiStyle", ui);
          if (ui === "modern") localStorage.setItem("theme", "dark");
        },
        [lang, style],
      );
      await page.goto("/license");
      await expect(page.getByTestId("license-checklist")).toBeVisible();
      // 勾上最长的一项，让「已核对」与清除按钮也进入布局。
      await row(page, "class-limits").getByRole("checkbox").click();
      const over = await overflowOf(page);
      expect(over.page, "整页横向溢出").toBeLessThanOrEqual(1);
      expect(over.card, "清单卡片自身横向溢出").toBeLessThanOrEqual(1);
    });
  }
}
