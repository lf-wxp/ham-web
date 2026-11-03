import type { Page } from "@playwright/test";
import { expect, test } from "@playwright/test";
import { AUDIT } from "./layout_audit";

// 布局审计（`i18n_layout.spec.ts` 用它扫全站）本身的自测。
//
// 为什么需要：审计里加了两条「有意伸出」的豁免（装饰层、负外边距出血），放宽之后如果没人验证它**还抓不抓得到
// 真问题**，它就从一道门变成了摆设。这里用合成页面把边界钉住：该放过的放过，该抓的仍然抓。
// 不依赖站点，纯浏览器内的几何计算，所以不需要起服务。

async function audit(page: Page, body: string) {
  await page.setViewportSize({ width: 800, height: 600 });
  await page.setContent(`<!doctype html><html><body style="margin:0"><main>${body}</main></body></html>`);
  return page.evaluate(AUDIT);
}

test.describe("布局审计：文本裁剪", () => {
  test("宿主自己的文字被 overflow:hidden 裁掉 —— 抓", async ({ page }) => {
    const r = await audit(
      page,
      `<div style="width:100px;overflow:hidden;white-space:nowrap">一段很长很长很长很长很长很长的文字</div>`,
    );
    expect(r.clipped).toHaveLength(1);
  });

  test("后代用普通宽度伸出宿主（没有外边距、不是装饰）—— 抓", async ({ page }) => {
    const r = await audit(
      page,
      `<div style="width:200px;overflow:hidden"><span style="display:block;width:260px">被裁掉的文字</span></div>`,
    );
    expect(r.clipped.length).toBeGreaterThanOrEqual(1);
  });

  test("负外边距出血（悬停底色比文字宽一圈）—— 放过", async ({ page }) => {
    // 与首页 `.md-link-row` 同构：`padding-inline: 8px` + `margin-inline: -8px`，盒子比宿主宽 16px。
    const r = await audit(
      page,
      `<div style="width:200px;display:grid"><a style="display:flex;padding-inline:8px;margin-inline:-8px">入口</a></div>`,
    );
    expect(r.clipped).toHaveLength(0);
  });

  test("出血：伸出量超过外边距的部分 —— 仍然抓", async ({ page }) => {
    // 外边距只给了 8px，却伸出 8 + 40px：多出来的 40px 是真的越界。
    const r = await audit(
      page,
      `<div style="width:200px;display:grid"><a style="display:block;margin-inline:-8px;width:256px">入口入口</a></div>`,
    );
    expect(r.clipped.length).toBeGreaterThanOrEqual(1);
  });

  test("出血元素内部的子元素 —— 跟着放过，不单独报", async ({ page }) => {
    const r = await audit(
      page,
      `<div style="width:200px;display:grid"><a style="display:flex;padding-inline:8px;margin-inline:-8px"><span>圆点</span><span>文字文字</span></a></div>`,
    );
    expect(r.clipped).toHaveLength(0);
  });

  test("aria-hidden 的装饰层伸出被裁剪的宿主 —— 放过", async ({ page }) => {
    // 与首页 hero 的 `.md-aurora` 同构：绝对定位、比宿主宽、`aria-hidden`、没有文字，宿主 `overflow:hidden` 裁住它。
    const r = await audit(
      page,
      `<section style="position:relative;width:300px;overflow:hidden">正文<div aria-hidden="true" style="position:absolute;left:-50px;width:500px;height:20px"></div></section>`,
    );
    expect(r.clipped).toHaveLength(0);
  });

  test("同样的绝对定位宽元素，没有 aria-hidden —— 抓", async ({ page }) => {
    const r = await audit(
      page,
      `<section style="position:relative;width:300px;overflow:hidden">正文<div style="position:absolute;left:-50px;width:500px;height:20px">别的内容</div></section>`,
    );
    expect(r.clipped.length).toBeGreaterThanOrEqual(1);
  });

  test('aria-hidden="false" 的宽元素对读屏是可见的，不是装饰 —— 抓', async ({ page }) => {
    const r = await audit(
      page,
      `<section style="position:relative;width:300px;overflow:hidden">正文<div aria-hidden="false" style="position:absolute;left:-50px;width:500px;height:20px">真实内容</div></section>`,
    );
    expect(r.clipped.length).toBeGreaterThanOrEqual(1);
  });

  test("装饰层豁免只在 aria-hidden 之内：祖先 aria-hidden 之外的兄弟照旧判定", async ({ page }) => {
    const r = await audit(
      page,
      `<section style="width:300px;overflow:hidden"><div aria-hidden="true" style="width:500px;height:10px"></div><p style="width:500px;margin:0">真实的正文</p></section>`,
    );
    expect(r.clipped.length).toBeGreaterThanOrEqual(1);
  });
});

test.describe("布局审计：未取整的浮点数", () => {
  test("普通位置出现 6 位以上小数 —— 抓", async ({ page }) => {
    const r = await audit(page, `<p>距离 12.3456789 km</p>`);
    expect(r.longDecimals).toHaveLength(1);
  });

  test("测验选项里的同样数字（题库原文）—— 放过", async ({ page }) => {
    const r = await audit(
      page,
      `<button class="md-quiz-opt"><span class="md-quiz-text">0.0000035×N（千瓦小时）</span></button>`,
    );
    expect(r.longDecimals).toHaveLength(0);
  });

  test("豁免只限测验选项：旁边普通文字里的同样数字仍然抓", async ({ page }) => {
    const r = await audit(
      page,
      `<button class="md-quiz-opt"><span>0.0000035</span></button><p>0.0000035</p>`,
    );
    expect(r.longDecimals).toHaveLength(1);
  });
});
