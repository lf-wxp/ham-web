import { stat } from "node:fs/promises";
import { expect, test } from "./fixtures";

/** 定位某个工具的 section（导航 / 锚点用的 id 与 section 的 id 一致）。 */
function tool(page: import("@playwright/test").Page, id: string) {
  return page.locator(`section#${id}`);
}

// 60 个计算器分批挂载（`crates/app/src/pages/tools/tool_mount.rs`）：首屏只挂前几个，其余每帧放行一批。
// 「站内跳进来 → 首帧计数 → 点目录 → 两帧后计数」放在同一次 `evaluate` 里：分批在一两百毫秒内就走完，
// 跨一次 Playwright 往返就看不到中间态了（整页 `goto` 更不行，加载期间分批早已结束）。
test("小工具：计算器分批挂载，挂载途中点目录先挂全再跳，最终全部就位", async ({ page }) => {
  const busy = page.locator('main section[aria-busy="true"]');
  const sections = page.locator("main section[id]");
  await page.goto("/countdown");
  await expect(page.getByRole("heading", { level: 1 })).toBeVisible();

  const seen = await page.evaluate(async () => {
    const frame = () => new Promise<void>((r) => requestAnimationFrame(() => r()));
    const countBusy = () => document.querySelectorAll('main section[aria-busy="true"]').length;
    // 站内导航：路由器拦截的是真实的链接点击，这里放一个临时链接点一下。
    const a = document.createElement("a");
    a.href = "/tools";
    document.body.append(a);
    a.click();
    a.remove();
    for (let i = 0; i < 120 && !document.querySelector("#freq-wavelength input"); i++) await frame();
    const atFirstFrame = countBusy();
    const btn = [...document.querySelectorAll<HTMLButtonElement>("aside nav button")].find(
      (b) => b.textContent?.trim() === "数字模式编码",
    );
    if (!btn) return { atFirstFrame, afterJump: -1 };
    btn.click();
    await frame();
    await frame();
    return { atFirstFrame, afterJump: countBusy() };
  });
  // 首帧时确实还有没挂上的：说明是分批的，而不是一口气挂完。
  expect(seen.atFirstFrame, "首帧时其余计算器应还在排队").toBeGreaterThan(40);
  expect(seen.afterJump, "侧栏目录里没找到「数字模式编码」").toBeGreaterThanOrEqual(0);
  // 点目录：先把剩下的全部挂上，再按真实高度滚过去 —— 两帧之内不该还有排队的。
  expect(seen.afterJump, "点目录后应立刻挂上全部计算器，而不是等分批走完").toBe(0);
  await expect
    .poll(() => tool(page, "mode-encoder").evaluate((el) => Math.round(el.getBoundingClientRect().top)))
    .toBeLessThan(240);
  expect(await tool(page, "mode-encoder").evaluate((el) => el.getBoundingClientRect().top)).toBeGreaterThanOrEqual(0);

  // 最终：60 节全部有内容，没有遗留的占位。
  await expect(busy).toHaveCount(0);
  const total = await sections.count();
  expect(total, "工具页的计算器节数").toBeGreaterThanOrEqual(60);
  const empty = await sections.evaluateAll(
    (els) => els.filter((s) => !s.querySelector("input, select, button, svg, table, canvas, textarea")).map((s) => s.id),
  );
  expect(empty, "这些计算器没有挂上主体").toEqual([]);
});

test("小工具：频率 ↔ 波长双向换算", async ({ page }) => {
  await page.goto("/tools");
  await expect(page.getByRole("heading", { level: 1, name: "小工具" })).toBeVisible();
  const section = tool(page, "freq-wavelength");
  await expect(section).toBeVisible();

  await section.getByLabel("频率（MHz）").fill("100");
  await expect(section.getByText(/100\.00 MHz ≈ 3\.00 m/)).toBeVisible();

  // 反向：填波长反推频率
  await section.getByLabel("波长（m）").fill("2");
  await expect(section.getByText(/150\.00 MHz ≈ 2\.00 m/)).toBeVisible();
});

test("小工具：驻波比换算反射系数", async ({ page }) => {
  await page.goto("/tools#swr");
  const section = tool(page, "swr");
  await expect(section).toBeVisible();

  await section.getByLabel("驻波比 SWR").fill("3");
  await expect(section.getByText(/反射系数 \|Γ\| = 0\.5000/)).toBeVisible();
  // 文案在 i18n 重构里改成了「回波损耗取正值为 {} dB」，这里只钉数值
  await expect(section.getByText(/回波损耗.*6\.02 dB/)).toBeVisible();
});

test("小工具：欧姆定律填两项算第三项", async ({ page }) => {
  await page.goto("/tools#ohms-law");
  const section = tool(page, "ohms-law");
  await expect(section).toBeVisible();

  // 未填够两项时给出引导
  await expect(section.getByText("填写任意两项（U、I、R）后自动计算。")).toBeVisible();

  await section.getByLabel("电压 U（V）").fill("12");
  await section.getByLabel("电流 I（A）").fill("2");
  await expect(section.getByText(/R = 6\.00/)).toBeVisible();
  await expect(section.getByText(/24\.00/)).toBeVisible();
});

test("小工具：菲涅尔区净空判定随障碍物高度变化", async ({ page }) => {
  await page.goto("/tools#fresnel-zone");
  const section = tool(page, "fresnel-zone");
  await expect(section).toBeVisible();

  // 默认：145 MHz、30 km、两端 100 m、中点无障碍，只剩地球曲率 → 净空达标
  await expect(section.getByText(/净空达标/)).toBeVisible();
  await expect(section.getByText(/地球曲率隆起 13\.24 m/)).toBeVisible();
  await expect(section.getByText(/占第一菲涅尔区半径的 70%/)).toBeVisible();

  // 抬高障碍物：先侵入第一菲涅尔区，再高过视线
  await section.getByLabel("障碍物顶高度（m）").fill("40");
  await expect(section.getByText(/部分遮挡/)).toBeVisible();
  await expect(section.getByText(/还差 .* m/)).toBeVisible();

  await section.getByLabel("障碍物顶高度（m）").fill("200");
  await expect(section.getByText(/视线被挡/)).toBeVisible();

  // 障碍物放到端点上不合法：给出引导而不是算出无穷大
  await section.getByLabel("障碍物距 A 端（km）").fill("30");
  await expect(section.getByText(/障碍物须位于 A、B 之间/)).toBeVisible();
});

test("小工具：路径剖面找出最危险的点并给出抬高建议", async ({ page }) => {
  await page.goto("/tools#path-profile");
  const section = tool(page, "path-profile");
  await expect(section).toBeVisible();
  const points = section.getByLabel("剖面采样点");

  // 示例剖面：310 m 的山脊挡在 15 km 处
  await expect(section.getByText(/视线被挡/)).toBeVisible();
  await expect(section.getByText(/距 A 端 15\.00 km，海拔 310\.00 m/)).toBeVisible();
  await expect(section.getByText(/两端天线同时抬高/)).toBeVisible();
  await expect(section.getByRole("img", { name: "路径剖面图" })).toBeVisible();
  // 剖面表：7 个点 + 表头
  await expect(section.getByRole("row")).toHaveCount(8);

  // 改成平地 + 100 m 天线：与菲涅尔区工具的默认结论一致（净空达标、约 70%）
  await points.fill("0, 0\n15, 0\n30, 0");
  await section.getByLabel("天线 A 高度（m）").fill("100");
  await section.getByLabel("天线 B 高度（m）").fill("100");
  await expect(section.getByText(/净空达标/)).toBeVisible();
  await expect(section.getByRole("cell", { name: "70%", exact: true })).toBeVisible();
  await expect(section.getByRole("row")).toHaveCount(4);
  await expect(section.getByText(/距 60% 净空线仍有 .* m 余量/)).toBeVisible();

  // 输入不合法时给出带行号的引导，而不是算出奇怪的数
  await points.fill("0, 0\n5\n10, 0");
  await expect(section.getByRole("alert")).toContainText("第 2 行");
  await points.fill("0, 0\n10, 5");
  await expect(section.getByRole("alert")).toContainText("至少需要 3 个点");
  await points.fill("0, 0\n10, 0\n10, 0");
  await expect(section.getByRole("alert")).toContainText("第 3 个点的距离必须大于前一个点");
});

test("小工具：磁环绕线由目标电感反推匝数，向上取整并给出公差与窗口提示", async ({ page }) => {
  await page.goto("/tools#toroid-winding");
  const section = tool(page, "toroid-winding");
  await expect(section).toBeVisible();

  // 默认磁环 5943000301（AL 440 nH/匝²，±20%），目标 50 µH：
  // N = √(1000·50/440) = 10.66 → 11 匝；L = 440·11²/1000 = 53.24 µH；±20% → 42.59 – 63.89。
  await expect(section.getByText("5943000301 · 43 材料")).toBeVisible();
  await expect(section.getByRole("status")).toContainText("需要绕 11 匝（计算值 10.66");
  await expect(section.getByText(/实际约 53\.24 µH.*42\.59 – 63\.89 µH/)).toBeVisible();
  // 线径 0.6 mm、内径 7.15 mm → 内圈周长 22.46 mm → 单层约 37 匝：11 匝放得下
  await expect(section.getByText(/内圈单层最多约 37 匝：放得下/)).toBeVisible();

  // 目标 1000 µH → 48 匝 > 37：提示放不下
  await section.getByLabel("目标电感（µH）").fill("1000");
  await expect(section.getByRole("status")).toContainText("需要绕 48 匝");
  await expect(section.getByText(/内圈单层最多约 37 匝：放不下/)).toBeVisible();

  // 换成大磁环（AL 1075，内径 35.55 mm）：同样 1000 µH 只要 31 匝，内圈放得下
  await section.getByRole("combobox", { name: "磁环" }).click();
  await page.getByRole("option", { name: /5943003801/ }).click();
  await expect(section.getByRole("status")).toContainText("需要绕 31 匝");
  await expect(section.getByText(/放得下/)).toBeVisible();

  // 自定义：自己填 AL，不再判断窗口
  await section.getByRole("combobox", { name: "磁环" }).click();
  await page.getByRole("option", { name: /自定义/ }).click();
  await section.getByLabel("AL（nH/匝²）").fill("100");
  await section.getByLabel("目标电感（µH）").fill("1");
  // N = √(1000·1/100) = 3.16 → 4 匝
  await expect(section.getByRole("status")).toContainText("需要绕 4 匝");
  await expect(section.getByText(/自定义磁环没有尺寸/)).toBeVisible();

  // 目标电感不合法：给出引导而不是算出奇怪的数
  await section.getByLabel("目标电感（µH）").fill("0");
  await expect(section.getByText(/请输入有效的 AL/)).toBeVisible();
});

/** 共模扼流表里某个波段的一行：`[波段, 频率, |Z|, R, X, 电阻占比, 插损]`。 */
async function chokeRow(section: import("@playwright/test").Locator, band: string) {
  const row = section.getByRole("row").filter({ has: section.page().getByRole("cell", { name: band, exact: true }) });
  return row.getByRole("cell").allTextContents();
}

// 期望值来自独立 Python 脚本（自带对数插值，不复用页面代码），复磁导率取自 Fair-Rite 官方 CSV：
// FT-240-43（5943003801）、10 匝、50 Ω 回路，40 m 中心 7.1 MHz。
test("小工具：共模扼流圈按官方复磁导率给出各波段阻抗与插损，换材料 / 匝数 / 回路都会变", async ({ page }) => {
  await page.goto("/tools#common-mode-choke");
  const section = tool(page, "common-mode-choke");
  await expect(section).toBeVisible();

  // 默认 FT-240-43、10 匝、50 Ω：160 m 到 6 m 共 11 个波段。
  await expect(section.getByRole("status")).toContainText("5943003801 · 43 号材料");
  await expect(section.getByRole("table", { name: "共模扼流各波段阻抗与插入损耗" }).getByRole("row")).toHaveCount(12); // 表头 + 11
  await expect.poll(() => chokeRow(section, "40m")).toEqual(["40m", "7.10", "3408.2", "2410.2", "2409.8", "71", "36.76"]);
  await expect.poll(() => chokeRow(section, "20m")).toEqual(["20m", "14.18", "4486.5", "3552.8", "2739.6", "79", "39.14"]);

  // 换成同尺寸的 61 号材料：20 m 几乎是纯电感（电阻占比 2%），阻抗也小得多。
  await section.getByRole("combobox", { name: "扼流磁环" }).click();
  await page.getByRole("option", { name: /5961003801/ }).click();
  await expect.poll(() => chokeRow(section, "20m")).toEqual(["20m", "14.18", "1636.6", "39.01", "1636.2", "2", "30.31"]);

  // 回到 43 号、改成 5 匝并把回路阻抗调到 200 Ω：阻抗按匝数平方缩放（10 匝 → 5 匝是 1/4），插损随回路变小。
  await section.getByRole("combobox", { name: "扼流磁环" }).click();
  await page.getByRole("option", { name: /5943003801/ }).click();
  await section.getByLabel("穿孔匝数").fill("5");
  await section.getByLabel("共模回路阻抗 (Ω)").fill("200");
  await expect.poll(() => chokeRow(section, "20m")).toEqual(["20m", "14.18", "1121.6", "888.21", "684.91", "79", "16.16"]);

  // 不合法的匝数：给一句话，不画一张全是 NaN 的表。
  await section.getByLabel("穿孔匝数").fill("0");
  await expect(section.getByRole("status")).toContainText("匝数要是 1 到 100 的整数");
  await expect(section.getByRole("table")).toHaveCount(0);

  // 穿孔次数只能是 1–100 的整数：小数、超限、极端大数都不画表，也不出现 inf / NaN。
  for (const bad of ["2.5", "101", "1e200"]) {
    await section.getByLabel("穿孔匝数").fill(bad);
    await expect(section.getByRole("status"), bad).toContainText("匝数要是 1 到 100 的整数");
    await expect(section.getByRole("table"), bad).toHaveCount(0);
  }
  await section.getByLabel("穿孔匝数").fill("100");
  await expect(section.getByRole("table", { name: "共模扼流各波段阻抗与插入损耗" })).toBeVisible();
  await expect(section.getByRole("table")).not.toContainText(/NaN|inf/i);
});

test("小工具：SMD 电阻与瓷片电容丝印代码解码", async ({ page }) => {
  await page.goto("/tools#smd-resistor");

  // SMD 电阻：472 = 47×10² = 4.70 kΩ；1001 = 100×10¹ = 1 kΩ；68X（EIA-96）= 49.9 Ω。
  const smd = tool(page, "smd-resistor");
  await expect(smd).toBeVisible();
  await expect(smd.getByText("4.70 kΩ")).toBeVisible();
  await smd.getByLabel("丝印代码").fill("1001");
  await expect(smd.getByText("1.00 kΩ")).toBeVisible();
  await smd.getByLabel("丝印代码").fill("68X");
  await expect(smd.getByText("49.90 Ω")).toBeVisible();
  // 无法识别的代码给出提示
  await smd.getByLabel("丝印代码").fill("abc");
  await expect(smd.getByText("无法识别的代码")).toBeVisible();

  // 瓷片电容：104 = 10×10⁴ pF = 100 nF；末尾字母是容差。
  const cap = tool(page, "capacitor-code");
  await expect(cap).toBeVisible();
  await expect(cap.getByText("100.00 nF")).toBeVisible();
  await cap.getByLabel("丝印代码").fill("104K");
  await expect(cap.getByText("100.00 nF · ±10%")).toBeVisible();
  await cap.getByLabel("丝印代码").fill("47");
  await expect(cap.getByText("47.00 pF")).toBeVisible();
  await cap.getByLabel("丝印代码").fill("x");
  await expect(cap.getByText("无法识别的代码")).toBeVisible();
});

test("小工具：单短截线匹配给出接入点与短截线长度", async ({ page }) => {
  await page.goto("/tools#stub-match");
  const section = tool(page, "stub-match");
  await expect(section).toBeVisible();

  // 默认 100 Ω + j0（zL = 2，短路）：d = l = 0.152λ；14.2 MHz、vf 0.66 → λ ≈ 13.93 m。
  await expect(
    section.getByText(/接入点距负载 0\.152λ（2\.12 m）· 短截线长 0\.152λ（2\.12 m）/),
  ).toBeVisible();

  // 50 Ω 纯阻：已匹配，无需短截线。
  await section.getByLabel("负载电阻 R（Ω）").fill("50");
  await section.getByLabel("负载电抗 X（Ω）").fill("0");
  await expect(section.getByText("负载已匹配，无需短截线。")).toBeVisible();

  // 纯电抗负载：无法匹配。
  await section.getByLabel("负载电阻 R（Ω）").fill("0");
  await expect(section.getByText("纯电抗负载无法用无耗短截线匹配。")).toBeVisible();
});

test("小工具：ATU 覆盖综合逐波段判定 Pi / T 网络能否兜住负载区域", async ({ page }) => {
  await page.goto("/tools#atu-synth");
  const section = tool(page, "atu-synth");
  await expect(section).toBeVisible();
  // 行的可访问名就是「波段 覆盖 C下限 L下限」，按名字开头定位，不依赖单元格间的空白。
  const row = (band: string) =>
    section
      .getByRole("table", { name: "各波段的覆盖情况" })
      .getByRole("row", { name: new RegExp(`^${band}\\b`) });

  // 默认 Pi：R 10–500 Ω、X −200…+200 Ω（63 个负载点），C 10–500 pF、L 0.3–30 μH。
  // 期望值来自独立的 Python 实现（与 `crates/core/src/atu_synth.rs` 的单测同一组）：
  // 160 m 只覆盖 12 点、C 必要下限 3536.8 pF；20 m 覆盖 59 点；6 m 覆盖 46 点。
  await expect(row("160m")).toContainText("12 / 63");
  await expect(row("160m")).toContainText("3536.8");
  await expect(row("20m")).toContainText("59 / 63");
  await expect(row("20m")).toContainText("454.7");
  await expect(row("6m")).toContainText("46 / 63");

  // 换成 T 高通：低频段 63 点全覆盖，6 m 只有 49 点（电容下限 10 pF 挡住了）。
  await section.getByRole("radio", { name: /T 高通/ }).click();
  await expect(row("160m")).toContainText("63 / 63");
  await expect(row("160m")).toContainText("125.1");
  await expect(row("6m")).toContainText("49 / 63");

  // 放宽元件范围后 6 m 也能全覆盖：电容下限放到 4 pF。
  await section.getByLabel("电容下限（pF）").fill("4");
  await expect(row("6m")).toContainText("63 / 63");

  // 范围不合法（电阻上限小于下限）：给出提示、不出表。
  await section.getByLabel("负载电阻上限 R（Ω）").fill("5");
  await expect(section.getByText(/请输入有效的阻抗范围/)).toBeVisible();
  await expect(section.getByRole("table", { name: "各波段的覆盖情况" })).toHaveCount(0);
});

test("小工具：台站链路预算给出余量、可达距离与回收潜力排名", async ({ page }) => {
  await page.goto("/tools#station-link");
  const section = tool(page, "station-link");
  await expect(section).toBeVisible();
  const table = section.getByRole("table", { name: "各环节单独做到理想后的回收潜力" });
  const firstRow = table.getByRole("row").nth(1);
  const rowOf = (name: string) => table.getByRole("row", { name: new RegExp(`^${name}`) });

  // 默认：145.5 MHz、200 km、5 W、RG-213 各 10 m、增益 3 / 3 dBi、附加损耗 1 dB、NF 2 dB、
  // Fa = 0、12.5 kHz、所需 SNR 12 dB。期望值来自独立的 Python 原型（噪声用逐级功率累加，
  // 不共用 Rust 里的公式；RG-213 的 √f 插值按 `feedline.rs` 的参考点另算）：
  //   单段馈线 1.03 dB，余量 37.24 dB，到达 SNR 49.24 dB。
  await expect(section.getByText("余量 37.24 dB：到达 SNR 49.24 dB，所需 SNR 为 12.00 dB")).toBeVisible();
  await expect(section.getByText("馈线损耗：发射 1.03 dB · 接收 1.03 dB")).toBeVisible();
  // 只有热噪声：外部噪声一点可回收的都没有，接收链的损耗一比一。NF 2 dB 比单段馈线 1.03 dB 大，排第一。
  await expect(section.getByText(/超出热噪声的部分：外部噪声占 0\.0%，接收链（馈线 \+ 接收机）占 100\.0%/)).toBeVisible();
  await expect(firstRow).toContainText("接收机噪声系数");
  await expect(firstRow).toContainText("2.00");
  await expect(rowOf("外部噪声")).toContainText("0.00");
  await expect(rowOf("接收馈线损耗")).toContainText("1.03");

  // 外部噪声拉到 40 dB（嘈杂的 HF 环境）：噪声几乎全部来自外部（99.99%），排名翻转 ——
  // 外部噪声单独做到理想可多出 36.97 dB，接收馈线与接收机 NF 合起来只剩 0.0005 dB 量级，
  // 而发射馈线与附加损耗不进噪声，仍是一比一。余量只剩 0.27 dB。
  await section.getByLabel("外部噪声 Fa（dB）").fill("40");
  await expect(section.getByText(/外部噪声占 99\.99?%|外部噪声占 100\.0%/)).toBeVisible();
  await expect(firstRow).toContainText("外部噪声");
  await expect(firstRow).toContainText("36.97");
  await expect(rowOf("接收馈线损耗")).toContainText("0.00");
  await expect(rowOf("接收机噪声系数")).toContainText("0.00");
  await expect(rowOf("发射馈线损耗")).toContainText("1.03");
  await expect(section.getByText(/^余量 0\.27 dB/)).toBeVisible();

  // 把所需 SNR 提到远高于到达值：给出「不够，差 … dB」。
  await section.getByLabel("所需 SNR（dB）").fill("200");
  await expect(section.getByText(/^不够，差 \d+\.\d\d dB/)).toBeVisible();

  // 距离缩到 10 m（145.5 MHz 下约 5 个波长，不足 10 个）：落入近场，给出提示而不是悄悄套自由空间公式；
  // 拉回 200 km 提示消失。
  await section.getByLabel("距离（km）").fill("0.01");
  await expect(section.getByText(/距离不足 10 个波长（20\.\d\d m）：已落入近场/)).toBeVisible();
  await section.getByLabel("距离（km）").fill("200");
  await expect(section.getByText(/已落入近场/)).toHaveCount(0);

  // 输入不合法（负的馈线长度）：给出提示、不出表。
  await section.getByLabel("发射馈线长度（m）").fill("-5");
  await expect(section.getByText(/^请输入有效的数值/)).toBeVisible();
  await expect(table).toHaveCount(0);
});

test("小工具：台站链路预算的外部噪声来源按 P.372 曲线给出 Fa，超出范围不外推", async ({ page }) => {
  await page.goto("/tools#station-link");
  const section = tool(page, "station-link");
  await expect(section).toBeVisible();
  const table = section.getByRole("table", { name: "各环节单独做到理想后的回收潜力" });
  const manualFa = section.getByLabel("外部噪声 Fa（dB）");
  const pick = async (name: RegExp) => {
    await section.getByRole("combobox", { name: "外部噪声来源" }).click();
    await page.getByRole("option", { name }).click();
  };

  // 默认「自填」：手填的 Fa 栏在、结果与没有预设时逐位相同（Fa = 0 → 余量 37.24 dB）。
  await expect(manualFa).toBeVisible();
  await expect(section.getByText("余量 37.24 dB：到达 SNR 49.24 dB，所需 SNR 为 12.00 dB")).toBeVisible();

  // 城市（曲线 A）@ 145.5 MHz：Fa = 76.8 − 27.7·lg 145.5 = 16.89 dB（Python 独立算）。
  // 链路其余部分不变，只有噪声变：独立的逐级功率累加路径给出噪声 −116.05 dBm、SNR 35.29 dB、
  // 余量 23.29 dB，外部噪声占超出热噪声部分的 97.9%，单独做到理想可多出 13.95 dB。
  await pick(/^城市/);
  await expect(manualFa).toHaveCount(0);
  await expect(
    section.getByText("城市（P.372 曲线 A）：145.5 MHz 处外部噪声 Fa 的中位数为 16.89 dB"),
  ).toBeVisible();
  await expect(section.getByText("余量 23.29 dB：到达 SNR 35.29 dB，所需 SNR 为 12.00 dB")).toBeVisible();
  await expect(section.getByText(/折算到天线端子的噪声 -116\.05 dBm/)).toBeVisible();
  await expect(section.getByText(/外部噪声占 97\.9%，接收链（馈线 \+ 接收机）占 2\.1%/)).toBeVisible();
  const firstRow = table.getByRole("row").nth(1);
  await expect(firstRow).toContainText("外部噪声");
  await expect(firstRow).toContainText("13.95");

  // 2 m 上安静乡村（曲线 D 只到 30 MHz）没有定义：不外推，说明原因，不出表，也不是泛泛的「输入无效」。
  await pick(/^安静乡村/);
  await expect(
    section.getByText("安静乡村（P.372 曲线 D） 只覆盖 0.3–30 MHz，145.5 MHz 在范围外（曲线不外推）：请改用「自填」。"),
  ).toHaveCount(1);
  await expect(table).toHaveCount(0);
  await expect(section.getByText(/^请输入有效的数值/)).toHaveCount(0);

  // 边界含在内：30 MHz 有值（53.6 − 28.6·lg 30 = 11.35 dB），30.1 MHz 就没有。
  await section.getByLabel("频率（MHz）").fill("30");
  await expect(
    section.getByText("安静乡村（P.372 曲线 D）：30 MHz 处外部噪声 Fa 的中位数为 11.35 dB"),
  ).toBeVisible();
  await section.getByLabel("频率（MHz）").fill("30.1");
  await expect(section.getByText(/30\.1 MHz 在范围外/)).toBeVisible();

  // 20 m（14.2 MHz）：安静乡村 20.64 dB；银河噪声 25.50 dB（曲线 E 从 10 MHz 起，9.9 MHz 就没有）。
  await section.getByLabel("频率（MHz）").fill("14.2");
  await expect(
    section.getByText("安静乡村（P.372 曲线 D）：14.2 MHz 处外部噪声 Fa 的中位数为 20.64 dB"),
  ).toBeVisible();
  await pick(/^银河噪声/);
  await expect(
    section.getByText("银河噪声（P.372 曲线 E，理论下限）：14.2 MHz 处外部噪声 Fa 的中位数为 25.50 dB"),
  ).toBeVisible();
  await section.getByLabel("频率（MHz）").fill("9.9");
  await expect(section.getByText(/只覆盖 10–100 MHz，9\.9 MHz 在范围外/)).toBeVisible();

  // 切回「自填」：手填栏回来，原来填的值（0）还在，结果回到起点之外的同一条链路（频率现在是 9.9 MHz，
  // 所以只断言不再是范围提示、且有结果表）。
  await pick(/^自填/);
  await expect(manualFa).toBeVisible();
  await expect(manualFa).toHaveValue("0");
  await expect(section.getByText(/在范围外/)).toHaveCount(0);
  await expect(table).toHaveCount(1);
});

test("小工具：台站链路预算可选大气噪声（P.372），随月份与当地时间变化", async ({ page }) => {
  await page.goto("/tools#station-link");
  const section = tool(page, "station-link");
  await expect(section).toBeVisible();
  const pick = async (name: RegExp) => {
    await section.getByRole("combobox", { name: "外部噪声来源" }).click();
    await page.getByRole("option", { name }).click();
  };
  // 7 月 40°N 116°E、UTC 12（当地约 19:44，傍晚雷电最旺）、20 m。
  // 参照值由官方 P372 程序给出（`crates/core/reference/p372_check.py`）：Fa = 46.30 dB。
  // 大气噪声只定义到 30 MHz（原文的适用频段），默认的 145.5 MHz 会落到「输入无效」。
  await section.getByLabel("频率（MHz）").fill("7");
  await pick(/^大气噪声/);
  const month = section.getByLabel("月份（1–12）");
  const hour = section.getByLabel("UTC 小时（0–23）");
  await expect(month).toHaveValue("7");
  await expect(hour).toHaveValue("12");
  await expect(section.getByLabel("纬度（°，北正）")).toHaveValue("40");
  await expect(section.getByLabel("经度（°，东正）")).toHaveValue("116");
  // 手填的 Fa 栏在大气噪声这一项下不显示。
  await expect(section.getByLabel("外部噪声 Fa（dB）")).toHaveCount(0);
  await expect(section.getByRole("table", { name: "各环节单独做到理想后的回收潜力" })).toHaveCount(1);
  // Fa = 46.30 dB 走进整条链路：其余输入是页面默认（5 W、两端 10 m RG-213、增益各 3 dBi、附加损耗 1 dB、
  // NF 2 dB、带宽 12.5 kHz、所需 SNR 12 dB、距离 200 km），7 MHz 下 RG-213 10 m 损耗 0.2121 dB。
  // 期望值由 Python 独立算（噪声 = -174 + 10lg B + 10lg(Fa + Lr·F_rx - 1)；信号取天线端子处，不扣接收馈线）：
  // 噪声 -86.73 dBm、SNR 33.13 dB、余量 21.13 dB。把大气噪声接成别的值（或根本没接进去）都过不了这三个数。
  await expect(section.getByText(/折算到天线端子的噪声 -86\.73 dBm/)).toBeVisible();
  await expect(section.getByText("余量 21.13 dB：到达 SNR 33.13 dB，所需 SNR 为 12.00 dB")).toBeVisible();

  // 当地时间变化：UTC 18（当地约 01:44）Fa = 43.72 dB，比傍晚弱 2.6 dB：噪声 -89.31 dBm、余量 23.72 dB。
  await hour.fill("18");
  await expect(section.getByText(/折算到天线端子的噪声 -89\.31 dBm/)).toBeVisible();
  await expect(section.getByText("余量 23.72 dB：到达 SNR 35.72 dB，所需 SNR 为 12.00 dB")).toBeVisible();

  // 越界输入（13 月）给出提示、不出表。重新进页面：工具页的计算器是分批挂载的，
  // 同一页里前面的输入步骤会让「工具目录」重新定位、section 被卸载重挂，旧 locator 会失效。
  await page.goto("/tools#station-link");
  await section.getByLabel("频率（MHz）").fill("7");
  await section.getByRole("combobox", { name: "外部噪声来源" }).click();
  await page.getByRole("option", { name: /^大气噪声/ }).click();
  await section.getByLabel("月份（1–12）").fill("13");
  await expect(section.getByText(/^请输入有效的数值（频率、距离、功率/)).toBeVisible();
  await expect(section.getByRole("table", { name: "各环节单独做到理想后的回收潜力" })).toHaveCount(0);
});

test("小工具：磁环小环天线估算电感 / 效率 / Q / 调谐电容", async ({ page }) => {
  await page.goto("/tools#magnetic-loop");
  const section = tool(page, "magnetic-loop");
  await expect(section).toBeVisible();

  // 默认 1 m 环、10 mm 铜管 @ 14.2 MHz、100 W：L ≈ 2.94 µH，调谐电容 ≈ 42 pF。
  await expect(section.getByText(/电感 2\.94 µH · 调谐电容 42\.\d+ pF/)).toBeVisible();
  // 小环辐射电阻极小、Q 高、电容电压到 kV 级。
  await expect(section.getByText(/辐射电阻 0\.1\d Ω · 损耗电阻 0\.1\d Ω · 效率 \d+%/)).toBeVisible();
  await expect(section.getByText(/Q \d+ · 带宽 \d+\.\d kHz/)).toBeVisible();
  await expect(section.getByText(/电容两端电压 \d+\.\d kV/)).toBeVisible();

  // 输入不合法给出提示
  await section.getByLabel("环直径 D（m）").fill("0");
  await expect(section.getByText("请输入有效的环径、导体直径、频率、匝数与功率。")).toBeVisible();
});

test("小工具：离网供电逐日能量平衡判断能否自持", async ({ page }) => {
  await page.goto("/tools#off-grid");
  const section = tool(page, "off-grid");
  await expect(section).toBeVisible();

  // 默认 100 W 板、4 h 日照、300 Wh 负载：发电 342 Wh、电池可用 640 Wh，每日盈余 42 Wh 自持。
  await expect(section.getByText(/每日发电 342 Wh · 电池可用 640 Wh/)).toBeVisible();
  await expect(section.getByText(/每日盈余 42 Wh，可长期自持/)).toBeVisible();

  // 负载提到 500 Wh：每日亏空 158 Wh，满电电池可撑约 4.1 天。
  await section.getByLabel("每日负载（Wh）").fill("500");
  await expect(section.getByText(/每日亏空 158 Wh，满电电池可撑 4\.\d 天/)).toBeVisible();

  // 输入不合法给出提示
  await section.getByLabel("太阳能板功率（W）").fill("0");
  await expect(section.getByText("请输入有效的功率、日照、负载与电池参数。")).toBeVisible();
});

test("小工具：微带线给出 εeff，带状线给出特性阻抗", async ({ page }) => {
  await page.goto("/tools#tx-line");
  const section = tool(page, "tx-line");
  await expect(section).toBeVisible();

  // 微带线（εr=2.25、W/h=1.9）除 Z0 外还给出 εeff ≈ 1.86。
  await expect(section.getByText(/εeff ≈ 1\.86/)).toBeVisible();

  // 带状线（εr=2.25、W/b=1）默认 Z0 ≈ 43.6 Ω。
  await expect(section.getByText(/带状线特性阻抗 ≈ 43\.6\d Ω/)).toBeVisible();

  // 地平面间距减半（W/b=2）：Z0 降到 ≈ 25.7 Ω。
  await section.getByLabel("地平面间距 b（mm）").fill("0.5");
  await expect(section.getByText(/带状线特性阻抗 ≈ 25\.7\d Ω/)).toBeVisible();
});

test("小工具：电短天线加载线圈估算谐振电感", async ({ page }) => {
  await page.goto("/tools#loading-coil");
  const section = tool(page, "loading-coil");
  await expect(section).toBeVisible();

  // 默认 2.4 m 鞭、2 mm 导体 @ 7 MHz：辐射电阻 ≈ 1.24 Ω，加载电感 ≈ 29 µH。
  await expect(section.getByText(/辐射电阻 1\.24 Ω · 天线电容 17\.8\d pF/)).toBeVisible();
  await expect(section.getByText(/容抗 1\.27 kΩ · 加载电感 28\.\d+ µH/)).toBeVisible();

  // 输入不合法给出提示
  await section.getByLabel("天线高度 h（m）").fill("0");
  await expect(section.getByText("请输入有效的天线高度、导体直径与频率。")).toBeVisible();
});

test("小工具：数字模式编码可导出基带 WAV", async ({ page }) => {
  await page.goto("/tools#mode-encoder");
  const section = tool(page, "mode-encoder");
  await expect(section).toBeVisible();

  // 默认文本 "CQ CQ DE BG1AAA" 编出摩尔斯与 RTTY 比特流。
  await expect(section.getByText("CW 摩尔斯电码", { exact: true })).toBeVisible();
  await expect(section.getByText("RTTY（ITA2）比特流", { exact: true })).toBeVisible();

  // 下载 CW WAV：文件名 cw.wav。
  const dl1 = page.waitForEvent("download");
  await section.getByRole("button", { name: "下载 CW WAV" }).click();
  expect((await dl1).suggestedFilename()).toBe("cw.wav");

  // 下载 RTTY WAV：文件名 rtty.wav。
  const dl2 = page.waitForEvent("download");
  await section.getByRole("button", { name: "下载 RTTY WAV" }).click();
  expect((await dl2).suggestedFilename()).toBe("rtty.wav");

  // 下载 PSK31 WAV：文件名 psk31.wav。
  const dl3 = page.waitForEvent("download");
  await section.getByRole("button", { name: "下载 PSK31 WAV" }).click();
  expect((await dl3).suggestedFilename()).toBe("psk31.wav");

  // WSPR：110.592 秒 × 12 kHz × 2 字节 + 44 字节 WAV 头，长度是钉死的。
  const dl4 = page.waitForEvent("download");
  await section.getByRole("button", { name: "下载 WSPR WAV" }).click();
  const wspr = await dl4;
  expect(wspr.suggestedFilename()).toBe("wspr.wav");
  const wsprPath = await wspr.path();
  expect(wsprPath).not.toBeNull();
  const { size } = await stat(wsprPath as string);
  expect(size).toBe(44 + 162 * 8192 * 2);

  // NAVTEX：报头 + 正文，长度随正文变化，只钉文件名与「不是空文件」。
  const dl5 = page.waitForEvent("download");
  await section.getByRole("button", { name: "下载 NAVTEX WAV" }).click();
  const navtex = await dl5;
  expect(navtex.suggestedFilename()).toBe("navtex.wav");
  const navtexPath = await navtex.path();
  expect(navtexPath).not.toBeNull();
  const navtexFile = await stat(navtexPath as string);
  expect(navtexFile.size).toBeGreaterThan(2000);

  // Feld Hell：字库里没有的字符会被跳过，长度随正文变化，同样只钉文件名与非空。
  const dl6 = page.waitForEvent("download");
  await section.getByRole("button", { name: "下载 Hell WAV" }).click();
  const hell = await dl6;
  expect(hell.suggestedFilename()).toBe("hell.wav");
  const hellPath = await hell.path();
  expect(hellPath).not.toBeNull();
  expect((await stat(hellPath as string)).size).toBeGreaterThan(2000);

  // FT8：整个 15 秒时隙，12 kHz × 15 s × 2 字节 + 44 字节 WAV 头，长度是钉死的。
  const dl7 = page.waitForEvent("download");
  await section.getByRole("button", { name: "下载 FT8 WAV" }).click();
  const ft8 = await dl7;
  expect(ft8.suggestedFilename()).toBe("ft8.wav");
  const ft8Path = await ft8.path();
  expect(ft8Path).not.toBeNull();
  expect((await stat(ft8Path as string)).size).toBe(44 + 15 * 12000 * 2);
});

test("小工具：FT8 发端拒绝无法编码的报文", async ({ page }) => {
  await page.goto("/tools#mode-encoder");
  const section = tool(page, "mode-encoder");
  // 认不出标准格式就按自由文本，超过 13 个字符编不进 71 位。
  await section.getByLabel("FT8 发端报文").fill("THIS IS WAY TOO LONG");
  await section.getByRole("button", { name: "下载 FT8 WAV" }).click();
  await expect(page.getByText(/报文无法编码/)).toBeVisible();
});

test("小工具：WSPR 发端拒绝非法呼号 / 网格", async ({ page }) => {
  await page.goto("/tools#mode-encoder");
  const section = tool(page, "mode-encoder");
  await section.getByLabel("WSPR 发端网格").fill("ZZZZ");
  await section.getByRole("button", { name: "下载 WSPR WAV" }).click();
  // 页面上有多个 `role="status"`（各工具的内联结果区），这里只认提示文案。
  await expect(page.getByText(/呼号与网格要合法/)).toBeVisible();
});

test("小工具：NAVTEX 发端拒绝非法报头", async ({ page }) => {
  await page.goto("/tools#mode-encoder");
  const section = tool(page, "mode-encoder");
  await section.getByLabel("NAVTEX 台站字母").fill("1");
  await section.getByRole("button", { name: "下载 NAVTEX WAV" }).click();
  await expect(page.getByText(/台站与类型各填一个字母/)).toBeVisible();
});

test("小工具：陷波器给出两波段 trap 偶极子设计", async ({ page }) => {
  await page.goto("/tools#trap");
  const section = tool(page, "trap");
  await expect(section).toBeVisible();

  // 默认 40m+20m、67 pF：电感 ≈ 1.88 µH，内段 ≈ 5.04 m；外段 143/7.05/2 − 5.04 = 5.10 m
  // 扣掉 trap 在低波段的感性加载（0.64 m）后是 4.46 m。
  await expect(section.getByText(/电感 1\.88 µH · 内段 5\.04 m · 外段 4\.46 m/)).toBeVisible();
  await expect(section.getByText(/低波段加载电抗 110\.\d+ Ω/)).toBeVisible();

  // 电容改小 → 电感变大 → 低波段加载更重 → 外段更短（30 pF 时 3.84 m，独立 Python 算得）。
  await section.getByLabel("trap 电容（pF）").fill("30");
  await expect(section.getByText(/电感 4\.20 µH · 内段 5\.04 m · 外段 3\.84 m/)).toBeVisible();

  // 输入不合法给出提示（高波段不大于低波段）。
  await section.getByLabel("高波段频率（MHz）").fill("7");
  await section.getByLabel("低波段频率（MHz）").fill("14");
  await expect(
    section.getByText("请输入有效的高 / 低波段频率（高波段需大于低波段）与电容。"),
  ).toBeVisible();
});

test("小工具：AX.25 软 TNC 编码下载与解码往返", async ({ page }) => {
  await page.goto("/tools#soft-tnc");
  const section = tool(page, "soft-tnc");
  await expect(section).toBeVisible();

  // 下载 AFSK1200 WAV：文件名 afsk1200.wav。
  const dl = page.waitForEvent("download");
  await section.getByRole("button", { name: "下载 AFSK1200 WAV" }).click();
  const file = await dl;
  expect(file.suggestedFilename()).toBe("afsk1200.wav");

  // 把下载的 WAV 上传回解码，解出源 / 目的 / 信息。
  // 解调在 Worker 里跑（wasm 首次加载较慢），放宽到 30s，与其它解码页一致。
  await section.locator('input[type="file"]').setInputFiles(await file.path());
  await expect(section.getByText(/源 BG1AAA → 目的 APRS：>hello/)).toBeVisible({
    timeout: 30000,
  });
});

test("小工具：AX.25 软 TNC 信息域超长时单独提示，而不是怪呼号", async ({ page }) => {
  // `ax25_ui_frame` 对「呼号非法」与「整帧超长」都返回 None：以前一律报「呼号格式不合法」，
  // 用户会去检查一个其实没有错的呼号（回归：超长要单独提示，不能怪呼号）。
  await page.goto("/tools#soft-tnc");
  const section = tool(page, "soft-tnc");
  await expect(section).toBeVisible();

  await section.getByLabel("信息", { exact: true }).fill("x".repeat(3000));
  await section.getByRole("button", { name: "下载 AFSK1200 WAV" }).click();
  await expect(page.getByText(/信息域超过 \d+ 字节/)).toBeVisible();
  await expect(page.getByText("呼号格式不合法")).toHaveCount(0);

  // 呼号确实非法时仍然报呼号。
  await section.getByLabel("信息", { exact: true }).fill(">hello");
  await section.getByLabel("源呼号").fill("TOOLONGCALL-99");
  await section.getByRole("button", { name: "下载 AFSK1200 WAV" }).click();
  await expect(page.getByText(/呼号格式不合法/)).toBeVisible();
});

// `/tools` 里有很多带表格 / 图表的块，表格自带的 `min-w-[…]` 放进网格子项里会把整列撑宽（需要 `min-w-0`），
// 手机宽度下整页横向滚动。i18n_layout 只在桌面宽度量，所以这里单独守一次。
for (const [code, name] of [
  ["zh", "中文"],
  ["en", "English"],
  ["es", "Español"],
] as const) {
  test(`小工具：手机宽度下整页不产生横向滚动（${name}）`, async ({ page }) => {
    await page.addInitScript((c) => localStorage.setItem("locale", c), code);
    for (const width of [320, 390]) {
      await page.setViewportSize({ width, height: 900 });
      await page.goto("/tools");
      await expect(page.locator("section#path-profile")).toBeVisible();
      await expect(page.locator("section#toroid-winding")).toBeVisible();
      await expect(page.locator("section#intermod")).toBeVisible();
      await expect(page.locator("section#parabolic-dish")).toBeVisible();
      // 计算器主体是分批挂上的（`tool_mount.rs`）：卡片外壳一开始就可见，主体还没到时带 `aria-busy`。
      // 不等它们全部就位，量到的只是一排占位块，溢出检查就成了摆设。
      await expect(page.locator('main section[aria-busy="true"]')).toHaveCount(0);
      const overflow = await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth);
      expect(overflow, `${width}px`).toBeLessThanOrEqual(1);
    }
  });
}

test("小工具：互调与镜频列出产物并标出落入业余波段的", async ({ page }) => {
  await page.goto("/tools#intermod");
  const section = tool(page, "intermod");
  await expect(section).toBeVisible();
  const freqs = section.getByLabel("发射频率（MHz）");

  // 默认 14.200 / 14.205：谐波 8 + 2 阶 2 + 3 阶 4 = 14 个产物，其中 5 个在业余波段内
  // （28.4、28.41、28.405 在 10m；14.195、14.21 在 20m）
  await expect(section.getByRole("status")).toContainText("共 14 个产物");
  await expect(section.getByRole("status")).toContainText("其中 5 个落入业余波段");
  await expect(section.getByRole("row")).toHaveCount(6); // 表头 + 5
  const low = section.getByRole("row", { name: /2·f1 − f2/ });
  await expect(low).toContainText("14.195");
  await expect(low).toContainText("20m");
  // 表达式单元格精确匹配：`f1 + f2` 不能同时命中 `2·f1 + f2`
  const rowOf = (expr: string) =>
    section.getByRole("row").filter({ has: page.getByRole("cell", { name: expr, exact: true }) });
  await expect(rowOf("f1 + f2")).toContainText("28.405");
  // 差频 0.005 与 42.6 不在业余波段：默认被过滤掉
  await expect(section.getByRole("cell", { name: "0.005", exact: true })).toHaveCount(0);

  // 取消过滤：全部 14 个都显示，不在业余波段的标 —
  await section.getByRole("checkbox", { name: "只显示落入业余波段的产物" }).click();
  await expect(section.getByRole("row")).toHaveCount(15);
  await expect(rowOf("f2 − f1")).toContainText("0.005");

  // 3.65 MHz 的谐波（7.3 / 10.95 / 14.6 / 18.25）全部落空：过滤时给出说明而不是空表
  await section.getByRole("checkbox", { name: "只显示落入业余波段的产物" }).click();
  await freqs.fill("3.65");
  await expect(section.getByText(/没有产物落入业余波段/)).toBeVisible();
  await expect(section.getByRole("table")).toHaveCount(0);

  // 输入不合法：给出带序号的引导
  await freqs.fill("14.2, abc");
  await expect(section.getByRole("alert")).toContainText("第 2 个值不是正数");
  await freqs.fill("");
  await expect(section.getByRole("alert")).toContainText("请至少输入一个频率");
  await freqs.fill("1 2 3 4 5 6 7 8 9");
  await expect(section.getByRole("alert")).toContainText("最多支持 8 个不同的频率");
  // 重复的频率不占名额
  await freqs.fill("1 2 3 4 5 6 7 8 8 8");
  await expect(section.getByRole("alert")).toHaveCount(0);
});

test("小工具：镜频按高端 / 低端本振给出两侧并判断是否落入业余波段", async ({ page }) => {
  await page.goto("/tools#intermod");
  const section = tool(page, "intermod");
  const rx = section.getByLabel("接收频率（MHz）");
  const ifm = section.getByLabel("中频（MHz）");

  // 默认 145 MHz / 10.7 MHz：166.4 与 123.6，都不在业余波段
  await expect(section.getByText(/高端本振.*镜频 166\.4 MHz，不在业余波段内/)).toBeVisible();
  await expect(section.getByText(/低端本振.*镜频 123\.6 MHz，不在业余波段内/)).toBeVisible();

  // 14.1 MHz、中频 3.5：21.1（15m）与 7.1（40m）
  await rx.fill("14.1");
  await ifm.fill("3.5");
  await expect(section.getByText(/镜频 21\.1 MHz，落入 15m 波段/)).toBeVisible();
  await expect(section.getByText(/镜频 7\.1 MHz，落入 40m 波段/)).toBeVisible();

  // 中频太高：低端镜频为负，没有这一侧
  await rx.fill("7.1");
  await ifm.fill("4");
  await expect(section.getByText(/镜频 15\.1 MHz/)).toBeVisible();
  await expect(section.getByText(/中频过高，镜频为负/)).toBeVisible();

  // 不合法
  await ifm.fill("0");
  await expect(section.getByText(/请输入有效的接收频率与中频/)).toBeVisible();
});

test("小工具：抛物面天线增益、波束与焦点几何", async ({ page }) => {
  await page.goto("/tools#parabolic-dish");
  const section = tool(page, "parabolic-dish");
  await expect(section).toBeVisible();

  // 默认值在输入框里要显示成干净的数字（0.55 × 100 的浮点误差曾显示成 55.00000000000001）
  await expect(section.getByLabel("孔径效率（%）")).toHaveValue("55");
  await expect(section.getByLabel("焦径比 f/D")).toHaveValue("0.4");
  // 默认 10.368 GHz / 0.9 m / 55% / f/D 0.4（预期值由独立脚本算出）：
  // λ = 0.0289 m，口径 31.13 个波长；G = 0.55·(π·D/λ)² = 5258.9 倍 = 37.21 dBi；A_e = 0.3499 m²
  await expect(section.getByText(/波长 0\.0289 m　｜　口径 31\.13 个波长/)).toBeVisible();
  await expect(section.getByRole("status")).toContainText("增益 37.21 dBi（5258.9 倍）");
  await expect(section.getByRole("status")).toContainText("有效面积 0.3499 m²");
  await expect(section.getByText(/波束宽度约 2\.25°/)).toBeVisible();
  // f = 0.36 m，深 0.1406 m，焦点在口径平面前 0.2194 m，张角 128.02°
  await expect(section.getByText(/焦距 0\.3600 m　｜　凹陷深度 0\.1406 m/)).toBeVisible();
  await expect(section.getByText(/口径平面前方 0\.2194 m 处/)).toBeVisible();
  await expect(section.getByText(/全张角 128\.02°/)).toBeVisible();
  await expect(section.getByText(/增益公式在这里不再可靠/)).toHaveCount(0);

  // 口径翻倍：增益 +6.02 dB、波束减半
  await section.getByLabel("口径（m）").fill("1.8");
  await expect(section.getByRole("status")).toContainText("增益 43.23 dBi");
  await expect(section.getByText(/波束宽度约 1\.12°/)).toBeVisible();

  // f/D = 0.25：焦点恰在口径平面上，张角 180°
  await section.getByLabel("口径（m）").fill("0.9");
  await section.getByLabel("焦径比 f/D").fill("0.25");
  await expect(section.getByText(/焦点恰在口径平面上/)).toBeVisible();
  await expect(section.getByText(/全张角 180\.00°/)).toBeVisible();

  // f/D = 0.2：深碗，焦点在碗内 0.1012 m，张角超过 180°
  await section.getByLabel("焦径比 f/D").fill("0.2");
  await expect(section.getByText(/焦点在碗内，距口径平面 0\.1012 m（深碗）/)).toBeVisible();
  await expect(section.getByText(/全张角 205\.36°/)).toBeVisible();

  // 效率翻倍到合法上限之外：给出引导
  await section.getByLabel("孔径效率（%）").fill("120");
  await expect(section.getByText(/请输入有效的频率、口径、焦径比/)).toBeVisible();
  await section.getByLabel("孔径效率（%）").fill("55");

  // 低频小口径：145 MHz、1 m，口径不到半个波长
  await section.getByLabel("频率（MHz）").fill("145");
  await section.getByLabel("口径（m）").fill("1");
  await section.getByLabel("焦径比 f/D").fill("0.4");
  await expect(section.getByText(/增益公式在这里不再可靠/)).toBeVisible();
});

test("小工具：抛物面天线用实测凹陷深度反推焦距", async ({ page }) => {
  await page.goto("/tools#parabolic-dish");
  const section = tool(page, "parabolic-dish");
  const depth = section.getByLabel("实测凹陷深度（m）");

  // 口径 0.9 m、深 0.15 m：f = 0.81/(16·0.15) = 0.3375 m，f/D = 0.375
  await expect(section.getByText(/焦距 0\.3375 m　｜　f\/D = 0\.3750/)).toBeVisible();
  // 深度 = D/4 = 0.225 → f/D = 0.25
  await depth.fill("0.225");
  await expect(section.getByText(/f\/D = 0\.2500/)).toBeVisible();

  await depth.fill("0");
  await expect(section.getByText(/请输入大于 0 的口径与深度/)).toBeVisible();
});

test("小工具：地波场强按 P.368 给出场强与传输损耗，数值与 NTIA 官方程序一致", async ({ page }) => {
  await page.goto("/tools#ground-wave");
  const section = tool(page, "ground-wave");
  await expect(section).toBeVisible();
  const table = section.getByRole("table", { name: "地波场强随距离" });
  const pick = async (label: string, name: RegExp) => {
    await section.getByRole("combobox", { name: label }).click();
    await page.getByRole("option", { name }).click();
  };

  // 期望值全部来自 NTIA 随 P.368-10 发布的 LFMF 源码（clang 编成命令行程序后喂同一批输入）：
  // 默认 1 MHz、100 km、中等干燥地（εr 15、σ 1 mS/m）、垂直极化、天线贴地、1 kW、Ns 315。
  await expect(section.getByText("场强 29.39 dB(µV/m)；基本传输损耗 112.60 dB；接收功率 -43.04 dBm（4.77 dBi 参考天线）")).toBeVisible();
  await expect(section.getByText("解法：残留级数（绕射区）")).toBeVisible();
  await expect(section.getByText("地面常数：εr = 15，σ = 0.001 S/m")).toBeVisible();
  // 1 MHz 的分界在 80·f^(−1/3) = 80 km：50 km 走平地近似，100 km 起走残留级数。
  await expect(table.getByRole("row", { name: /^1 104\.89 37\.09 平地近似/ })).toBeVisible();
  await expect(table.getByRole("row", { name: /^50 42\.73 99\.26 平地近似/ })).toBeVisible();
  await expect(table.getByRole("row", { name: /^100 29\.39 112\.60 残留级数/ })).toBeVisible();
  await expect(table.getByRole("row", { name: /^1000 -63\.13 205\.11 残留级数/ })).toBeVisible();

  // 100 km 的 29.39 dB(µV/m) 是能收到的信号：不出「低于噪声底」提示；拉到 1000 km（-63.13）才出。
  const floorNote = section.getByText(/^场强低于 -40 dB\(µV\/m\)，已远在任何实际噪声底之下/);
  await expect(floorNote).toHaveCount(0);
  await section.getByLabel("距离（km）").fill("1000");
  await expect(section.getByText(/^场强 -63\.13 dB\(µV\/m\)/)).toBeVisible();
  await expect(floorNote).toBeVisible();
  await section.getByLabel("距离（km）").fill("100");
  await expect(floorNote).toHaveCount(0);

  // 水平极化衰减快得多：同一条 100 km 路径只剩 −26.04 dB(µV/m)。
  await pick("极化", /^水平/);
  await expect(section.getByText(/^场强 -26\.04 dB\(µV\/m\)；基本传输损耗 168\.03 dB/)).toBeVisible();
  await pick("极化", /^垂直/);

  // 海水（平均盐度，εr 70、σ 5 S/m）在 7 MHz、500 km：25.15 dB(µV/m)。
  await pick("地面类型", /^海水（平均盐度）/);
  await section.getByLabel("频率（MHz）").fill("7");
  await section.getByLabel("距离（km）").fill("500");
  await expect(section.getByText(/^场强 25\.15 dB\(µV\/m\)；基本传输损耗 133\.74 dB/)).toBeVisible();
  await expect(section.getByText("地面常数：εr = 70，σ = 5 S/m")).toBeVisible();
  // 预设时没有 εr / σ 输入框，免得出现一个填了不生效的栏。
  await expect(section.getByLabel("相对介电常数 εr")).toHaveCount(0);

  // 自己填 εr / σ：30 与 0.01（湿地）、两端天线抬高、100 W、14 MHz、30 km。
  await pick("地面类型", /^自己填/);
  await expect(section.getByLabel("相对介电常数 εr")).toHaveValue("70");
  await section.getByLabel("相对介电常数 εr").fill("30");
  await section.getByLabel("电导率 σ（S/m）").fill("0.01");
  await section.getByLabel("频率（MHz）").fill("14");
  await section.getByLabel("距离（km）").fill("30");
  await section.getByLabel("发射天线高度（m）").fill("10");
  await section.getByLabel("接收天线高度（m）").fill("20");
  await section.getByLabel("发射功率（W）").fill("100");
  await expect(section.getByText(/^场强 22\.94 dB\(µV\/m\)；基本传输损耗 131\.96 dB/)).toBeVisible();
  await expect(section.getByText("解法：平地近似 + 曲率修正")).toBeVisible();

  // 距离越界：给出范围提示、不出主结果；其余输入合法时距离表仍在（表不依赖这一项）。
  await section.getByLabel("距离（km）").fill("20000");
  await expect(section.getByText(/^输入无效：频率 0\.01–30 MHz/)).toBeVisible();
  await expect(table).toHaveCount(1);
  // 电导率为 0：整张表都不成立，表一并消失。
  await section.getByLabel("电导率 σ（S/m）").fill("0");
  await expect(table).toHaveCount(0);
});

test("小工具：外部噪声按业余波段列出大气 / 人为 / 银河与合成，数值与 P.372 官方程序一致", async ({ page }) => {
  await page.goto("/tools#external-noise");
  const section = tool(page, "external-noise");
  await expect(section).toBeVisible();
  const table = section.getByRole("table", { name: "按业余波段的外部噪声" });
  const row = (re: RegExp) => table.getByRole("row", { name: re });
  const pick = async (name: RegExp) => {
    await section.getByRole("combobox", { name: "外部噪声来源" }).click();
    await page.getByRole("option", { name }).click();
  };

  // 期望值来自 ITU-R SG3 的官方 P372 程序（编成命令行后逐行喂同样的输入；官方的月份是 0 起的）：
  // 默认「住宅、7 月、UTC 12 时、北纬 40° 东经 116°」。人为 / 银河两列由公式另算：
  // 住宅 72.5 − 27.7·lg f，银河 52 − 23·lg f。
  await expect(table.getByRole("row")).toHaveCount(12); // 表头 + 11 个频率
  await expect(row(/^0\.475 78\.0 81\.5 59\.4 81\.0 \+11\.4 \/ −10\.5 人为/)).toBeVisible();
  await expect(row(/^7\.1 46\.1 48\.9 32\.4 50\.5 \+10\.1 \/ −4\.6 人为/)).toBeVisible();
  await expect(row(/^28\.5 -6\.7 32\.2 18\.5 32\.3 \+10\.6 \/ −5\.2 人为/)).toBeVisible();

  // 安静乡村：7.1 MHz 大气噪声（46.1）盖过人为（29.3），28.5 MHz 则掉到银河（18.5）之下。
  await pick(/^安静乡村/);
  await expect(row(/^7\.1 46\.1 29\.3 32\.4 46\.4 \+7\.0 \/ −6\.5 大气/)).toBeVisible();
  await expect(row(/^28\.5 -6\.7 12\.0 18\.5 18\.5 \+6\.5 \/ −2\.0 银河/)).toBeVisible();

  // 1 月 UTC 0 时：3.65 MHz 上银河（39.1）略高于大气（38.0），到 5.36 MHz 又翻回大气 —— 主导来源会换手。
  await section.getByLabel("月份（1–12）").fill("1");
  await section.getByLabel("UTC 小时（0–23）").fill("0");
  await expect(row(/^3\.65 38\.0 37\.5 39\.1 43\.2 \+7\.5 \/ −5\.3 银河/)).toBeVisible();
  await expect(row(/^5\.36 35\.9 32\.7 35\.2 39\.7 \+7\.3 \/ −5\.7 大气/)).toBeVisible();

  // 月份必须是 1–12 的整数：7.5 不能悄悄截成 7，13 也不行；都给提示、不出表。
  await section.getByLabel("月份（1–12）").fill("7.5");
  await expect(section.getByText(/^输入无效：月份为 1–12 的整数/)).toBeVisible();
  await expect(table).toHaveCount(0);
  await section.getByLabel("月份（1–12）").fill("13");
  await expect(section.getByText(/^输入无效：月份为 1–12 的整数/)).toBeVisible();
  await section.getByLabel("月份（1–12）").fill("1");
  await expect(table).toHaveCount(1);
});
