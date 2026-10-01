import { test as base, type Page } from "@playwright/test";

export { expect } from "@playwright/test";

// 练习 / 考试首次进入会自动弹出快捷键说明，默认标记为已看过，避免遮挡后续操作
export const test = base.extend<{ helpSeen: boolean }>({
  helpSeen: [true, { option: true }],
  page: async ({ page, helpSeen }, use) => {
    if (helpSeen) {
      await page.addInitScript(() => {
        localStorage.setItem("ui:shortcutsHelpSeen:practice", "1");
        localStorage.setItem("ui:shortcutsHelpSeen:exam", "1");
      });
    }
    await use(page);
  },
});

/** 当前题目中字母为 `letter` 的选项：练习 / 考试为 radio 或 checkbox，错题重练为切换按钮。 */
export function option(page: Page, letter: string) {
  const main = page.locator("main");
  const name = new RegExp(`^${letter}(\\s*\\.|\\s)`);
  return main
    .getByRole("radio", { name })
    .or(main.getByRole("checkbox", { name }))
    .or(main.locator("button[aria-pressed]").filter({ hasText: name }))
    .first();
}

/** 练习页「正确答案：A, C」中的字母。 */
export async function correctLetters(page: Page): Promise<string[]> {
  const text = await page.getByText(/^正确答案：/).first().innerText();
  return text.replace(/^正确答案：/, "").split(/[,，\s]+/).filter(Boolean);
}

/** 第一个不在正确答案中的选项字母。 */
export function wrongLetter(correct: string[]): string {
  const letter = ["A", "B", "C", "D"].find((l) => !correct.includes(l));
  if (!letter) throw new Error(`no wrong option among ${correct.join(",")}`);
  return letter;
}
