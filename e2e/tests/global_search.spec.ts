import { expect, test } from "./fixtures";

/**
 * 全局搜索命令面板（`/` 唤起）：领域前缀语法 + 页面入口 + 本地日志 QSO。
 */

const SEARCH = "搜索术语、频率、呼号、天线、元件……";

test("全局搜索：领域前缀（dxcc: / band:）与页面入口", async ({ page }) => {
  await page.goto("/");
  // 等应用挂载完成再按快捷键：wasm 未就绪时按 `/` 不会有任何响应（同 search.spec.ts）。
  await expect(page.getByRole("button", { name: "搜索" })).toBeVisible();
  await page.keyboard.press("/");
  const input = page.locator(`input[placeholder="${SEARCH}"]`);
  await expect(input).toBeVisible();

  // dxcc: 前缀 → DXCC 实体条目
  await input.fill("dxcc:by");
  await expect(page.getByText(/DXCC 实体：中国/).first()).toBeVisible();

  // band: 前缀 → 业余波段规划条目
  await input.fill("band:20m");
  await expect(page.getByText("20m 波段").first()).toBeVisible();

  // 普通关键词 → 页面入口（导航注册表入索引）。导航菜单里也有同名链接但处于
  // 折叠状态（hidden）—— 断言限定为可见的那条（对话框里的结果）。
  await input.fill("RTTY 解码器");
  await expect(
    page.getByText("RTTY 解码器").filter({ visible: true }).first(),
  ).toBeVisible();
});

test("全局搜索：公式速查与单位换算按条目命中", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByRole("button", { name: "搜索" })).toBeVisible();
  await page.keyboard.press("/");
  const input = page.locator(`input[placeholder="${SEARCH}"]`);
  await expect(input).toBeVisible();

  // 公式：按公式名搜（core 索引）。此前整页都不在索引里，只能靠页面名「公式速查」命中。
  await input.fill("欧姆定律");
  await expect(page.getByText("U = I · R").first()).toBeVisible();

  // 单位换算：条目名是 i18n key，由前端按当前语言取译文后匹配。
  await input.fill("dBm");
  await expect(page.getByText("dBm = 10·lg(P/mW)").first()).toBeVisible();
});

test("全局搜索：本地日志 QSO 命中且排在最前", async ({ page }) => {
  // 预置一条本地日志（localStorage 快照，与日志页同一存储格式）
  await page.addInitScript(() => {
    localStorage.setItem(
      "logbook",
      JSON.stringify({
        entries: [
          {
            id: 1,
            operator: "",
            station_id: 0,
            date: "2026-10-01",
            time: "08:30",
            time_off: "08:31",
            freq: "14.200",
            band: "20m",
            mode: "FT8",
            callsign: "BA1AAA",
            rst: "",
            rst_sent: "599",
            rst_rcvd: "599",
            tx_pwr: "5",
            gridsquare: "OM89",
            name: "Test",
            qth: "Beijing",
            remark: "",
          },
        ],
      }),
    );
  });
  await page.goto("/");
  // 等应用挂载完成再按快捷键：wasm 未就绪时按 `/` 不会有任何响应（同 search.spec.ts）。
  await expect(page.getByRole("button", { name: "搜索" })).toBeVisible();
  await page.keyboard.press("/");
  const input = page.locator(`input[placeholder="${SEARCH}"]`);
  await input.fill("BA1AAA");
  // QSO 结果排在最前：验证**排序**本身，而不只是「日志这两个字出现过」
  // （导航里也有「日志」链接，只断言可见性的话排到后面也照样绿）。
  const dialog = page.getByRole("dialog", { name: "全站搜索" });
  const firstGroupTitle = dialog.locator("div.mb-1.flex.items-center.gap-2 > span").first();
  await expect(firstGroupTitle).toHaveText("日志");
  await expect(page.getByText(/BA1AAA · 2026-10-01 08:30/).first()).toBeVisible();
});
