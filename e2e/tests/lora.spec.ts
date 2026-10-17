import { expect, pickOption, test, type Page } from "./fixtures";

/**
 * LoRa 链路计算器与 LoRa / Meshtastic / AREDN 专题。
 * 计算器期望值来自独立脚本（照 Semtech AN1200.13 公式，不复用页面 / 内核任何逻辑）：
 *   默认（SF7 / 125 kHz / CR 4/5 / PL12 / 前导码 8 / 显式 / CRC / 低速率优化自动 → 关闭）：
 *     符号时间 1.024 ms，前导码 12.544 ms，28 个符号，空中时间 41.216 ms，2329.19 bps；
 *     灵敏度 −124.53 dBm，接收 −119.87 dBm，裕量 4.66 dB，最大距离 854.96 km（470 MHz / 14 dBm / 各 3 dBi）。
 *   SF12：灵敏度 −137.03 dBm，最大距离 3605.35 km。
 *   PL50：83 个符号，空中时间 97.536 ms。
 */
function tool(page: Page) {
  return page.locator("section#lora");
}

test("小工具：LoRa 链路按 AN1200.13 算出空中时间、灵敏度与最大距离，改 SF / PL 随之变化", async ({ page }) => {
  await page.goto("/tools#lora");
  const s = tool(page);
  await expect(s).toBeVisible();

  // 默认参数的结果。
  await expect(s.getByText(/符号时间 1\.024 ms \/ 前导码 12\.544 ms \/ 有效载荷 28 个符号 \/ 空中时间 41\.216 ms/)).toBeVisible();
  await expect(s.getByText(/有效数据速率 2329\.2 bps/)).toBeVisible();
  await expect(s.getByText("灵敏度 -124.53 dBm / 接收功率 -119.87 dBm / 链路裕量 4.66 dB / 最大距离 854.96 km")).toBeVisible();

  // 扩频因子升到 12：灵敏度提高、最大距离变远，且符号时间变长（空中时间暴涨）。
  await pickOption(page, "扩频因子 SF", "SF12");
  await expect(s.getByText(/灵敏度 -137\.03 dBm/)).toBeVisible();
  await expect(s.getByText(/最大距离 3605\.3 km/)).toBeVisible();
  await expect(s.getByText(/空中时间 1155\.072 ms/)).toBeVisible(); // SF12 / PL12 → 23 个符号

  // 有效载荷 50 字节（回到 SF7 才能对上独立脚本）。
  await pickOption(page, "扩频因子 SF", "SF7");
  await s.getByLabel("有效载荷（字节）").fill("50");
  await expect(s.getByText(/有效载荷 83 个符号 \/ 空中时间 97\.536 ms/)).toBeVisible();

  // 清空有效载荷：给出参数提示，而不是用旧值继续算。
  await s.getByLabel("有效载荷（字节）").fill("");
  await expect(s.getByText(/请检查输入/)).toBeVisible();
});

test("小工具：LoRa 链路低速率优化对 SF11 生效（自动档），隐式报头与关 CRC 改变有效载荷符号数", async ({ page }) => {
  await page.goto("/tools#lora");
  const s = tool(page);
  await expect(s).toBeVisible();

  // 有效载荷 10 字节：SF7 显式 + CRC → 28 个符号。
  await s.getByLabel("有效载荷（字节）").fill("10");
  await expect(s.getByText(/有效载荷 28 个符号/)).toBeVisible();

  // 关 CRC：少了 +16 → 23 个符号。
  await pickOption(page, "循环冗余校验 CRC", "不启用 CRC");
  await expect(s.getByText(/有效载荷 23 个符号/)).toBeVisible();

  // 开回 CRC，切隐式报头：−20 → 23 个符号。
  await pickOption(page, "循环冗余校验 CRC", "启用 CRC");
  await pickOption(page, "报头模式", "隐式报头");
  await expect(s.getByText(/有效载荷 23 个符号/)).toBeVisible();

  // 回显式 + SF11 + 20 字节：低速率优化自动开 → 33 个符号、741.376 ms。
  await pickOption(page, "报头模式", "显式报头");
  await pickOption(page, "扩频因子 SF", "SF11");
  await s.getByLabel("有效载荷（字节）").fill("20");
  await expect(s.getByText(/有效载荷 33 个符号 \/ 空中时间 741\.376 ms/)).toBeVisible();

  // 手动关掉低速率优化：28 个符号、659.456 ms。
  await pickOption(page, "低速率优化", "强制关闭");
  await expect(s.getByText(/有效载荷 28 个符号 \/ 空中时间 659\.456 ms/)).toBeVisible();
});

test("小工具：LoRa 低速率优化自动档看符号时间（>16 ms），不是单看 SF；最窄带宽是 62.5 kHz", async ({ page }) => {
  await page.goto("/tools#lora");
  const s = tool(page);
  await expect(s).toBeVisible();

  // SF12 + 500 kHz：符号时间 8.192 ms，自动档不开 LDRO → 18 个符号、247.808 ms
  // （若仍按「SF ≥ 11 就开」会得到 23 个符号、288.768 ms）。
  await pickOption(page, "带宽（kHz）", "500 kHz");
  await pickOption(page, "扩频因子 SF", "SF12");
  await expect(s.getByText(/有效载荷 18 个符号 \/ 空中时间 247\.808 ms/)).toBeVisible();

  // 62.5 kHz + SF7：符号时间 2.048 ms，前导码 25.088 ms，28 个符号，空中时间 82.432 ms。
  await pickOption(page, "带宽（kHz）", "62.5 kHz");
  await pickOption(page, "扩频因子 SF", "SF7");
  await expect(
    s.getByText(/符号时间 2\.048 ms \/ 前导码 25\.088 ms \/ 有效载荷 28 个符号 \/ 空中时间 82\.432 ms/),
  ).toBeVisible();

  // 62.5 kHz + SF10：符号时间 16.384 ms > 16 ms，自动档开 LDRO → 28 个符号、659.456 ms
  // （关闭 LDRO 会是 23 个符号、577.536 ms）。
  await pickOption(page, "扩频因子 SF", "SF10");
  await expect(s.getByText(/有效载荷 28 个符号 \/ 空中时间 659\.456 ms/)).toBeVisible();
});

test("小工具：LoRa 有效载荷必须是整数，非整数不悄悄四舍五入，控件本身标红", async ({ page }) => {
  await page.goto("/tools#lora");
  const s = tool(page);
  await expect(s).toBeVisible();
  const payload = s.getByLabel("有效载荷（字节）");

  await expect(payload).not.toHaveAttribute("aria-invalid", "true");
  await payload.fill("12.5");
  await expect(s.getByText(/请检查输入/).first()).toBeVisible();
  await expect(payload).toHaveAttribute("aria-invalid", "true");

  await payload.fill("12");
  await expect(payload).not.toHaveAttribute("aria-invalid", "true");
  await expect(s.getByText(/有效载荷 28 个符号/)).toBeVisible();
});

test("小工具：LoRa 链路目录项存在且锚点直达，描述与合规提示可见", async ({ page }) => {
  await page.goto("/tools");
  const s = tool(page);
  await expect(s).toBeVisible(); // 直接 /tools 也会渲染（目录在页面内）
  // 合规提示（罗列 470–510 / 433.05–434.79 / 430–440 MHz 的差异）必须可见。
  await expect(s.getByText(/470–510 MHz/)).toBeVisible();
  await expect(s.getByText(/433\.05–434\.79 MHz/)).toBeVisible();
  await expect(s.getByText(/430–440 MHz 是业余业务/)).toBeVisible();
  await expect(s.getByText(/868 \/ 915 MHz/)).toBeVisible();
});

test("专题页：LoRa / Meshtastic / AREDN 有核心概念、调制参数与合规差异表", async ({ page }) => {
  await page.goto("/lora");
  await expect(page.getByRole("heading", { name: "LoRa / Meshtastic / AREDN" })).toBeVisible();

  // 核心概念：三种技术各自成条。
  const body = page.getByText("LoRa", { exact: true });
  await expect(body.first()).toBeVisible();
  await expect(page.getByText("Meshtastic", { exact: true })).toBeVisible();
  await expect(page.getByText("AREDN", { exact: true })).toBeVisible();
  await expect(page.getByText(/线性调频扩频（CSS）调制/)).toBeVisible();

  // 调制参数表与合规差异表的标题。
  await expect(page.getByRole("heading", { name: "LoRa 调制参数" })).toBeVisible();
  await expect(page.getByRole("heading", { name: "合规差异" })).toBeVisible();

  // 合规差异的关键行（频段与法律身份）。
  await expect(page.getByRole("cell", { name: "470–510 MHz" })).toBeVisible();
  await expect(page.getByRole("cell", { name: "业余业务（次要业务）" })).toBeVisible();
  await expect(page.getByRole("cell", { name: /≤ 50 mW/ })).toBeVisible();

  // 要点。
  await expect(page.getByText(/同一段 430–440 MHz 里/)).toBeVisible();
});

test("专题页：三语界面标题与 section 标题都有译文（正文翻译由 knowledge-i18n-check 保证）", async ({ page }) => {
  // 界面部分（页面标题 / section 标题）走界面词典，dev 下即生效；
  // 知识正文（核心概念 / 表格 / 要点）走 knowledge-i18n 词典，只在 release 构建（postbuild）后加载，
  // 其翻译正确性由 `cargo make knowledge-i18n-check` 保证（drift / conflicts / 覆盖率），这里不重复。
  await page.addInitScript(() => localStorage.setItem("locale", "en"));
  await page.goto("/lora");
  await expect(page.getByRole("heading", { name: "LoRa / Meshtastic / AREDN" })).toBeVisible();
  await expect(page.getByRole("heading", { name: "LoRa modulation parameters" })).toBeVisible();
  await expect(page.getByRole("heading", { name: "Compliance differences" })).toBeVisible();
  // 频段列是纯 ASCII（不涉翻译），三语下都原样可见。
  await expect(page.getByRole("cell", { name: "470–510 MHz" })).toBeVisible();
  await expect(page.getByRole("cell", { name: "430–440 MHz" })).toBeVisible();
});
