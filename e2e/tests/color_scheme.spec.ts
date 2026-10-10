import AxeBuilder from "@axe-core/playwright";
import type { Page } from "@playwright/test";
import { expect, test } from "./fixtures";

// 配色方案：与明暗正交的第二个外观选择（经典 / 森林 / 海洋 / 晚霞 / 石墨，每个都有亮暗两套）。
// 方案表在 crates/core/src/color_scheme.rs；每一对字 / 底的对比度另由 `cargo make pixel-check` 在
// 构建期审计，这里验证的是「真实页面里用户看到的」：切换、持久化、回退、以及 axe。

const SCHEMES = [
  { id: "classic", name: "经典像素" },
  { id: "forest", name: "森林" },
  { id: "ocean", name: "海洋" },
  { id: "sunset", name: "晚霞" },
  { id: "graphite", name: "石墨" },
] as const;

/** 在页面脚本跑之前写入偏好，等价于「用户上次选过」。 */
async function preset(page: Page, scheme: string, theme: "light" | "dark") {
  await page.addInitScript(
    ([s, t]) => {
      localStorage.setItem("ui:colorScheme", s);
      localStorage.setItem("theme", t);
    },
    [scheme, theme],
  );
}

const token = (page: Page, name: string) =>
  page.evaluate(
    (n) => getComputedStyle(document.documentElement).getPropertyValue(n).trim(),
    name,
  );

test("设置里切换配色方案：写入 <html>、持久化、刷新后保留", async ({ page }) => {
  await page.goto("/");
  const html = page.locator("html");
  await expect(html).toHaveAttribute("data-scheme", "classic");
  const classicBg = await token(page, "--background");

  await page.getByRole("button", { name: "外观与动效" }).first().click();
  const dialog = page.getByRole("dialog", { name: "外观与动效" });
  await dialog.getByRole("radio", { name: "森林" }).click();

  await expect(html).toHaveAttribute("data-scheme", "forest");
  expect(await token(page, "--background")).not.toBe(classicBg);
  await expect
    .poll(() => page.evaluate(() => localStorage.getItem("ui:colorScheme")))
    .toBe("forest");
  await expect(dialog.getByRole("radio", { name: "森林" })).toBeChecked();

  await page.reload();
  await expect(html).toHaveAttribute("data-scheme", "forest");
  await page.getByRole("button", { name: "外观与动效" }).first().click();
  await expect(
    page.getByRole("dialog", { name: "外观与动效" }).getByRole("radio", { name: "森林" }),
  ).toBeChecked();
});

test("方案与明暗正交：十种组合的页面底色互不相同", async ({ browser }) => {
  const seen = new Map<string, string>();
  for (const { id } of SCHEMES) {
    for (const theme of ["light", "dark"] as const) {
      const ctx = await browser.newContext();
      const page = await ctx.newPage();
      await preset(page, id, theme);
      await page.goto("/");
      await expect(page.locator("html")).toHaveAttribute("data-scheme", id);
      await expect(page.locator("html")).toHaveClass(theme === "dark" ? /dark/ : /^(?!.*dark)/);
      const bg = await token(page, "--background");
      expect(bg, `${id}/${theme} 没有 --background`).toMatch(/^#[0-9a-f]{6}$/);
      expect(seen.get(bg), `${id}/${theme} 与 ${seen.get(bg)} 的底色相同`).toBeUndefined();
      seen.set(bg, `${id}/${theme}`);
      await ctx.close();
    }
  }
  expect(seen.size).toBe(SCHEMES.length * 2);
});

test("不认识的方案 id 回退经典，并把存储规范化", async ({ page }) => {
  await page.addInitScript(() => localStorage.setItem("ui:colorScheme", "no-such-scheme"));
  await page.goto("/");
  await expect(page.locator("html")).toHaveAttribute("data-scheme", "classic");
  const classicBg = await token(page, "--background");
  expect(classicBg).toBe("#f2e8cf");

  // 用户在设置里再选一次，存储里就是合法 id。
  await page.getByRole("button", { name: "外观与动效" }).first().click();
  await page
    .getByRole("dialog", { name: "外观与动效" })
    .getByRole("radio", { name: "海洋" })
    .click();
  await expect
    .poll(() => page.evaluate(() => localStorage.getItem("ui:colorScheme")))
    .toBe("ocean");
});

test("切换方案不会改变明暗模式", async ({ page }) => {
  await preset(page, "classic", "dark");
  await page.goto("/");
  const html = page.locator("html");
  await expect(html).toHaveClass(/dark/);
  await page.getByRole("button", { name: "外观与动效" }).first().click();
  const dialog = page.getByRole("dialog", { name: "外观与动效" });
  for (const { name } of SCHEMES) {
    await dialog.getByRole("radio", { name }).click();
    await expect(html).toHaveClass(/dark/);
  }
  await expect(dialog.getByRole("radio", { name: "深色" })).toBeChecked();
});

test("方案名随界面语言切换", async ({ page }) => {
  await page.addInitScript(() => localStorage.setItem("locale", "en"));
  await page.goto("/");
  await page.getByRole("button", { name: "Appearance & motion" }).first().click();
  const dialog = page.getByRole("dialog", { name: "Appearance & motion" });
  for (const name of ["Classic", "Forest", "Ocean", "Sunset", "Graphite"]) {
    await expect(dialog.getByRole("radio", { name })).toBeVisible();
  }
});

// 新方案（经典已被 smoke.spec.ts 覆盖）× 明暗 × 代表性页面的 axe 对比度。
// 减少动态效果：避开入场动画中间帧的半透明，量的是用户最终看到的状态。
test.describe("各方案的无障碍对比度", () => {
  test.use({ reducedMotion: "reduce" });

  const pages = ["/", "/practice", "/browse", "/map", "/formulas"];
  for (const { id } of SCHEMES.filter((s) => s.id !== "classic")) {
    for (const theme of ["light", "dark"] as const) {
      test(`${id} · ${theme}`, async ({ page }) => {
        await preset(page, id, theme);
        const violations: string[] = [];
        for (const route of pages) {
          await page.goto(route);
          await expect(page.locator("main h1").first()).toBeAttached();
          await page.waitForTimeout(500);
          const res = await new AxeBuilder({ page })
            .withTags(["wcag2a", "wcag2aa", "wcag21a", "wcag21aa", "best-practice"])
            .exclude(".leaflet-container")
            .analyze();
          for (const v of res.violations) {
            violations.push(
              `${route} ${v.id}: ${v.nodes.slice(0, 2).map((n) => n.target.join(" ")).join(" | ")}`,
            );
          }
        }
        expect(violations, `${id}/${theme}`).toEqual([]);
      });
    }

    test(`${id} · 设置对话框打开时`, async ({ page }) => {
      await preset(page, id, "light");
      await page.goto("/");
      await page.getByRole("button", { name: "外观与动效" }).first().click();
      await expect(page.getByRole("dialog", { name: "外观与动效" })).toBeVisible();
      await page.waitForTimeout(500);
      const res = await new AxeBuilder({ page })
        .withTags(["wcag2a", "wcag2aa", "wcag21a", "wcag21aa", "best-practice"])
        .analyze();
      expect(res.violations.map((v) => `${v.id}: ${v.nodes[0]?.target.join(" ")}`)).toEqual([]);
    });
  }
});
