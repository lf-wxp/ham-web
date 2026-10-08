import { expect, pickTime, test, type Page } from "./fixtures";

/** 一条「干净」的通联：网格、频率、波段都对得上。 */
const JA1AA = `<CALL:5>JA1AA <QSO_DATE:8>20240501 <TIME_ON:4>1230 <BAND:3>20m <MODE:3>SSB <FREQ:6>14.200 <GRIDSQUARE:4>PM95 <EOR>`;

/** 缺网格 + 频率越界（27.555 是 CB 频段，不在业余波段内）。 */
const DL1A = `<CALL:4>DL1A <QSO_DATE:8>20240502 <TIME_ON:4>0100 <BAND:3>11m <MODE:2>AM <FREQ:6>27.555 <EOR>`;

/** 一条干净的通联，用来做「重复导入」的那份。 */
const W1AW = `<CALL:4>W1AW <QSO_DATE:8>20240503 <TIME_ON:4>0200 <BAND:3>40m <MODE:2>CW <FREQ:5>7.030 <GRIDSQUARE:4>FN31 <EOR>`;

function adif(...rows: string[]): Buffer {
  return Buffer.from(`Generated for e2e\n<ADIF_VER:5>3.1.4 <EOH>\n${rows.join("\n")}\n`);
}

/** 导入一份 ADIF（导入完成的提示框自动接受）。 */
async function importAdif(page: Page, buffer: Buffer, name = "health.adi") {
  await page
    .locator('input[type="file"][accept*=".adi"]')
    .setInputFiles({ name, mimeType: "text/plain", buffer });
}

/** 体检面板。 */
function panel(page: Page) {
  return page.locator("section", { hasText: "日志体检" });
}

test("日志体检：频率越界与缺网格各报一条", async ({ page }) => {
  page.on("dialog", (d) => d.accept());
  await page.goto("/log");
  await importAdif(page, adif(JA1AA, DL1A, W1AW));
  await expect(page.getByText("共 3 条")).toBeVisible();

  const health = panel(page);
  await expect(health.getByText("频率不在业余波段内（1）")).toBeVisible();
  await expect(health.getByText("缺对方网格（1）")).toBeVisible();
  // 这两类都**不能**自动修：越界得人判断，缺网格补了就是掺假。
  await expect(health.getByText("发现 2 处问题 · 可自动修正 0")).toBeVisible();
  // 明细带上定位信息与补充值。
  // 同一条记录可能同时命中两类问题（这里是缺网格 + 频率越界），取第一条即可。
  await expect(health.getByText(/DL1A · 2024-05-02 01:00 · 11m AM/).first()).toBeVisible();
  await expect(health.getByText(/27\.555/)).toBeVisible();
});

test("日志体检：一键修正把波段按频率改回来", async ({ page }) => {
  page.on("dialog", (d) => d.accept());
  await page.goto("/log");
  // 频率 14.200 属于 20m，记录里却写着 40m。
  await importAdif(
    page,
    adif(
      `<CALL:5>K1TTT <QSO_DATE:8>20240504 <TIME_ON:4>0300 <BAND:3>40m <MODE:3>SSB <FREQ:6>14.200 <GRIDSQUARE:4>FN42 <EOR>`,
    ),
  );

  const health = panel(page);
  await expect(health.getByText("波段与频率不符（1）")).toBeVisible();
  await expect(health.getByText(/→ 20m/)).toBeVisible();

  await health.getByRole("button", { name: /一键修正/ }).click();
  // 修正后体检通过：说明 `band` 字段真的被改写成了频率推出的那个。
  await expect(health.getByText("体检通过：没有发现明显问题。")).toBeVisible();
});

/** 用表单加一条（时间显式指定，两条才能落在同一分钟、构成真正的重复项）。 */
async function addEntry(page: Page, call: string, time: string) {
  await page.locator('input[placeholder="BG4XXX"]').fill(call);
  await page.locator('input[placeholder="OM89EW"]').fill("PM95AA");
  await pickTime(page, "时间（UTC）", time);
  await page.getByRole("button", { name: "添加记录" }).click();
}

test("日志体检：合并重复项后只剩一条", async ({ page }) => {
  page.on("dialog", (d) => d.accept());
  await page.goto("/log");
  // 重复项来自**表单**：导入会按 qso_key 去重，表单不会（同一分钟连加两次同一条通联）。
  await addEntry(page, "JA1AA", "00:01");
  await addEntry(page, "JA1AA", "00:01");
  await expect(page.getByText("共 2 条")).toBeVisible();

  const health = panel(page);
  await expect(health.getByText("重复通联（1）")).toBeVisible();
  // 合并是破坏性的：先出确认框（自动接受），再给结果提示。
  await health.getByRole("button", { name: "修正（1）" }).click();
  await expect(page.getByText("共 1 条")).toBeVisible();
  await expect(health.getByText("体检通过：没有发现明显问题。")).toBeVisible();
});

test("日志体检：未来的时间只提示、不给自动修", async ({ page }) => {
  page.on("dialog", (d) => d.accept());
  await page.goto("/log");
  // 2030 年：即便按本地时解释也仍在未来，猜不出该怎么改，所以不给修法。
  await importAdif(
    page,
    adif(
      `<CALL:4>W1AW <QSO_DATE:8>20300101 <TIME_ON:4>0200 <BAND:3>40m <MODE:2>CW <FREQ:5>7.030 <GRIDSQUARE:4>FN31 <EOR>`,
    ),
  );

  const health = panel(page);
  await expect(health.getByText("时间落在未来（1）")).toBeVisible();
  await expect(health.getByText(/W1AW · 2030-01-01/)).toBeVisible();
  // 这一条没有「修正」入口，也没有可一键修正的项。
  await expect(health.getByRole("button", { name: /修正（/ })).toHaveCount(0);
  await expect(health.getByRole("button", { name: /一键修正/ })).toBeDisabled();
});

test("日志体检：话务落在 CW 段里被挑出来", async ({ page }) => {
  page.on("dialog", (d) => d.accept());
  await page.goto("/log");
  // 14.050 按三区规划只给 CW（`bandplan`）；同一频率上的 CW 不该被报。
  await importAdif(
    page,
    adif(
      `<CALL:5>JA1AA <QSO_DATE:8>20240507 <TIME_ON:4>0300 <BAND:3>20m <MODE:3>SSB <FREQ:6>14.050 <GRIDSQUARE:4>PM95 <EOR>`,
      `<CALL:5>JA2AA <QSO_DATE:8>20240508 <TIME_ON:4>0300 <BAND:3>20m <MODE:2>CW <FREQ:6>14.050 <GRIDSQUARE:4>PM95 <EOR>`,
    ),
  );

  const health = panel(page);
  await expect(health.getByText("模式与频率不匹配（1）")).toBeVisible();
  // 明细要指出落在哪个子段里。
  await expect(health.getByText(/14\.000–14\.150 · CW/)).toBeVisible();
  await expect(health.getByText("发现 1 处问题 · 可自动修正 0")).toBeVisible();
});
