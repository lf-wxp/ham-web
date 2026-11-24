import { expect, test, type Page } from "./fixtures";

/**
 * 数字模式时隙钟（`/ft8` 与 `/tools#mode-encoder`）。
 *
 * 用 `page.clock.setFixedTime` 把 `Date.now()` 钉在一个具体的 UTC 时刻，定时器照常真实运行
 * （组件每 100 ms 取一次时间），所以每次 tick 拿到的都是同一个值，显示是确定的；换一个固定时刻，
 * 下一次 tick（≤ 100 ms）就更新。期望值全部由时隙定义手算：FT8 15 s、FT4 7.5 s、WSPR 120 s，
 * 都与 UTC 整分钟对齐；2026-11-25T10:00:00Z 是整点也是偶数分钟，其 15 s 序号为偶数。
 */

/** 2026-11-25 10:00:00 UTC 之后 `seconds` 秒（允许小数）。 */
const at = (seconds: number) => new Date(Date.UTC(2026, 10, 25, 10, 0, 0) + seconds * 1000);

const row = (page: Page, key: "ft8" | "ft4" | "wspr") => page.locator(`[data-slot="slot-${key}"]`);

test("时隙钟：FT8 / FT4 按 UTC 对齐，显示本时隙起点、奇偶与剩余时间", async ({ page }) => {
  await page.clock.setFixedTime(at(7));
  await page.goto("/ft8");
  const clock = page.locator('[data-slot="slot-clock"]');
  await expect(clock).toBeVisible();

  // 10:00:07.000：FT8 在 10:00:00 起的偶数时隙里，还剩 8.0 s；FT4（7.5 s）同一个起点，还剩 0.5 s。
  await expect(clock.locator('[data-slot="slot-utc"]')).toHaveText("10:00:07 UTC");
  await expect(row(page, "ft8")).toContainText("距下一时隙 8.0 秒");
  await expect(row(page, "ft8")).toContainText("本时隙始于 10:00:00 UTC");
  await expect(row(page, "ft8")).toContainText("偶数时隙");
  await expect(row(page, "ft4")).toContainText("距下一时隙 0.5 秒");
  await expect(row(page, "ft4")).toContainText("本时隙始于 10:00:00 UTC");

  // 10:00:07.600：FT4 已进入 10:00:07.5 起的奇数时隙（起点落在半秒上，要带小数）；FT8 仍在原时隙。
  await page.clock.setFixedTime(at(7.6));
  await expect(row(page, "ft4")).toContainText("距下一时隙 7.4 秒");
  await expect(row(page, "ft4")).toContainText("本时隙始于 10:00:07.5 UTC");
  await expect(row(page, "ft4")).toContainText("奇数时隙");
  await expect(row(page, "ft8")).toContainText("距下一时隙 7.4 秒");
  await expect(row(page, "ft8")).toContainText("偶数时隙");

  // 差 1 ms 到边界：剩余时间向上取整成 0.1 s，不能显示 0.0（那会让人以为已经开始）。
  await page.clock.setFixedTime(at(14.999));
  await expect(row(page, "ft8")).toContainText("距下一时隙 0.1 秒");
  await expect(row(page, "ft8")).toContainText("本时隙始于 10:00:00 UTC");

  // 恰在边界：新时隙，剩余是整个周期 15.0 s（不是 0），序号变奇数。FT4 此刻回到偶数时隙。
  await page.clock.setFixedTime(at(15));
  await expect(row(page, "ft8")).toContainText("距下一时隙 15.0 秒");
  await expect(row(page, "ft8")).toContainText("本时隙始于 10:00:15 UTC");
  await expect(row(page, "ft8")).toContainText("奇数时隙");
  await expect(row(page, "ft4")).toContainText("本时隙始于 10:00:15 UTC");
  await expect(row(page, "ft4")).toContainText("偶数时隙");

  // 整分钟：:45 → 下一分钟的 :00 回绕，FT8 又是偶数时隙。
  await page.clock.setFixedTime(at(60));
  await expect(clock.locator('[data-slot="slot-utc"]')).toHaveText("10:01:00 UTC");
  await expect(row(page, "ft8")).toContainText("本时隙始于 10:01:00 UTC");
  await expect(row(page, "ft8")).toContainText("偶数时隙");

  // 进度条：时隙过半时约一半（取固定时刻 10:00:07.5 = FT8 的 50%）。
  await page.clock.setFixedTime(at(67.5));
  await expect
    .poll(async () => {
      const bar = row(page, "ft8").locator("div.bg-primary");
      return Math.round(parseFloat((await bar.getAttribute("style"))?.match(/width:\s*([\d.]+)%/)?.[1] ?? "-1"));
    })
    .toBe(50);
});

test("时隙钟：发端页多一行 WSPR（120 秒周期、偶数分钟起算、发射偏移 1 秒、不画奇偶）", async ({ page }) => {
  await page.clock.setFixedTime(at(7));
  await page.goto("/tools#mode-encoder");
  const section = page.locator("section#mode-encoder");
  await expect(section.locator('[data-slot="slot-clock"]')).toBeVisible();
  // 三个模式都在。
  for (const key of ["ft8", "ft4", "wspr"] as const) await expect(row(page, key)).toBeVisible();

  await expect(row(page, "wspr")).toContainText("距下一时隙 113.0 秒");
  await expect(row(page, "wspr")).toContainText("本时隙始于 10:00:00 UTC");
  await expect(row(page, "wspr")).toContainText("发射从周期的第 1 秒开始。");
  // WSPR 没有奇偶的通联约定，这一行不显示；FT8 / FT4 仍显示。
  await expect(row(page, "wspr")).not.toContainText(/偶数时隙|奇数时隙/);
  await expect(row(page, "ft8")).toContainText("偶数时隙");

  // 同一个 2 分钟周期内起点不变，周期结束才换：10:01:00 还在 10:00:00 起的周期里，10:02:00 是新周期。
  await page.clock.setFixedTime(at(60));
  await expect(row(page, "wspr")).toContainText("距下一时隙 60.0 秒");
  await expect(row(page, "wspr")).toContainText("本时隙始于 10:00:00 UTC");
  await page.clock.setFixedTime(at(120));
  await expect(row(page, "wspr")).toContainText("距下一时隙 120.0 秒");
  await expect(row(page, "wspr")).toContainText("本时隙始于 10:02:00 UTC");
});

test("时隙钟：英文 / 西语界面都有译文，不留中文", async ({ page }) => {
  for (const [code, next, odd] of [
    ["en", "Next slot in 8.0 s", "Even slot"],
    ["es", "Siguiente ranura en 8.0 s", "Ranura par"],
  ] as const) {
    await page.clock.setFixedTime(at(7));
    await page.addInitScript((c) => localStorage.setItem("locale", c), code);
    await page.goto("/ft8");
    await expect(row(page, "ft8")).toContainText(next);
    await expect(row(page, "ft8")).toContainText(odd);
    await expect(page.locator('[data-slot="slot-clock"]')).not.toContainText(/[\u4e00-\u9fff]/);
  }
});

/** 整页与时隙钟各自的横向溢出量（像素，≤ 1 视为没有溢出）。 */
async function overflowOf(page: Page) {
  return page.evaluate(() => {
    const el = document.querySelector('[data-slot="slot-clock"]') as HTMLElement;
    return {
      page: document.documentElement.scrollWidth - window.innerWidth,
      clock: el.scrollWidth - el.clientWidth,
    };
  });
}

test("时隙钟：溢出检测器自测——塞进 900px 宽的元素时必须量得出溢出", async ({ page }) => {
  await page.setViewportSize({ width: 320, height: 800 });
  await page.clock.setFixedTime(at(7));
  await page.goto("/ft8");
  await expect(page.locator('[data-slot="slot-clock"]')).toBeVisible();
  // 先确认基线没溢出，再注入：否则「测得出」可能只是基线就是坏的。
  const before = await overflowOf(page);
  expect(before.page).toBeLessThanOrEqual(1);
  expect(before.clock).toBeLessThanOrEqual(1);
  await page.evaluate(() => {
    const wide = document.createElement("div");
    wide.style.cssText = "width:900px;height:4px";
    document.querySelector('[data-slot="slot-clock"]')!.appendChild(wide);
  });
  const after = await overflowOf(page);
  expect(after.clock, "时隙钟里放 900px 宽的元素，量到的溢出应远大于 1").toBeGreaterThan(100);
  expect(after.page, "整页同理").toBeGreaterThan(100);
});

// 三语最长的文案（西语的说明与发射偏移）在 320 px 下不能把整页或时钟撑出横向滚动；
// 两套皮肤各量一次（现代风按 `E2E_UI_STYLE=modern` 的口径设成暗色主题）。
// 全站的溢出扫描会走到这两个路由，但这里钉住的是时隙钟本身，失败时一眼看出是它。
for (const lang of ["zh", "en", "es"]) {
  for (const style of ["pixel", "modern"]) {
    for (const route of ["/ft8", "/tools#mode-encoder"]) {
      test(`时隙钟：320px 下不横向溢出（${lang} · ${style} · ${route}）`, async ({ page }) => {
        await page.setViewportSize({ width: 320, height: 800 });
        await page.addInitScript(
          ([code, ui]) => {
            localStorage.setItem("locale", code);
            localStorage.setItem("ui:uiStyle", ui);
            if (ui === "modern") localStorage.setItem("theme", "dark");
          },
          [lang, style],
        );
        await page.clock.setFixedTime(at(7.6));
        await page.goto(route);
        await expect(page.locator('[data-slot="slot-clock"]').first()).toBeVisible();
        const overflow = await overflowOf(page);
        expect(overflow.page, "整页横向溢出").toBeLessThanOrEqual(1);
        expect(overflow.clock, "时隙钟自身横向溢出").toBeLessThanOrEqual(1);
      });
    }
  }
}
