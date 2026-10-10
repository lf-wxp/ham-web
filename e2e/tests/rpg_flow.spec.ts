import { readFileSync } from "node:fs";
import { join } from "node:path";
import AxeBuilder from "@axe-core/playwright";
import type { Page } from "@playwright/test";
import { expect, option, test } from "./fixtures";

// 闯关地图 / 回合战 / 怪物图鉴 / 外观设置 / 底部 Tab 的回归。
// 作答靠读题库文件判对错（与 smoke.spec.ts 读路由表同一思路），不依赖页面会泄露答案的文案。

type Q = { question: string; answer_keys: string[]; options: { key: string }[] };
const bank: Q[] = JSON.parse(readFileSync(join(__dirname, "../../public/questions/A.json"), "utf8"));
const byStem = new Map(bank.map((q) => [q.question.trim(), q]));

const stem = (page: Page) => page.locator("main [data-slot=card-content] .whitespace-pre-line").first();
const finished = (page: Page) => page.getByRole("heading", { name: /^(关卡通过！|复仇成功！|败北…)$/ });

/** 选出当前题的答案；`correct=false` 时保证答错。
 *
 *  答错有两种办法：点一个不在答案里的选项；或（多选题里每个选项都是答案时没有这样的选项）漏选一个。
 *  只写第一种会在随机抽到「全选才对」的多选题时选不出选项 —— 一个靠题库数据碰运气的用例。 */
async function pick(page: Page, correct: boolean) {
  const text = (await stem(page).innerText()).trim();
  const q = byStem.get(text);
  expect(q, `题库里找不到这道题：${text}`).toBeTruthy();
  const want = new Set(q!.answer_keys);
  let letters = q!.answer_keys;
  if (!correct) {
    const wrong = q!.options.map((o) => o.key).find((k) => !want.has(k));
    letters = wrong ? [wrong] : q!.answer_keys.slice(1);
  }
  expect(letters.length, `没法给「${text}」构造一个错误答案`).toBeGreaterThan(0);
  for (const l of letters) await option(page, l).click();
}

/** 出招并进入下一回合（或结算）。 */
async function turn(page: Page, correct: boolean) {
  await pick(page, correct);
  await page.getByRole("button", { name: "攻击", exact: true }).click();
  await page.getByRole("button", { name: /^(继续|结算)$/ }).click();
}

/** 打到结算为止；前 `wrongTurns` 回合故意答错。 */
async function play(page: Page, wrongTurns = 0) {
  for (let i = 0; i < 40; i++) {
    await expect(stem(page).or(finished(page))).toBeVisible();
    if (await finished(page).isVisible()) return;
    await turn(page, i >= wrongTurns);
  }
  throw new Error("40 回合内没有分出胜负");
}

test.describe("闯关地图与回合战", () => {
  test("通关首关：给三星、写入存档，并解锁下一关", async ({ page }) => {
    await page.goto("/map");
    // 只有第一关开放，其余 9 关锁定。
    await expect(page.getByText("未解锁")).toHaveCount(9);
    await expect(page.getByText(/已通关 0 \/ 10 关/)).toBeVisible();

    await page.getByRole("link", { name: "开始战斗" }).click();
    await expect(page).toHaveURL(/\/battle\?.*stage=/);
    await play(page);

    await expect(finished(page)).toHaveText("关卡通过！");
    // 全程没答错：满血通关 = 三星。
    await expect(page.getByRole("img", { name: "3 / 3 星" })).toBeVisible();
    await expect.poll(() => page.evaluate(() => localStorage.getItem("rpg-stars"))).toContain('"best"');

    await page.getByRole("link", { name: "返回地图" }).click();
    await expect(page.getByText(/已通关 1 \/ 10 关/)).toBeVisible();
    await expect(page.getByText("未解锁")).toHaveCount(8);
  });

  test("答错：扣一点生命、题目稍后重出，并收进图鉴", async ({ page }) => {
    await page.goto("/map");
    await page.getByRole("link", { name: "开始战斗" }).click();
    await expect(page.getByText("第 1 / 10 题")).toBeVisible();

    await turn(page, false);
    // 错题放回队尾：总题数 +1；生命 5 → 4。
    await expect(page.getByText("第 2 / 11 题")).toBeVisible();
    await expect(page.getByText("生命 4 / 5")).toBeVisible();

    await page.goto("/bestiary");
    await expect(page.getByRole("link", { name: "挑战", exact: true })).toHaveCount(1);
  });

  test("直接输入未解锁关卡的地址：被拦下并指回地图", async ({ page }) => {
    await page.goto("/battle?bank=A&stage=调制");
    await expect(page.getByRole("alert")).toContainText("还没解锁");
    await page.getByRole("link", { name: "返回地图" }).click();
    await expect(page).toHaveURL(/\/map/);
  });

  test("没有战斗目标：给出提示而不是空白", async ({ page }) => {
    await page.goto("/battle");
    await expect(page.getByRole("alert")).toContainText("没有指定战斗目标");
  });
});

test.describe("怪物图鉴", () => {
  test("空图鉴：引导去闯关", async ({ page }) => {
    await page.goto("/bestiary");
    await expect(page.getByText("图鉴还是空的")).toBeVisible();
    await page.getByRole("link", { name: "去闯关" }).click();
    await expect(page).toHaveURL(/\/map/);
  });

  test("复仇：连续答对到掌握后，怪物记入已击败", async ({ page }) => {
    await page.goto("/map");
    await page.getByRole("link", { name: "开始战斗" }).click();
    await turn(page, false); // 制造一只怪物
    await page.goto("/battle?mode=revenge");
    // 掌握需要连续答对 3 次，且复仇队不开暴击 —— 一场只有一回合，所以要打三场。
    for (let round = 0; round < 3; round++) {
      await play(page);
      await expect(finished(page)).toHaveText("复仇成功！");
      if (round < 2) await page.goto("/battle?mode=revenge");
    }
    await expect.poll(() => page.evaluate(() => localStorage.getItem("rpg-bestiary"))).toContain("defeated");
    await page.goto("/bestiary");
    // 只制造了一只怪物且已被击败：图鉴进度 100%。别写成 `not.toContainText("0%")` ——
    // 「100%」本身就含有子串「0%」。
    await expect(page.getByText("已击败 100%")).toBeVisible();
  });
});

test.describe("外观与动效设置", () => {
  test("像素动效 / 易读字体：写入偏好、作用于 <html>、刷新后保留", async ({ page }) => {
    await page.goto("/");
    const html = page.locator("html");
    await expect(html).toHaveAttribute("data-pixel-motion", "on");

    await page.getByRole("button", { name: "外观与动效" }).first().click();
    const dialog = page.getByRole("dialog", { name: "外观与动效" });
    await dialog.getByRole("switch", { name: "像素动效" }).click();
    await dialog.getByRole("switch", { name: "易读字体" }).click();
    await expect(html).toHaveAttribute("data-pixel-motion", "off");
    await expect(html).toHaveAttribute("data-readable-font", "on");
    await expect.poll(() => page.evaluate(() => localStorage.getItem("ui:pixelMotion"))).toBe("off");

    await page.reload();
    await expect(html).toHaveAttribute("data-pixel-motion", "off");
    await expect(html).toHaveAttribute("data-readable-font", "on");
  });

  // 数「正在跑且时长 > 0」的 CSS 动画：时长被压成 0 的等价于直接落在终态，不算在动。
  const running = (page: Page) =>
    page.evaluate(
      () =>
        [...document.querySelectorAll("*")].filter((e) => {
          const cs = getComputedStyle(e);
          return cs.animationName !== "none" && cs.animationPlayState === "running" && parseFloat(cs.animationDuration) > 0.011;
        }).length,
    );

  test("默认有像素动画；关掉「像素动效」后全部静止", async ({ page }) => {
    await page.goto("/");
    await expect.poll(() => running(page)).toBeGreaterThan(0);
    await page.evaluate(() => localStorage.setItem("ui:pixelMotion", "off"));
    await page.reload();
    await expect(page.locator("html")).toHaveAttribute("data-pixel-motion", "off");
    await expect.poll(() => running(page)).toBe(0);
  });

  test.describe("系统开启「减少动态效果」", () => {
    test.use({ reducedMotion: "reduce" });
    test("无动画在跑", async ({ page }) => {
      await page.goto("/");
      await expect(page.locator("main h1").first()).toBeVisible();
      await expect.poll(() => running(page)).toBe(0);
    });
  });
});

test.describe("手机底部 Tab", () => {
  test.use({ viewport: { width: 375, height: 740 } });

  test("基地 / 地图 / 考试 / 图鉴 / 更多：可跳转，顶栏不撑出屏幕", async ({ page }) => {
    await page.goto("/");
    const tabs = page.getByRole("navigation", { name: "底部导航" });
    await expect(tabs).toBeVisible();
    await expect(tabs.getByRole("link", { name: "基地" })).toHaveAttribute("aria-current", "page");

    const overflow = await page.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth);
    expect(overflow, "文档横向溢出").toBeLessThanOrEqual(0);

    await tabs.getByRole("link", { name: "地图" }).click();
    await expect(page).toHaveURL(/\/map/);
    await expect(tabs.getByRole("link", { name: "地图" })).toHaveAttribute("aria-current", "page");

    await tabs.getByRole("button", { name: "更多" }).click();
    await expect(page.getByRole("navigation", { name: "移动端导航" })).toBeVisible();
  });

  test("答题页让位：战斗中不显示底部 Tab", async ({ page }) => {
    await page.goto("/map");
    await page.getByRole("link", { name: "开始战斗" }).click();
    await expect(stem(page)).toBeVisible();
    await expect(page.getByRole("navigation", { name: "底部导航" })).toBeHidden();
  });
});

// 现有的 `i18n_layout.spec.ts` 只在 1280px 量：手机与平板宽度下顶栏（品牌 + 语言 + HUD + 汉堡）
// 在西语 / 英语里最容易被撑出屏幕，曾经 320–768px 多个宽度都溢出过。
test.describe("顶栏：各宽度 × 各语言都不撑出屏幕", () => {
  for (const lang of ["zh", "en", "es"]) {
    test(`${lang}`, async ({ page }) => {
      await page.addInitScript((code) => localStorage.setItem("locale", code), lang);
      const bad: string[] = [];
      for (const width of [320, 375, 640, 768, 1024, 1280]) {
        await page.setViewportSize({ width, height: 800 });
        await page.goto("/");
        await expect(page.locator("[data-nav]")).toBeVisible();
        await waitSettledNav(page);
        const over = await page.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth);
        if (over > 0) bad.push(`${width}px 溢出 ${over}px`);
      }
      expect(bad, "文档横向溢出").toEqual([]);
    });
  }
});

/** 等顶栏里的异步内容（语言包、HUD 档案）到位再量，否则量到的是中间态。 */
async function waitSettledNav(page: Page) {
  await page.waitForFunction(() => document.querySelector("[data-slot=hud]") !== null);
  await page.waitForTimeout(400);
}

// 冒烟用例访问 `/battle` 不带参数，只会扫到「没有战斗目标」的错误页；真正用到文字色 token 的
// 反馈条、结算窗口、Boss 判词得走到那个状态才扫得到。
async function contrast(page: Page) {
  const res = await new AxeBuilder({ page })
    .withTags(["wcag2a", "wcag2aa", "wcag21a", "wcag21aa", "best-practice"])
    .exclude(".leaflet-container")
    .analyze();
  return res.violations.map((v) => `${v.id}: ${v.nodes.slice(0, 3).map((n) => n.target.join(" ")).join(" | ")}`);
}

// 减少动态效果：入场动画（首页分栏卡片要 ~800ms 才落定）的中间帧不透明度 < 1，axe 会按混合后的
// 颜色算对比度 —— 通过与否就取决于机器快慢。关掉动画，量的才是用户最终看到的状态。
for (const colorScheme of ["light", "dark"] as const) {
  test.describe(`对战各状态的无障碍（${colorScheme}）`, () => {
    test.use({ colorScheme, reducedMotion: "reduce" });

    test("战斗中 → 答错反馈 → 答对反馈 → 结算", async ({ page }) => {
      await page.goto("/map");
      await page.getByRole("link", { name: "开始战斗" }).click();
      await expect(stem(page)).toBeVisible();
      expect(await contrast(page), "战斗中").toEqual([]);

      await pick(page, false);
      await page.getByRole("button", { name: "攻击", exact: true }).click();
      await expect(page.getByText("受伤了…")).toBeVisible();
      expect(await contrast(page), "答错反馈").toEqual([]);
      await page.getByRole("button", { name: "继续", exact: true }).click();

      await pick(page, true);
      await page.getByRole("button", { name: "攻击", exact: true }).click();
      await expect(page.getByText("命中！")).toBeVisible();
      expect(await contrast(page), "答对反馈").toEqual([]);
      await page.getByRole("button", { name: "继续", exact: true }).click();

      await play(page);
      await expect(finished(page)).toBeVisible();
      expect(await contrast(page), "结算").toEqual([]);
    });

    test("基地的任务栏、图鉴与地图", async ({ page }) => {
      for (const route of ["/", "/map", "/bestiary"]) {
        await page.goto(route);
        await expect(page.locator("main h1").first()).toBeAttached();
        expect(await contrast(page), route).toEqual([]);
      }
    });
  });
}

test.describe("模拟考试 Boss 战", () => {
  const submit = async (page: Page) => {
    await page.getByRole("button", { name: "交卷", exact: true }).first().click();
    await page.getByRole("dialog", { name: "确认交卷？" }).getByRole("button", { name: "确认交卷" }).click();
    return page.getByRole("dialog", { name: "成绩" });
  };

  test("作答期间 Boss 被封印、血量满格，不泄露对错", async ({ page }) => {
    await page.goto("/exam");
    const stage = page.getByRole("region", { name: "考场 Boss" });
    await expect(stage).toContainText("封印中");
    await expect(stage).toContainText("封印解除 0 / 40");
    await expect(stage.getByRole("progressbar", { name: "Boss 生命" })).toHaveAttribute("aria-valuenow", "100");

    // 答一题（无论对错）：Boss 血量不变，只有「封印解除」进度推进。
    await option(page, "A").click();
    await expect(stage).toContainText("封印解除 1 / 40");
    await expect(stage.getByRole("progressbar", { name: "Boss 生命" })).toHaveAttribute("aria-valuenow", "100");
  });

  for (const colorScheme of ["light", "dark"] as const) {
    test(`交卷空卷：评级 C、Boss 满血、战利品只有参与经验（${colorScheme}）`, async ({ page }) => {
      await page.emulateMedia({ colorScheme, reducedMotion: "reduce" });
      await page.goto("/exam");
      await expect(page.getByText(/第 1 \/ 40 题/).first()).toBeVisible();
      const result = await submit(page);
      await expect(result).toBeVisible();

      const verdict = result.getByRole("region", { name: "Boss 还活着…" });
      await expect(verdict).toContainText("C");
      await expect(verdict).toContainText("Boss 剩余生命 100%");
      // 没及格也没满分：只有参与经验 30，没有合格 / 满分的额外奖励。
      await expect(verdict).toContainText("战利品 +30 经验");
      expect(await contrast(page), "Boss 判词").toEqual([]);

      // 交卷后舞台上的 Boss 血量同步（空卷：仍是满血）。
      await result.getByRole("button", { name: "继续浏览题目" }).click();
      const stage = page.getByRole("region", { name: "考场 Boss" });
      await expect(stage).toContainText("Boss 剩余生命 100%");
    });
  }

  test("满分交卷：评级 S、Boss 倒下、战利品含合格与满分奖励", async ({ page }) => {
    // 把 `/exam` 抽到的题目对上题库，逐题选出正确答案；40 题全对 = 满分。
    await page.goto("/exam");
    await expect(page.getByText(/第 1 \/ 40 题/).first()).toBeVisible();
    for (let i = 0; i < 40; i++) {
      const text = (await page.locator("main [data-slot=card-content] .whitespace-pre-line").first().innerText()).trim();
      const q = byStem.get(text);
      expect(q, `题库里找不到这道题：${text}`).toBeTruthy();
      for (const l of q!.answer_keys) await option(page, l).click();
      if (i < 39) await page.getByRole("button", { name: "下一题", exact: true }).first().click();
    }
    const result = await submit(page);
    const verdict = result.getByRole("region", { name: "Boss 被击败！" });
    await expect(verdict).toContainText("S");
    await expect(verdict).toContainText("Boss 剩余生命 0%");
    // 参与 30 + 合格 120 + 满分 200。
    await expect(verdict).toContainText("战利品 +350 经验");
    // 经验由存档推算：这一场之后 HUD 等级应已上升（经验 = 40 题 × 10 + 350）。
    await page.goto("/");
    await expect(page.locator("[data-slot=hud]")).not.toHaveAttribute("aria-label", /^等级 1 /);
  });
});
