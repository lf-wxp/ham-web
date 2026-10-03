#!/usr/bin/env node
/**
 * e2e 覆盖率统计。
 *
 * 统计口径（三条互不替代，一起看才有意义）：
 *
 *   1. 冒烟覆盖率   —— smoke.spec.ts / i18n_layout.spec.ts 从路由表读取全部路由逐一体检，
 *                      因此恒为 100%（新增页面自动纳入，无需改测试）。
 *   2. 交互覆盖率   —— 至少有一条「功能用例」通过 page.goto 显式访问过的路由数 / 路由总数。
 *                      只统计显式 goto，不含点击链接间接到达的页面（口径保守、可复现）。
 *   3. 关键流程覆盖率 —— e2e/coverage-targets.json 中列出的核心用户旅程里，
 *                      已有用例标题命中的比例。这是真正反映「业务场景覆盖」的指标。
 *
 * 用法：
 *   node scripts/coverage.mjs                    # 打印 Markdown 报告并写 test-results/coverage.{json,md}
 *   E2E_MIN_JOURNEY=90 node scripts/coverage.mjs # 关键流程覆盖率低于 90% 时退出码 1
 *   E2E_MIN_ROUTE=15   node scripts/coverage.mjs # 交互覆盖率低于 15% 时退出码 1
 */

import { mkdirSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

const here = fileURLToPath(new URL(".", import.meta.url));
const e2eDir = join(here, "..");
const repoDir = join(e2eDir, "..");
const testsDir = join(e2eDir, "tests");
const outDir = join(e2eDir, "test-results");

/** 路由表以 main_content.rs 为准（与 smoke.spec.ts 保持同一来源）。 */
function readRoutes() {
  const source = readFileSync(join(repoDir, "crates/app/src/app/main_content.rs"), "utf8");
  return [...source.matchAll(/path!\("([^"]+)"\)/g)]
    .map((m) => m[1])
    .filter((p) => !p.includes(":") && !p.includes("*"));
}

/** 会遍历全部路由的「全量体检」用例，不计入交互覆盖率（否则恒为 100%，失去意义）。 */
const SWEEP_FILES = new Set(["smoke.spec.ts", "i18n_layout.spec.ts"]);

function readSpecs() {
  return readdirSync(testsDir)
    .filter((f) => f.endsWith(".spec.ts"))
    .sort()
    .map((file) => {
      const text = readFileSync(join(testsDir, file), "utf8");
      const titles = [...text.matchAll(/test\(\s*["'`]([^"'`]+)["'`]/g)].map((m) => m[1]);
      // 显式 goto 的路由（去掉 query / hash）
      const gotos = [...text.matchAll(/goto\(\s*["'`]([^"'`]+)["'`]/g)]
        .map((m) => m[1].split(/[?#]/)[0])
        .filter((p) => p.startsWith("/"));
      const sweeps = /for \(const route of routes\)/.test(text);
      return { file, titles, gotos, sweeps };
    });
}

const routes = readRoutes();
const specs = readSpecs();
const targets = JSON.parse(readFileSync(join(e2eDir, "coverage-targets.json"), "utf8"));

// ── 口径一：冒烟覆盖 ─────────────────────────────────────────────
const sweeps = specs.filter((s) => s.sweeps).map((s) => s.file);
const smokeCovered = sweeps.length ? routes.length : 0;

// ── 口径二：交互覆盖 ─────────────────────────────────────────────
const routeHits = new Map();
for (const spec of specs) {
  if (SWEEP_FILES.has(spec.file)) continue;
  for (const path of spec.gotos) {
    if (!routes.includes(path)) continue;
    if (!routeHits.has(path)) routeHits.set(path, []);
    routeHits.get(path).push(spec.file);
  }
}
const interactCovered = routeHits.size;
const uncoveredRoutes = routes.filter((r) => !routeHits.has(r));

// ── 口径三：关键流程覆盖 ─────────────────────────────────────────
const allTitles = specs.flatMap((s) => s.titles);
const journeys = targets.journeys.map((j) => {
  const hit = allTitles.filter((t) => t.includes(j.grep));
  return {
    ...j,
    covered: hit.length > 0,
    matched: hit,
    ambiguous: hit.length > 1,
  };
});
const journeyCovered = journeys.filter((j) => j.covered).length;

const pct = (n, d) => (d === 0 ? 100 : Math.round((n / d) * 1000) / 10);

const report = {
  generatedAt: new Date().toISOString(),
  routes: {
    total: routes.length,
    smokeCovered,
    smokePercent: pct(smokeCovered, routes.length),
    interactCovered,
    interactPercent: pct(interactCovered, routes.length),
    uncovered: uncoveredRoutes,
  },
  journeys: {
    total: journeys.length,
    covered: journeyCovered,
    percent: pct(journeyCovered, journeys.length),
    missing: journeys.filter((j) => !j.covered).map((j) => `${j.group} / ${j.name}`),
    ambiguous: journeys.filter((j) => j.ambiguous).map((j) => `${j.group} / ${j.name}`),
  },
  tests: {
    total: allTitles.length,
    perFile: Object.fromEntries(specs.map((s) => [s.file, s.titles.length])),
  },
  sweepFiles: sweeps,
};

// ── 输出 ─────────────────────────────────────────────────────────
const lines = [];
lines.push("# e2e 覆盖率报告");
lines.push("");
lines.push(`生成时间：${report.generatedAt}`);
lines.push("");
lines.push("## 统计口径");
lines.push("");
lines.push("| 指标 | 定义 | 结果 |");
lines.push("| --- | --- | --- |");
lines.push(
  `| 冒烟覆盖率 | ${sweeps.join(" / ")} 从路由表遍历全部路由做渲染 + 无障碍体检 | ` +
    `${smokeCovered}/${routes.length}（${report.routes.smokePercent}%） |`,
);
lines.push(
  `| 交互覆盖率 | 至少一条功能用例通过 page.goto 显式访问的路由 | ` +
    `${interactCovered}/${routes.length}（${report.routes.interactPercent}%） |`,
);
lines.push(
  `| 关键流程覆盖率 | coverage-targets.json 中核心用户旅程已有对应用例的比例 | ` +
    `${journeyCovered}/${journeys.length}（${report.journeys.percent}%） |`,
);
lines.push("");
lines.push(
  `静态声明用例数：${allTitles.length}（${specs.length} 个 spec 文件；` +
    `另有 ${sweeps.join(" / ")} 按路由表动态生成的用例未计入）`,
);
lines.push("");
lines.push("## 各文件用例数");
lines.push("");
lines.push("| 文件 | 用例数 |");
lines.push("| --- | --- |");
for (const [file, n] of Object.entries(report.tests.perFile)) {
  lines.push(`| \`${file}\` | ${n} |`);
}
lines.push("");
if (report.journeys.missing.length) {
  lines.push("## 未覆盖的关键流程");
  lines.push("");
  for (const m of report.journeys.missing) lines.push(`- ${m}`);
  lines.push("");
}
if (report.routes.uncovered.length) {
  lines.push(`## 无功能用例显式访问的路由（${report.routes.uncovered.length} 个）`);
  lines.push("");
  lines.push("> 这些路由已由冒烟测试覆盖渲染与无障碍，但尚无专属功能用例。");
  lines.push("");
  lines.push("```");
  lines.push(report.routes.uncovered.join("  "));
  lines.push("```");
  lines.push("");
}

const md = lines.join("\n");
mkdirSync(outDir, { recursive: true });
writeFileSync(join(outDir, "coverage.json"), JSON.stringify(report, null, 2));
writeFileSync(join(outDir, "coverage.md"), md);
process.stdout.write(`${md}\n`);

// ── 阈值门禁（CI 可选开启） ───────────────────────────────────────
const minJourney = Number(process.env.E2E_MIN_JOURNEY ?? NaN);
const minRoute = Number(process.env.E2E_MIN_ROUTE ?? NaN);
let failed = false;
if (Number.isFinite(minJourney) && report.journeys.percent < minJourney) {
  console.error(`关键流程覆盖率 ${report.journeys.percent}% 低于阈值 ${minJourney}%`);
  failed = true;
}
if (Number.isFinite(minRoute) && report.routes.interactPercent < minRoute) {
  console.error(`交互覆盖率 ${report.routes.interactPercent}% 低于阈值 ${minRoute}%`);
  failed = true;
}
if (report.journeys.ambiguous.length) {
  console.error(`以下 journey 的 grep 命中多条用例，请收敛：${report.journeys.ambiguous.join(", ")}`);
  failed = true;
}
process.exit(failed ? 1 : 0);
