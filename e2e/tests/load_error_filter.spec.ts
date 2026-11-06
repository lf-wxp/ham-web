import { expect, test } from "@playwright/test";
import { ignoredLoadError } from "./fixtures";

// `ignoredLoadError` 的自测：它放宽了「控制台报错算不算问题」这道门，必须钉住
// **该放过的放过、该抓的仍然抓**（约定见 docs/engineering/testing.md「写用例的约定」）。
// 纯函数，不用起服务。

const BASE = "http://127.0.0.1:4173";
const LOAD_FAILED = "Failed to load resource: net::ERR_CONNECTION_REFUSED";

test.describe("资源加载失败的过滤规则", () => {
  test("第三方脚本在无外网环境里加载失败 —— 放过", () => {
    // 页面里的 AdSense：地址是外域，与本站代码无关（本地 / CI 都没有外网）。
    expect(
      ignoredLoadError(
        LOAD_FAILED,
        "https://pagead2.googlesyndication.com/pagead/js/adsbygoogle.js",
        BASE,
      ),
    ).toBe(true);
  });

  test("本站静态资源加载失败 —— 抓", () => {
    expect(ignoredLoadError(LOAD_FAILED, `${BASE}/data/questions/A.json`, BASE)).toBe(false);
  });

  test("本站 /api 失败 —— 放过（降级由各页面自己兜底）", () => {
    // 后台轮询（卫星提醒、太阳活动）在测试里必然失败，不该让「页面没崩」这类用例红。
    expect(ignoredLoadError(LOAD_FAILED, `${BASE}/api/spots`, BASE)).toBe(true);
  });

  test("不是「资源加载失败」的报错 —— 不算放过，照旧算问题", () => {
    expect(ignoredLoadError("Uncaught TypeError: x is not a function", `${BASE}/app.js`, BASE)).toBe(
      false,
    );
  });
});
