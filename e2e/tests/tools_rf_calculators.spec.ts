import { expect, pickOption, test, type Page } from "./fixtures";

/**
 * 小工具里的四个射频计算器：占空比与平均功率、IP3 级联、矩形波导、雨衰。
 * 期望值来自独立脚本（不复用页面或内核的任何逻辑）；占空比的几个数是 FCC OET-65 补充 B 的算例。
 */
function tool(page: Page, id: string) {
  return page.locator(`section#${id}`);
}

test("小工具：占空比与平均功率按模式与滑动窗口折算，并复现 OET-65B 的算例", async ({ page }) => {
  await page.goto("/tools#duty-power");
  const s = tool(page, "duty-power");
  await expect(s).toBeVisible();

  // 默认：100 W、SSB 无压缩（20%）、发 120 s / 停 120 s、6 分钟窗口 → 窗内最坏 240 s = 66.67%。
  await expect(s.getByText("平均功率 13.33 W（41.25 dBm）")).toBeVisible();
  await expect(s.getByText("最坏窗内发射时间 66.67%，总占空比 13.33%")).toBeVisible();
  await expect(s.getByText(/距离折算系数 0\.37/)).toBeVisible();

  // OET-65B：1500 W SSB，7 分钟发 / 7 分钟停。6 分钟窗口里最坏是连续 6 分钟 → 300 W。
  await s.getByLabel("峰值包络功率 PEP（W）").fill("1500");
  await s.getByLabel("每次发射（秒）").fill("420");
  await s.getByLabel("每次停歇（秒）").fill("420");
  await expect(s.getByText("平均功率 300.00 W")).toBeVisible();
  await expect(s.getByText(/最坏窗内发射时间 100\.00%/)).toBeVisible();

  // 30 分钟窗口：任意 30 分钟里最多 16 分钟在发射 → 160 W（公告取两位小数 0.53 写作 159 W）。
  await pickOption(page, "平均时间窗", "30 分钟（不受控暴露）");
  await expect(s.getByText("平均功率 160.00 W（52.04 dBm）")).toBeVisible();
  await expect(s.getByText(/最坏窗内发射时间 53\.33%，总占空比 10\.67%/)).toBeVisible();

  // 改用「窗内发射时间」：公告对 2 分钟发 / 2 分钟停写的是 30 分钟里 20 分钟 → 200 W。
  await pickOption(page, "发射方式", "窗内发射时间");
  await s.getByLabel("窗内发射时间（秒）").fill("1200");
  await expect(s.getByText("平均功率 200.00 W")).toBeVisible();
  // 窗内发射时间不能超过窗口本身。
  await s.getByLabel("窗内发射时间（秒）").fill("2000");
  await expect(s.getByText("窗内发射时间不能超过窗口长度")).toBeVisible();

  // 脉冲：10 kW 峰值、1 µs、1 kHz → 占空比 0.1%，平均 10 W = 40 dBm；脉宽过大则脉冲重叠。
  await pickOption(page, "发射方式", "脉冲（脉宽 × 重复频率）");
  await pickOption(page, "模式（占空比因子）", "连续载波 / 按住不放 · 100%");
  await s.getByLabel("峰值包络功率 PEP（W）").fill("10000");
  await expect(s.getByText("平均功率 10.00 W（40.00 dBm）")).toBeVisible();
  await s.getByLabel("脉宽（µs）").fill("2000");
  await expect(s.getByText("脉宽 × 重复频率超过 100%：脉冲重叠了")).toBeVisible();
});

test("小工具：IP3 级联给出总 IIP3 / OIP3、瓶颈级，并支持 OIP3 输入与增减级数", async ({ page }) => {
  await page.goto("/tools#ip3-cascade");
  const s = tool(page, "ip3-cascade");
  await expect(s).toBeVisible();

  // 默认链：衰减 −3 → LNA +20/IIP3 5 → 滤波 −6 → 混频 +15/15 → 中放 +10/25。
  await expect(s.getByText("总增益 36.00 dB，总 IIP3 -2.59 dBm，总 OIP3 33.41 dBm")).toBeVisible();
  // 最后一级的 IIP3 看着最高，但前面累计增益 +26 dB，它反而是瓶颈（69.34%）。
  await expect(s.getByText("瓶颈：第 5 级，占总互调的 69.34%")).toBeVisible();
  const row = (stage: string) =>
    s.getByRole("row").filter({ has: page.getByRole("cell", { name: stage, exact: true }) });
  await expect(row("5").getByRole("cell")).toHaveText(["5", "26.00", "25.00", "69.34%"]);
  await expect(row("4").getByRole("cell")).toHaveText(["4", "11.00", "15.00", "21.93%"]);
  // 无源级（IP3 留空）没有折算值与占比。
  await expect(row("1").getByRole("cell")).toHaveText(["1", "0", "—", "—"]);

  // 把第 5 级 IIP3 抬到 40：瓶颈挪到第 4 级。
  await s.getByLabel("第 5 级 IIP3（dBm，留空 = 无源）").fill("40");
  await expect(s.getByText("总增益 36.00 dB，总 IIP3 2.24 dBm，总 OIP3 38.24 dBm")).toBeVisible();
  await expect(s.getByText("瓶颈：第 4 级，占总互调的 66.75%")).toBeVisible();

  // 改按 OIP3 给：同一条链（IIP3 5 / 15 / 25 加上各级增益 → OIP3 25 / 30 / 35）得到同样的结果。
  await pickOption(page, "每级的 IP3 按哪一端给", "OIP3（折算到本级输出）");
  await s.getByLabel("第 2 级 OIP3（dBm，留空 = 无源）").fill("25");
  await s.getByLabel("第 4 级 OIP3（dBm，留空 = 无源）").fill("30");
  await s.getByLabel("第 5 级 OIP3（dBm，留空 = 无源）").fill("35");
  await expect(s.getByText("总增益 36.00 dB，总 IIP3 -2.59 dBm，总 OIP3 33.41 dBm")).toBeVisible();

  // 增减级数：最少 1 级，最多 8 级。
  const add = s.getByRole("button", { name: "添加一级" });
  const remove = s.getByRole("button", { name: "删除最后一级" });
  await remove.click();
  await expect(s.getByLabel(/^第 5 级增益/)).toHaveCount(0);
  await expect(s.getByLabel(/^第 4 级增益/)).toHaveCount(1);
  for (let i = 0; i < 4; i += 1) await add.click();
  await expect(s.getByLabel(/^第 8 级增益/)).toHaveCount(1);
  await expect(add).toBeDisabled();
  for (let i = 0; i < 7; i += 1) await remove.click();
  await expect(remove).toBeDisabled();
  await expect(s.getByLabel(/^第 2 级增益/)).toHaveCount(0);

  // 所有级的 IP3 都留空：没有三阶截点，只给总增益。
  await s.getByLabel("第 1 级增益（dB）").fill("-3");
  await s.getByLabel("第 1 级 OIP3（dBm，留空 = 无源）").fill("");
  await expect(s.getByText("所有级都是无源（理想线性）：没有三阶截点，总增益 -3.00 dB")).toBeVisible();
});

test("小工具：矩形波导算截止、模式、导波波长、导体损耗，低于截止时给出衰减", async ({ page }) => {
  await page.goto("/tools#waveguide");
  const s = tool(page, "waveguide");
  await expect(s).toBeVisible();

  // 默认 WR-90 @ 10.368 GHz（10 GHz 业余波段），铜壁。
  await expect(s.getByText("TE10 截止频率 6.56 GHz")).toBeVisible();
  await expect(s.getByText("单模范围 6.56 – 13.11 GHz（到下一个模式截止为止）")).toBeVisible();
  await expect(s.getByText("公布的推荐工作频段 8.20 – 12.40 GHz")).toBeVisible();
  await expect(
    s.getByText("最低的几个模式：TE10 6.56 GHz · TE20 13.11 GHz · TE01 14.75 GHz · TE11 16.15 GHz · TM11 16.15 GHz"),
  ).toBeVisible();
  await expect(s.getByText("介质中波长 28.92 mm，导波波长 37.33 mm")).toBeVisible();
  await expect(s.getByText("相速 / 光速 1.291，群速 / 光速 0.775，TE10 波阻抗 486.35 Ω")).toBeVisible();
  await expect(s.getByText("导体损耗（理论下限）0.1055 dB/m")).toBeVisible();

  // 低于截止：5 GHz，不传输，给出倏逝波衰减。
  await s.getByLabel("工作频率（GHz）").fill("5");
  await expect(
    s.getByText("低于截止，不传输：衰减 772.26 dB/m；每个截止波长（2a）衰减 35.31 dB。", { exact: false }),
  ).toBeVisible();

  // 超过下一个模式（TE20 13.11 GHz）的截止：多模。
  await s.getByLabel("工作频率（GHz）").fill("14");
  await expect(s.getByText("已超过下一个模式的截止，会出现多模传输")).toBeVisible();
  await expect(s.getByText("介质中波长 21.41 mm，导波波长 24.24 mm")).toBeVisible();

  // 换成 WR-42 看 24 GHz：尺寸自动填入，推荐频段取表中公布值。
  await pickOption(page, "尺寸", "WR-42");
  await s.getByLabel("工作频率（GHz）").fill("24.048");
  await expect(s.getByLabel("内宽 a（mm，长边）")).toHaveValue("10.668");
  await expect(s.getByText("TE10 截止频率 14.05 GHz")).toBeVisible();
  await expect(s.getByText("公布的推荐工作频段 18.00 – 26.50 GHz")).toBeVisible();
  await expect(s.getByText("导体损耗（理论下限）0.3398 dB/m")).toBeVisible();

  // 手改尺寸 → 变成「自定义」，推荐频段按 1.25–1.9 倍截止的惯例算；内高大于内宽给出引导。
  await s.getByLabel("内宽 a（mm，长边）").fill("20");
  await expect(s.getByRole("combobox", { name: "尺寸" })).toContainText("自定义");
  await expect(s.getByText(/常规推荐工作频段（1\.25–1\.9 倍截止）/)).toBeVisible();
  await s.getByLabel("内高 b（mm，短边）").fill("30");
  await expect(s.getByText("内高 b 不能大于内宽 a（a 取长边）")).toBeVisible();
});

test("小工具：雨衰按 P.838-3 / P.530-17 估算，并对越界输入与适用范围给出说明", async ({ page }) => {
  await page.goto("/tools#rain-fade");
  const s = tool(page, "rain-fade");
  await expect(s).toBeVisible();

  // 默认：10.368 GHz、20 km、R0.01 = 40 mm/h、水平极化、0.01% 的时间。
  await expect(s.getByText("雨衰 14.50 dB（一年里 0.01% 的时间会被超过）")).toBeVisible();
  await expect(s.getByText("0.01% 时间被超过：14.53 dB")).toBeVisible();
  await expect(s.getByText("单位长度雨衰 1.37 dB/km（k = 0.0141，α = 1.240）")).toBeVisible();
  await expect(s.getByText("路径因子 r = 0.531，有效路径长度 10.62 km")).toBeVisible();

  // 24.048 GHz、10 km、50 mm/h、垂直极化：0.01% 时 34.35 dB，放宽到 0.1% 的时间只剩 12.96 dB。
  await s.getByLabel("频率（GHz，1–1000）").fill("24.048");
  await s.getByLabel("距离（km）").fill("10");
  await s.getByLabel("0.01% 降雨率 R0.01（mm/h）").fill("50");
  await pickOption(page, "极化", "垂直极化");
  await expect(s.getByText("雨衰 34.35 dB（一年里 0.01% 的时间会被超过）")).toBeVisible();
  await expect(s.getByText("单位长度雨衰 5.93 dB/km（k = 0.1410，α = 0.956）")).toBeVisible();
  await s.getByLabel("时间百分比 p（%，0.001–1）").fill("0.1");
  await expect(s.getByText("雨衰 12.96 dB（一年里 0.1% 的时间会被超过）")).toBeVisible();

  // 超出常引用的适用范围：照算，但给警告。
  await s.getByLabel("频率（GHz，1–1000）").fill("76");
  await expect(s.getByText("频率高于 40 GHz：超出常引用的适用范围，仅供参考")).toBeVisible();
  await s.getByLabel("频率（GHz，1–1000）").fill("10.368");
  await s.getByLabel("距离（km）").fill("80");
  await expect(s.getByText("路径长于 60 km：超出常引用的适用范围，仅供参考")).toBeVisible();

  // 雨小、路径长、频率低：路径因子公式失效，取上限 2.5（有效路径 = 2.5 × 33 = 82.50 km）。
  await s.getByLabel("频率（GHz，1–1000）").fill("2");
  await s.getByLabel("距离（km）").fill("33");
  await s.getByLabel("0.01% 降雨率 R0.01（mm/h）").fill("0.5");
  await s.getByLabel("时间百分比 p（%，0.001–1）").fill("0.01");
  await expect(s.getByText("雨小而路径较长，距离因子公式失效，已按上限 2.5 计")).toBeVisible();
  await expect(s.getByText(/有效路径长度 82\.50 km/)).toBeVisible();

  // P.838 只覆盖 1–1000 GHz：144 / 432 MHz 不适用，给出引导而不是编一个数。
  await s.getByLabel("频率（GHz，1–1000）").fill("0.432");
  await expect(s.getByText("频率需在 1 到 1000 GHz 之间（P.838 的有效范围，144 / 432 MHz 不适用）")).toBeVisible();
  await s.getByLabel("频率（GHz，1–1000）").fill("10");
  await s.getByLabel("时间百分比 p（%，0.001–1）").fill("5");
  await expect(s.getByText("时间百分比需在 0.001 到 1 之间")).toBeVisible();
});

test("小工具：新增的四个射频计算器都出现在目录里，点目录项会滚到对应章节", async ({ page }) => {
  await page.goto("/tools");
  for (const [id, title] of [
    ["waveguide", "矩形波导"],
    ["rain-fade", "雨衰"],
    ["duty-power", "占空比与平均功率"],
    ["ip3-cascade", "三阶截点（IP3）级联"],
  ] as const) {
    await expect(page.locator(`section#${id} h2`)).toHaveText(title);
    await page.locator("nav button", { hasText: title }).first().click();
    await expect(page.locator(`section#${id}`)).toBeInViewport();
  }
});
