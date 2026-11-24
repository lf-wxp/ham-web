import { expect, pickOption, test } from "./fixtures";

/** 假的 Kenwood 电台：收到 FA; / MD; 查询后按 Kenwood 格式应答（故意把应答切成两半）。
 * 页面里用 `__setRigFreq("00014074000")` / `__setRigMode("3")` 改电台此刻的频率与模式
 * （Kenwood 模式码：1 / 2 = SSB，3 = CW，4 = FM，5 = AM，6 = RTTY）。 */
async function installFakeRig(page: import("@playwright/test").Page) {
  await page.addInitScript(() => {
    const queue: Uint8Array[] = [];
    let wake: (() => void) | null = null;
    let freq = "00014074000";
    let mode = "3";
    const enc = new TextEncoder();
    const push = (s: string) => {
      const b = enc.encode(s);
      queue.push(b.slice(0, 3), b.slice(3));
      wake?.();
    };
    const w = window as unknown as {
      __setRigFreq: (f: string) => void;
      __setRigMode: (m: string) => void;
      __written: string[];
    };
    w.__written = [];
    w.__setRigFreq = (f) => {
      freq = f;
    };
    w.__setRigMode = (m) => {
      mode = m;
    };
    const port = {
      async open() {},
      async close() {},
      readable: {
        getReader: () => ({
          async read() {
            while (queue.length === 0) await new Promise<void>((r) => (wake = r));
            return { value: queue.shift(), done: false };
          },
          async cancel() {
            wake?.();
          },
          releaseLock() {},
        }),
      },
      writable: {
        getWriter: () => ({
          async write(b: Uint8Array) {
            const cmd = new TextDecoder().decode(b);
            w.__written.push(cmd);
            if (cmd === "FA;") push(`FA${freq};`);
            if (cmd === "MD;") push(`MD${mode};`);
          },
          releaseLock() {},
        }),
      },
    };
    Object.defineProperty(navigator, "serial", {
      value: { requestPort: async () => port },
      configurable: true,
    });
  });
}

/** 让电台换到某个频率（MHz）与 Kenwood 模式码。 */
async function tuneRig(page: import("@playwright/test").Page, mhz: number, modeCode: string) {
  const hz = String(Math.round(mhz * 1e6)).padStart(11, "0");
  await page.evaluate(
    ([f, m]) => {
      const w = window as unknown as {
        __setRigFreq: (f: string) => void;
        __setRigMode: (m: string) => void;
      };
      w.__setRigFreq(f);
      w.__setRigMode(m);
    },
    [hz, modeCode],
  );
}

test("电台 CAT：模拟 Kenwood 串口，回填频率与模式", async ({ page }) => {
  await installFakeRig(page);

  await page.goto("/log");
  await expect(page.getByRole("heading", { level: 1, name: "通联日志" })).toBeVisible();
  await page.getByRole("button", { name: "连接电台" }).first().click();

  await expect(page.getByText("已连接 · 14.074 MHz · CW").first()).toBeVisible();
  const freq = page.getByLabel("频率（MHz）").first();
  await expect(freq).toHaveValue("14.074");

  // 电台换频后表单跟着变
  await page.evaluate(() => (window as unknown as { __setRigFreq: (f: string) => void }).__setRigFreq("00007025500"));
  await expect(freq).toHaveValue("7.0255");

  await page.getByRole("button", { name: "断开" }).first().click();
  await expect(page.getByRole("button", { name: "连接电台" }).first()).toBeVisible();
});

test("电台 CAT：频率护栏按读到的频率、模式与操作证类别亮灯（只读）", async ({ page }) => {
  await installFakeRig(page);
  await page.goto("/log");
  await expect(page.getByRole("heading", { level: 1, name: "通联日志" })).toBeVisible();
  const guard = page.locator('[data-slot="rig-guard"]').first();

  // 没连电台：没有护栏条。
  await expect(guard).toHaveCount(0);
  await page.getByRole("button", { name: "连接电台" }).first().click();
  await expect(page.getByText("已连接 · 14.074 MHz · CW").first()).toBeVisible();
  await expect(guard).toBeVisible();

  // 没选类别：20 m CW 没问题（绿灯、带波段名），并提示选了类别还能判断证外；不会因「证外」变红。
  await expect(guard.getByText("绿灯 · 未发现问题")).toBeVisible();
  await expect(guard.getByText("20m")).toBeVisible();
  await expect(guard.getByText(/选择类别后还会判断「证外」/)).toBeVisible();

  // A 类只有 30–3000 MHz：同一个频率立刻变红，并点名是哪条规则。
  await pickOption(page, "操作证类别", "A 类");
  await expect(guard.getByText("红灯 · 此刻发射不合规")).toBeVisible();
  await expect(guard.getByText("A 类操作证无权使用这个频率（办法第三十条）")).toBeVisible();
  await expect(guard.getByText(/发射功率上限/)).toHaveCount(0);

  // 换到 2 m 调频：A 类可用，绿灯并给出 25 W 的上限；选了类别后提示消失。
  await tuneRig(page, 145.5, "4");
  await expect(guard.getByText("绿灯 · 未发现问题")).toBeVisible();
  await expect(guard.getByText("发射功率上限 25 W")).toBeVisible();
  await expect(guard.getByText(/选择类别后还会判断/)).toHaveCount(0);

  // C 类、14.030 MHz 发话务（SSB）：那一段是电报 / 数据，红灯。
  await pickOption(page, "操作证类别", "C 类");
  await tuneRig(page, 14.03, "2");
  await expect(guard.getByText("红灯 · 此刻发射不合规")).toBeVisible();
  await expect(guard.getByText(/20m 波段规划里 14\.0\d+–14\.\d+ MHz 不用于话务/)).toBeVisible();
  // 同一频率换成 CW 就没问题（CW 在那里是对路的）。
  await tuneRig(page, 14.03, "3");
  await expect(guard.getByText("绿灯 · 未发现问题")).toBeVisible();
  await expect(guard.getByText(/发射功率上限 1000 W/)).toBeVisible();

  // 3.8 MHz 发 CW：话务专用子段，只给黄灯「偏离波段规划」（不是违规），外加这一段本来就要协调。
  await tuneRig(page, 3.8, "3");
  await expect(guard.getByText("黄灯 · 需留意")).toBeVisible();
  await expect(guard.getByText(/话务专用子段.*偏离波段规划（只是提醒，不算违规）/)).toBeVisible();
  await expect(guard.getByText(/频率协调（办法第十五条）/)).toBeVisible();
  await expect(guard.getByText("红灯 · 此刻发射不合规")).toHaveCount(0);

  // 不在业余频段（7.3 MHz 在中国的 40 m 之外）：灰灯，说明只收听可忽略，没有任何「发现」。
  await tuneRig(page, 7.3, "2");
  await expect(guard.getByText(/灰灯 · 不在业余频段/)).toBeVisible();
  await expect(guard.getByRole("listitem")).toHaveCount(0);

  // 只读：整个过程只发过查询，没写过任何设频 / 设模式 / 发射命令（Kenwood 的写命令带数字参数）。
  const written = await page.evaluate(() => (window as unknown as { __written: string[] }).__written);
  expect(written.length).toBeGreaterThan(4);
  expect(written.every((c) => c === "FA;" || c === "MD;"), written.join(" ")).toBe(true);

  // 类别记在这台设备上：刷新、重新连电台后还在。
  await page.reload();
  await page.getByRole("button", { name: "连接电台" }).first().click();
  await expect(guard).toBeVisible();
  await expect(page.getByRole("combobox", { name: "操作证类别" })).toContainText("C 类");

  // 断开后护栏条消失。
  await page.getByRole("button", { name: "断开" }).first().click();
  await expect(guard).toHaveCount(0);
});

/** 整页与护栏条各自的横向溢出量（像素，≤ 1 视为没有溢出）。 */
async function overflowOf(page: import("@playwright/test").Page) {
  return page.evaluate(() => ({
    page: document.documentElement.scrollWidth - window.innerWidth,
    guard: (() => {
      const el = document.querySelector('[data-slot="rig-guard"]') as HTMLElement;
      return el.scrollWidth - el.clientWidth;
    })(),
  }));
}

test("电台 CAT：溢出检测器自测——页面与护栏条里放进超宽元素时必须量得出溢出", async ({ page }) => {
  await page.setViewportSize({ width: 320, height: 800 });
  await installFakeRig(page);
  await page.goto("/log");
  await page.getByRole("button", { name: "连接电台" }).first().click();
  await expect(page.locator('[data-slot="rig-guard"]').first()).toBeVisible();
  // 先确认基线本身没溢出，再注入：否则「测得出」可能只是基线就是坏的。
  const before = await overflowOf(page);
  expect(before.page).toBeLessThanOrEqual(1);
  expect(before.guard).toBeLessThanOrEqual(1);

  await page.evaluate(() => {
    const wide = document.createElement("div");
    wide.style.cssText = "width:900px;height:4px";
    document.querySelector('[data-slot="rig-guard"]')!.appendChild(wide);
  });
  const after = await overflowOf(page);
  expect(after.guard, "护栏条里放 900px 宽的元素，量到的溢出应远大于 1").toBeGreaterThan(100);
  expect(after.page, "整页同理").toBeGreaterThan(100);
});

// 护栏条是整页里文字最长的一块（黄灯同时带「偏离规划」与「协调」两句），而全站的溢出扫描不会连电台，
// 覆盖不到这个状态：这里在 320 px 下、三语 × 两套皮肤各量一次，整页不能出现横向滚动。
// 现代风按 `E2E_UI_STYLE=modern` 的口径设成暗色主题，与 `i18n_layout.spec.ts` 一致。
for (const lang of [
  { code: "zh", name: "中文", connect: "连接电台" },
  { code: "en", name: "English", connect: "Connect rig" },
  { code: "es", name: "Español", connect: "Conectar equipo" },
]) {
  for (const style of ["pixel", "modern"]) {
    test(`电台 CAT：频率护栏在 320px 下不横向溢出（${lang.name} · ${style}）`, async ({ page }) => {
      await page.setViewportSize({ width: 320, height: 800 });
      await page.addInitScript(
        ([code, ui]) => {
          localStorage.setItem("locale", code);
          localStorage.setItem("ui:uiStyle", ui);
          if (ui === "modern") localStorage.setItem("theme", "dark");
        },
        [lang.code, style],
      );
      await installFakeRig(page);
      await page.goto("/log");
      await page.getByRole("button", { name: lang.connect }).first().click();
      const guard = page.locator('[data-slot="rig-guard"]').first();
      await expect(guard).toBeVisible();

      // 3.8 MHz CW、未选类别：「偏离规划」与「协调」两句，加上「选类别」的提示，是这块里最长的一屏
      // （类别无权使用时不会再叠协调，所以发现最多两条）。
      await tuneRig(page, 3.8, "3");
      await expect(guard.getByRole("listitem")).toHaveCount(2);
      const overflow = await overflowOf(page);
      expect(overflow.page, "整页横向溢出").toBeLessThanOrEqual(1);
      expect(overflow.guard, "护栏条自身横向溢出").toBeLessThanOrEqual(1);
    });
  }
}
