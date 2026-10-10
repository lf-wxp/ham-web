import { expect, test } from "./fixtures";

test("模考：机考仿真开考前先过考场规则演练", async ({ page }) => {
  // 预置机考仿真开关（exam:strict=1），进入 /exam 时开考先弹规则演练。
  await page.addInitScript(() => {
    window.localStorage.setItem("exam:strict", "1");
  });
  await page.goto("/exam");

  const dialog = page.getByRole("dialog", { name: "考场规则演练" });
  await expect(dialog).toBeVisible();
  // 规则对照：真实考场与本站仿真的差异如实列出。
  // 列头是精确词（描述文案里也有「真实考场」字样，用 exact 区分）。
  await expect(dialog.getByText("真实考场", { exact: true })).toBeVisible();
  await expect(dialog.getByText("本站仿真", { exact: true })).toBeVisible();
  await expect(dialog.getByText(/允许续考/)).toBeVisible();
  // 考前准备一条龙：证件照处理器与练习。
  const photo = dialog.getByRole("link", { name: /报名用证件照/ });
  await expect(photo).toHaveAttribute("href", "/photo-processor");

  // 确认后真正开考：倒计时出现。
  await dialog.getByRole("button", { name: "我已了解，开始机考" }).click();
  await expect(dialog).toBeHidden();
  await expect(page.getByText(/剩余时间：\d{2}:\d{2}/).first()).toBeVisible();
});

test("模考：规则演练可以暂不开始", async ({ page }) => {
  await page.addInitScript(() => {
    window.localStorage.setItem("exam:strict", "1");
  });
  await page.goto("/exam");
  const dialog = page.getByRole("dialog", { name: "考场规则演练" });
  await expect(dialog).toBeVisible();
  await dialog.getByRole("link", { name: "暂不开始" }).click();
  await expect(page).toHaveURL(/practice/);
});

test("模考：按 Esc 关闭规则演练同样回到练习页，而不是留在空考场", async ({ page }) => {
  await page.addInitScript(() => {
    window.localStorage.setItem("exam:strict", "1");
  });
  await page.goto("/exam");
  const dialog = page.getByRole("dialog", { name: "考场规则演练" });
  await expect(dialog).toBeVisible();
  // 中途关闭（Esc / 遮罩 / ×）= 没确认开考：挂起的组卷作废，与「暂不开始」一致。
  await page.keyboard.press("Escape");
  await expect(page).toHaveURL(/practice/);
});
