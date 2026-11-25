import { expect, test, type Page } from "./fixtures";

/**
 * 话音通联练习（`/voice-qso`）：对方（语音合成）叫过来，限时回一整段。
 *
 * 语音合成用桩记录「朗读了什么、用的语言标签」；麦克风用 Chromium 的假设备，只断言能开始、能结束、
 * 回放出现，**不断言录到的内容**（页面本来就不做识别）。呼号是否拼读正确用这里独立写的 ITU 字母表反推。
 */

const ME = "BG4XYZ";
const NATO = [
  "Alfa", "Bravo", "Charlie", "Delta", "Echo", "Foxtrot", "Golf", "Hotel", "India", "Juliett", "Kilo", "Lima",
  "Mike", "November", "Oscar", "Papa", "Quebec", "Romeo", "Sierra", "Tango", "Uniform", "Victor", "Whiskey",
  "X-ray", "Yankee", "Zulu",
];
const DIGITS_EN = ["Zero", "One", "Two", "Three", "Four", "Five", "Six", "Seven", "Eight", "Niner"];

/** 把英文拼读（`Bravo Golf Four X-ray`）还原成呼号。 */
function unspell(words: string): string {
  return words
    .trim()
    .split(/\s+/)
    .map((w) => {
      const l = NATO.indexOf(w);
      if (l >= 0) return String.fromCharCode(65 + l);
      const d = DIGITS_EN.indexOf(w);
      expect(d, `不认识的拼读词「${w}」`).toBeGreaterThanOrEqual(0);
      return String(d);
    })
    .join("");
}

/** 所在地对应的呼号前缀（独立写一份）。 */
const PREFIX: Record<string, string> = {
  Boston: "K", Denver: "W", Osaka: "JA", Hamburg: "DL", Leeds: "G", Perth: "VK", Wellington: "ZL",
  "Sao Paulo": "PY", Calgary: "VE", Ghent: "ON", Wuhan: "BG", Chengdu: "BH", Xian: "BA", Harbin: "BD",
};

const ITEMS_ZH = [
  "双方呼号（用字母解释法拼读）",
  "给对方的信号报告（如「五九」）",
  "你的名字",
  "你的 QTH（所在地）",
  "你的设备",
  "你的天线",
  "以 73 收尾，再报双方呼号",
];

test.use({
  launchOptions: { args: ["--use-fake-ui-for-media-stream", "--use-fake-device-for-media-stream"] },
  permissions: ["microphone"],
});

async function open(page: Page, opts: { clock?: boolean; station?: boolean } = {}) {
  await page.addInitScript((station) => {
    if (station) {
      localStorage.setItem(
        "station-info",
        JSON.stringify({ callsign: "BG4XYZ", operator: "", gridsquare: "PM01", rig: "", antenna: "" }),
      );
    }
    // 语音合成桩：记录朗读的文本与语言，不真的出声。
    const spoken: { text: string; lang: string }[] = [];
    (window as unknown as { __spoken: typeof spoken }).__spoken = spoken;
    const synth = window.speechSynthesis;
    synth.speak = ((u: SpeechSynthesisUtterance) => {
      spoken.push({ text: u.text, lang: u.lang });
    }) as typeof synth.speak;
    synth.cancel = (() => {}) as typeof synth.cancel;
  }, opts.station !== false);
  const t0 = new Date("2026-11-25T10:00:00Z");
  if (opts.clock) await page.clock.install({ time: t0 });
  await page.goto("/voice-qso");
  await expect(page.getByTestId("voice-reply")).toBeVisible();
  // `install` 之后时间仍会自然流逝（断言重试的几秒也会被算进去）：钉住，之后只靠 `runFor` 推进。
  if (opts.clock) await page.clock.pauseAt(new Date(t0.getTime() + 60_000));
}

const clockText = (page: Page) => page.getByTestId("voice-clock");
const start = (page: Page) => page.getByRole("button", { name: "开始回复" });
const end = (page: Page) => page.getByRole("button", { name: "回复完毕" });
const item = (page: Page, i: number) => page.locator("[data-voice-item]").nth(i).getByRole("checkbox");
const spoken = (page: Page) =>
  page.evaluate(() => (window as unknown as { __spoken: { text: string; lang: string }[] }).__spoken);

test("话音通联：播放来电朗读的就是页面上的文字，英文用 en-US；呼号按字母解释法拼读且与所在地对得上", async ({ page }) => {
  await open(page);
  await page.getByRole("button", { name: "英文通话" }).click();
  await page.getByRole("button", { name: "看文字" }).click();
  const text = (await page.getByTestId("partner-text").textContent())?.trim() ?? "";

  await page.getByRole("button", { name: "播放来电" }).click();
  const said = await spoken(page);
  expect(said).toHaveLength(1);
  expect(said[0].lang).toBe("en-US");
  expect(said[0].text.trim()).toBe(text);

  // 「X, this is Y. You are five nine. My name is …」：两个呼号都能还原，第一个是我。
  const m = /^(.+?), this is (.+?)\. You are (five \w+)\. My name is (\w+), \4\. My QTH is (.+?)\. I am running a (\S+) and a (.+?)\. /.exec(text);
  expect(m, `来电文字格式不对：${text}`).not.toBeNull();
  expect(unspell(m![1])).toBe(ME);
  const partner = unspell(m![2]);
  expect(partner.startsWith(PREFIX[m![5]]), `${partner} 与 ${m![5]} 的前缀对不上`).toBe(true);
  expect(text.trimEnd().endsWith("over.")).toBe(true);
  // 结尾再报一遍：呼号拼读出现两次。
  expect(text.split(m![2]).length - 1).toBeGreaterThanOrEqual(2);
});

test("话音通联：中文通话用 zh-CN 朗读，文字是中文脚本", async ({ page }) => {
  await open(page);
  await page.getByRole("button", { name: "中文通话" }).click();
  await page.getByRole("button", { name: "看文字" }).click();
  const text = (await page.getByTestId("partner-text").textContent())?.trim() ?? "";
  await page.getByRole("button", { name: "播放来电" }).click();
  const said = await spoken(page);
  expect(said[0].lang).toBe("zh-CN");
  expect(said[0].text.trim()).toBe(text);
  expect(text).toContain("这里是");
  expect(text).toContain("请回答。");
  expect(text).not.toContain("over");
});

test("话音通联：倒计时、恰好用完不算超时、超一秒算一秒，结束后给出用时与超时", async ({ page }) => {
  await open(page, { clock: true });
  await expect(clockText(page)).toHaveText("限时 60 秒");
  await start(page).click();
  await expect(clockText(page)).toHaveText("还剩 60 秒");
  await page.clock.runFor(10_000);
  await expect(clockText(page)).toHaveText("还剩 50 秒");
  await page.clock.runFor(50_000);
  await expect(clockText(page)).toHaveText("还剩 0 秒");
  await page.clock.runFor(1_000);
  await expect(clockText(page)).toHaveText("已超时 1 秒");

  // 七项全勾，仍因超时而不算通过。
  for (let i = 0; i < 7; i++) await item(page, i).click();
  await end(page).click();
  await expect(clockText(page)).toHaveText("用时 61 秒");
  const result = page.getByTestId("voice-result");
  await expect(result.getByTestId("voice-overtime")).toHaveText("超时 1 秒（限时 60 秒）。");
  await expect(result).not.toContainText("七项都说到了");
  await expect(result.getByTestId("voice-missing")).toHaveCount(0);
});

test("话音通联：恰好 60 秒、七项齐全才算通过", async ({ page }) => {
  await open(page, { clock: true });
  await start(page).click();
  await page.clock.runFor(60_000);
  for (let i = 0; i < 7; i++) await item(page, i).click();
  await end(page).click();
  await expect(clockText(page)).toHaveText("用时 60 秒");
  const result = page.getByTestId("voice-result");
  await expect(result).toContainText("七项都说到了，也没有超时。");
  await expect(result.getByTestId("voice-overtime")).toHaveCount(0);
  await expect(result.getByTestId("voice-missing")).toHaveCount(0);
});

test("话音通联：漏项按建议顺序列出，每一项都能单独判漏", async ({ page }) => {
  await open(page, { clock: true });
  await start(page).click();
  await page.clock.runFor(20_000);
  // 只勾名字、呼号、73。
  await item(page, 2).click();
  await item(page, 0).click();
  await item(page, 6).click();
  await end(page).click();
  const missing = page.getByTestId("voice-missing");
  await expect(missing).toHaveText(`漏了：${[ITEMS_ZH[1], ITEMS_ZH[3], ITEMS_ZH[4], ITEMS_ZH[5]].join(" · ")}`);
  await expect(page.getByTestId("voice-result")).not.toContainText("七项都说到了");

  // 结束后还能改自评：补勾一项，漏项列表随之变短。
  await item(page, 3).click();
  await expect(missing).toHaveText(`漏了：${[ITEMS_ZH[1], ITEMS_ZH[4], ITEMS_ZH[5]].join(" · ")}`);
});

test("话音通联：结束后显示对方的文字与参考答案（参考答案七项齐全）", async ({ page }) => {
  await open(page);
  await page.getByRole("button", { name: "英文通话" }).click();
  await expect(page.getByTestId("partner-text")).toHaveCount(0);
  await start(page).click();
  await end(page).click();
  // 结束后自动亮出对方文字，方便对照。
  await expect(page.getByTestId("partner-text")).toBeVisible();
  const model = (await page.getByTestId("voice-model").textContent()) ?? "";
  const partnerText = (await page.getByTestId("partner-text").textContent()) ?? "";
  const m = /^(.+?), this is (.+?)\. /.exec(partnerText)!;
  const meSpelled = m[1];
  const partnerSpelled = m[2];
  for (const marker of ["Your report is five", "My name is", "my QTH is", "I am running a", "Seventy-three"]) {
    expect(model, `参考答案缺「${marker}」`).toContain(marker);
  }
  expect(model).toContain(meSpelled);
  expect(model).toContain(partnerSpelled);
  expect(unspell(meSpelled)).toBe(ME);
});

test("话音通联：用麦克风录音——能开始、能结束，结束后出现回放", async ({ page }) => {
  await open(page);
  await start(page).click();
  await expect(page.getByTestId("voice-mic")).toContainText("正在录音");
  // 录一小会儿再停，保证录音器吐出数据。
  await page.waitForTimeout(600);
  await end(page).click();
  await expect(page.getByTestId("voice-mic")).toHaveText("");
  const audio = page.getByTestId("voice-playback");
  await expect(audio).toBeVisible();
  expect(await audio.getAttribute("src")).toMatch(/^blob:/);

  // 换个对手：回放与结果一并清掉，不留旧录音。
  await page.getByRole("button", { name: "换个对手" }).click();
  await expect(page.getByTestId("voice-playback")).toHaveCount(0);
  await expect(page.getByTestId("voice-result")).toHaveCount(0);
  await expect(clockText(page)).toHaveText("限时 60 秒");
});

test("话音通联：换个对手重置计时、自评与文字", async ({ page }) => {
  await open(page, { clock: true });
  await page.getByRole("button", { name: "看文字" }).click();
  await start(page).click();
  await page.clock.runFor(5_000);
  await item(page, 1).click();
  await page.getByRole("button", { name: "换个对手" }).click();
  await expect(clockText(page)).toHaveText("限时 60 秒");
  await expect(item(page, 1)).toHaveAttribute("aria-checked", "false");
  await expect(page.getByTestId("partner-text")).toHaveCount(0);
  await expect(start(page)).toBeVisible();
});

test("话音通联：你的呼号取自日志的台站信息", async ({ page }) => {
  await open(page);
  await expect(page.getByTestId("voice-me")).toContainText(ME);
});

test("话音通联：没填台站信息时用示例呼号 BG1AAA", async ({ page }) => {
  await open(page, { station: false });
  await expect(page.getByTestId("voice-me")).toContainText("BG1AAA");
});

test("话音通联：英文 / 西语界面都有译文，不留中文（默认英文脚本）", async ({ page }) => {
  for (const [code, title, start_] of [
    ["en", "Voice QSO practice", "Start reply"],
    ["es", "Práctica de QSO en fonía", "Empezar a responder"],
  ] as const) {
    await page.addInitScript((c) => localStorage.setItem("locale", c), code);
    await open(page);
    await expect(page.getByRole("heading", { name: title, level: 1 })).toBeVisible();
    await page.getByRole("button", { name: start_ }).click();
    await page.getByRole("checkbox").first().click();
    await page.getByRole("button", { name: /Done replying|He terminado/ }).click();
    await expect(page.getByTestId("voice-result")).toBeVisible();
    await expect(page.locator("main")).not.toContainText(/[\u4e00-\u9fff]/);
  }
});

test("话音通联：没有麦克风（getUserMedia 被拒）时只计时、给出提示，也没有回放", async ({ page }) => {
  // 用桩让 getUserMedia 拒绝：与「没有设备 / 用户拒绝授权」走同一条失败路径。
  await page.addInitScript(() => {
    navigator.mediaDevices.getUserMedia = () =>
      Promise.reject(new DOMException("denied", "NotAllowedError"));
  });
  await open(page, { clock: true });
  await start(page).click();
  await expect(page.getByTestId("voice-mic")).toContainText("没有可用的麦克风或没有授权");
  await page.clock.runFor(15_000);
  await expect(clockText(page)).toHaveText("还剩 45 秒");
  await end(page).click();
  await expect(clockText(page)).toHaveText("用时 15 秒");
  await expect(page.getByTestId("voice-result")).toBeVisible();
  await expect(page.getByTestId("voice-playback")).toHaveCount(0);
});

/** 整页与结果卡片各自的横向溢出（像素，≤ 1 视为没有溢出）。 */
async function overflowOf(page: Page) {
  // 现代风点按钮会放一圈 560 ms 的按压涟漪（`.md-ping`），动画放大时短暂伸出视口，之后自清理。
  await expect(page.locator(".md-ping")).toHaveCount(0);
  return page.evaluate(() => {
    const el = document.querySelector('[data-testid="voice-result"]') as HTMLElement;
    return {
      page: document.documentElement.scrollWidth - window.innerWidth,
      card: el.scrollWidth - el.clientWidth,
    };
  });
}

test("话音通联：溢出检测器自测——塞进 900px 宽的元素时必须量得出溢出", async ({ page }) => {
  await page.setViewportSize({ width: 320, height: 800 });
  await open(page);
  await start(page).click();
  await end(page).click();
  await expect(page.getByTestId("voice-result")).toBeVisible();
  const before = await overflowOf(page);
  expect(before.page).toBeLessThanOrEqual(1);
  expect(before.card).toBeLessThanOrEqual(1);
  await page.evaluate(() => {
    const wide = document.createElement("div");
    wide.style.cssText = "width:900px;height:4px";
    document.querySelector('[data-testid="voice-result"]')!.appendChild(wide);
  });
  const after = await overflowOf(page);
  expect(after.card, "卡片里放 900px 宽的元素，量到的溢出应远大于 1").toBeGreaterThan(100);
  expect(after.page, "整页同理").toBeGreaterThan(100);
});

// 三语里最长的说明与评判在 320 px 下不能把整页或结果卡撑出横向滚动；两套皮肤各量一次。
for (const lang of ["zh", "en", "es"]) {
  for (const style of ["pixel", "modern"]) {
    test(`话音通联：320px 下不横向溢出（${lang} · ${style}）`, async ({ page }) => {
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
      await page.locator("[data-testid='voice-reply'] button").first().click();
      await page.waitForTimeout(500);
      await page.locator("[data-testid='voice-reply'] button").first().click();
      await expect(page.getByTestId("voice-result")).toBeVisible();
      const over = await overflowOf(page);
      expect(over.page, "整页横向溢出").toBeLessThanOrEqual(1);
      expect(over.card, "评判卡片自身横向溢出").toBeLessThanOrEqual(1);
    });
  }
}
