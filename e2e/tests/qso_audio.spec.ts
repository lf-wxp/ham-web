import { readFile } from "node:fs/promises";
import { confirmDialog, expect, test, type Page } from "./fixtures";

/**
 * 通联录音（`/log` 的编辑表单）：一条通联挂一段本机录音（选文件或站内录），存 IndexedDB，
 * 不上传、不同步，备份里默认不带（单独勾选「通联录音」）。
 *
 * 断言挂上与卸下、刷新后还在、备份往返与映射；**不断言录到什么声音**（站内录音用 Chromium 的假麦克风，
 * 只验证能开始、能结束、播放器出现）。选文件的用例会比对存下的字节与原文件一致，那是在核对「存的就是用户选的」，
 * 不是在听内容。
 */

test.use({
  launchOptions: { args: ["--use-fake-ui-for-media-stream", "--use-fake-device-for-media-stream"] },
  permissions: ["microphone"],
});

/** 一段 0.1 秒、8 kHz、16 位单声道的静音 WAV：头部按规格手写，不借任何库。 */
function wav(seconds = 0.1, fill = 0): Buffer {
  const rate = 8000;
  const samples = Math.round(rate * seconds);
  const data = Buffer.alloc(samples * 2, fill);
  const head = Buffer.alloc(44);
  head.write("RIFF", 0);
  head.writeUInt32LE(36 + data.length, 4);
  head.write("WAVEfmt ", 8);
  head.writeUInt32LE(16, 16); // fmt 块长度
  head.writeUInt16LE(1, 20); // PCM
  head.writeUInt16LE(1, 22); // 单声道
  head.writeUInt32LE(rate, 24);
  head.writeUInt32LE(rate * 2, 28); // 字节率
  head.writeUInt16LE(2, 32); // 块对齐
  head.writeUInt16LE(16, 34); // 位深
  head.write("data", 36);
  head.writeUInt32LE(data.length, 40);
  return Buffer.concat([head, data]);
}

const callInput = (page: Page) => page.locator('input[placeholder="BG4XXX"]');
const panel = (page: Page) => page.getByTestId("qso-audio");
const player = (page: Page) => page.getByTestId("qso-audio-player");
const issue = (page: Page) => page.getByTestId("qso-audio-issue");
/** 面板上的可见按钮（FileInput 里隐藏的 <input type=file> 也带同名 aria-label，别按 role 取）。 */
const btn = (page: Page, text: string) => panel(page).locator("button").filter({ hasText: text });
const fileInput = (page: Page) => panel(page).locator('input[type="file"]');
const index = (page: Page) => page.evaluate(() => localStorage.getItem("qso-audio-index"));

/** 添加一条通联并打开它的编辑表单。 */
async function addAndEdit(page: Page, call = "JA1AA") {
  await page.goto("/log");
  await callInput(page).fill(call);
  await page.getByRole("button", { name: "添加记录" }).click();
  await expect(page.getByText("共 1 条")).toBeVisible();
  await page.getByRole("button", { name: "编辑" }).first().click();
  await expect(panel(page)).toBeVisible();
}

test("通联录音：新记录先提示保存，编辑已有记录才出现入口", async ({ page }) => {
  await page.goto("/log");
  await callInput(page).fill("JA1AA");
  await page.getByRole("button", { name: "添加记录" }).click();
  await expect(page.getByText("保存这条通联后即可挂录音")).toBeVisible();
  await page.getByRole("button", { name: "编辑" }).first().click();
  await expect(page.getByText("保存这条通联后即可挂录音")).toHaveCount(0);
  await expect(btn(page, "选择录音文件")).toBeVisible();
  await expect(btn(page, "站内录音")).toBeVisible();
  await expect(player(page)).toHaveCount(0);
});

test("通联录音：选一个音频文件挂上，存的就是用户选的字节，刷新后还在，可替换、可删除", async ({ page }) => {
  await addAndEdit(page);
  const first = wav(0.1);
  await fileInput(page).setInputFiles({ name: "qso.wav", mimeType: "audio/wav", buffer: first });

  await expect(player(page)).toBeVisible();
  const src = (await player(page).getAttribute("src")) ?? "";
  expect(src).toMatch(/^data:audio\/wav;base64,/);
  expect(Buffer.from(src.replace(/^data:audio\/wav;base64,/, ""), "base64").equals(first)).toBe(true);
  expect(await index(page)).toBe("[1]");
  await expect(btn(page, "换一个录音文件")).toBeVisible();

  // 刷新后仍在：本体是 IndexedDB 里的，索引是 localStorage 里的小数据。
  await page.reload();
  await page.getByRole("button", { name: "编辑" }).first().click();
  await expect(player(page)).toBeVisible();
  expect(await player(page).getAttribute("src")).toBe(src);

  // 换一个文件：覆盖，不是追加。
  const second = wav(0.2, 1);
  await fileInput(page).setInputFiles({ name: "qso2.wav", mimeType: "audio/wav", buffer: second });
  await expect
    .poll(async () => Buffer.from(((await player(page).getAttribute("src")) ?? "").split(",")[1], "base64").equals(second))
    .toBe(true);
  expect(await index(page)).toBe("[1]");

  // 删除：确认后回到入口，索引整个删掉（不留空数组），刷新后不复现。
  await btn(page, "删除录音").click();
  await confirmDialog(page, "删除录音");
  await expect(player(page)).toHaveCount(0);
  await expect(btn(page, "选择录音文件")).toBeVisible();
  expect(await index(page)).toBeNull();
  await page.reload();
  await page.getByRole("button", { name: "编辑" }).first().click();
  await expect(player(page)).toHaveCount(0);
});

test("通联录音：站内录音——能开始、能停止并保存，随后出现播放器与索引", async ({ page }) => {
  await addAndEdit(page);
  await btn(page, "站内录音").click();
  await expect(panel(page).getByRole("status")).toContainText("正在录音");
  await expect(btn(page, "停止并保存")).toBeVisible();
  // 录音期间不能再选文件 / 删除，入口换成「停止」。
  await expect(btn(page, "选择录音文件")).toHaveCount(0);
  await page.waitForTimeout(700); // 让录音器吐出数据
  await btn(page, "停止并保存").click();

  await expect(player(page)).toBeVisible();
  expect(await player(page).getAttribute("src")).toMatch(/^data:audio\/(webm|ogg|mp4)/);
  expect(await index(page)).toBe("[1]");
  await expect(panel(page).getByRole("status")).toHaveCount(0);

  await page.reload();
  await page.getByRole("button", { name: "编辑" }).first().click();
  await expect(player(page)).toBeVisible();
});

test("通联录音：类型不对、太大、空文件当场给出提示，也不会挂上", async ({ page }) => {
  await addAndEdit(page);
  const cases: [string, string, Buffer, string][] = [
    ["notes.txt", "text/plain", Buffer.from("not audio"), "不支持这种音频"],
    ["clip.mp4", "video/mp4", Buffer.from("video"), "不支持这种音频"],
    ["huge.wav", "audio/wav", Buffer.alloc(8 * 1024 * 1024 + 1), "录音文件太大"],
    ["empty.wav", "audio/wav", Buffer.alloc(0), "没有录到声音"],
  ];
  for (const [name, mimeType, buffer, message] of cases) {
    await fileInput(page).setInputFiles({ name, mimeType, buffer });
    await expect(issue(page), name).toContainText(message);
    await expect(player(page)).toHaveCount(0);
    expect(await index(page), name).toBeNull();
  }
  // 小文件正常放行；体积上限的闭区间边界由核心单测钉住。
  await fileInput(page).setInputFiles({ name: "edge.wav", mimeType: "audio/wav", buffer: wav(0.1) });
  await expect(player(page)).toBeVisible();
  await expect(issue(page)).toHaveCount(0);
});

test("通联录音：没有麦克风（getUserMedia 被拒）时给出提示，也不会挂上", async ({ page }) => {
  await page.addInitScript(() => {
    navigator.mediaDevices.getUserMedia = () => Promise.reject(new DOMException("denied", "NotAllowedError"));
  });
  await addAndEdit(page);
  await btn(page, "站内录音").click();
  await expect(issue(page)).toContainText("没有可用的麦克风，或没有授权");
  await expect(player(page)).toHaveCount(0);
  await expect(btn(page, "停止并保存")).toHaveCount(0);
  expect(await index(page)).toBeNull();
});

test("通联录音：删掉通联连录音一起清，id 被复用时新通联不会挂上旧录音", async ({ page }) => {
  await addAndEdit(page, "JA1AA");
  await fileInput(page).setInputFiles({ name: "a.wav", mimeType: "audio/wav", buffer: wav() });
  await expect(player(page)).toBeVisible();
  expect(await index(page)).toBe("[1]");
  // 刷新回到列表，删除这条通联。
  await page.reload();
  await page.getByRole("row").filter({ hasText: "JA1AA" }).getByRole("button", { name: "删除", exact: true }).click();
  await confirmDialog(page, "删除");
  await expect(page.getByText("暂无记录，添加第一条通联日志吧。")).toBeVisible();
  expect(await index(page), "删除通联后索引要清掉").toBeNull();

  // 新通联会复用 id 1：它不能挂着刚删掉的那段录音。
  await callInput(page).fill("W1AW");
  await page.getByRole("button", { name: "添加记录" }).click();
  const book = await page.evaluate(() => JSON.parse(localStorage.getItem("logbook") ?? "null"));
  expect(book.entries.map((e: { id: number }) => e.id)).toEqual([1]);
  await page.getByRole("button", { name: "编辑" }).first().click();
  await expect(panel(page)).toBeVisible();
  await expect(player(page)).toHaveCount(0);
});

// ---------------- 备份 ----------------

const readJson = async (download: import("@playwright/test").Download) =>
  JSON.parse(await readFile((await download.path())!, "utf8")) as Record<string, string>;

const toastText = (page: Page) => page.locator('[data-slot="toast-text"]');

test("通联录音：备份里默认不带，勾选「通联录音」才带上本体、索引与日志上下文", async ({ page }) => {
  await addAndEdit(page);
  const clip = wav(0.1);
  await fileInput(page).setInputFiles({ name: "qso.wav", mimeType: "audio/wav", buffer: clip });
  await expect(player(page)).toBeVisible();
  const stored = (await player(page).getAttribute("src"))!;

  await page.goto("/backup");
  const section = page.locator("section#backup");
  const audioBox = section.getByLabel(/^通联录音/);
  // 与 QSL 影像不同：默认不勾。
  await expect(audioBox).toHaveAttribute("aria-checked", "false");
  await expect(section.getByLabel(/^QSL 影像（扫描件/)).toHaveAttribute("aria-checked", "true");

  // 默认导出：没有录音，连索引也没有（索引随「通联录音」模块走）。
  let dl = page.waitForEvent("download");
  await section.getByRole("button", { name: "导出备份" }).click();
  let data = await readJson(await dl);
  expect(Object.keys(data).filter((k) => k.startsWith("qso-audio"))).toEqual([]);
  expect(data["logbook"]).toBeDefined();
  await expect(toastText(page)).toHaveCount(0, { timeout: 15000 });

  // 勾上：带本体与索引。
  await audioBox.check();
  dl = page.waitForEvent("download");
  await section.getByRole("button", { name: "导出备份" }).click();
  data = await readJson(await dl);
  expect(data["qso-audio:1"]).toBe(stored);
  expect(data["qso-audio-index"]).toBe("[1]");
  await expect(toastText(page)).toHaveCount(0, { timeout: 15000 });

  // 只勾「通联录音」：仍要带上日志上下文（导入时按通联身份映射 id），但不带日志本身与别的数据。
  await section.getByLabel(/^通联数据/).uncheck();
  await section.getByLabel(/^QSL 影像（扫描件/).uncheck();
  await section.getByLabel(/^其他本地数据/).uncheck();
  dl = page.waitForEvent("download");
  await section.getByRole("button", { name: "导出备份" }).click();
  data = await readJson(await dl);
  expect(data["qso-audio:1"]).toBe(stored);
  expect(data["qso-audio-index"]).toBe("[1]");
  expect(data["logbook"]).toBeUndefined();
  expect(data["qsl-backup-logbook"]).toBeDefined();
});

test("通联录音：只勾「通联录音」时也能导出（不会因为别的模块都没勾而禁用）", async ({ page }) => {
  await page.goto("/backup");
  const section = page.locator("section#backup");
  await section.getByLabel(/^通联数据/).uncheck();
  await section.getByLabel(/^QSL 影像（扫描件/).uncheck();
  await section.getByLabel(/^其他本地数据/).uncheck();
  await expect(section.getByRole("button", { name: "导出备份" })).toBeDisabled();
  await section.getByLabel(/^通联录音/).check();
  await expect(section.getByRole("button", { name: "导出备份" })).toBeEnabled();
});

const entry = (id: number, callsign: string, date: string) => ({
  id, date, time: "12:00", freq: "14.074", mode: "FT8", callsign, remark: "",
});
const A = "data:audio/wav;base64,QUFB";
const B = "data:audio/wav;base64,QkJC";

async function importFile(page: Page, merge: boolean, data: Record<string, string>) {
  const inputs = page.locator('section#backup input[type="file"]');
  await expect(toastText(page)).toHaveCount(0);
  await inputs.nth(merge ? 1 : 0).setInputFiles({
    name: "audio.json", mimeType: "application/json", buffer: Buffer.from(JSON.stringify(data)),
  });
  await expect(toastText(page)).toContainText(merge ? /^已合并/ : /^已恢复/);
  await expect(inputs.first()).toBeEnabled();
  await expect(toastText(page)).toHaveCount(0);
}

test("通联录音：覆盖导入恢复录音与索引；不合格的值与 key 被拒", async ({ page }) => {
  await page.goto("/backup");
  await importFile(page, false, {
    logbook: JSON.stringify({ entries: [entry(7, "JA1AAA", "2026-01-01"), entry(8, "W1AW", "2026-01-02")] }),
    "qso-audio-index": "[7,8,9]",
    "qso-audio:7": A,
    "qso-audio:8": "data:text/html;base64,PGI+", // 不是音频
    "qso-audio:x": A, // key 不合格
    "qso-audio:007": A, // 前导零：会与 7 互相覆盖
  });
  // 只有合格的那一段落了盘；索引按「实际存在的」重建，不信备份里写的 [7,8,9]。
  expect(await index(page)).toBe("[7]");
  await page.goto("/log");
  await page.getByRole("row").filter({ hasText: "JA1AAA" }).getByRole("button", { name: "编辑" }).click();
  await expect(player(page)).toBeVisible();
  expect(await player(page).getAttribute("src")).toBe(A);
});

test("通联录音：跨设备合并按通联身份重映射 id，保留本机已有的录音", async ({ page }) => {
  await page.goto("/backup");
  await importFile(page, false, {
    logbook: JSON.stringify({ entries: [entry(1, "JA1AAA", "2026-01-01")] }),
    "qso-audio-index": "[1]",
    "qso-audio:1": A,
  });
  // 另一台设备的备份：同样是 id 1，但是另一条通联（W1AW）。合并后它应落在新 id 2 上，而不是顶掉本机 id 1 的录音。
  const backup = {
    logbook: JSON.stringify({ entries: [entry(1, "W1AW", "2026-01-02")] }),
    "qso-audio-index": "[1]",
    "qso-audio:1": B,
  };
  await importFile(page, true, backup);
  await importFile(page, true, backup); // 幂等：再合并一次不会多出东西
  const book = await page.evaluate(() => JSON.parse(localStorage.getItem("logbook") ?? "null"));
  expect(book.entries.map((e: { id: number; callsign: string }) => [e.id, e.callsign])).toEqual([
    [1, "JA1AAA"],
    [2, "W1AW"],
  ]);
  expect(await index(page)).toBe("[1,2]");

  // 导出（勾上录音）验证两段各归各的。
  const section = page.locator("section#backup");
  await section.getByLabel(/^通联录音/).check();
  const dl = page.waitForEvent("download");
  await section.getByRole("button", { name: "导出备份" }).click();
  const data = await readJson(await dl);
  expect(data["qso-audio:1"]).toBe(A);
  expect(data["qso-audio:2"]).toBe(B);
});

test("通联录音：覆盖导入换掉日志时，id 对不上的旧录音被清掉，不会挂到别人名下", async ({ page }) => {
  await page.goto("/backup");
  await importFile(page, false, {
    logbook: JSON.stringify({ entries: [entry(1, "JA1AAA", "2026-01-01")] }),
    "qso-audio-index": "[1]",
    "qso-audio:1": A,
  });
  expect(await index(page)).toBe("[1]");
  // 换成另一份日志：id 1 现在是 W1AW，不能继续挂着 JA1AAA 的录音。
  await importFile(page, false, {
    logbook: JSON.stringify({ entries: [entry(1, "W1AW", "2026-01-02")] }),
  });
  expect(JSON.parse((await index(page)) ?? "[]"), "旧录音要被清掉").toEqual([]);
  await page.goto("/log");
  await page.getByRole("row").filter({ hasText: "W1AW" }).getByRole("button", { name: "编辑" }).click();
  await expect(player(page)).toHaveCount(0);
});

test("通联录音：没用过录音的人恢复备份，不会多出空的 qso-audio-index", async ({ page }) => {
  await page.goto("/backup");
  await importFile(page, false, { logbook: JSON.stringify({ entries: [entry(1, "JA1AAA", "2026-01-01")] }) });
  expect(await index(page)).toBeNull();
  await importFile(page, true, { logbook: JSON.stringify({ entries: [entry(1, "W1AW", "2026-01-02")] }) });
  expect(await index(page)).toBeNull();
});
