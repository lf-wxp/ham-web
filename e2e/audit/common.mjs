// 全站审计脚本的共用部分：站点地址、路由表、退出码。
//
// 为什么抽出来：三份脚本各自写死 `http://127.0.0.1:3031`、各自用相对 cwd 的
// `grep ../crates/...` 取路由 —— 按文档从仓库根运行时 grep 找不到文件，管道末尾的
// `sort` 仍返回 0，路由表静默为空，脚本只扫了一页 404 还照样报「0 串风格」。
// 现在路径按本文件定位（与 cwd 无关），取不到路由直接失败，有问题就以非 0 退出。

import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

/** 被测站点：与 `playwright.config.ts` / `scripts/screenshots.mjs` 同一个环境变量。 */
export const baseUrl = (process.env.E2E_BASE_URL ?? "http://127.0.0.1:3030").replace(/\/$/, "");

/** registry 里登记的全部路由（去重、排序），不含 404。 */
export function registryRoutes() {
  const file = fileURLToPath(new URL("../../crates/core/src/registry.rs", import.meta.url));
  const source = readFileSync(file, "utf8");
  const routes = [...new Set([...source.matchAll(/path: "(\/[^"]*)"/g)].map((m) => m[1]))].sort();
  if (routes.length < 100) {
    // registry 当前有 180 条：远少于这个数说明解析口径（`path: "…"`）变了，别拿残缺的表去报「全绿」。
    throw new Error(`只解析到 ${routes.length} 条路由（${file}）：registry 的写法变了？`);
  }
  return routes;
}

/** 只认 `modern` / `pixel`，其余直接报错（拼错参数不该默默扫成 modern）。 */
export function parseMode(argv) {
  const mode = argv[2] ?? "modern";
  if (mode !== "modern" && mode !== "pixel") {
    throw new Error(`用法：node <脚本> [modern|pixel]（收到 ${mode}）`);
  }
  return mode;
}

/** 与 `localStorage` 种子一致的初始化脚本参数：风格 + 明暗 + 跳过首次快捷键帮助。 */
export function seedScript([m]) {
  if (window !== window.top) return;
  localStorage.setItem("ui:uiStyle", m);
  localStorage.setItem("theme", m === "modern" ? "dark" : "light");
  localStorage.setItem("ui:shortcutsHelpSeen:practice", "1");
  localStorage.setItem("ui:shortcutsHelpSeen:exam", "1");
}

/** 等到应用真的挂载（`#route-content` 带上签名），而不是睡一个固定时长。 */
export async function waitMounted(page, timeout = 20_000) {
  await page.waitForSelector("#route-content[data-sig]", { timeout });
}
