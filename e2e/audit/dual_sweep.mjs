// 全量隔离扫描：每条路由在一种风格下渲染，DOM 里不许出现另一套的类名。
//
//   node e2e/audit/dual_sweep.mjs modern    # 现代风：不许有 `pxl-*`
//   node e2e/audit/dual_sweep.mjs pixel     # 像素风：不许有 `md-*`
//
// 前置：站点已在运行（`trunk serve ... --port 3030`，或设 `E2E_BASE_URL`）；路径与 cwd 无关，
// 仓库根或 `e2e/` 下都能跑。有任何串风格 / 页面报错 / 致命页 / 超时，以非 0 退出。

import { chromium } from "@playwright/test";
import { baseUrl, parseMode, registryRoutes, seedScript, waitMounted } from "./common.mjs";

const mode = parseMode(process.argv);
const isModern = mode === "modern";

// 404 不在 registry 里，但它也是「页面」，必须两套皮肤都适配。
const routes = [...registryRoutes(), "/no-such-page-xyz"];
console.log(`routes ${routes.length} · mode ${mode} · ${baseUrl}`);

const browser = await chromium.launch();
const bad = [];
let pageErrors = 0;

async function worker(list) {
  for (const route of list) {
    const ctx = await browser.newContext({ viewport: { width: 1280, height: 950 } });
    const page = await ctx.newPage();
    let pageErr = 0;
    page.on("pageerror", () => (pageErr += 1));
    await page.addInitScript(seedScript, [mode]);
    try {
      await page.goto(baseUrl + route, { waitUntil: "domcontentloaded", timeout: 20_000 });
      // 等应用挂载而不是睡固定时长：没挂上时 DOM 里只有启动页，扫出来必然是空，会被当成「全绿」。
      await waitMounted(page);
      await page.waitForLoadState("networkidle", { timeout: 5_000 }).catch(() => undefined);
      const info = await page.evaluate(([modern]) => {
        const foreign = new Set();
        for (const el of document.querySelectorAll("[class]")) {
          for (const c of el.classList) {
            if (modern && c.startsWith("pxl-")) foreign.add(c);
            if (!modern && c.startsWith("md-")) foreign.add(c);
          }
        }
        return {
          foreign: [...foreign],
          fatal: !!document.querySelector("#fatal"),
          // 正向对照：现代风下必须真的渲染出 `.md-root`，否则「没有 pxl-*」什么也证明不了。
          styled: modern ? !!document.querySelector(".md-root") : true,
        };
      }, [isModern]);
      if (pageErr) pageErrors += 1;
      if (info.foreign.length || info.fatal || pageErr || !info.styled) {
        bad.push({ route, ...info, pageErr });
      }
    } catch (e) {
      bad.push({ route, error: String(e).slice(0, 80) });
    }
    await ctx.close();
  }
}

const chunks = [[], [], []];
routes.forEach((r, i) => chunks[i % 3].push(r));
await Promise.all(chunks.map(worker));
await browser.close();

console.log(`\n== ${mode}：有问题的页面 ${bad.length} ==`);
for (const x of bad.slice(0, 30)) console.log(" ", JSON.stringify(x));
console.log("页面报错累计:", pageErrors);
if (bad.length) process.exitCode = 1;
