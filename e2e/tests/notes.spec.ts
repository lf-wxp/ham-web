import { correctLetters, expect, option, test, wrongLetter } from "./fixtures";

test("错题重练：答后可记笔记，重新进入仍在", async ({ page }) => {
  // 先在练习里答错一题，让错题本有数据
  await page.goto("/practice");
  await expect(page.getByText(/第 1 \/ \d+ 题/)).toBeVisible();
  const correct = await correctLetters(page);
  await option(page, wrongLetter(correct)).click();
  await page.getByRole("button", { name: "下一题" }).first().click();
  await expect(page.getByText(/第 2 \/ \d+ 题/)).toBeVisible();

  await page.goto("/mistakes");
  await page.getByRole("button", { name: "全部重练" }).click();
  await expect(page.getByText(/第 1 \/ 1 题/)).toBeVisible();
  for (const letter of correct) {
    await option(page, letter).click();
  }
  await page.getByRole("button", { name: "提交" }).click();
  await expect(page.getByText(/^正确！/)).toBeVisible();

  // 答后（解析出现时）应有笔记编辑区
  const editor = page.getByRole("textbox", { name: "笔记内容" });
  await expect(editor).toBeVisible();
  await editor.fill("易错点：记住这个是错的");
  await page.getByRole("button", { name: "保存笔记" }).click();
  await expect(page.getByText("已保存")).toBeVisible();

  // 换页再回来，笔记应持久化
  await page.goto("/mistakes");
  await page.getByRole("button", { name: "全部重练" }).click();
  await expect(page.getByText(/第 1 \/ 1 题/)).toBeVisible();
  for (const letter of correct) {
    await option(page, letter).click();
  }
  await page.getByRole("button", { name: "提交" }).click();
  await expect(
    page.getByRole("textbox", { name: "笔记内容" }),
  ).toHaveValue("易错点：记住这个是错的");
});

test("收藏集：笔记按需展开，不占满列表", async ({ page }) => {
  // 练习里收藏一题
  await page.goto("/practice");
  await expect(page.getByText(/第 1 \/ \d+ 题/)).toBeVisible();
  await page.getByRole("button", { name: "收藏" }).first().click();

  await page.goto("/bookmarks");
  await expect(page.getByRole("heading", { level: 1, name: "收藏集" })).toBeVisible();

  // 默认不铺开输入框
  await expect(page.getByRole("textbox", { name: "笔记内容" })).toHaveCount(0);
  const toggle = page.getByRole("button", { name: "笔记" }).first();
  await expect(toggle).toBeVisible();
  await expect(toggle).toHaveAttribute("aria-expanded", "false");

  await toggle.click();
  await expect(toggle).toHaveAttribute("aria-expanded", "true");
  const editor = page.getByRole("textbox", { name: "笔记内容" });
  await expect(editor).toBeVisible();
  await editor.fill("重点题");
  await page.getByRole("button", { name: "保存笔记" }).click();
  await expect(page.getByText("已保存")).toBeVisible();

  await page.reload();
  // 有笔记的条目会带一个指示点（按钮内含一个圆点元素）
  await expect(page.getByRole("button", { name: "笔记" }).first().locator("span")).toBeVisible();
});

test("快捷键：? 唤起帮助面板，Esc 可关闭", async ({ page }) => {
  await page.goto("/practice");
  await expect(page.getByText(/第 1 \/ \d+ 题/)).toBeVisible();

  await page.keyboard.press("?");
  const help = page.getByRole("dialog", { name: "快捷键" });
  await expect(help).toBeVisible();
  // 键位说明由 SHORTCUT_HELP 渲染，至少包含这几项
  await expect(help.getByText("上一题 / 下一题")).toBeVisible();
  await expect(help.getByText("显示快捷键帮助")).toBeVisible();

  await page.keyboard.press("Escape");
  await expect(help).toBeHidden();
});

test("快捷键：输入框内按 ? 不弹出帮助面板", async ({ page }) => {
  await page.goto("/practice");
  await expect(page.getByText(/第 1 \/ \d+ 题/)).toBeVisible();

  // 打开题目搜索框（顺序模式的 Enter / 顶部按钮），在输入框里打 ?
  await page.keyboard.press("Enter");
  const search = page.getByRole("dialog", { name: /搜索/ }).first();
  const input = search.getByRole("textbox").first();
  await expect(input).toBeVisible();
  await input.fill("?");

  await expect(page.getByRole("dialog", { name: "快捷键" })).toHaveCount(0);
});
