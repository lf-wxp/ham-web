// 生成 README 顶部用的截图（docs/images/）。
//
// 为什么要有脚本：截图是「真实页面」的证据，但手工截很快就会和界面脱节（换了配色、改了
// 标题、加了新卡片）。这里把路由、视口与主题写死，谁改完界面跑一次就能刷新，不用记参数。
//
// 用法（先让站点跑起来，截图的是 release 产物才不会拍到 dev 的字号差异）：
//   cargo make build-web && cargo make serve
//   cd e2e && node scripts/screenshots.mjs
//   # 也可以指向已部署站点：E2E_BASE_URL=https://ham.onlyxp.me node scripts/screenshots.mjs
import { chromium } from '@playwright/test';
import { fileURLToPath } from 'node:url';
import path from 'node:path';

const base = process.env.E2E_BASE_URL ?? 'http://127.0.0.1:8080';
const outDir = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../../docs/images');

// 浅色 / 深色各一张首页，再补一张核心流程（闯关地图）与手机宽度：README 的位置有限，
// 只展示「一眼能看出这是像素风题库」的四张。
const shots = [
  { file: 'home-desktop.webp', route: '/', width: 1440, height: 900, theme: 'light' },
  { file: 'home-dark.webp', route: '/', width: 1440, height: 900, theme: 'dark' },
  { file: 'map-desktop.webp', route: '/map?bank=A', width: 1440, height: 900, theme: 'light' },
  { file: 'home-mobile.webp', route: '/', width: 390, height: 844, theme: 'light' },
];

const browser = await chromium.launch();
try {
  for (const shot of shots) {
    const context = await browser.newContext({
      viewport: { width: shot.width, height: shot.height },
      deviceScaleFactor: 1,
      // 固定语言与主题：截图里不该出现上一次运行时留下的偏好。
      locale: 'zh-CN',
    });
    await context.addInitScript((theme) => {
      localStorage.setItem('theme', theme);
      localStorage.setItem('locale', 'zh');
    }, shot.theme);

    const page = await context.newPage();
    await page.goto(base + shot.route, { waitUntil: 'networkidle' });
    // wasm 挂载完成前 <main> 只有启动画面；等到真的有标题再截。
    await page.waitForFunction(
      () => (document.querySelector('main')?.textContent ?? '').trim().length > 0,
      null,
      { timeout: 30_000 },
    );
    // 像素字体是异步分片加载的，字体没到就截会拍到抗锯齿回退字形。
    await page.evaluate(() => document.fonts.ready);
    await page.waitForTimeout(1200);

    const file = path.join(outDir, shot.file);
    await page.screenshot({ path: file, type: 'webp' });
    console.log(`✓ ${path.relative(process.cwd(), file)}`);
    await context.close();
  }
} finally {
  await browser.close();
}
