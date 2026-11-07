import type { Page } from "@playwright/test";
import {
  bankQuestion,
  bankQuestions,
  confirmDialog,
  correctLetters,
  expect,
  explanationHead,
  mistakeRecord,
  option,
  settleFrames,
  test,
  waitSettled,
  wrongLetter,
} from "./fixtures";

test("练习：答错后翻页，题号前进且错题本记录该题", async ({ page }) => {
  await page.goto("/practice");
  await expect(page.getByText(/第 1 \/ \d+ 题/)).toBeVisible();

  const correct = await correctLetters(page);
  await option(page, wrongLetter(correct)).click();
  await page.getByRole("button", { name: "下一题" }).first().click();
  await expect(page.getByText(/第 2 \/ \d+ 题/)).toBeVisible();

  await page.goto("/mistakes");
  await expect(page.getByRole("heading", { level: 1, name: "错题集" })).toBeVisible();
  await expect(page.getByText(/共 1 道错题/)).toBeVisible();
});

test("练习：未作答不显示正确答案，「显示正确答案」只展开、不写作答", async ({ page }) => {
  await page.goto("/practice");
  await expect(page.getByText(/第 1 \/ \d+ 题/)).toBeVisible();
  // 答案行不能在没作答时就出现，否则练习等于开卷。
  await expect(page.getByText(/^正确答案：/)).toHaveCount(0);
  await expect(page.getByText(/已作答 0 \/ \d+/).first()).toBeVisible();

  await page.getByRole("button", { name: "显示正确答案" }).click();
  await expect(page.getByText(/^正确答案：/)).toBeVisible();
  await expect(page.getByRole("button", { name: "显示正确答案" })).toHaveCount(0);
  // 只是看了一眼：进度仍是 0，没有写作答。
  await expect(page.getByText(/已作答 0 \/ \d+/).first()).toBeVisible();
});

test("练习：看过答案再答对不计入作答统计，自己答对的才计", async ({ page }) => {
  await page.goto("/practice");
  await expect(page.getByText(/第 1 \/ \d+ 题/)).toBeVisible();

  // 第 1 题：先看答案再照着选对 —— 不是自测，不该算进「本周作答」。
  for (const letter of await correctLetters(page)) {
    await option(page, letter).click();
  }
  await page.getByRole("button", { name: "下一题" }).first().click();
  await expect(page.getByText(/第 2 \/ \d+ 题/)).toBeVisible();

  // 第 2 题：没看答案，先随手选一个，再按页面给出的答案改成正确的（答案行作答后才出现）。
  await option(page, "A").click();
  const correct = await correctLetters(page);
  for (const letter of correct) {
    if (!(await option(page, letter).isChecked())) await option(page, letter).click();
  }
  for (const letter of ["A", "B", "C", "D"]) {
    const o = option(page, letter);
    if ((await o.count()) > 0 && !correct.includes(letter) && (await o.isChecked())) await o.click();
  }
  await page.getByRole("button", { name: "下一题" }).first().click();
  await expect(page.getByText(/第 3 \/ \d+ 题/)).toBeVisible();

  await page.goto("/weekly");
  const card = page.getByText("本周作答", { exact: true }).locator("xpath=..");
  await expect(card).toHaveText(/^1\s*本周作答$/);
});

test("练习：刷新后可恢复进度", async ({ page }) => {
  await page.goto("/practice");
  await expect(page.getByText(/第 1 \/ \d+ 题/)).toBeVisible();
  // 至少作答 3 题才会提示恢复（RESUME_MIN_ANSWERED）
  for (const n of [2, 3, 4]) {
    await option(page, "A").click();
    await page.getByRole("button", { name: "下一题" }).first().click();
    await expect(page.getByText(new RegExp(`第 ${n} / \\d+ 题`))).toBeVisible();
  }

  await page.reload();
  const resume = page.getByRole("dialog", { name: "发现上次练习记录" });
  await expect(resume).toBeVisible();
  await resume.getByRole("button", { name: "继续上次" }).click();
  await expect(resume).toBeHidden();
  await expect(page.getByText(/第 4 \/ \d+ 题/)).toBeVisible();
});

test("模拟考试：作答并交卷后显示成绩", async ({ page }) => {
  await page.goto("/exam");
  await expect(page.getByText(/考试类别：/).first()).toBeVisible();
  await option(page, "A").click();
  await expect(page.getByText(/已作答 1 \/ \d+/).first()).toBeVisible();

  await page.getByRole("button", { name: "交卷", exact: true }).first().click();
  const confirm = page.getByRole("dialog", { name: "确认交卷？" });
  await expect(confirm).toBeVisible();
  await confirm.getByRole("button", { name: "确认交卷" }).click();

  const result = page.getByRole("dialog", { name: "成绩" });
  await expect(result).toBeVisible();
  await expect(result.getByText(/得分：\d+ \/ \d+/)).toBeVisible();
  await expect(result.getByText(/^不合格/)).toBeVisible();
  await result.getByRole("button", { name: "继续浏览题目" }).click();
  await expect(result).toBeHidden();
  await expect(page.getByRole("button", { name: "已交卷" }).first()).toBeVisible();

  // 交卷后回顾：没作答的题也要直接看到正确答案，不能再逐题点「显示正确答案」。
  await page.getByRole("button", { name: "下一题" }).first().click();
  await expect(page.getByText(/^正确答案：/)).toBeVisible();
  await expect(page.getByRole("button", { name: "显示正确答案" })).toHaveCount(0);
});

test("薄弱项组卷：按常规配额出卷，交卷后显示分类对比且不计入历史趋势", async ({ page }) => {
  await page.goto("/exam?bank=A&mode=weak");
  await expect(page.getByRole("heading", { level: 1, name: "薄弱项组卷" })).toBeAttached();
  await expect(page.getByText(/试题数：40（单选 32，多选 8）/).first()).toBeVisible();
  await option(page, "A").click();
  await page.getByRole("button", { name: "交卷", exact: true }).first().click();
  await page.getByRole("dialog", { name: "确认交卷？" }).getByRole("button", { name: "确认交卷" }).click();

  const result = page.getByRole("dialog", { name: "成绩" });
  await expect(result.getByText("分类对比（本次 / 以往）")).toBeVisible();
  await expect(result.getByText(/不计入备考状态/)).toBeVisible();
  const history = await page.evaluate(() => JSON.parse(localStorage.getItem("exam-history") ?? "[]"));
  expect(history).toHaveLength(1);
  expect(history[0].weak).toBe(true);
  expect(await page.evaluate(() => Object.keys(localStorage).some((k) => k.startsWith("exam:savedState")))).toBe(false);
});

test("错题本：重练答对后提示正确", async ({ page }) => {
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
});

test("错题本：显示记忆巩固分布", async ({ page }) => {
  // 造一条错题（刚答错，复习间隔为 0，落在「学习中」档）
  await page.goto("/practice");
  await expect(page.getByText(/第 1 \/ \d+ 题/)).toBeVisible();
  const correct = await correctLetters(page);
  await option(page, wrongLetter(correct)).click();
  await page.getByRole("button", { name: "下一题" }).first().click();
  await expect(page.getByText(/第 2 \/ \d+ 题/)).toBeVisible();

  await page.goto("/mistakes");
  await expect(page.getByRole("heading", { level: 1, name: "错题集" })).toBeVisible();
  await expect(page.getByRole("heading", { name: "记忆巩固" })).toBeVisible();
  await expect(page.getByText("学习中 1 道")).toBeVisible();
});

test("错题集：题目展示与收藏集是同一张卡，带题目标签、答案解析默认展示", async ({ page }) => {
  const q = bankQuestion(0);
  await page.addInitScript(
    (records) => localStorage.setItem("mistake-book", JSON.stringify({ records })),
    [mistakeRecord(q)],
  );

  await page.goto("/mistakes");
  await expect(page.getByRole("heading", { level: 1, name: "错题集" })).toBeVisible();

  const card = page.locator("main article").first();
  // 题目标签：题号（`A-1` 这类，本身带题库类字母）与官方分类码
  await expect(card.getByText(q.id, { exact: true })).toBeVisible();
  const codes = q.codes as { J: string; P: string };
  await expect(card.getByText(codes.P, { exact: true })).toBeVisible();
  // 答案解析不再折叠成按钮：不用点开，正文已经在页面上
  await expect(card.getByRole("button", { name: "答案解析" })).toHaveCount(0);
  await expect(card).toContainText(explanationHead(q));
  // 错题独有的信息仍在卡片末尾
  await expect(card.getByText(/^我的答案：/)).toBeVisible();
  await expect(card.getByText("错因", { exact: true })).toBeVisible();
});

test("错题集：解析读题库现在的，题库认不出的题才沿用存档里的", async ({ page }) => {
  const q = bankQuestion(0);
  // 与题库同一道题，但存档里的解析是订正前的旧文字
  const stale = mistakeRecord({ ...q, explanation: "过期的旧解析" });
  // 题库里没有这道题（编号和题干都对不上）：只能用存档里的
  const orphan = mistakeRecord({
    ...bankQuestion(1),
    id: "A-99999",
    question: "只存在于存档里的题干",
    explanation: "只存在于存档里的解析",
  });
  await page.addInitScript(
    (records) => localStorage.setItem("mistake-book", JSON.stringify({ records })),
    [stale, orphan],
  );

  await page.goto("/mistakes");
  await expect(page.getByRole("heading", { level: 1, name: "错题集" })).toBeVisible();
  const main = page.locator("main");
  // 解析默认铺开：等题库异步加载完、现在的解析换上来
  await expect(main).toContainText(explanationHead(q));
  await expect(page.getByText("过期的旧解析")).toHaveCount(0);
  await expect(main).toContainText("只存在于存档里的解析");

  // 换上来的是展示，不改存档
  const saved = await page.evaluate(() => localStorage.getItem("mistake-book") ?? "");
  expect(saved).toContain("过期的旧解析");
});

test("错题集：题库解析晚到时，卡片先显示存档里的解析、到了只换文字", async ({ page }) => {
  const q = bankQuestion(0);
  // 先把题库响应扣住：模拟弱网下索引迟迟不到
  let release: () => void = () => {};
  const gate = new Promise<void>((resolve) => {
    release = resolve;
  });
  await page.route("**/questions/A.json*", async (route) => {
    await gate;
    await route.continue();
  });
  await page.addInitScript(
    (records) => localStorage.setItem("mistake-book", JSON.stringify({ records })),
    [mistakeRecord({ ...q, explanation: "过期的旧解析" })],
  );

  try {
    await page.goto("/mistakes");
    // 索引没到之前沿用存档里的（解析默认铺开，不用点开）
    await expect(page.getByText("过期的旧解析")).toBeVisible();
    // 给卡片打个记号：之后它还在，就说明只换了文字、没有重建整张卡
    await page.locator("main article").first().evaluate((el) => el.setAttribute("data-probe", "1"));
  } finally {
    release();
  }

  await expect(page.locator("main")).toContainText(explanationHead(q));
  await expect(page.getByText("过期的旧解析")).toHaveCount(0);
  await expect(page.locator("main article").first()).toHaveAttribute("data-probe", "1");
});

// 错题集的列表行为在两套皮肤下各跑一遍：题目卡是两套不同的 DOM。
// `E2E_UI_STYLE` 只对 `mobile_overflow` / `i18n_layout` 生效，这里必须在用例里显式切皮肤。
type Style = "pixel" | "modern";
const STYLES: Style[] = ["pixel", "modern"];

/** 以指定皮肤、预置错题打开错题集。种子只在文档加载时写入，用例内不要 reload。 */
async function openMistakes(page: Page, style: Style, records: unknown[], lang = "zh") {
  await page.addInitScript(
    ([s, rs, code]) => {
      if (window !== window.top) return;
      localStorage.setItem("ui:uiStyle", s);
      localStorage.setItem("locale", code);
      localStorage.setItem("mistake-book", JSON.stringify({ records: rs }));
    },
    [style, records, lang] as const,
  );
  await page.goto("/mistakes");
  // 中文有稳定的页标题可等；其它语言等第一张卡渲染出来
  if (lang === "zh") {
    await expect(page.getByRole("heading", { level: 1, name: "错题集" })).toBeVisible();
  } else {
    await expect(page.locator("main article").first()).toBeVisible();
  }
}

for (const style of STYLES) {
  test(`错题集（${style}）：错题很多时分页显示，可加载更多，切换题库回到第一页`, async ({
    page,
  }) => {
    const aTexts = new Set(
      Array.from({ length: 100 }, (_, i) => bankQuestion(i, "A").question),
    );
    // 只取题面不与前 100 道 A 类重复的 B 类题，否则会被并成同一条错题
    const onlyB = bankQuestions("B")
      .filter((q) => q.id.startsWith("B-") && !aTexts.has(q.question))
      .slice(0, 5);
    expect(onlyB).toHaveLength(5);
    const records = [
      ...Array.from({ length: 100 }, (_, i) => mistakeRecord(bankQuestion(i, "A"))),
      ...onlyB.map((q) => mistakeRecord(q)),
    ];
    await openMistakes(page, style, records);
    const cards = page.locator("main article");
    await expect(cards.first()).toBeVisible();
    await expect(cards).toHaveCount(40);

    await page.getByRole("button", { name: /加载更多（已显示 40 \/ 105）/ }).click();
    await expect(cards).toHaveCount(80);

    // 筛选变了回到第一页：B 类只有 5 道，没有「加载更多」
    await page.getByRole("radio", { name: "B 类" }).click();
    await expect(cards).toHaveCount(5);
    await expect(page.getByRole("button", { name: /加载更多/ })).toHaveCount(0);

    // 切回全部：不沿用之前展开到的 80，而是从第一页重新数
    await page.getByRole("radio", { name: "全部" }).click();
    await expect(cards).toHaveCount(40);
    await expect(page.getByRole("button", { name: /加载更多（已显示 40 \/ 105）/ })).toBeVisible();
  });

  test(`错题集（${style}）：我选错的项标有「我的选择」，正确项不被误标`, async ({ page }) => {
    const q = bankQuestion(0);
    const wrong = wrongLetter(q.answer_keys);
    await openMistakes(page, style, [{ ...mistakeRecord(q), my_answer: [wrong] }]);

    const card = page.locator("main article").first();
    // 文字标记（不只靠颜色），且只标选中的那一项
    await expect(card.getByText("我的选择", { exact: true })).toHaveCount(1);
    const picked = card.locator('[data-chosen="true"]');
    await expect(picked).toHaveCount(1);
    await expect(picked).toContainText(wrong);
    await expect(picked).toHaveAttribute("data-right", "false");
  });

  test(`错题集（${style}）：移出一道错题，别的卡片不重建`, async ({ page }) => {
    const records = [0, 1, 2].map((i) => mistakeRecord(bankQuestion(i)));
    await openMistakes(page, style, records);
    const cards = page.locator("main article");
    await expect(cards).toHaveCount(3);

    // 给最后一张卡打记号：它在移出别的卡之后还在，就说明列表是按 key 增删而不是整体重建
    await cards.nth(2).evaluate((el) => el.setAttribute("data-probe", "1"));
    // 「移出」的无障碍名带题号，一屏多张卡时读屏分得清
    await cards
      .nth(0)
      .getByRole("button", { name: /^移出错题 / })
      .click();
    await confirmDialog(page, "移出");

    await expect(cards).toHaveCount(2);
    await expect(page.locator('main article[data-probe="1"]')).toHaveCount(1);
  });
}

// 错题集卡片末尾（现代风专属 `md-mistake-*` 版式）：三行各自折行，三语 × 窄 / 宽屏都不能撑出卡片。
// 全站版式审计（`mobile_overflow` / `i18n_layout`）只打开空的错题页，卡片页脚不在它们的覆盖里，
// 所以这里自己量；种子带「错因」与较低的难度系数，让选中胶囊（加粗）与语义色都出现。
const FOOT_LANGS = [
  { code: "zh", remove: /^移出错题 / },
  { code: "en", remove: /^Remove mistake / },
  { code: "es", remove: /^Quitar el error / },
] as const;
const footRecord = () => ({ ...mistakeRecord(bankQuestion(0)), cause: "careless", ease: 1.5 });

for (const lang of FOOT_LANGS) {
  for (const width of [320, 1280] as const) {
    test(`错题集（modern·${lang.code}）：${width}px 下页脚三行折行、不撑出卡片`, async ({
      page,
    }) => {
      await page.setViewportSize({ width, height: 900 });
      await openMistakes(page, "modern", [footRecord()], lang.code);
      const card = page.locator("main article").first();
      // 「移出」的名字带题号，也借它等语言包到位（语言包异步加载，先量会量到中文）
      await expect(card.getByRole("button", { name: lang.remove })).toBeVisible();
      await waitSettled(page);

      const m = await card.evaluate((el) => {
        const foot = el.querySelector(".md-mistake-foot");
        if (!foot) return { missing: true as const };
        const box = foot.getBoundingClientRect();
        const visible = [...foot.querySelectorAll("*")].filter((n) => {
          const r = n.getBoundingClientRect();
          return r.width > 0 && r.height > 0;
        });
        return {
          missing: false as const,
          rows: [...foot.children].map((c) => c.className.split(" ")[0]),
          // 越过页脚左右边的元素：折行失败（被撑出）时它们会越界
          outside: visible
            .filter((n) => {
              const r = n.getBoundingClientRect();
              return r.left < box.left - 1 || r.right > box.right + 1;
            })
            .map((n) => `${n.tagName}.${n.className}`),
          footOverflow: foot.scrollWidth - foot.clientWidth,
          docOverflow: document.documentElement.scrollWidth - window.innerWidth,
        };
      });

      expect(m.missing, "现代风错题卡没有渲染专属页脚 .md-mistake-foot").toBe(false);
      if (m.missing) return;
      // 复盘 / 进度 / 错因自评三行，结构不能悄悄塌回一堆行内文字
      expect(m.rows).toEqual(["md-mistake-row", "md-mistake-stats", "md-mistake-row"]);
      expect(m.outside, `页脚里有元素越出了卡片内沿：${m.outside.join(", ")}`).toEqual([]);
      expect(m.footOverflow, "页脚内容横向溢出").toBeLessThanOrEqual(0);
      expect(m.docOverflow, "文档横向溢出").toBeLessThanOrEqual(0);
    });
  }
}

test.describe("错题集页脚（现代风·触屏）", () => {
  test.use({ hasTouch: true, isMobile: true, viewport: { width: 390, height: 844 } });

  test("「移出」与错因胶囊的命中区不小于 44px", async ({ page }) => {
    await openMistakes(page, "modern", [footRecord()]);
    const foot = page.locator("main article").first().locator(".md-mistake-foot");
    await expect(foot).toBeVisible();
    const targets = [
      foot.getByRole("button", { name: /^移出错题 / }),
      ...(await foot.getByRole("button", { pressed: false }).all()),
    ];
    expect(targets.length, "页脚里没找到可点目标").toBeGreaterThanOrEqual(2);
    for (const t of targets) {
      const b = await t.boundingBox();
      expect(b?.height ?? 0, `命中区太矮：${await t.innerText()}`).toBeGreaterThanOrEqual(43.5);
    }
  });
});

test.describe("首次进入", () => {
  test.use({ helpSeen: false });

  test("练习页自动弹出快捷键说明，仅一次", async ({ page }) => {
    await page.goto("/practice");
    const help = page.getByRole("dialog", { name: "设置" });
    await expect(help).toBeVisible();
    await page.keyboard.press("Escape");
    await expect(help).toBeHidden();
    await page.reload();
    await expect(page.getByText(/第 1 \/ \d+ 题/)).toBeVisible();
    // 负向断言：等「该弹的都弹过了」（加载占位消失 + 入场动画结束）再确认它没出现。
    await waitSettled(page);
    await settleFrames(page);
    await expect(help).toBeHidden();
  });

  // 触屏上考试设置里的键位表只在现代风被隐藏（`.md-shortcut-list`）：
  // 自动弹出的弹层只剩几个常规设置，白白挡住首屏 → 现代风不弹、直接记成已看过；
  // 像素风的键位表仍在（带键盘的平板用得上），首次进入照常弹出。
  test.describe("考试页 · 触屏", () => {
    test.use({ viewport: { width: 390, height: 844 }, hasTouch: true, isMobile: true });

    test("现代风：不自动弹出设置，并记成已看过", async ({ page }) => {
      await page.addInitScript(() => {
        if (window !== window.top) return;
        localStorage.setItem("ui:uiStyle", "modern");
      });
      await page.goto("/exam");
      await expect(page.locator("main .md-opt").first()).toBeVisible();
      // 负向断言：等「该弹的都弹过了」再确认它没出现；记号写入是它走了「跳过」分支的正向证据。
      await waitSettled(page);
      await settleFrames(page);
      await expect(page.getByRole("dialog", { name: "设置" })).toBeHidden();
      expect(
        await page.evaluate(() => localStorage.getItem("ui:shortcutsHelpSeen:exam")),
        "现代风触屏应直接记成已看过",
      ).toBe("1");
    });

    test("像素风：仍自动弹出设置", async ({ page }) => {
      await page.goto("/exam");
      await expect(page.getByRole("dialog", { name: "设置" })).toBeVisible();
    });
  });
});
