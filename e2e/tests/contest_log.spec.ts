import { expect, pickOption, test, type Page } from "./fixtures";

test("竞赛录入：呼号过短不记录，正常呼号入库并提示实体", async ({ page }) => {
  await page.goto("/contest-log");
  await expect(page.getByRole("heading", { level: 1, name: "竞赛录入" })).toBeVisible();
  await expect(page.getByText("还没有通联，输入呼号开始吧")).toBeVisible();

  const call = page.getByLabel("对方呼号");
  await call.fill("J");
  await page.getByRole("button", { name: "记录" }).click();
  await expect(page.getByText("还没有通联，输入呼号开始吧")).toBeVisible();

  await call.fill("ja1aa");
  // 呼号自动大写，并给出实体提示
  await expect(call).toHaveValue("JA1AA");
  await expect(page.locator("#contest-call-hint")).toContainText(/CQ \d+/);

  await page.getByRole("button", { name: "记录" }).click();
  await expect(page.locator("#contest-call-hint")).toContainText("已记录 JA1AA");
  await expect(page.getByRole("row").filter({ hasText: "JA1AA" })).toHaveCount(1);
});

test("竞赛录入：重复呼号标为重复且不计分", async ({ page }) => {
  await page.goto("/contest-log");
  await expect(page.getByRole("heading", { level: 1, name: "竞赛录入" })).toBeVisible();

  const call = page.getByLabel("对方呼号");
  await call.fill("JA1AA");
  await page.getByRole("button", { name: "记录" }).click();
  await expect(page.locator("#contest-call-hint")).toContainText("已记录 JA1AA");

  // 同一波段再来一次：先给出重复预警
  await call.fill("JA1AA");
  await expect(page.locator("#contest-call-hint")).toContainText("重复：本波段已通联过");
  await page.getByRole("button", { name: "记录" }).click();
  await expect(page.locator("#contest-call-hint")).toContainText("（重复，不计分）");
});

test("竞赛录入：空格跳到交换信息，Esc 清空输入", async ({ page }) => {
  await page.goto("/contest-log");
  await expect(page.getByRole("heading", { level: 1, name: "竞赛录入" })).toBeVisible();

  const call = page.getByLabel("对方呼号");
  await call.fill("JA1AA");
  await call.press(" ");
  // 焦点跳到「收到的…」输入框
  const rcvd = page.getByLabel(/^收到的/);
  await expect(rcvd).toBeFocused();

  await call.focus();
  await page.keyboard.press("Escape");
  await expect(call).toHaveValue("");
  await expect(rcvd).toHaveValue("");
});

test("竞赛录入：可导出 Cabrillo 并可开始新一场", async ({ page }) => {
  await page.goto("/contest-log?contest=CQ-WW-CW");
  await expect(page.getByRole("heading", { level: 1, name: "竞赛录入" })).toBeVisible();

  await page.getByLabel("对方呼号").fill("JA1AA");
  await page.getByRole("button", { name: "记录" }).click();
  await expect(page.getByRole("row").filter({ hasText: "JA1AA" })).toHaveCount(1);

  const dl = page.waitForEvent("download");
  await page.getByRole("button", { name: "导出 Cabrillo" }).click();
  const file = await dl;
  // 未设置本台呼号时文件名以 log 结尾
  expect(file.suggestedFilename()).toMatch(/^.+-\w+\.cbr$/);

  await page.getByRole("button", { name: "开始新一场" }).click();
  await expect(page.locator("#contest-call-hint")).toContainText("已开始新一场");
});

/**
 * 种子：把一批竞赛通联、当前场次与本台呼号直接写进 localStorage（只在首次加载时写）。
 *
 * 数据与期望值来自独立脚本（不复用页面或内核的任何逻辑）：本台 BG4ABC（中国、亚洲），
 * CQ WW CW；W1AW / DL1ABC 跨洲 3 分，JA1ABC 同洲 1 分。
 */
type ContestSeed = { time: string; call: string; freq: string; zone: string; date?: string; mode?: string };

/** 只种本台呼号（不种通联、不种场次）：导入 Cabrillo 要靠它认领台站。 */
async function seedStation(page: Page) {
  await page.addInitScript((book) => {
    if (!sessionStorage.getItem("e2e:seeded-station")) {
      localStorage.setItem("station-book", book);
      sessionStorage.setItem("e2e:seeded-station", "1");
    }
  }, JSON.stringify({ next_id: 2, profiles: [{ id: 1, callsign: "BG4ABC" }], active_id: 1 }));
}

async function seedContest(page: Page, contest: string, rows: ContestSeed[]) {
  const entries = rows.map((r, i) => ({
    id: i + 1,
    date: r.date ?? "2026-10-31",
    time: r.time,
    freq: r.freq,
    mode: r.mode ?? "CW",
    callsign: r.call,
    contest_id: contest,
    stx: "24",
    srx: r.zone,
    remark: "",
  }));
  await page.addInitScript(
    ([log, session, book]) => {
      if (!sessionStorage.getItem("e2e:seeded-contest")) {
        localStorage.setItem("logbook", log);
        localStorage.setItem("contest-session", session);
        localStorage.setItem("station-book", book);
        sessionStorage.setItem("e2e:seeded-contest", "1");
      }
    },
    [
      JSON.stringify({ entries }),
      JSON.stringify({
        contest_id: contest,
        start: "2026-10-31 00:00",
        my_exch: "24",
        freq: "14.025",
        mode: "CW",
        power: "LOW",
      }),
      JSON.stringify({
        next_id: 2,
        profiles: [{ id: 1, callsign: "BG4ABC" }],
        active_id: 1,
      }),
    ],
  );
}

const ANALYSIS_LOG: ContestSeed[] = [
  { time: "00:10", call: "W1AW", freq: "14.020", zone: "5" },
  { time: "00:20", call: "JA1ABC", freq: "14.021", zone: "25" },
  { time: "00:50", call: "DL1ABC", freq: "14.022", zone: "14" },
  { time: "01:05", call: "W1AW", freq: "7.010", zone: "5" },
  { time: "01:15", call: "JA1ABC", freq: "14.021", zone: "25" }, // 20m 重复
  { time: "01:30", call: "JA1ABC", freq: "7.011", zone: "25" },
  { time: "02:40", call: "DL1ABC", freq: "7.012", zone: "14" },
  { time: "02:45", call: "W1AW", freq: "21.020", zone: "5" },
  { time: "02:50", call: "JA1ABC", freq: "21.021", zone: "25" },
];

function analysis(page: Page) {
  return page.locator("section").filter({ has: page.getByRole("heading", { name: "赛后分析" }) });
}

test("竞赛录入：赛后分析给出每小时速率、最佳 60 分钟、空闲时段与乘数缺口", async ({ page }) => {
  await seedContest(page, "CQ-WW-CW", ANALYSIS_LOG);
  await page.goto("/contest-log");
  const sec = analysis(page);
  await expect(sec).toBeVisible();

  // 与上方自报分数一致：9 条里 1 条重复，有效 8 条、18 分、16 个乘数。
  const stats = page.locator("div.grid").filter({ hasText: "有效通联" }).first();
  await expect(stats).toContainText("8");

  // 任意连续 60 分钟最多 4 条，起点是 00:10（整点小时最多只有 3 条）。
  await expect(sec.getByText("任意连续 60 分钟内最多 4 条（起于 10-31 00:10 UTC）")).toBeVisible();

  // 每小时：通联 / 点数 / 新乘数（末尾的速率条是 aria-hidden 的装饰，不在无障碍树里）。
  const row = (label: string) => sec.getByRole("row").filter({ hasText: label });
  await expect(row("10-31 00:00").getByRole("cell")).toHaveText(["10-31 00:00", "3", "7", "6"]);
  await expect(row("10-31 01:00").getByRole("cell")).toHaveText(["10-31 01:00", "2", "4", "4"]);
  await expect(row("10-31 02:00").getByRole("cell")).toHaveText(["10-31 02:00", "3", "7", "6"]);

  // 空闲时段：默认 30 分钟，恰好 30 分钟的那段也算。
  await expect(sec.getByText("2 段，合计 1 h 40 min")).toBeVisible();
  await expect(sec.getByRole("listitem").filter({ hasText: "10-31 00:20 → 10-31 00:50" })).toContainText("30 min");
  await expect(sec.getByRole("listitem").filter({ hasText: "10-31 01:30 → 10-31 02:40" })).toContainText("1 h 10 min");

  // 下限调到 60 分钟：只剩 70 分钟那一段。
  await pickOption(page, "空闲段下限", "60 分钟");
  await expect(sec.getByText("1 段，合计 1 h 10 min")).toBeVisible();
  await expect(sec.getByRole("listitem").filter({ hasText: "10-31 00:20 → 10-31 00:50" })).toHaveCount(0);

  // 乘数缺口：分区 14 与德国在 40m / 20m 有、15m 没有；其余三个波段都有的不列。
  await expect(sec.getByText("CQ 分区 14：已有 40m, 20m；缺 15m")).toBeVisible();
  await expect(sec.getByRole("listitem").filter({ hasText: /^德国.*：已有 40m, 20m；缺 15m$/ })).toHaveCount(1);
  await expect(sec.getByText(/CQ 分区 (5|25)：/)).toHaveCount(0);

  // 各波段还缺的分区：15m 只拿到 5 和 25。
  await expect(sec.getByText("40m：已有 3 个，缺 37 个：")).toBeVisible();
  await expect(sec.getByText(/^15m：已有 2 个，缺 38 个：1, 2, 3, 4, 6, 7, /)).toBeVisible();
});

test("竞赛录入：赛后分析在没有通联时给出说明", async ({ page }) => {
  await page.goto("/contest-log");
  await expect(analysis(page).getByText("还没有有效通联。录入几条之后")).toBeVisible();
});

test("竞赛录入：赛后分析只用一个波段、含无时间通联时给出说明", async ({ page }) => {
  await seedContest(page, "CQ-WW-CW", [
    { time: "00:10", call: "W1AW", freq: "14.020", zone: "5" },
    { time: "00:20", call: "JA1ABC", freq: "14.021", zone: "25" },
    // 日期能识别、时间为空：仍是有效通联，但不进速率与空闲统计。
    { time: "", call: "DL1ABC", freq: "14.022", zone: "14", date: "2026-11-01" },
  ]);
  await page.goto("/contest-log");
  const sec = analysis(page);
  // 只用了 20m：谈不上跨波段缺口，但该波段的分区仍可列。
  await expect(sec.getByText("目前只用了一个波段，还谈不上跨波段缺口。")).toBeVisible();
  await expect(sec.getByText("有 1 条通联的日期或时间无法识别")).toBeVisible();
  // 三条有效，其中一条无时间：每小时表只算前两条。
  await expect(sec.getByRole("row").filter({ hasText: "10-31 00:00" }).getByRole("cell").nth(1)).toHaveText("2");
  await expect(sec.getByText("20m：已有 3 个，缺 37 个：")).toBeVisible();
});

test("竞赛录入：赛后分析在乘数不分波段的竞赛里不列波段缺口", async ({ page }) => {
  // WPX 的乘数是前缀、不分波段，没有「在别的波段补齐」这种缺口；速率与空闲照常。
  await seedContest(page, "CQ-WPX-CW", ANALYSIS_LOG);
  await page.goto("/contest-log");
  const sec = analysis(page);
  await expect(sec.getByText("这个竞赛的乘数不按波段分别计")).toBeVisible();
  await expect(sec.getByText("各波段还缺的 CQ 分区")).toHaveCount(0);
  await expect(sec.getByRole("row").filter({ hasText: "10-31 00:00" })).toHaveCount(1);
  await expect(sec.getByText(/段，合计/)).toBeVisible();
});

/** 一张 Stat 卡片里的数值：按标签找到那张卡，再读它的数字。 */
function stat(page: Page, label: string) {
  return page.locator("div.pxl-window").filter({ has: page.getByText(label, { exact: true }) });
}

/**
 * 一份 Cabrillo 日志：本台 BG4ABC，CQ WW CW，日期放在过去 —— 这样默认场次（今天开始）
 * 看不到它们，只有导入后页面把场次切到这场比赛，数字才会出来。
 * 数字同上面的赛后分析用例（独立脚本算出）：有效 8、重复 1、点数 18、乘数 16、自报 288。
 */
const CABRILLO = [
  "START-OF-LOG: 3.0",
  "CREATED-BY: e2e",
  "CONTEST: CQ-WW-CW",
  "CALLSIGN: BG4ABC",
  "GRID-LOCATOR: OM89",
  "QSO: 14020 CW 2025-11-29 0010 BG4ABC 599 24 W1AW 599 5",
  "QSO: 14021 CW 2025-11-29 0020 BG4ABC 599 24 JA1ABC 599 25",
  "QSO: 14022 CW 2025-11-29 0050 BG4ABC 599 24 DL1ABC 599 14",
  "QSO: 7010 CW 2025-11-29 0105 BG4ABC 599 24 W1AW 599 5",
  "QSO: 14021 CW 2025-11-29 0115 BG4ABC 599 24 JA1ABC 599 25",
  "QSO: 7011 CW 2025-11-29 0130 BG4ABC 599 24 JA1ABC 599 25",
  "QSO: 7012 CW 2025-11-29 0240 BG4ABC 599 24 DL1ABC 599 14",
  "QSO: 21020 CW 2025-11-29 0245 BG4ABC 599 24 W1AW 599 5",
  "QSO: 21021 CW 2025-11-29 0250 BG4ABC 599 24 JA1ABC 599 25",
  "X-QSO: 21022 CW 2025-11-29 0251 BG4ABC 599 24 VK2XX 599 30",
  // 多字段交换：列数对不上，必须跳过而不是读错（第 16 行）。
  "QSO: 14023 CW 2025-11-29 0300 BG4ABC 599 24 A 72 W9XYZ 599 5 B 73",
  "END-OF-LOG:",
].join("\n");

/** 「导入 Cabrillo」的文件输入（页面上还有载入 .scp 的文件输入，不能用全局选择器）。 */
function cabrilloInput(page: Page) {
  return page.locator('input[type="file"][aria-label="导入 Cabrillo"]');
}

function cabrilloFile(text: string, name = "e2e.cbr") {
  return { name, mimeType: "text/plain", buffer: Buffer.from(text) };
}

test("竞赛录入：导入 Cabrillo 后切到那场比赛，记分、分析与被跳过的行都对得上", async ({ page }) => {
  await seedStation(page);
  await page.goto("/contest-log");
  await expect(analysis(page).getByText("还没有有效通联")).toBeVisible();

  await cabrilloInput(page).setInputFiles(cabrilloFile(CABRILLO));

  // 9 条读进来；X-QSO 与那条多字段的行没读。
  await expect(page.getByRole("status")).toContainText("已导入 9 条通联（CQ WW DX CW）");
  const panel = page.locator("section").filter({ has: page.getByRole("heading", { name: "Cabrillo 导入结果" }) });
  await expect(panel).toBeVisible();
  await expect(panel.getByText("1 条标为 X-QSO")).toBeVisible();
  await expect(panel.getByText("1 行无法识别，没有导入：")).toBeVisible();
  await expect(panel.getByText("第 16 行：列数不是认得的格式")).toBeVisible();
  await expect(panel.getByText("A 72 W9XYZ 599 5 B 73")).toBeVisible();
  // 模板里的竞赛：不弹「按猜的格式」警告。
  await expect(panel.getByText(/按「报告 \+ 一个交换」/)).toHaveCount(0);
  // 通联是过去的：页面把场次切到那场比赛的第一条通联，否则导入成功却一条也看不见。
  await expect(panel.getByText(/本场开始时间调整为 2025-11-29 00:10 UTC/)).toBeVisible();

  // 与记分一致：9 条里 1 条在同一波段重复。
  await expect(stat(page, "有效通联")).toContainText("8");
  await expect(stat(page, "重复")).toContainText("1");
  await expect(stat(page, "点数")).toContainText("18");
  await expect(stat(page, "乘数")).toContainText("16");
  await expect(stat(page, "自报总分")).toContainText("288");
  // 赛后分析读到同一批数据。
  await expect(analysis(page).getByRole("row").filter({ hasText: "11-29 00:00" }).getByRole("cell")).toHaveText([
    "11-29 00:00",
    "3",
    "7",
    "6",
  ]);

  // 切换是持久的：刷新后仍在这场比赛里。
  await page.reload();
  await expect(stat(page, "有效通联")).toContainText("8");

  // 再导入同一份：全部已在日志里，不会再多出一条。
  await cabrilloInput(page).setInputFiles(cabrilloFile(CABRILLO));
  await expect(panel.getByText("9 条已经在日志里，已跳过")).toBeVisible();
  await expect(stat(page, "有效通联")).toContainText("8");
  await expect(stat(page, "重复")).toContainText("1");
});

test("竞赛录入：自己导出的 Cabrillo 再导入不会重复（模式大类一致就算同一条）", async ({ page }) => {
  // 日志里原来是 USB、时间写成 `0:10`；Cabrillo 里只有 PH。精确键对不上，宽松键必须对得上。
  await seedContest(page, "CQ-WW-SSB", [
    { time: "0:10", call: "W1AW", freq: "14.200", zone: "5", mode: "USB" },
    { time: "00:20", call: "JA1ABC", freq: "14.250", zone: "25", mode: "USB" },
  ]);
  await page.goto("/contest-log");
  await expect(stat(page, "有效通联")).toContainText("2");

  const download = page.waitForEvent("download");
  await page.getByRole("button", { name: "导出 Cabrillo" }).click();
  const path = await (await download).path();
  await cabrilloInput(page).setInputFiles(path);

  await expect(page.getByRole("status")).toContainText("已导入 0 条通联（CQ WW DX SSB）");
  await expect(page.getByText("2 条已经在日志里，已跳过")).toBeVisible();
  await expect(stat(page, "有效通联")).toContainText("2");
});

test("竞赛录入：模板外的竞赛按常见格式读并提醒核对，不切场次；不是 Cabrillo 的文件被拒", async ({ page }) => {
  await seedStation(page);
  await page.goto("/contest-log");

  // 不在内置模板里的竞赛：没有报告的 8 列格式，只能按常见布局猜。
  const other = [
    "START-OF-LOG: 3.0",
    "CONTEST: NAQP-CW",
    "CALLSIGN: BG4ABC",
    "QSO: 14020 CW 2025-11-29 0010 BG4ABC 24 W1AW 5",
    "QSO: 14021 CW 2025-11-29 0020 BG4ABC 24 JA1ABC 25",
    "END-OF-LOG:",
  ].join("\n");
  await cabrilloInput(page).setInputFiles(cabrilloFile(other, "naqp.cbr"));
  await expect(page.getByRole("status")).toContainText("已导入 2 条通联（NAQP-CW）");
  const panel = page.locator("section").filter({ has: page.getByRole("heading", { name: "Cabrillo 导入结果" }) });
  await expect(panel.getByText(/竞赛「NAQP-CW」不在内置模板里/)).toBeVisible();
  // 没有模板就没有场次可切：竞赛录入页上仍是空的，通联在通联日志里。
  await expect(panel.getByText(/已切换到/)).toHaveCount(0);
  await expect(analysis(page).getByText("还没有有效通联")).toBeVisible();

  // 不是 Cabrillo：给出明确的错误，并且不留下上一次的结果面板。
  await cabrilloInput(page).setInputFiles(cabrilloFile("hello\nthis is not a log\n", "notes.txt"));
  await expect(page.getByRole("status")).toContainText("这不像是 Cabrillo 日志");
  await expect(panel).toHaveCount(0);
});

/**
 * 呼号补全。本站不带 SCP 数据，候选 = 自己的通联日志 + 用户载入的 `.scp` 文件。
 * 下面的 `.scp` 夹具是真实 MASTER.SCP（2026-10-14 版）的文件头加一小撮真实呼号，格式（CRLF、`!!Order`、
 * `# Release`）一字不改；期望值来自独立脚本。
 */
const SCP_CALLS = [
  "2D0OMN", "2E0AOZ", "G5Y/P", "GM0FRT/P", "I/DL3NC", "N6QEK/KL7", "VO2DX/9",
  "W1ABK", "W1ABY", "W1AEC", "W1AM", "W1AN", "W1AO", "W1AQ", "W1AW", "W1CW", "W1MAW", "W1TW", "W1YW",
  "W2AW", "W3AW", "W6AW", "WX1AW",
];
const SCP_FILE =
  ["!!Order,1,1", "#", "# Super Check Partial", "# Release 2026.10.14", "# Generated by bb-scp", "#", ...SCP_CALLS].join("\r\n") +
  "\r\n";

function scpFile(text: string, name = "master.scp") {
  return { name, mimeType: "text/plain", buffer: Buffer.from(text) };
}

/** 数据来源面板里的文件输入（页面上还有导入 Cabrillo 的文件输入，不能用全局选择器）。 */
function scpInput(page: Page) {
  return page.locator('[data-slot="scp-source"] input[type="file"]');
}

/** 候选区。 */
function suggest(page: Page) {
  return page.locator('[data-slot="scp-suggest"]');
}

const LOG_CALLS: ContestSeed[] = [
  { time: "00:10", call: "JA1ABC", freq: "14.020", zone: "25" },
  { time: "00:20", call: "DL1ABC", freq: "14.022", zone: "14" },
];

test("竞赛录入：呼号补全只用自己的日志时给出包含与只差一位的候选，精确命中会标出", async ({ page }) => {
  await seedContest(page, "CQ-WW-CW", LOG_CALLS);
  await page.goto("/contest-log");
  const call = page.getByLabel("对方呼号");

  // 输入不足两位不出候选。
  await call.fill("J");
  await expect(suggest(page)).toHaveCount(0);

  // 包含：JA1 只命中日志里的 JA1ABC。
  await call.fill("ja1");
  await expect(suggest(page).getByText("包含「JA1」的呼号（共 1 个）：")).toBeVisible();
  await expect(suggest(page).getByRole("button", { name: "选用 JA1ABC" })).toBeVisible();

  // 精确命中。
  await call.fill("JA1ABC");
  await expect(suggest(page).getByText("JA1ABC 在候选库里（你的日志或载入的 SCP 文件）")).toBeVisible();

  // 只差一位（打错）：JA1ABD 不在库里，相近的有 JA1ABC。
  await call.fill("JA1ABD");
  await expect(suggest(page).getByText("候选库里没有 JA1ABD")).toBeVisible();
  await expect(suggest(page).getByText("只差一位的呼号（共 1 个）：")).toBeVisible();
  await expect(suggest(page).getByRole("button", { name: "选用 JA1ABC" })).toBeVisible();

  // 完全没有相近的。
  await call.fill("ZZ9ZZZ");
  await expect(suggest(page).getByText("候选库里没有 ZZ9ZZZ，也没有相近的呼号")).toBeVisible();
});

test("竞赛录入：载入真实格式的 .scp 文件后补全，点候选填入呼号，刷新后仍在，可清除", async ({ page }) => {
  await seedContest(page, "CQ-WW-CW", LOG_CALLS);
  await page.goto("/contest-log");
  const call = page.getByLabel("对方呼号");

  await page.getByText("呼号补全数据", { exact: true }).click();
  const panel = page.locator('[data-slot="scp-source"]');
  await expect(panel.getByText("来自你的通联日志：2 个不同的呼号")).toBeVisible();
  await expect(panel.getByText("还没有载入 SCP 文件")).toBeVisible();
  // 说明里写明了为什么不带数据。
  await expect(panel.getByText(/保留所有权利，没有再分发许可/)).toBeVisible();

  await scpInput(page).setInputFiles(scpFile(SCP_FILE));
  await expect(page.getByRole("status").first()).toContainText("已载入 23 个呼号（版本 2026.10.14）");
  await expect(panel.getByText("来自载入的 SCP 文件：23 个呼号（版本 2026.10.14）")).toBeVisible();

  // W1A：以它开头的 8 个，字母序（独立脚本算出）。
  await call.fill("w1a");
  await expect(suggest(page).getByText("包含「W1A」的呼号（共 8 个）：")).toBeVisible();
  const chips = suggest(page).getByRole("button");
  await expect(chips).toHaveText(["W1ABK", "W1ABY", "W1AEC", "W1AM", "W1AN", "W1AO", "W1AQ", "W1AW"]);

  // W1AW 与库里的 12 个呼号只差一位；日志里的呼号与文件里的合并后不重复。
  await call.fill("W1AW");
  await expect(suggest(page).getByText("W1AW 在候选库里（你的日志或载入的 SCP 文件）")).toBeVisible();
  await expect(suggest(page).getByText("只差一位的呼号（共 12 个）：")).toBeVisible();
  await expect(suggest(page).getByRole("button", { name: "选用 WX1AW" })).toBeVisible();

  // 带斜杠的呼号：输入中间一段也能命中。
  await call.fill("DL3NC");
  await expect(suggest(page).getByRole("button", { name: "选用 I/DL3NC" })).toBeVisible();

  // 点候选：填入呼号，焦点跳到交换信息。
  await call.fill("W1A");
  await suggest(page).getByRole("button", { name: "选用 W1ABK" }).click();
  await expect(call).toHaveValue("W1ABK");
  await expect(page.getByLabel(/^收到的/)).toBeFocused();

  // 刷新后文件仍在（存在 IndexedDB 里），不需要重新载入。
  await page.reload();
  await page.getByLabel("对方呼号").fill("W1A");
  await expect(suggest(page).getByText("包含「W1A」的呼号（共 8 个）：")).toBeVisible();

  // 清除：回到只用日志的候选。
  await page.getByText("呼号补全数据", { exact: true }).click();
  await page.getByRole("button", { name: "清除已载入的文件" }).click();
  await expect(page.getByRole("status").first()).toContainText("已清除载入的 SCP 文件");
  await expect(suggest(page).getByText("候选库里没有 W1A，也没有相近的呼号")).toBeVisible();
  await expect(page.getByRole("button", { name: "清除已载入的文件" })).toBeDisabled();
  await page.reload();
  await page.getByLabel("对方呼号").fill("W1A");
  await expect(suggest(page).getByText("候选库里没有 W1A，也没有相近的呼号")).toBeVisible();
});

test("竞赛录入：不是 SCP 的文件被拒并说明原因，不会改动已载入的数据", async ({ page }) => {
  await seedContest(page, "CQ-WW-CW", LOG_CALLS);
  await page.goto("/contest-log");
  await page.getByText("呼号补全数据", { exact: true }).click();
  await scpInput(page).setInputFiles(scpFile(SCP_FILE));
  await expect(page.getByRole("status").first()).toContainText("已载入 23 个呼号");

  // 一个呼号也没有。
  await scpInput(page).setInputFiles(scpFile("hello\nthis is not a callsign list\n", "notes.txt"));
  await expect(page.getByRole("status").first()).toContainText("文件里没有找到呼号，这不像 SCP 文件，没有载入。");
  // 选成了 Cabrillo：大多数行不像呼号。
  await scpInput(page).setInputFiles(
    scpFile("W1AW\nSTART-OF-LOG: 3.0\nCONTEST: CQ-WW-CW\nQSO: 14021 CW 2026-10-31 0010\n", "log.cbr"),
  );
  await expect(page.getByRole("status").first()).toContainText("文件里大多数行不像呼号，多半选错了文件，没有载入。");

  // 之前载入的数据没被覆盖。
  await expect(page.locator('[data-slot="scp-source"]').getByText("来自载入的 SCP 文件：23 个呼号")).toBeVisible();
  await page.getByLabel("对方呼号").fill("W1A");
  await expect(suggest(page).getByText("包含「W1A」的呼号（共 8 个）：")).toBeVisible();

  // 带 BOM、LF 换行、小写、缺文件头的文件也能读。
  await scpInput(page).setInputFiles(scpFile("\uFEFFbg4abc\nja1abc\n\nw1aw\n", "mine.scp"));
  await expect(page.getByRole("status").first()).toContainText("已载入 3 个呼号（版本 —）");
});
