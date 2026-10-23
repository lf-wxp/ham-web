# PWA 与离线

> `postbuild` 生成 Service Worker 的那套机制与缓存策略、更新提示、图标生成都在这份。

- `cargo make build-web` 的最后一步 `postbuild` 会扫描 `dist/`，为所有静态资源计算 SHA-256 修订号并生成 `dist/sw.js`（模板：`crates/tools/templates/sw.js`）；
- 缓存策略与原版保持一致：
  - 题库 JSON（`/questions/*.json`）：**NetworkFirst**，离线回退缓存（最多 10 条，7 天）；
  - 题目图片（`/questions/images/*`）：**StaleWhileRevalidate**（最多 300 张，30 天）；
  - 页面导航：离线时回退到预缓存的 `index.html`；其余静态资源预缓存；
- 部署新版本后，页面可见时与此后每小时检查一次更新；发现新 Service Worker 时在右下角提示「发现新版本」并列出更新内容（读取 `/changelog.json`），点击「刷新以更新」即切换到新版。提示不遮挡页面，考试中可以先忽略；
- 刷新后首次打开会显示一次「已更新到新版本」及更新内容；首次访问的用户不会看到；
- 更新内容写在 `crates/core/src/changelog.rs` 的 `CHANGELOG` 中（新版本插到最前面，日期 + 条目），`postbuild` 会导出为 `dist/changelog.json`；
- `postbuild` 还会为每个题库计算内容哈希，写入 `dist/questions/config.json` 的 `banks.*.rev`；前端请求题库时带上 `?v=<rev>`，题库内容变化后一定会重新下载。前端记录上次看到的题目摘要，题库变化时提示「题库已更新：修改 N 题，新增 N 题…」；
- 应用图标由 `cargo make icons` 渲染（纯 Rust，resvg），按三份设计源产出三类图标：
  - `public/pwa-icon.svg`（圆角母版）→ `pwa-icon-192.png` / `pwa-icon-512.png` 与 `favicon.ico`（16 / 32 / 48 三档打包）：保留品牌圆角，供 manifest 的 `any`、`favicon` 与 `og:image` 使用（这些场景不会替我们裁切）—— ICO 的 16 / 32 两档改用 `public/favicon-16.svg`，那是同一套构图按 16px 重取值的一版（母版的塔腿在这个尺度只有 1–2px，插值出来是一团灰；32 档是它的 2 倍整数放大，仍是整像素）；
  - `public/pwa-maskable.svg`（满幅版）→ `apple-touch-icon*.png` / `pwa-maskable-*.png`：**满幅不透明方形**，圆角交给系统遮罩。iOS 的 `apple-touch-icon` 是按原图直接使用的（不裁圆），自带圆角的 PNG 在四个角会把透明像素合成到白底上，因此这类图标四角必须不透明；图形还缩到 0.8 并居中，落在 maskable 的 80% 安全圆内（启动器的圆形遮罩只保证圆里的内容）。
  - `public/pwa-badge.svg`（单色版）→ `pwa-badge-96.png`：Web Push 通知栏的 badge。Android 只取它的 alpha 通道再按系统栏颜色着色，拿彩色图标当 badge 会渲染成一个实心方块。

