import { expect, test, type Locator, type Page } from "@playwright/test";

/// 结果条（阻抗 / 驻波比 / 增益 / 前后比 / 仰角）。
function summary(page: Page): Locator {
  return page.getByText(/馈电点阻抗 .+ Ω/);
}

/// 效率条（效率 / 损耗 / 方向性系数）。
function efficiency(page: Page): Locator {
  return page.getByText(/效率 [\d.]+%/);
}

/// 导线表 / 负载表（按 `data-slot` 取，不靠 DOM 顺序：页面上还有方案库 / 传输线表，
/// 展开哪一段都会让 `nth()` 错位）。
function wireTable(page: Page): Locator {
  return page.locator('[data-slot="nec-wires"]');
}
function loadTable(page: Page): Locator {
  return page.locator('[data-slot="nec-loads"]');
}

/// 从一段文本里抽数值。
function num(text: string, re: RegExp): number {
  return Number(text.match(re)?.[1] ?? "0");
}

/// 读结果条并抽某个字段。
const readNum = (loc: Locator, re: RegExp) =>
  expect.poll(async () => num(await loc.innerText(), re));

test("天线矩量法求解：预设切换与尺寸改动驱动结果与方向图", async ({ page }) => {
  await page.goto("/nec");

  // 先切到自由空间：默认「10 m 架高 + 理想地面」的增益（约 8 dBi）比八木还高，
  // 拿它当基准比不出八木的优势。
  //
  // 求解是 250ms 防抖的（`nec_page.rs` 的 `debounce("nec-solve", 250, …)`）：点完立刻读
  // 会拿到**上一个地面模型**的结果（7.96 dBi 正是「10 m + 理想导体地面」的值），基准偏大
  // 到 10.96，后面「八木比偶极高 3 dB」就必然失败。这里先等基准落到自由空间偶极的量级
  // （约 2.15 dBi，取 < 5 与地面反射值明显区分），顺便钉住「换地面模型确实重算」。
  await page.getByRole("radio", { name: "自由空间" }).click();
  await expect(summary(page)).toBeVisible();
  await expect(page.getByText(/波长 .+ m/)).toBeVisible();
  await expect
    .poll(async () => num(await summary(page).innerText(), /增益 ([\d.-]+) dBi/))
    .toBeLessThan(5);
  const dipoleGain = num(await summary(page).innerText(), /增益 ([\d.-]+) dBi/);
  expect(dipoleGain).toBeGreaterThan(0);

  await expect(page.getByRole("img", { name: "方位面方向图" })).toBeVisible();
  await expect(page.getByRole("img", { name: "仰角面方向图" })).toBeVisible();

  // 三维方向图：拖拽能改变视角。断言走宿主上的 `data-yaw` / `data-pitch`，
  // WebGL2 与 SVG 回退两条渲染路径都能过（WebGL 画布读不出路径字符串）。
  const sphere = page.getByRole("img", { name: "三维方向图" });
  // 三维视图在页面底部：不滚进视口的话鼠标事件会落到别处（这一步踩过坑）。
  await sphere.scrollIntoViewIfNeeded();
  await expect(sphere).toBeVisible();
  const renderer = await sphere.getAttribute("data-renderer");
  expect(["webgl2", "svg"]).toContain(renderer);
  if (renderer === "webgl2") {
    // WebGL2 路径：画布真的挂上去了。
    await expect(sphere.locator("canvas")).toBeVisible();
  } else {
    // 回退路径：SVG 线框仍然画出来。
    const shape = await sphere.locator("path").getAttribute("d");
    expect((shape ?? "").length).toBeGreaterThan(500);
  }
  const yawBefore = await sphere.getAttribute("data-yaw");
  const pitchBefore = await sphere.getAttribute("data-pitch");
  const box = await sphere.boundingBox();
  const cx = box!.x + box!.width / 2;
  const cy = box!.y + box!.height / 2;
  await page.mouse.move(cx, cy);
  await page.mouse.down();
  await page.mouse.move(cx + 70, cy + 25, { steps: 6 });
  await page.mouse.up();
  await expect
    .poll(async () => sphere.getAttribute("data-yaw"))
    .not.toBe(yawBefore);
  await expect
    .poll(async () => sphere.getAttribute("data-pitch"))
    .not.toBe(pitchBefore);

  // 切到三单元八木：导线表变成三行，增益应比偶极高 3 dB 以上。
  await page.getByRole("radio", { name: "三单元八木" }).click();
  await expect(wireTable(page).getByRole("row")).toHaveCount(4); // 表头 + 3 根导线
  await readNum(summary(page), /增益 ([\d.-]+) dBi/).toBeGreaterThan(dipoleGain + 3);
  await readNum(summary(page), /前后比 ([\d.-]+) dB/).toBeGreaterThan(6);

  // 改主尺寸：预设参数会按当前预设重建导线表，结果随之变化。
  const len = page.locator('input[aria-label="主尺寸（m，振子长度 / 垂直高度 / 倒 V 总臂展）"]');
  const yagi = await summary(page).innerText();
  await len.fill("9.6");
  await expect(summary(page)).not.toHaveText(yagi);

  // 切到垂直：导线表回到一行，且出现「架设高度」输入。
  await page.getByRole("radio", { name: "垂直（λ/4）" }).click();
  await expect(page.getByText("架设高度（m，0 为自由空间）")).toBeVisible();
  await expect(wireTable(page).getByRole("row")).toHaveCount(2);
  await expect(summary(page)).toBeVisible();
});

test("天线矩量法求解：可编辑导线表与集总负载", async ({ page }) => {
  await page.goto("/nec");
  const before = await summary(page).innerText();

  // 直接改导线坐标：几何变了，结果必须跟着变（这是「可编辑几何」的核心）。
  const ax = page.locator('input[aria-label="导线起点 X 1"]');
  await expect(ax).toBeVisible();
  await ax.fill("-4.5");
  await expect(summary(page)).not.toHaveText(before);
  await expect(summary(page)).toHaveText(/馈电点阻抗 .+ Ω/);

  // 加一根导线：表格多一行，几何仍可求解（新导线是原导线平移出来的，不是零长度）。
  const shifted = await summary(page).innerText();
  await page.getByRole("button", { name: "添加导线" }).click();
  await expect(wireTable(page).getByRole("row")).toHaveCount(3);
  await expect(summary(page)).not.toHaveText(shifted);
  await expect(summary(page)).toHaveText(/馈电点阻抗 .+ Ω/);

  // 删掉刚加的那根，回到一行。
  await wireTable(page).getByRole("button", { name: /删除 2/ }).click();
  await expect(wireTable(page).getByRole("row")).toHaveCount(2);
  // 求解是 250ms 防抖的：删完立刻读结果条，读到的还是**两根导线**那份（前后比 2.6 dB）。
  // 拿它当「无负载」基准，最后一步「删掉负载后应回到基准」就永远对不上 —— 先等结果条
  // 落回删导线之前的值（`shifted`），再快照。
  await expect(summary(page)).toHaveText(shifted);

  // 添加集总负载：加感会明显抬高电抗，结果条必须变化。
  await expect(page.getByText("还没有负载。")).toBeVisible();
  const noLoad = await summary(page).innerText();
  await page.getByRole("button", { name: "添加负载" }).click();
  await page.locator('input[aria-label="负载电感 L（µH） 1"]').fill("20");
  await expect(summary(page)).not.toHaveText(noLoad);
  await loadTable(page).getByRole("button", { name: /删除 1/ }).click();
  await expect(page.getByText("还没有负载。")).toBeVisible();
  await expect(summary(page)).toHaveText(noLoad);
});

test("天线矩量法求解：地面模型与导线材质影响效率", async ({ page }) => {
  await page.goto("/nec");
  const eff = efficiency(page);
  await expect(eff).toBeVisible();

  // 默认是铜导线 + 理想地面：欧姆损耗很小但确实存在。
  await readNum(eff, /效率 ([\d.]+)%/).toBeGreaterThan(90);
  await readNum(eff, /效率 ([\d.]+)%/).toBeLessThan(100);

  // 理想导体导线：效率升到接近 100%（残余是方向图积分的求积误差）。
  await page.getByRole("radio", { name: "理想导体（无损耗）" }).click();
  await readNum(eff, /效率 ([\d.]+)%/).toBeGreaterThan(99);

  // 铜导线：欧姆损耗回来，效率降到 90–99 之间。
  await page.getByRole("radio", { name: "铜 Cu" }).click();
  await readNum(eff, /效率 ([\d.]+)%/).toBeLessThan(100);

  // 有耗地面：10 m 高的水平偶极效率应降到 70–85%（镜像变弱，一部分功率进了地里）。
  await page.getByRole("radio", { name: "有耗地面" }).click();
  await expect(page.getByText("相对介电常数 εr")).toBeVisible();
  await readNum(eff, /效率 ([\d.]+)%/).toBeLessThan(85);

  // 增益 = 方向性系数 − 损耗，三者口径一致。
  const gain = num(await summary(page).innerText(), /增益 ([\d.-]+) dBi/);
  const dir = num(await eff.innerText(), /方向性系数 ([\d.-]+) dBi/);
  expect(gain).toBeLessThan(dir);

  // 湿土（σ 更大）应比干土损耗小、效率高。
  const dry = num(await eff.innerText(), /效率 ([\d.]+)%/);
  await page.locator('input[aria-label="电导率 σ（S/m）"]').fill("0.02");
  await expect.poll(async () => num(await eff.innerText(), /效率 ([\d.]+)%/)).toBeGreaterThan(dry);

  // 介电常数也会影响：εr 很大（湿土）时更接近理想导体。
  await page.locator('input[aria-label="相对介电常数 εr"]').fill("80");
  await expect.poll(async () => num(await eff.innerText(), /效率 ([\d.]+)%/)).toBeGreaterThan(85);
});

test("天线矩量法求解：导入 .nec 覆盖几何、导出并可回到预设", async ({ page }) => {
  await page.goto("/nec");
  await page.getByText("导入 / 导出 .nec").click();

  const box = page.getByRole("textbox", { name: /把 \.nec 文本粘贴/ });
  await box.fill(
    [
      "CM 测试文件",
      "GW 1, 21, -5.31, 0.0, 10.0, 5.31, 0.0, 10.0, 0.001",
      // LD 的 L 按 NEC-2 手册是**亨利**（内层模型是 µH）：2.0E-05 H = 20 µH。
      // 写成 0.02 会被正确解析成 20000 µH（曾经按 mH 换算，跨软件交换差 1e3）。
      "LD 0, 1, 11, 0, 100.0, 2.0E-05, 0.0",
      "GN 2, 0, 0, 0, 13.0, 0.005",
      "EX 0, 1, 11, 0, 1.0, 0.0",
      "FR 0, 1, 14.1, 0.0",
      "RP 0, 91, 1, 1000, 0.0, 0.0, 5.0, 5.0",
      "EN",
    ].join("\n"),
  );
  await page.getByRole("button", { name: "导入" }).click();

  // 只有确实没实现的卡片才该被列出（GN / LD 现在都支持）。
  await expect(page.getByText(/已忽略未支持的卡片：RP/)).toBeVisible();
  await expect(wireTable(page).getByRole("row")).toHaveCount(2);
  await expect(summary(page)).toBeVisible();
  // 负载与有耗地面都被解析进来了。
  await expect(page.getByText("相对介电常数 εr")).toBeVisible();
  await expect(page.locator('input[aria-label="负载电感 L（µH） 1"]')).toHaveValue("20.000");
  await expect(page.locator('input[aria-label="负载所在导线（序号） 1"]')).toHaveValue("1");

  // 导出：文本框里应出现当前几何、负载与地面卡片。
  await page.getByRole("button", { name: "导出当前几何" }).click();
  await expect(box).toHaveValue(/GW 1 21/);
  await expect(box).toHaveValue(/LD 0, 1, 11/);
  await expect(box).toHaveValue(/GN 2/);
  await expect(box).toHaveValue(/FR 0, 1, 14\.1000/);

  // 切预设回到常规几何。
  await page.getByRole("radio", { name: "水平偶极" }).click();
  await expect(wireTable(page).getByRole("row")).toHaveCount(2);
  await expect(summary(page)).toBeVisible();
});

test("天线矩量法求解：导入 .nec 后馈电标记跟着 EX 卡片走", async ({ page }) => {
  await page.goto("/nec");
  await page.getByText("导入 / 导出 .nec").click();
  const box = page.getByRole("textbox", { name: /把 \.nec 文本粘贴/ });

  // 两次导入的几何完全相同，只有 EX 卡片指的段号不同。
  const importWithFeedSegment = async (seg: number) => {
    await box.fill(
      [
        "CM 测试文件",
        "GW 1, 21, -5.31, 0.0, 10.0, 5.31, 0.0, 10.0, 0.001",
        `EX 0, 1, ${seg}, 0, 1.0, 0.0`,
        "FR 0, 1, 14.1, 0.0",
        "EN",
      ].join("\n"),
    );
    await page.getByRole("button", { name: "导入" }).click();
    await expect(wireTable(page).getByRole("row")).toHaveCount(2);
  };

  const feedX = async () => {
    const rect = page.locator('[data-feed="1"]');
    await expect(rect).toHaveCount(1);
    const x = await rect.getAttribute("x");
    expect(x, "馈电标记应有 x 坐标").not.toBeNull();
    return Number(x);
  };

  // 第 1 段在导线起点、第 11 段在中点。导入曾经完全不写 `feed` 信号，
  // 于是两次都会停在中点、标记一动不动（阻抗/增益也就全按中点算了）。
  await importWithFeedSegment(1);
  const atStart = await feedX();
  await importWithFeedSegment(11);
  const atMiddle = await feedX();
  expect(atStart).toBeLessThan(atMiddle - 30);
});

test("天线矩量法求解：非法尺寸给出提示", async ({ page }) => {
  await page.goto("/nec");
  const sphere = page.getByRole("img", { name: "三维方向图" });
  await sphere.scrollIntoViewIfNeeded();
  await expect(sphere).toBeVisible();

  const cell = page.locator('input[aria-label="导线终点 X 1"]');
  await cell.fill("");
  await expect(page.getByText(/参数无法求解/)).toBeVisible();
  // 无解时三维方向图整块隐藏；宿主与 WebGL2 上下文**保留**（不卸载重建），
  // 恢复合法尺寸后必须原样回来 —— 这条钉住的是「上下文不随每次改参数重建」。
  await expect(sphere).toBeHidden();
  await cell.fill("5.05");
  await expect(summary(page)).toBeVisible();
  await expect(sphere).toBeVisible();
});

test("天线矩量法求解：频带扫描给出 SWR 曲线与可用带宽", async ({ page }) => {
  await page.goto("/nec");
  // 折叠状态下不做任何求解，只给一段说明。
  await expect(page.getByText(/展开后可看整段频带上的驻波比曲线/)).toBeVisible();

  await page.getByRole("button", { name: "展开扫频" }).click();
  const chart = page.getByRole("img", { name: "频带扫描曲线" });
  await expect(chart).toBeVisible();
  await expect(chart.locator("path")).toBeVisible();
  // 中心频率 SWR 与最低 SWR 一并给出。
  await expect(
    page.getByText(/中心频率 SWR [\d.]+　｜　最低 SWR [\d.]+（在 [\d.]+ MHz）/),
  ).toBeVisible();

  // 自由空间里的 0.475λ 偶极（默认几何）中心 SWR ≈ 1.7 → 给出 SWR ≤ 2 的可用带宽。
  await page.getByRole("radio", { name: "自由空间" }).click();
  await expect(
    page.getByText(/SWR ≤ 2\.0 带宽：13\.9\d+ – 14\.\d+ MHz　｜　约 \d+ kHz（相对带宽 [\d.]+%）/),
  ).toBeVisible();

  // 换成 10 m 架高 + 理想地面：地面镜像把电抗推大，中心 SWR ≈ 2.2 > 2 →
  // 没有落在工作频率上的可用带宽（最低 SWR 移到了更高的频率）。
  await page.getByRole("radio", { name: "理想导体地面" }).click();
  await expect(
    page.getByText("中心频率上的 SWR 已超过 2.0，没有可用的匹配带宽。"),
  ).toBeVisible();

  // 频点数可切换，曲线仍在；收起后回到说明文案。
  await page.getByRole("radio", { name: "11", exact: true }).click();
  await expect(chart.locator("path")).toBeVisible();
  await page.getByRole("button", { name: "收起扫频" }).click();
  await expect(page.getByText(/展开后可看整段频带上的驻波比曲线/)).toBeVisible();
});

test("天线矩量法求解：Yagi 设计向导给出尺寸并可加载", async ({ page }) => {
  await page.goto("/nec");
  await expect(page.getByText("选好单元数后点「开始设计」。")).toBeVisible();

  // 四单元：跑坐标下降（前端约 1–2 秒），给出尺寸表与实测指标。
  await page.getByRole("radio", { name: "4", exact: true }).click();
  await page.getByRole("button", { name: "开始设计" }).click();
  const designLine = page.getByText(/增益 [\d.]+ dBi　｜　前后比 [\d.]+ dB　｜　SWR [\d.]+　｜　评估 \d+ 次/);
  await expect(designLine).toBeVisible({ timeout: 60_000 });

  // 尺寸表：表头 + 反射器 / 有源振子 / 引向器 1 / 引向器 2。
  const designTable = page.locator('[data-slot="nec-design"]');
  await expect(designTable.getByRole("row")).toHaveCount(5);
  await expect(designTable.getByText("反射器")).toBeVisible();
  await expect(designTable.getByText("有源振子")).toBeVisible();
  await expect(designTable.getByText("引向器 2")).toBeVisible();

  // 换成理想导体导线再加载：向导是在理想导体下优化的，这样两边口径完全一致
  // （默认的铜导线会多出约 0.1 dB 欧姆损耗，那是物理差异、不是口径问题）。
  await page.getByRole("radio", { name: "理想导体（无损耗）" }).click();
  // 加载到导线表：变成四根导线，有源振子是第二根并已馈电（并切到自由空间模型）。
  await page.getByRole("button", { name: "加载到导线表" }).click();
  await expect(page.getByText("已加载到导线表（自由空间模型），可继续手工微调。")).toBeVisible();
  const wireTable = page.locator('[data-slot="nec-wires"]');
  await expect(wireTable.getByRole("row")).toHaveCount(5);
  await expect(page.getByText(/馈电点阻抗 .+ Ω/)).toBeVisible();

  // 口径一致：向导报的增益就是加载后主结果条里的增益（同一个求解器、同一组尺寸）。
  // 用轮询断言 —— `innerText()` 不等重渲染，直接读会拿到上一次的值。
  const reported = Number((await designLine.innerText()).match(/增益 ([\d.]+) dBi/)![1]);
  await expect
    .poll(async () => {
      const text = await page.getByText(/馈电点阻抗 .+ Ω/).innerText();
      const actual = Number(text.match(/增益 ([\d.]+) dBi/)?.[1] ?? "0");
      return Math.abs(reported - actual);
    })
    .toBeLessThan(0.02);
  const finalGain = Number(
    (await page.getByText(/馈电点阻抗 .+ Ω/).innerText()).match(/增益 ([\d.]+) dBi/)![1],
  );
  expect(finalGain).toBeGreaterThan(6.5);
});

test("方案库：从天线 DIY 把 .nec 模板加载进求解器", async ({ page }) => {
  await page.goto("/antenna-diy");
  // 四行方案都挂了模型。
  await expect(page.getByRole("link", { name: "在求解器中打开" })).toHaveCount(4);
  await page.getByRole("link", { name: "在求解器中打开" }).first().click();

  await expect(page).toHaveURL(/\/nec/);
  await expect(page.getByText(/已加载方案库模板：半波偶极/)).toBeVisible();
  await expect(page.getByText(/馈电点阻抗 .+ Ω/)).toBeVisible();

  // 几何确实是模板里的那份（10.14 m、架高 10 m 的偶极），不是页面默认几何。
  await page.getByText("导入 / 导出 .nec").click();
  await page.getByRole("button", { name: "导出当前几何" }).click();
  const box = page.getByRole("textbox", { name: /把 \.nec 文本粘贴/ });
  await expect(box).toHaveValue(/GW 1 21 -5\.0700 0\.0000 10\.0000/);

  // 交接是一次性的：再次进入 /nec 不该重复加载。
  await page.goto("/nec");
  await expect(page.getByText(/已加载方案库模板/)).toHaveCount(0);
  await expect(page.getByText(/馈电点阻抗 .+ Ω/)).toBeVisible();
});

test("方案库：倒 V 模板（折角几何）加载后结果正常", async ({ page }) => {
  await page.goto("/practical-antennas");
  // 五条方案里只有 EFHW 与倒 V 挂了模型，其余如实标「暂无可用模型」。
  await expect(page.getByRole("link", { name: "在求解器中打开" })).toHaveCount(2);
  // 「暂无可用模型」只出现在 title 属性里（`getByText` 匹配不到渲染文本，写了等于没写），
  // 必须按属性定位：5 条方案里 3 条没有模型，各给一个占位符 + title 说明。
  await expect(page.locator('span[title="暂无可用模型"]')).toHaveCount(3);
  await page.getByRole("link", { name: "在求解器中打开" }).nth(1).click();

  await expect(page).toHaveURL(/\/nec/);
  await expect(page.getByText(/已加载方案库模板：倒 V/)).toBeVisible();
  // 折角几何曾是求解器的坑（接点方向没归一化时解出负电阻），这里把「电阻为正」
  // 作为端到端断言钉住。
  const line = page.getByText(/馈电点阻抗 .+ Ω/);
  await expect(line).toBeVisible();
  await expect.poll(async () => {
    const text = await line.innerText();
    return Number(text.match(/馈电点阻抗 (-?[\d.]+)/)?.[1] ?? "0");
  }).toBeGreaterThan(1);
  // 导线表：两条臂。
  await expect(wireTable(page).getByRole("row")).toHaveCount(3);
});

test("天线矩量法求解：传输线段参与求解", async ({ page }) => {
  await page.goto("/nec");
  await expect(page.getByText("还没有传输线。")).toBeVisible();
  const before = await page.getByText(/馈电点阻抗 .+ Ω/).innerText();

  await page.getByRole("button", { name: "添加传输线" }).click();
  await expect(page.getByText(/共 1 段传输线。/)).toBeVisible();
  // 默认行：端口 A = 导线 1 末端、端口 B = 导线 1 起点，Z0 = 50 Ω、线长 5.3 m。
  // 它把偶极两端经一段线接了起来，结果必须变（不变就说明线没进方程）。
  await expect(page.locator('input[aria-label="端口 A 导线 1"]')).toHaveValue("1");
  await expect(page.locator('input[aria-label="线长（m） 1"]')).toHaveValue("5.3");
  await expect
    .poll(async () => {
      const text = await page.getByText(/馈电点阻抗 .+ Ω|参数无法求解/).first().innerText();
      return text;
    })
    .not.toBe(before);

  // 线长可改：改成 λ/2 仍是合法解，页面不崩。
  await page.locator('input[aria-label="线长（m） 1"]').fill("10.63");
  await expect(page.getByText(/馈电点阻抗 .+ Ω|参数无法求解/).first()).toBeVisible();

  // 删掉之后回到「还没有传输线」，结果回到原值。
  // 「删除 1」在导线表里也有一个，这里按 `data-slot` 限定在传输线表上。
  await page
    .locator('[data-slot="nec-lines"]')
    .getByRole("button", { name: /删除 1/ })
    .click();
  await expect(page.getByText("还没有传输线。")).toBeVisible();
  await expect(page.getByText(/馈电点阻抗 .+ Ω/)).toHaveText(before);
});

test("天线矩量法求解：画布拖拽端点与阵列复制", async ({ page }) => {
  await page.goto("/nec");
  const canvas = page.getByRole("img", { name: "几何画布" });
  // 画布在页面中部：不滚进视口的话鼠标事件会落到别处。
  await canvas.scrollIntoViewIfNeeded();
  await expect(canvas).toBeVisible();
  await expect(canvas.locator("[data-handle]")).toHaveCount(2);
  await expect(canvas.locator("[data-feed]")).toHaveCount(1);

  // 拖拽终点手柄：对应的两个坐标写回导线表，另一端不动。
  // 默认偶极沿 X 铺开，所以投影平面的水平轴是 X（标题里也写着）。
  const endX = page.locator('input[aria-label="导线终点 X 1"]');
  const startX = page.locator('input[aria-label="导线起点 X 1"]');
  const beforeEnd = Number(await endX.inputValue());
  const beforeStart = await startX.inputValue();
  const handle = canvas.locator('[data-handle="0-b"]');
  const box = (await handle.boundingBox())!;
  const cx = box.x + box.width / 2;
  const cy = box.y + box.height / 2;
  const summary = page.getByText(/馈电点阻抗 .+ Ω/);
  const resultBefore = await summary.innerText();
  const handleBefore = await handle.getAttribute("cx");

  await page.mouse.move(cx, cy);
  await page.mouse.down();
  await page.mouse.move(cx + 60, cy, { steps: 8 });
  // 还没松手：图形已经跟着走了（预览），但**结果条一个字都没变** ——
  // 这条就是「拖起来跟手」的关键：拖动期间不触发求解。
  await expect.poll(async () => handle.getAttribute("cx")).not.toBe(handleBefore);
  await expect(summary).toHaveText(resultBefore);

  await page.mouse.up();
  // 松手才提交：表格与结果一起更新。
  await expect.poll(async () => Number(await endX.inputValue())).toBeGreaterThan(beforeEnd);
  await expect(startX).toHaveValue(beforeStart);
  await expect.poll(async () => summary.innerText()).not.toBe(resultBefore);

  // 阵列复制：默认沿 X、间距 2 m、3 份 → 再多两根，间距严格是 2 m。
  const startsX = page.locator('input[aria-label^="导线起点 X"]');
  await expect(startsX).toHaveCount(1);
  await page.getByRole("button", { name: "阵列复制" }).click();
  await expect(startsX).toHaveCount(3);
  await expect(canvas.locator("[data-handle]")).toHaveCount(6);
  const x1 = Number(await page.locator('input[aria-label="导线起点 X 1"]').inputValue());
  const x2 = Number(await page.locator('input[aria-label="导线起点 X 2"]').inputValue());
  const x3 = Number(await page.locator('input[aria-label="导线起点 X 3"]').inputValue());
  expect(x2 - x1).toBeCloseTo(2.0, 2);
  expect(x3 - x2).toBeCloseTo(2.0, 2);
  await expect(page.getByText(/馈电点阻抗 .+ Ω/)).toBeVisible();
});

test("天线矩量法求解：NVIS 仰角-距离覆盖图", async ({ page }) => {
  await page.goto("/nec");
  const chart = page.getByRole("img", { name: "仰角-距离覆盖图" });
  await chart.scrollIntoViewIfNeeded();
  await expect(chart).toBeVisible();

  // 默认 10 m 架高（14.1 MHz 上 0.47λ）：主瓣压得低，直接给「不是 NVIS 天线」的诊断。
  await expect(page.getByText(/主瓣仰角 \d+° 偏低/)).toBeVisible();

  // 压到 0.1λ ≈ 2.1 m：主瓣抬到天顶附近，落点应进 350 km 以内。
  const height = page.locator("label", { hasText: "架设高度" }).locator("input");
  await height.fill("2.1");
  await expect(page.getByText(/主瓣已经抬得够高/)).toBeVisible();
  const peakLine = page.getByText(/主瓣落在 \d+ km（仰角 \d+°）/);
  await expect(peakLine).toBeVisible();
  await expect
    .poll(async () => Number((await peakLine.innerText()).match(/主瓣落在 (\d+) km/)![1]))
    .toBeLessThan(350);

  // 电离层虚高可改：0.1λ 架高的主瓣正对天顶，落点恒为 0 km，所以这里只确认
  // 改参数不崩、曲线仍在（层高对落点的影响由核心单测覆盖 —— 那里用的是主瓣
  // 不在天顶的几何，才能看出差别）。
  await page.locator('input[aria-label="电离层虚高（km）"]').fill("600");
  await expect(page.getByRole("img", { name: "仰角-距离覆盖图" })).toBeVisible();
  await expect(peakLine).toBeVisible();

  // 改回高架：诊断回到「偏低」，且整个页面仍能求解。
  await height.fill("10");
  await expect(page.getByText(/主瓣仰角 \d+° 偏低/)).toBeVisible();
  await expect(page.getByText(/馈电点阻抗 .+ Ω/)).toBeVisible();
});
