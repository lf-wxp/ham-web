// 生成 README 顶部用的截图（docs/images/）：像素风 4 张 + 现代风 5 张。
//
// 为什么要有脚本：截图是「真实页面」的证据，但手工截很快就会和界面脱节（换了配色、改了
// 标题、加了新卡片）。这里把路由、视口与主题写死，谁改完界面跑一次就能刷新，不用记参数。
//
// 用法（先让站点跑起来，截图的是 release 产物才不会拍到 dev 的字号差异）：
//   cargo make build-web && cargo make serve
//   cd e2e && node scripts/screenshots.mjs            # 全拍（像素风 4 张 + 现代风 5 张）
//   cd e2e && node scripts/screenshots.mjs modern     # 只重拍现代风（pixel 同理）
//   # 也可以指向已部署站点：E2E_BASE_URL=https://ham.onlyxp.me node scripts/screenshots.mjs
import { chromium } from '@playwright/test';
import { fileURLToPath } from 'node:url';
import path from 'node:path';

const base = process.env.E2E_BASE_URL ?? 'http://127.0.0.1:8080';
const outDir = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../../docs/images');

// 两套皮肤各拍一组，场景一一对照：浅色 / 深色首页、闯关地图、手机首页；现代风再多一张练习页
// （答题版式与像素风完全不同，是「不只是换配色」最直观的证据）。README 的位置有限，不再多拍。
//
// `style` 缺省为 pixel（= 站点默认值，不写任何偏好）；`modern` 会写 `ui:uiStyle`。
// 用 `node scripts/screenshots.mjs modern` 只重拍现代风、`pixel` 只重拍像素风，不带参数则全拍。
const shots = [
  { file: 'home-desktop.webp', route: '/', width: 1440, height: 900, theme: 'light' },
  { file: 'home-dark.webp', route: '/', width: 1440, height: 900, theme: 'dark' },
  { file: 'map-desktop.webp', route: '/map?bank=A', width: 1440, height: 900, theme: 'light' },
  { file: 'home-mobile.webp', route: '/', width: 390, height: 844, theme: 'light' },

  { file: 'home-modern-dark.webp', style: 'modern', route: '/', width: 1440, height: 900, theme: 'dark' },
  { file: 'home-modern-light.webp', style: 'modern', route: '/', width: 1440, height: 900, theme: 'light' },
  { file: 'map-modern.webp', style: 'modern', route: '/map?bank=A', width: 1440, height: 900, theme: 'light' },
  { file: 'practice-modern.webp', style: 'modern', route: '/practice?bank=A', width: 1440, height: 900, theme: 'dark', practice: true },
  { file: 'home-modern-mobile.webp', style: 'modern', route: '/', width: 390, height: 844, theme: 'dark' },
];

const only = process.argv[2];
if (only && !['pixel', 'modern'].includes(only)) {
  console.error(`参数只能是 pixel / modern（收到 ${only}）`);
  process.exit(2);
}
const selected = shots.filter((shot) => !only || (shot.style ?? 'pixel') === only);

const browser = await chromium.launch();
try {
  for (const shot of selected) {
    const context = await browser.newContext({
      viewport: { width: shot.width, height: shot.height },
      deviceScaleFactor: 1,
      // 固定语言与主题：截图里不该出现上一次运行时留下的偏好。
      locale: 'zh-CN',
    });
    await context.addInitScript(([theme, style, practice]) => {
      // 只写顶层页面：页面里若有 iframe（预览 / 嵌入），别让它们也被改写。
      if (window !== window.top) return;
      localStorage.setItem('theme', theme);
      localStorage.setItem('locale', 'zh');
      // 像素风是站点默认值：不写偏好才是「用户第一次打开」看到的样子；现代风显式写入。
      if (style === 'modern') localStorage.setItem('ui:uiStyle', 'modern');
      // 练习页首次进入会弹快捷键说明，遮住题卡。
      if (practice) localStorage.setItem('ui:shortcutsHelpSeen:practice', '1');
    }, [shot.theme, shot.style ?? 'pixel', !!shot.practice]);

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

    // 拍到的必须是想要的那套皮肤与主题；否则宁可失败，也不要把默认风格存成「现代风」截图。
    const actual = await page.evaluate(() => ({
      style: document.documentElement.dataset.uiStyle,
      dark: document.documentElement.classList.contains('dark'),
    }));
    const wantStyle = shot.style ?? 'pixel';
    if (actual.style !== wantStyle || actual.dark !== (shot.theme === 'dark')) {
      throw new Error(
        `${shot.file}：想要 ${wantStyle}/${shot.theme}，页面实际是 ${actual.style}/${actual.dark ? 'dark' : 'light'}`,
      );
    }

    const file = path.join(outDir, shot.file);
    await page.screenshot({ path: file, type: 'webp' });
    console.log(`✓ ${path.relative(process.cwd(), file)}`);
    await context.close();
  }
} finally {
  await browser.close();
}
