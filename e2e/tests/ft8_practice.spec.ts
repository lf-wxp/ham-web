import { expect, test, type Page } from "./fixtures";

/**
 * FT8 通话练习（`/ft8`）：对方刚发了这句，下一句该发什么。
 *
 * 场景（对方的呼号、网格、报告）由页面随机生成，所以用例先从页面读出场景，再用**这里独立写的**规则
 * 拼出期望：报文是「收件人 发件人 内容」、收件人在前。核心单测已用 `ham-web-ft8` 的打包逻辑
 * 核对过每条报文的格式，这里钉住页面：选择、输入、判错说明、成绩、角色切换、三语与 320 px 溢出。
 */

const ME = "BG4XYZ";
const GRID = "PM01";

async function open(page: Page, station: boolean = true) {
  await page.addInitScript((s) => {
    if (s) {
      localStorage.setItem(
        "station-info",
        JSON.stringify({ callsign: "BG4XYZ", operator: "", gridsquare: "PM01", rig: "", antenna: "" }),
      );
    }
  }, station);
  await page.goto("/ft8");
  await expect(card(page)).toBeVisible();
}

const card = (page: Page) => page.getByTestId("ft8-practice");
const entries = (page: Page) => card(page).locator("[data-practice-entry]");
const feedback = (page: Page) => card(page).getByTestId("practice-feedback");
const choice = (page: Page, kind: RegExp) => card(page).getByRole("button", { name: kind });
const ownInput = (page: Page) => card(page).getByRole("textbox");

/** 从页面读出这一轮的场景：按「呼号 + 网格」的形状取前两处（我、对方），不依赖界面语言。 */
async function scenario(page: Page) {
  const text = (await card(page).getByTestId("practice-scenario").textContent()) ?? "";
  const pairs = [...text.matchAll(/\b([A-Z0-9]{3,6}) ([A-R]{2}\d\d)/g)];
  const r = /([+-]\d\d) dB/.exec(text);
  expect(pairs.length, `场景行读不出来：${text}`).toBeGreaterThanOrEqual(2);
  expect(r, `场景行里没有报告：${text}`).not.toBeNull();
  return { me: pairs[0][1], meGrid: pairs[0][2], them: pairs[1][1], themGrid: pairs[1][2], myReport: r![1] };
}

const entryText = async (page: Page, i: number) => (await entries(page).nth(i).locator("span").nth(1).textContent())?.trim();

test("通话练习：我回应对方的 CQ，三步各选一条，收尾读到成绩", async ({ page }) => {
  await open(page);
  const sc = await scenario(page);
  expect([sc.me, sc.meGrid]).toEqual([ME, GRID]);

  // 对方的 CQ 先出现；轮到我时能看到它。
  await expect(entries(page)).toHaveCount(1);
  expect(await entryText(page, 0)).toBe(`CQ ${sc.them} ${sc.themGrid}`);
  await expect(card(page).getByTestId("practice-turn")).toContainText(`对方刚发：CQ ${sc.them} ${sc.themGrid}`);
  await expect(card(page).getByTestId("practice-turn")).toContainText("第 1 / 3 次发射");

  // 第 1 步：报网格。收件人在前、发件人在后。
  await choice(page, /^报网格/).click();
  await expect(feedback(page)).toContainText(`对，已发出：${sc.them} ${ME} ${GRID}`);
  await expect(entries(page)).toHaveCount(3);
  expect(await entryText(page, 1)).toBe(`${sc.them} ${ME} ${GRID}`);
  expect(await entryText(page, 2)).toMatch(new RegExp(`^${ME} ${sc.them} [+-]\\d\\d$`));
  await expect(card(page).getByTestId("practice-turn")).toContainText("第 2 / 3 次发射");

  // 第 2 步：对方给了我报告，我回「R + 我给的报告」。
  await choice(page, /^收到并回报/).click();
  await expect(entries(page)).toHaveCount(5);
  expect(await entryText(page, 3)).toBe(`${sc.them} ${ME} R${sc.myReport}`);
  const closing = await entryText(page, 4);
  expect(closing).toMatch(new RegExp(`^${ME} ${sc.them} (RR73|RRR)$`));

  // 第 3 步：不管对方发的是 RR73 还是 RRR，我都回 73。
  await choice(page, /^73 告别/).click();
  await expect(entries(page)).toHaveCount(6);
  expect(await entryText(page, 5)).toBe(`${sc.them} ${ME} 73`);
  await expect(card(page).getByTestId("practice-score")).toHaveText("这一轮结束：3 步里 3 步一次选对，共答错 0 次。");
  // 结束后选项与输入框还在、但都禁用：不再接受回复。
  await expect(choice(page, /^73 告别/)).toBeDisabled();
  await expect(ownInput(page)).toBeDisabled();
});

test("通话练习：我先呼叫 CQ，自己输入报文（小写、多余空格都行），收尾 RRR 也对", async ({ page }) => {
  await open(page);
  await card(page).getByRole("button", { name: "我先呼叫 CQ" }).click();
  const sc = await scenario(page);
  await expect(entries(page)).toHaveCount(0);
  await expect(card(page).getByTestId("practice-turn")).toContainText("你先呼叫 CQ");

  await ownInput(page).fill(`cq  ${ME.toLowerCase()}  ${GRID.toLowerCase()}`);
  await ownInput(page).press("Enter");
  await expect(feedback(page)).toContainText(`对，已发出：CQ ${ME} ${GRID}`);
  await expect(entries(page)).toHaveCount(2);
  expect(await entryText(page, 1)).toBe(`${ME} ${sc.them} ${sc.themGrid}`);

  // 报告：范围内任何带符号两位数都行（报告是我自己听出来的，没有标准答案）。
  await ownInput(page).fill(`${sc.them} ${ME} -19`);
  await card(page).getByRole("button", { name: "发送", exact: true }).click();
  await expect(feedback(page)).toContainText(`对，已发出：${sc.them} ${ME} -19`);
  await expect(entries(page)).toHaveCount(4);
  expect(await entryText(page, 3)).toMatch(new RegExp(`^${ME} ${sc.them} R[+-]\\d\\d$`));

  // 收尾：RRR 与 RR73 都对；记录里写的是我发的那一句。
  await ownInput(page).fill(`${sc.them} ${ME} RRR`);
  await ownInput(page).press("Enter");
  await expect(entries(page)).toHaveCount(6);
  expect(await entryText(page, 4)).toBe(`${sc.them} ${ME} RRR`);
  expect(await entryText(page, 5)).toBe(`${ME} ${sc.them} 73`);
  await expect(card(page).getByTestId("practice-score")).toContainText("3 步里 3 步一次选对");
});

test("通话练习：每一种答错都有对应的说明，答错不推进也不进记录", async ({ page }) => {
  await open(page);
  const sc = await scenario(page);
  const send = async (text: string) => {
    await ownInput(page).fill(text);
    await ownInput(page).press("Enter");
  };
  const wrong = async (text: string, message: string | RegExp) => {
    await send(text);
    await expect(feedback(page), text).toContainText(message);
    await expect(entries(page), "答错不进记录").toHaveCount(1);
  };

  await wrong("", "还没有输入报文");
  await wrong(`${ME} ${sc.them} ${GRID}`, "收件人和发件人写反了");
  await wrong(`W9XYZ ${ME} ${GRID}`, "收件人不对");
  await wrong(`${sc.them} W9XYZ ${GRID}`, "发件人不对");
  await wrong(`${sc.them} ${ME} FN31`, "网格不是你的");
  await wrong(`${sc.them} ${ME} HELLO`, "不是练习范围内的标准报文");
  await wrong(`${sc.them} ${ME}`, "不是练习范围内的标准报文");
  await wrong(`${sc.them} ${ME} -5`, "信号报告不对");
  await wrong(`${sc.them} ${ME} +33`, "信号报告不对");
  // 步骤不对：这一步该报网格，却发了 RR73 / 报告 / 73。
  await wrong(`${sc.them} ${ME} RR73`, "这一步该发「报网格」，你发的是「RR73 收到并告别」");
  await wrong(`${sc.them} ${ME} -07`, "这一步该发「报网格」，你发的是「报信号报告」");
  await wrong(`${sc.them} ${ME} 73`, "这一步该发「报网格」，你发的是「73 告别」");

  // 用选项也一样：点错的那条同样给出「该发什么」。
  await choice(page, /^CQ 呼叫/).click();
  await expect(feedback(page)).toContainText("这一步该发「报网格」，你发的是「CQ 呼叫」");

  // 最后答对，并把前面答错的次数算进成绩：12 次输入 + 1 次选项 = 13 次错。
  await choice(page, /^报网格/).click();
  await choice(page, /^收到并回报/).click();
  await choice(page, /^73 告别/).click();
  await expect(card(page).getByTestId("practice-score")).toHaveText("这一轮结束：3 步里 2 步一次选对，共答错 13 次。");
});

test("通话练习：新一轮重置记录，换角色同样重置", async ({ page }) => {
  await open(page);
  for (const k of [/^报网格/, /^收到并回报/, /^73 告别/]) await choice(page, k).click();
  await expect(card(page).getByTestId("practice-score")).toBeVisible();
  await expect(entries(page)).toHaveCount(6);

  await card(page).getByRole("button", { name: "新一轮" }).click();
  await expect(card(page).getByTestId("practice-score")).toHaveCount(0);
  await expect(entries(page)).toHaveCount(1);
  await expect(feedback(page)).toHaveText("");

  await card(page).getByRole("button", { name: "我先呼叫 CQ" }).click();
  await expect(entries(page)).toHaveCount(0);
  await card(page).getByRole("button", { name: "我回应对方的 CQ" }).click();
  await expect(entries(page)).toHaveCount(1);
});

test("通话练习：七种报文都能点选，没填台站信息时用示例呼号与网格", async ({ page }) => {
  await open(page, false);
  const sc = await scenario(page);
  expect([sc.me, sc.meGrid]).toEqual(["BG1AAA", "OM89"]);
  const names = ["CQ 呼叫", "报网格", "报信号报告", "收到并回报", "RR73 收到并告别", "RRR 收到", "73 告别"];
  for (const n of names) await expect(choice(page, new RegExp(`^${n}`))).toBeVisible();
  await expect(card(page).locator("button", { hasText: /^.*\n?/ }).filter({ hasText: /^(CQ 呼叫|报网格|报信号报告|收到并回报|RR73 收到并告别|RRR 收到|73 告别)/ })).toHaveCount(7);
  // 选项里的报文都是「我发给对方」的完整写法：收件人 = 对方、发件人 = 我。
  await expect(choice(page, /^报网格/)).toContainText(`${sc.them} BG1AAA OM89`);
  await expect(choice(page, /^CQ 呼叫/)).toContainText("CQ BG1AAA OM89");
  await expect(choice(page, /^报信号报告/)).toContainText(`${sc.them} BG1AAA ${sc.myReport}`);
});

test("通话练习：英文 / 西语界面都有译文，不留中文", async ({ page }) => {
  for (const [code, title, wrongStep] of [
    ["en", "FT8 QSO practice", /This step calls for “Send grid”, but you sent “RR73 received, goodbye”/],
    ["es", "Práctica de QSO en FT8", /En este paso toca «Enviar locator», pero enviaste «RR73 recibido, despedida»/],
  ] as const) {
    await page.addInitScript((c) => localStorage.setItem("locale", c), code);
    await open(page);
    const sc = await scenario(page);
    await expect(card(page).getByRole("heading", { name: title })).toBeVisible();
    await ownInput(page).fill(`${sc.them} ${ME} RR73`);
    await ownInput(page).press("Enter");
    await expect(feedback(page)).toContainText(wrongStep);
    await expect(card(page)).not.toContainText(/[\u4e00-\u9fff]/);
  }
});

/** 整页与练习卡片各自的横向溢出（像素，≤ 1 视为没有溢出）。 */
async function overflowOf(page: Page) {
  // 现代风点按钮会放一圈 560 ms 的按压涟漪（`.md-ping`），动画放大时短暂伸出视口，之后自清理。
  await expect(page.locator(".md-ping")).toHaveCount(0);
  return page.evaluate(() => {
    const el = document.querySelector('[data-testid="ft8-practice"]') as HTMLElement;
    return {
      page: document.documentElement.scrollWidth - window.innerWidth,
      card: el.scrollWidth - el.clientWidth,
    };
  });
}

test("通话练习：溢出检测器自测——塞进 900px 宽的元素时必须量得出溢出", async ({ page }) => {
  await page.setViewportSize({ width: 320, height: 800 });
  await open(page);
  const before = await overflowOf(page);
  expect(before.page).toBeLessThanOrEqual(1);
  expect(before.card).toBeLessThanOrEqual(1);
  await page.evaluate(() => {
    const wide = document.createElement("div");
    wide.style.cssText = "width:900px;height:4px";
    document.querySelector('[data-testid="ft8-practice"]')!.appendChild(wide);
  });
  const after = await overflowOf(page);
  expect(after.card, "卡片里放 900px 宽的元素，量到的溢出应远大于 1").toBeGreaterThan(100);
  expect(after.page, "整页同理").toBeGreaterThan(100);
});

// 三语里最长的说明（判错提示与七个选项）在 320 px 下不能把整页或卡片撑出横向滚动；两套皮肤各量一次。
for (const lang of ["zh", "en", "es"]) {
  for (const style of ["pixel", "modern"]) {
    test(`通话练习：320px 下不横向溢出（${lang} · ${style}）`, async ({ page }) => {
      await page.setViewportSize({ width: 320, height: 800 });
      await page.addInitScript(
        ([code, ui]) => {
          localStorage.setItem("locale", code);
          localStorage.setItem("ui:uiStyle", ui);
          if (ui === "modern") localStorage.setItem("theme", "dark");
        },
        [lang, style],
      );
      await open(page);
      const sc = await scenario(page);
      // 让最长的判错说明（「不是标准报文」）与记录行都出现。
      await ownInput(page).fill(`${sc.them} ${ME} HELLO`);
      await ownInput(page).press("Enter");
      await expect(feedback(page)).not.toHaveText("");
      const over = await overflowOf(page);
      expect(over.page, "整页横向溢出").toBeLessThanOrEqual(1);
      expect(over.card, "练习卡片自身横向溢出").toBeLessThanOrEqual(1);
    });
  }
}
