import { expect, pickOption, test } from "./fixtures";

test("听题模式：依次朗读题目、选项与答案，并记住位置", async ({ page }) => {
  // 用桩替换语音合成：记录朗读内容并立即结束，避免依赖系统语音
  await page.addInitScript(() => {
    const spoken: string[] = [];
    (window as unknown as { __spoken: string[] }).__spoken = spoken;
    const synth = {
      speak(u: SpeechSynthesisUtterance) {
        spoken.push(u.text);
        setTimeout(() => u.onend?.(new Event("end") as SpeechSynthesisEvent), 5);
      },
      cancel() {},
    };
    Object.defineProperty(window, "speechSynthesis", { value: synth, configurable: true });
  });
  await page.goto("/listen");
  await expect(page.getByRole("heading", { level: 1, name: "听题模式" })).toBeVisible();
  await pickOption(page, "思考时间", "3 秒");
  await page.getByRole("button", { name: "开始听题" }).click();

  await expect(page.getByText("公布答案")).toBeVisible({ timeout: 10_000 });
  await expect(page.getByText(/^正确答案：/)).toBeVisible();
  const spoken = await page.evaluate(() => (window as unknown as { __spoken: string[] }).__spoken);
  expect(spoken[0]).toBe("第 1 题，多选题。");
  expect(spoken.some((t) => t.startsWith("A，"))).toBe(true);
  expect(spoken.some((t) => t.startsWith("正确答案："))).toBe(true);

  await expect(page.getByText(/第 2 \/ \d+ 题/)).toBeVisible({ timeout: 10_000 });
  await page.getByRole("button", { name: "暂停" }).click();
  await expect(page.getByText("已暂停")).toBeVisible();

  await page.reload();
  await expect(page.getByText(/第 2 \/ \d+ 题/)).toBeVisible();
});
