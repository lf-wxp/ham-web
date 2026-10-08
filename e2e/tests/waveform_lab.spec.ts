import type { Page } from "@playwright/test";
import { expect, test } from "./fixtures";

/** 面板用 `section#id` 承载（与导航锚点一致）。 */
function panel(page: Page, id: string) {
  return page.locator(`section#${id}`);
}

test("波形实验室：切换调制方式后功率分配与卡森带宽随之更新", async ({ page }) => {
  await page.goto("/waveform-lab");
  await expect(page.getByRole("heading", { level: 1, name: "波形实验室" })).toBeVisible();

  const wave = panel(page, "wave");
  // 默认 AM、m = 1：总功率 1.5、边带占 1/3。
  await expect(wave.getByText(/总功率 1\.50/)).toBeVisible();
  await expect(wave.getByText(/边带占比（即效率上限）33\.3%/)).toBeVisible();

  // 两张图必须跟着参数重算：曾经 `Memo` 里用 `get_untracked()` 读上游而没建立依赖，
  // 图画一次就定死 —— 切调制方式、拖滑块都只改文案，曲线一动不动（见 code review 的 C2）。
  const timePath = wave.getByRole("img", { name: "调制波形" }).locator("path");
  const spectrumPath = wave.getByRole("img", { name: "幅度谱" }).locator("path");
  const timeBefore = await timePath.getAttribute("d");
  const spectrumBefore = await spectrumPath.getAttribute("d");
  expect(timeBefore).toBeTruthy();

  // 切到 FM：功率分配条换成卡森带宽。
  await wave.getByRole("radio", { name: "FM 调频" }).click();
  await expect(wave.getByText(/卡森带宽 BW ≈/)).toBeVisible();
  await expect(wave.getByText(/总功率/)).toHaveCount(0);
  await expect.poll(() => timePath.getAttribute("d")).not.toBe(timeBefore);
  await expect.poll(() => spectrumPath.getAttribute("d")).not.toBe(spectrumBefore);

  // 切到 USB：单边带仍显示功率分配（载波归一化为 1）。
  await wave.getByRole("radio", { name: "USB 上边带" }).click();
  await expect(wave.getByText(/总功率 1\.50/)).toBeVisible();

  // 调制指数降到 0.6（← 四次）：总功率 1 + 2·(m²/4) = 1.18。滑块是原生 range，
  // 用方向键驱动才真正走「用户输入 → input 事件 → 重算」这条链路。
  const index = wave.locator('input[aria-label="调制指数"]');
  await index.focus();
  for (let i = 0; i < 4; i += 1) await index.press("ArrowLeft");
  await expect(wave.getByText(/总功率 1\.18/)).toBeVisible();
});

test("波形实验室：改变滤波器阶数与类型后响应结果同步更新", async ({ page }) => {
  await page.goto("/waveform-lab");
  const filter = panel(page, "filter");
  await expect(filter).toBeVisible();

  // 默认低通 3 阶、7.1 MHz：频率刻度上应出现该值。
  await expect(filter.getByText("7.100 MHz")).toBeVisible();
  await expect(filter.getByText(/阶数 3（n 阶）/)).toBeVisible();

  // 改阶数 → 结论文案同步。
  await filter.getByRole("radio", { name: "5", exact: true }).click();
  await expect(filter.getByText(/阶数 5（n 阶）/)).toBeVisible();

  // 切带通 → 拓扑标注变成等效 2n 阶。
  await filter.getByRole("radio", { name: "带通" }).click();
  await expect(filter.getByText(/等效 2n 阶/)).toBeVisible();

  // 改中心频率 → 刻度一起更新。
  await filter.locator('input[aria-label="频率 MHz"]').fill("14.1");
  await expect(filter.getByText("14.100 MHz")).toBeVisible();
});

test("波形实验室：英文界面不出现中文阶数限定语", async ({ page }) => {
  // 直接种语言：这条用例只关心 en 下的文案，不关心语言切换交互。
  await page.addInitScript(() => localStorage.setItem("locale", "en"));
  await page.goto("/waveform-lab");
  const filter = panel(page, "filter");

  // 「阶数 4（n 阶）」这类限定语必须走词典：曾经把中文原文直接塞进 `tf()` 的实参，
  // 英文界面渲染成 `order 3 (n 阶)`。
  const lowpass = filter.getByText(/order 3 \(n-th order\)/);
  await expect(lowpass).toBeVisible();
  expect(await lowpass.innerText()).not.toMatch(/\p{Script=Han}/u);

  // 带通：限定语换成「等效 2n 阶」，整句同样不许残留汉字。
  await filter.getByRole("radio", { name: "Band-pass" }).click();
  const band = filter.getByText(/2n-th order equivalent/);
  await expect(band).toBeVisible();
  expect(await band.innerText()).not.toMatch(/\p{Script=Han}/u);
  expect(await band.innerText()).toContain("order 3");
});

test("波形实验室：切换 Chebyshev 原型与波纹档位", async ({ page }) => {
  await page.goto("/waveform-lab");
  const filter = panel(page, "filter");
  // 默认 Butterworth：没有波纹档位，结论文案里也没有波纹值。
  await expect(filter.getByText(/Butterworth$/)).toBeVisible();
  await expect(filter.getByText("通带波纹（dB）")).toHaveCount(0);

  // Chebyshev I：出现通带波纹档位，默认 1.0 dB。
  await filter.getByRole("radio", { name: "Chebyshev I（通带波纹）" }).click();
  await expect(filter.getByText("通带波纹（dB）")).toBeVisible();
  await expect(filter.getByText(/Chebyshev I · 1\.0 dB/)).toBeVisible();
  await filter.getByRole("radio", { name: "3 dB" }).click();
  await expect(filter.getByText(/Chebyshev I · 3\.0 dB/)).toBeVisible();

  // Chebyshev II：档位换成阻带最小衰减，默认 40 dB。
  await filter.getByRole("radio", { name: "Chebyshev II（阻带波纹）" }).click();
  await expect(filter.getByText("阻带最小衰减（dB）")).toBeVisible();
  await expect(filter.getByText("通带波纹（dB）")).toHaveCount(0);
  await expect(filter.getByText(/Chebyshev II · 40 dB/)).toBeVisible();
  await filter.getByRole("radio", { name: "80 dB" }).click();
  await expect(filter.getByText(/Chebyshev II · 80 dB/)).toBeVisible();

  // 切回 Butterworth：档位整行消失。
  await filter.getByRole("radio", { name: "Butterworth（最平坦）" }).click();
  await expect(filter.getByText("阻带最小衰减（dB）")).toHaveCount(0);
  await expect(filter.getByText(/阶数 3（n 阶）　｜　Butterworth$/)).toBeVisible();
});

test("史密斯圆图：拖动圆图改变负载阻抗", async ({ page }) => {
  await page.goto("/smith");
  await expect(page.getByRole("heading", { level: 1, name: "史密斯圆图与匹配" })).toBeVisible();
  await expect(page.getByText(/负载 Z = 100\.0/)).toBeVisible();

  // 圆图上 Γ = −0.5 处对应归一化 z = 1/3 → 50Ω 制下 16.7Ω。
  // 取 **svg 自身**的 rect（拖动区的几何就是它）：画布 360×360、圆心 (180,180)、
  // 半径 150，故 x = 180 − 0.5×150 = 105、y = 中心。
  const chart = page.locator('[data-slot="smith-chart"]');
  const box = await chart.boundingBox();
  expect(box, "圆图应有可见尺寸").not.toBeNull();
  if (!box) return;
  await page.mouse.click(box.x + (box.width * 105) / 360, box.y + box.height / 2);

  await expect(page.getByText(/负载电阻 R（Ω，对数刻度）：16\.7/)).toBeVisible();
  await expect(page.getByText(/负载 Z = 16\.7/)).toBeVisible();
});

test("史密斯圆图：自动 L 型匹配后驻波比回到 1", async ({ page }) => {
  await page.goto("/smith");
  await expect(page.getByText(/仍需调整/)).toBeVisible();

  await page.getByRole("button", { name: "自动 L 型匹配" }).click();

  // 归一化 2 − j1 属高阻侧负载：应判为先并后串。
  // 拓扑是互斥选择，用 `RadioGroup` 表达（`role=radio` + `aria-checked`）。
  await expect(page.getByRole("radio", { name: "先并后串" })).toBeChecked();
  await expect(page.getByText(/匹配后 Z = 50\.0 \+ j0\.0 Ω/)).toBeVisible();
  await expect(page.getByText(/SWR = 1\.000/)).toBeVisible();
  await expect(page.getByText(/已匹配/)).toBeVisible();

  // 元件值至少有一个被算出来（串联电感 / 并联电容）。
  await expect(page.getByText(/串联元件：(L|C) = /)).toBeVisible();
  await expect(page.getByText(/并联元件：(L|C) = /)).toBeVisible();

  // 清零元件后回到「仍需调整」。
  await page.getByRole("button", { name: "清零元件" }).click();
  await expect(page.getByText(/仍需调整/)).toBeVisible();
});

test("史密斯圆图：匹配后给出 SWR ≤ 2 的可用带宽", async ({ page }) => {
  await page.goto("/smith");
  // 未接元件时 100 − j50Ω 的 SWR 约 2.6，不应给出带宽。
  await expect(
    page.getByText("当前元件组合在中心频率上未达到 SWR ≤ 2.0，还谈不上带宽。"),
  ).toBeVisible();

  await page.getByRole("button", { name: "自动 L 型匹配" }).click();
  // 7.1 MHz 上匹配出一个有限带宽，并且两个边界都落在中心频率两侧。
  const band = page.getByText(/匹配带宽（SWR ≤ 2\.0）：(\d+\.\d+) – (\d+\.\d+) MHz/);
  await expect(band).toBeVisible();
  const text = await band.innerText();
  const [, lo, hi] = text.match(/：(\d+\.\d+) – (\d+\.\d+) MHz/) ?? [];
  expect(Number(lo)).toBeLessThan(7.1);
  expect(Number(hi)).toBeGreaterThan(7.1);
  await expect(page.getByText(/约 \d+ kHz（相对带宽 \d+\.\d%）/)).toBeVisible();

  // 元件清零后回到「谈不上带宽」。
  await page.getByRole("button", { name: "清零元件" }).click();
  await expect(
    page.getByText("当前元件组合在中心频率上未达到 SWR ≤ 2.0，还谈不上带宽。"),
  ).toBeVisible();
});

test("传播预测：24 小时热力矩阵随网格与时刻更新", async ({ page }) => {
  await page.goto("/muf");
  await expect(page.getByRole("heading", { name: "24 小时开通窗口" })).toBeVisible();

  // 每个波段一行 24 个格子（格子是可点选的按钮）；默认网格合法，应有 10 × 24。
  await expect(page.locator('[title^="20m · "]')).toHaveCount(24);
  await expect(page.locator('[title^="10m · "]')).toHaveCount(24);

  // 跳到最佳时刻：查看时刻随之改变。
  const hourLabel = page.getByText(/^查看时刻 \d{2}:00 UTC$/);
  const before = await hourLabel.innerText();
  await page.getByRole("button", { name: /跳到最佳时刻/ }).click();
  await expect(hourLabel).not.toHaveText(before);

  // 推荐波段列表有内容。
  await expect(page.getByText("该时刻推荐波段（按可靠度）")).toBeVisible();
});

test("传播预测：点选热力图格子给出路径细节", async ({ page }) => {
  await page.goto("/muf");
  await expect(
    page.getByText("点击热力图上的格子，查看该波段在该时刻的路径细节。"),
  ).toBeVisible();
  // 实时太阳活动：取到就用实时值，取不到按手动值（离线路径），两种文案都算通过。
  await expect(
    page.getByText(/实时太阳活动（数据源 HamQSL）|未取到实时太阳活动数据/),
  ).toBeVisible();

  await page.locator('button[aria-label^="20m · 12:00 UTC"]').click();
  await expect(page.getByText(/20m · 12:00 UTC　｜　频率 14\.100 MHz/)).toBeVisible();
  // 北京 → 伦敦约 8150 km，按 F2 单跳上限分 3 跳，仰角很小但为正。
  await expect(
    page.getByText(
      /路径中点地方时 \d+\.\d　｜　跳数 3　｜　单跳 \d+ km　｜　仰角 \d+\.\d°/,
    ),
  ).toBeVisible();

  // 换一个时刻，明细随之更新。
  await page.locator('button[aria-label^="10m · 03:00 UTC"]').click();
  await expect(page.getByText(/10m · 03:00 UTC　｜　频率 28\.400 MHz/)).toBeVisible();
});

test("传播预测：非法网格时给出输入提示", async ({ page }) => {
  await page.goto("/muf");
  await expect(page.locator('[title^="20m · "]')).toHaveCount(24);

  // 同一页的另一张卡片也有「接收端网格」，这里限定在热力图面板内。
  const rx = panel(page, "heatmap").locator('input[aria-label="接收端网格"]');
  await rx.fill("XX");
  await expect(page.getByText("请输入至少 4 位 Maidenhead 网格（如 OM89、IO91）。")).toBeVisible();
  await expect(page.locator('[title^="20m · "]')).toHaveCount(0);

  await rx.fill("IO91");
  await expect(page.locator('[title^="20m · "]')).toHaveCount(24);
});
