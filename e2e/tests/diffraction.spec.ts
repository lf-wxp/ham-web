import { expect, test, type Page } from "./fixtures";

/**
 * 路径剖面里的单刃绕射损耗（`/tools#path-profile`）。
 *
 * 期望值来源：几何（视线插值、地球曲率隆起、ν 公式）与**精确的菲涅尔积分**都在这里独立写——
 * 被测的是闭式近似（核心 `diffraction.rs`），精确积分用数值法另算一遍，两者不共用任何代码。
 */

const section = (page: Page) => page.locator("section#path-profile");
const block = (page: Page) => section(page).getByTestId("diffraction");

const EXAMPLE: [number, number][] = [
  [0, 120],
  [5, 180],
  [10, 260],
  [15, 310],
  [20, 240],
  [25, 170],
  [30, 130],
];

/** 独立写的几何：每个内部点的 ν（刃顶高出视线的高度，含 K = 4/3 的地球曲率隆起）。 */
function nuOf(points: [number, number][], freqMhz: number, txH: number, rxH: number): { nu: number[]; blocking: number } {
  const lambda = 299792458 / (freqMhz * 1e6);
  const total = (points[points.length - 1][0] - points[0][0]) * 1000;
  const txAbs = points[0][1] + txH;
  const rxAbs = points[points.length - 1][1] + rxH;
  const nu: number[] = [];
  let blocking = 0;
  for (let i = 1; i < points.length - 1; i += 1) {
    const d1 = (points[i][0] - points[0][0]) * 1000;
    const d2 = total - d1;
    const los = txAbs + ((rxAbs - txAbs) * d1) / total;
    const bulge = (d1 * d2) / (2 * (4 / 3) * 6371000);
    const h = points[i][1] + bulge - los;
    if (h > 0) blocking += 1;
    nu.push(h * Math.sqrt((2 * (d1 + d2)) / (lambda * d1 * d2)));
  }
  return { nu, blocking };
}

/** 精确的刃形绕射损耗：复合辛普森法积分菲涅尔积分 C(ν)、S(ν)。 */
function exactLossDb(nu: number): number {
  if (nu <= -3) return 0;
  const steps = Math.max(2000, Math.ceil(Math.abs(nu) / 2e-5)) & ~1;
  const h = nu / steps;
  const simpson = (f: (x: number) => number) => {
    let s = f(0) + f(nu);
    for (let i = 1; i < steps; i += 1) s += f(i * h) * (i % 2 === 1 ? 4 : 2);
    return (s * h) / 3;
  };
  const c = simpson((t) => Math.cos((Math.PI / 2) * t * t));
  const s = simpson((t) => Math.sin((Math.PI / 2) * t * t));
  return -10 * Math.log10(((0.5 - c) ** 2 + (0.5 - s) ** 2) / 2);
}

async function readEdge(page: Page) {
  const b = block(page);
  await expect(b).toBeVisible();
  return {
    nu: Number(await b.getAttribute("data-nu")),
    loss: Number(await b.getAttribute("data-loss")),
    blocking: Number(await b.getAttribute("data-blocking")),
  };
}

test("单刃绕射：示例山脊的 ν 与损耗，与独立的几何和精确菲涅尔积分一致", async ({ page }) => {
  await page.goto("/tools#path-profile");
  const edge = await readEdge(page);
  const { nu, blocking } = nuOf(EXAMPLE, 145, 30, 30);
  const expectedNu = Math.max(...nu);
  expect(edge.nu).toBeCloseTo(expectedNu, 2);
  expect(edge.blocking).toBe(blocking);
  // 闭式近似与精确积分的差不到 0.2 dB（核心实测最大 0.12 dB）。
  expect(Math.abs(edge.loss - exactLossDb(expectedNu))).toBeLessThan(0.2);
  // 示例是 310 m 的山脊挡在 30 km 路径中段，高出视线约 170 m：ν ≈ 1.9，损耗约 19 dB，不是零。
  expect(edge.loss).toBeGreaterThan(15);
  expect(edge.loss).toBeLessThan(25);
  await expect(block(page)).toContainText("单刃绕射");
  await expect(block(page)).toContainText(`ν = ${expectedNu.toFixed(2)}`);
  await expect(block(page)).toContainText(`附加损耗约 ${edge.loss.toFixed(1)} dB`);
});

test("单刃绕射：视线刚好擦过山顶是 6 dB，净空够就是 0 dB", async ({ page }) => {
  await page.goto("/tools#path-profile");
  const points = section(page).getByLabel("剖面采样点");
  // 两端各 30 m 的桅杆、中点曲率隆起 13.25 m：山顶取 16.75 m 时视线刚好擦过（ν ≈ 0）。
  const bulge = (15000 * 15000) / (2 * (4 / 3) * 6371000);
  const grazing = (30 - bulge).toFixed(2);
  await points.fill(`0, 0\n15, ${grazing}\n30, 0`);
  const edge = await readEdge(page);
  expect(Math.abs(edge.nu)).toBeLessThan(0.01);
  expect(Math.abs(edge.loss - 6.02)).toBeLessThan(0.1);

  // 平地 + 100 m 天线：净空远超第一菲涅尔区，ν 低于下界，损耗 0，并明说「按 0 dB 计」。
  await points.fill("0, 0\n15, 0\n30, 0");
  await section(page).getByLabel("天线 A 高度（m）").fill("100");
  await section(page).getByLabel("天线 B 高度（m）").fill("100");
  const clear = await readEdge(page);
  expect(clear.nu).toBeLessThan(-0.78);
  expect(clear.loss).toBe(0);
  expect(clear.blocking).toBe(0);
  await expect(block(page)).toContainText("附加损耗按 0 dB 计");
});

test("单刃绕射：同一座山，频率越高损耗越大；ν 与独立几何逐频率一致", async ({ page }) => {
  await page.goto("/tools#path-profile");
  const losses: number[] = [];
  for (const freq of [145, 435, 1296]) {
    await section(page).getByLabel("频率（MHz）").fill(String(freq));
    await expect(block(page)).toHaveAttribute("data-nu", new RegExp(".+"));
    // 等 ν 随频率更新到独立几何算出的值，再读损耗。
    const expected = Math.max(...nuOf(EXAMPLE, freq, 30, 30).nu);
    await expect.poll(async () => Number(await block(page).getAttribute("data-nu"))).toBeCloseTo(expected, 2);
    const edge = await readEdge(page);
    expect(Math.abs(edge.loss - exactLossDb(expected))).toBeLessThan(0.2);
    losses.push(edge.loss);
  }
  expect(losses[0]).toBeLessThan(losses[1]);
  expect(losses[1]).toBeLessThan(losses[2]);
});

test("单刃绕射：多个采样点挡住视线时提示只算了一个刃", async ({ page }) => {
  await page.goto("/tools#path-profile");
  const edge = await readEdge(page);
  expect(edge.blocking).toBeGreaterThanOrEqual(2);
  await expect(block(page)).toContainText(`挡住视线的采样点数：${edge.blocking}`);
  await expect(block(page)).toContainText("分属不同的山头");

  // 只有一个点挡住：不出现这条提示。
  await section(page).getByLabel("剖面采样点").fill("0, 0\n15, 80\n30, 0");
  await expect.poll(async () => Number(await block(page).getAttribute("data-blocking"))).toBe(1);
  await expect(block(page)).not.toContainText("挡住视线的采样点数");
});

test("单刃绕射：输入不合法时不显示这一块（只有一条带行号的提示）", async ({ page }) => {
  await page.goto("/tools#path-profile");
  await section(page).getByLabel("剖面采样点").fill("0, 0\n5\n10, 0");
  await expect(section(page).getByRole("alert")).toContainText("第 2 行");
  await expect(block(page)).toHaveCount(0);
});

test("单刃绕射：英文 / 西语界面都有译文，不留中文", async ({ page }) => {
  for (const code of ["en", "es"]) {
    await page.addInitScript((c) => localStorage.setItem("locale", c), code);
    // 示例（有「挡住视线的采样点数」那一行）与平地（有「按 0 dB 计」那一句）各验一遍。
    await page.goto("/tools#path-profile");
    await expect(block(page)).toBeVisible();
    await expect(block(page)).not.toContainText(/[\u4e00-\u9fff]/);
    await expect(section(page).getByTestId("diffraction")).toContainText(/ν = /);
    await section(page).locator("textarea").fill("0, 0\n15, 0\n30, 0");
    await expect.poll(async () => Number(await block(page).getAttribute("data-blocking"))).toBe(0);
    await expect(block(page)).not.toContainText(/[\u4e00-\u9fff]/);
  }
});

/** 整页与块各自的横向溢出（像素，≤ 1 视为没有溢出）。 */
async function overflowOf(page: Page) {
  // 现代风点按钮会放一圈 560 ms 的按压涟漪（`.md-ping`），动画放大时短暂伸出视口，之后自清理。
  await expect(page.locator(".md-ping")).toHaveCount(0);
  return page.evaluate(() => {
    const el = document.querySelector('[data-testid="diffraction"]') as HTMLElement;
    return {
      page: document.documentElement.scrollWidth - window.innerWidth,
      block: el.scrollWidth - el.clientWidth,
    };
  });
}

test("单刃绕射：溢出检测器自测——塞进 900px 宽的元素时必须量得出溢出", async ({ page }) => {
  await page.setViewportSize({ width: 320, height: 800 });
  await page.goto("/tools#path-profile");
  await expect(block(page)).toBeVisible();
  const before = await overflowOf(page);
  expect(before.page).toBeLessThanOrEqual(1);
  expect(before.block).toBeLessThanOrEqual(1);
  await page.evaluate(() => {
    const wide = document.createElement("div");
    wide.style.cssText = "width:900px;height:4px";
    document.querySelector('[data-testid="diffraction"]')!.appendChild(wide);
  });
  const after = await overflowOf(page);
  expect(after.block).toBeGreaterThan(100);
  expect(after.page).toBeGreaterThan(100);
});

// 三语里最长的几句（含「挡住视线的采样点数」那一行）在 320 px 下不能把整页或块撑出横向滚动；两套皮肤各量一次。
for (const lang of ["zh", "en", "es"]) {
  for (const style of ["pixel", "modern"]) {
    test(`单刃绕射：320px 下不横向溢出（${lang} · ${style}）`, async ({ page }) => {
      await page.setViewportSize({ width: 320, height: 800 });
      await page.addInitScript(
        ([code, ui]) => {
          localStorage.setItem("locale", code);
          localStorage.setItem("ui:uiStyle", ui);
          if (ui === "modern") localStorage.setItem("theme", "dark");
        },
        [lang, style],
      );
      await page.goto("/tools#path-profile");
      await expect(block(page)).toBeVisible();
      const over = await overflowOf(page);
      expect(over.page, "整页横向溢出").toBeLessThanOrEqual(1);
      expect(over.block, "单刃绕射块自身横向溢出").toBeLessThanOrEqual(1);
    });
  }
}
