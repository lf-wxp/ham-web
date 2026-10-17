import { expect, test } from "./fixtures";

/**
 * VNA 读数：上传 .s1p → 摘要 / 最低驻波 / 可用频段 / 两张图 → 导出。
 *
 * 夹具在测试里现场合成：纯电阻负载 r(f) = 1 + 4·((f − 7.15)/0.25)²（归一化，x = 0），
 * 于是 SWR = r（r ≥ 1），最低点在 7.15 MHz 恰为 1.00；SWR ≤ 2 对应 |f − 7.15| ≤ 0.125，
 * 页面在相邻实测点之间线性插值，边沿落在 7.02x / 7.27x MHz。
 * 串口路径需要真实 NanoVNA，自动化只覆盖离线可验证的文件入口。
 */
function synthS1p(): string {
  const lines = ["! synthetic test sweep", "# MHz S RI R 50"];
  for (let i = 0; i <= 10; i++) {
    const f = 6.9 + i * 0.05;
    const u = (f - 7.15) / 0.25;
    const r = 1 + 4 * u * u;
    const g = (r - 1) / (r + 1);
    lines.push(`${f.toFixed(3)} ${g.toExponential(9)} 0`);
  }
  return `${lines.join("\n")}\n`;
}

test("VNA 读数：导入 .s1p 后给出最低驻波、可用频段与轨迹", async ({ page }) => {
  await page.goto("/vna-reader");
  await expect(page.getByText("还没有数据")).toBeVisible();

  await page.setInputFiles('input[type="file"]', {
    name: "dipole.s1p",
    mimeType: "text/plain",
    buffer: Buffer.from(synthS1p()),
  });
  const result = page.locator('[data-slot="vna-result"]');
  await expect(result.locator('[data-slot="vna-summary"]')).toHaveText(
    "dipole.s1p · 11 个频点 · 6.900 MHz – 7.400 MHz",
  );
  await expect(result.locator('[data-slot="vna-best"]')).toHaveText("最低驻波比 1.00 @ 7.150 MHz");
  await expect(result.getByText("该点阻抗 50.0 + j0.0 Ω")).toBeVisible();
  await expect(result.locator('[data-slot="vna-band"]')).toHaveText(
    /驻波比 ≤ 2 的频段：7\.02\d MHz – 7\.27\d MHz/,
  );
  // 两张图都画出来了：轨迹折线带全部 11 个点。
  await expect(page.getByRole("img", { name: "实测 S11 的 Smith 圆图轨迹" })).toBeVisible();
  await expect(page.locator('[data-slot="vna-locus"] path[data-points="11"]')).toHaveCount(1);
  await expect(page.getByRole("img", { name: "驻波比随频率变化的曲线" })).toBeVisible();

  // 导出：RI 格式、参考阻抗 50 Ω。
  const download = page.waitForEvent("download");
  await page.getByRole("button", { name: "导出 .s1p" }).click();
  const file = await download;
  expect(file.suggestedFilename()).toBe("s11.s1p");
});

test("VNA 读数：多端口文件如实拒绝", async ({ page }) => {
  await page.goto("/vna-reader");
  await page.setInputFiles('input[type="file"]', {
    name: "two-port.s2p",
    mimeType: "text/plain",
    buffer: Buffer.from("# MHz S RI R 50\n7.0 0 0 1 0 1 0 0 0\n"),
  });
  await expect(page.getByRole("alert")).toHaveText(/无法解析：第 2 行不是单端口数据/);
  await expect(page.getByText("还没有数据")).toBeVisible();
});

/**
 * 合成一份「10 米处有一处失配」的 .s1p：Γ(f) = (1/3)·e^(−j·4π·f·L/v)，v = 0.66·c。
 * 页面默认速率因子 0.66，所以 TDR 应把峰值定位回 10 m。
 */
function synthTdrS1p(): string {
  const v = 0.66 * 299_792_458;
  const L = 10;
  const g0 = 1 / 3;
  const f0 = 1e6;
  const f1 = 900e6;
  const n = 256;
  const df = (f1 - f0) / (n - 1);
  const lines = ["! synthetic TDR sweep", "# Hz S RI R 50"];
  for (let i = 0; i < n; i++) {
    const f = f0 + i * df;
    const phase = (-4 * Math.PI * f * L) / v;
    lines.push(`${f.toExponential(9)} ${(g0 * Math.cos(phase)).toExponential(9)} ${(g0 * Math.sin(phase)).toExponential(9)}`);
  }
  return `${lines.join("\n")}\n`;
}

test("VNA 读数：TDR 故障定位把失配点定位回已知距离", async ({ page }) => {
  await page.goto("/vna-reader");
  await page.setInputFiles('input[type="file"]', {
    name: "tdr.s1p",
    mimeType: "text/plain",
    buffer: Buffer.from(synthTdrS1p()),
  });

  const tdr = page.locator('[data-slot="vna-tdr"]');
  await expect(page.getByRole("img", { name: "时域反射（TDR）距离回波曲线" })).toBeVisible();
  // 分辨率 0.11 m 按「11 cm」显示；最大不模糊距离 = v/(2·Δf) ≈ 28.06 m。
  await expect(tdr.getByText(/距离分辨率 ≈ 11 cm，最大不模糊距离 28\.\d+ m/)).toBeVisible();
  // 最强反射（0 dB）落在约 10 m：正是合成时的失配距离。
  await expect(tdr.getByText(/10\.\d+ m · 0\.0 dB/)).toBeVisible();

  // 清空速率因子（`type=number` 只可能停在空串这类半截状态）：要直说「输错了」，
  // 而不是把 `Field` 的 hint 重复一遍。
  await tdr.getByLabel("速率因子 vf").fill("");
  await expect(tdr.getByRole("alert")).toHaveText("速率因子必须在 0（不含）与 1（含）之间。");
});
