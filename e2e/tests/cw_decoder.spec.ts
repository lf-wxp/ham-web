import { expect, test } from "./fixtures";

test("CW 解码：模拟麦克风输入 600 Hz 电码并还原文字", async ({ page }) => {
  // 用振荡器 + 增益包络按标准时值发 “CQ CQ DE BG4XXX”，冒充麦克风输入
  await page.addInitScript(() => {
    const CODE: Record<string, string> = {
      C: "-.-.", Q: "--.-", D: "-..", E: ".", B: "-...", G: "--.", "4": "....-", X: "-..-",
    };
    navigator.mediaDevices.getUserMedia = async () => {
      const ctx = new AudioContext();
      await ctx.resume();
      const osc = ctx.createOscillator();
      osc.frequency.value = 600;
      const gain = ctx.createGain();
      gain.gain.value = 0;
      const dest = ctx.createMediaStreamDestination();
      osc.connect(gain).connect(dest);
      const dit = 0.06;
      let t = ctx.currentTime + 0.5;
      const key = (on: number) => {
        gain.gain.setTargetAtTime(on ? 0.5 : 0, t, 0.002);
      };
      for (const word of "CQ CQ DE BG4XXX".split(" ")) {
        for (const ch of word) {
          for (const s of CODE[ch]) {
            key(1);
            t += s === "." ? dit : dit * 3;
            key(0);
            t += dit;
          }
          t += dit * 2;
        }
        t += dit * 4;
      }
      osc.start();
      return dest.stream;
    };
  });

  await page.goto("/morse");
  const section = page.getByRole("region", { name: "CW 解码（麦克风）" });
  await section.getByRole("button", { name: "开始监听" }).click();
  await expect(section.getByRole("button", { name: "停止" })).toBeVisible();

  const log = section.getByRole("log", { name: "解码结果" });
  await expect(log).toContainText("CQ DE BG4XXX", { timeout: 20_000 });
  await expect(section.getByTestId("cw-wpm")).toHaveText(/速度 ≈ (1[7-9]|2[0-3]) WPM/);

  await section.getByRole("button", { name: "停止" }).click();
  await section.getByRole("button", { name: "清空" }).click();
  await expect(log).toHaveText("解码结果会显示在这里");
});
