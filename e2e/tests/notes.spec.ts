import type { Page } from "@playwright/test";
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

test("收藏集：题目展示与分类浏览是同一张卡，取消收藏后从列表消失", async ({ page }) => {
  await page.goto("/practice");
  await expect(page.getByText(/第 1 \/ \d+ 题/)).toBeVisible();
  await page.getByRole("button", { name: "收藏" }).first().click();

  await page.goto("/bookmarks");
  await expect(page.getByRole("heading", { level: 1, name: "收藏集" })).toBeVisible();

  // 与分类浏览同一张题目卡：直接铺开正确答案与解析，不再把解析折叠成按钮
  const card = page.locator("main article").first();
  await expect(card.getByText(/^(✓ )?正确答案/).first()).toBeVisible();
  await expect(card.getByRole("button", { name: "答案解析" })).toHaveCount(0);

  // 卡片上的收藏开关处于按下态（两套皮肤的无障碍名不同，按状态而不是名字定位）
  const bookmark = card.getByRole("button", { pressed: true });
  await expect(bookmark).toBeVisible();
  await bookmark.click();
  await expect(page.getByText("暂无收藏")).toBeVisible();
});

// 收藏集的列表行为在两套皮肤下各跑一遍：题目卡、分组区、笔记区是两套不同的 DOM。
// 注意 `E2E_UI_STYLE` 只对 `mobile_overflow` / `i18n_layout` 生效，这里必须在用例里显式切皮肤。
type Style = "pixel" | "modern";
const STYLES: Style[] = ["pixel", "modern"];

/** 预置收藏（A 类前 `count` 题）与分组，以指定皮肤打开收藏集。种子只在文档加载时写入，用例内不要 reload。 */
async function openBookmarks(
  page: Page,
  style: Style,
  count: number,
  groups: { name: string; ids: string[] }[] = [],
) {
  await page.addInitScript(
    ([s, n, g]) => {
      if (window !== window.top) return;
      localStorage.setItem("ui:uiStyle", s);
      localStorage.setItem(
        "bookmarks",
        JSON.stringify(Array.from({ length: n }, (_, i) => `A-${i + 1}`)),
      );
      localStorage.setItem("bookmark-groups", JSON.stringify({ groups: g }));
    },
    [style, count, groups] as const,
  );
  await page.goto("/bookmarks");
  await expect(page.getByRole("heading", { level: 1, name: "收藏集" })).toBeVisible();
}

for (const style of STYLES) {
  test(`收藏集（${style}）：取消一张收藏，别的卡片里展开的笔记与草稿不丢`, async ({ page }) => {
    await openBookmarks(page, style, 3);
    const cards = page.locator("main article");
    await expect(cards).toHaveCount(3);

    await cards.nth(1).getByRole("button", { name: "笔记" }).click();
    const editor = page.getByRole("textbox", { name: "笔记内容" });
    await editor.fill("未保存的草稿");

    await cards.nth(0).getByRole("button", { pressed: true }).click();
    await expect(cards).toHaveCount(2);
    // 列表不能整页重载：重载会重建全部卡片，展开状态与草稿一起消失
    await expect(editor).toBeVisible();
    await expect(editor).toHaveValue("未保存的草稿");
  });

  test(`收藏集（${style}）：取消收藏后分组计数立即更新，且不会被后续分组操作写回`, async ({
    page,
  }) => {
    await openBookmarks(page, style, 3, [{ name: "组甲", ids: ["A-1", "A-2"] }]);
    await expect(page.getByRole("button", { name: /^组甲\s*2$/ })).toBeVisible();

    await page.locator("main article").first().getByRole("button", { pressed: true }).click();
    await expect(page.getByRole("button", { name: /^组甲\s*1$/ })).toBeVisible();

    // 页面里的分组副本过期时，下面这次保存会把已清理的 A-1 写回存储
    await page.getByRole("textbox").first().fill("组乙");
    await page.getByRole("button", { name: "新建分组" }).first().click();
    await expect(page.getByRole("button", { name: /^组乙/ })).toBeVisible();
    const stored = await page.evaluate(() => localStorage.getItem("bookmark-groups") ?? "{}");
    const jia = (JSON.parse(stored).groups as { name: string; ids: string[] }[]).find(
      (g) => g.name === "组甲",
    );
    expect(jia?.ids).toEqual(["A-2"]);
  });

  test(`收藏集（${style}）：收藏很多时分页显示，可加载更多`, async ({ page }) => {
    await openBookmarks(page, style, 100);
    const cards = page.locator("main article");
    await expect(cards.first()).toBeVisible();
    await expect(cards).toHaveCount(40);

    const more = page.getByRole("button", { name: /加载更多（已显示 40 \/ 100）/ });
    await more.click();
    await expect(cards).toHaveCount(80);
    await expect(page.getByRole("button", { name: /加载更多（已显示 80 \/ 100）/ })).toBeVisible();
  });

  test(`收藏集（${style}）：取消收藏后，「加载更多」里的总数跟着减一`, async ({ page }) => {
    await openBookmarks(page, style, 100);
    const cards = page.locator("main article");
    await expect(cards).toHaveCount(40);
    await expect(page.getByRole("button", { name: /加载更多（已显示 40 \/ 100）/ })).toBeVisible();

    // 就地取消收藏：列表闭包不重跑，按钮要自己读到新的总数（按钮若只拿到挂载时的快照，这里会停在 100）。
    await cards.first().getByRole("button", { pressed: true }).click();
    await expect(page.getByRole("button", { name: /加载更多（已显示 40 \/ 99）/ })).toBeVisible();
    await expect(cards).toHaveCount(40);
  });
}

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
