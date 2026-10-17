import { readFileSync } from "node:fs";
import { join } from "node:path";
import { expect, pickOption, test } from "./fixtures";

const SAMPLES = join(__dirname, "../../crates/ft8/samples");

/**
 * 真实录音端到端：上传 `crates/ft8/samples/ft8_20m_busy.wav`（ft8_lib MIT 的真实 20m
 * 空中录音），走完整浏览器链路（统一 Worker → wasm 解码 → 消息列表）。
 *
 * 为什么必须有这一条：FT8 的弱信号特性决定了合成信号（无噪声、无频偏、无多径）解码总绿，
 * 但对真实空中信号可能解不出；真实录音才能钉住同步 / Costas / LDPC / CRC 在真实信号上的行为。
 */
test("FT8 解码：真实 20m 录音解出黄金标准消息", async ({ page }) => {
  await page.goto("/ft8-decode");
  await expect(page.getByRole("heading", { name: "FT8 / FT4 解码器" })).toBeVisible();

  await page.setInputFiles("#ft8-file", {
    name: "ft8_20m_busy.wav",
    mimeType: "audio/wav",
    buffer: readFileSync(join(SAMPLES, "ft8_20m_busy.wav")),
  });

  // wasm 首次加载 + FT8 全链路解码（93 次 3840 点 FFT + 候选打分 + LDPC）较慢，放宽到 90s。
  await expect(page.getByText("CQ OK6LZ JN99")).toBeVisible({ timeout: 90_000 });
  await expect(page.getByText("CQ IK4LZH JN54")).toBeVisible();
  await expect(page.getByText("CQ R8AU MO05")).toBeVisible();
  await expect(page.getByRole("button", { name: "解码其他文件" })).toBeVisible();
});

/**
 * 把单时隙样例首尾相接成 N 个时隙的 WAV：标准 44 字节头，只需改 RIFF 长度（偏移 4）与 data 长度（偏移 40）。
 */
function repeatSlots(wav: Buffer, times: number): Buffer {
  const header = Buffer.from(wav.subarray(0, 44));
  const data = wav.subarray(44);
  const total = data.length * times;
  header.writeUInt32LE(36 + total, 4);
  header.writeUInt32LE(total, 40);
  return Buffer.concat([header, ...Array.from({ length: times }, () => data)]);
}

test("FT8 解码：超过一个时隙的录音逐时隙解码，同一条消息在每个时隙各列一次", async ({ page }) => {
  await page.goto("/ft8-decode");
  await expect(page.getByRole("heading", { name: "FT8 / FT4 解码器" })).toBeVisible();

  await page.setInputFiles("#ft8-file", {
    name: "ft8_two_slots.wav",
    mimeType: "audio/wav",
    buffer: repeatSlots(readFileSync(join(SAMPLES, "ft8_20m_busy.wav")), 2),
  });

  // 以前第一个时隙收满就直接丢掉后面的部分；现在两个时隙都要解，并提示时隙数。
  await expect(page.getByText(/共 2 个时隙/)).toBeVisible({ timeout: 120_000 });
  await expect(page.getByText("CQ OK6LZ JN99")).toHaveCount(2);
  // 不再把「同步强度」冒充成 SNR。
  await expect(page.getByText(/同步强度/).first()).toBeVisible();
  await expect(page.getByText(/· SNR /)).toHaveCount(0);
});

test("FT8 解码：协议选 FT4 也能完整跑完（同一录音在 FT4 下可能解不出，但不报错）", async ({ page }) => {
  await page.goto("/ft8-decode");
  await expect(page.getByRole("heading", { name: "FT8 / FT4 解码器" })).toBeVisible();

  await pickOption(page, "协议", "FT4");
  await page.setInputFiles("#ft8-file", {
    name: "ft8_20m_busy.wav",
    mimeType: "audio/wav",
    buffer: readFileSync(join(SAMPLES, "ft8_20m_busy.wav")),
  });

  // FT4 解 FT8 录音不报错，走完整链路：结果元信息显示「解码到 0 条消息」，并提示「未解码到消息」。
  await expect(page.getByText(/解码到 0 条消息/)).toBeVisible({ timeout: 90_000 });
  await expect(page.getByText(/未解码到消息/)).toBeVisible();
});
