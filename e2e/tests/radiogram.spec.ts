import { expect, pickOption, test } from "./fixtures";

test("NTS Radiogram 生成：填表生成报文并自动计算检查", async ({ page }) => {
  await page.goto("/tools#radiogram");
  await expect(page.locator("#rg-number")).toBeVisible();

  await page.locator("#rg-number").fill("12");
  await page.locator("#rg-station").fill("W1AW");
  await page.locator("#rg-place").fill("NEWINGTON CT");
  await page.locator("#rg-date2").fill("JAN 15");
  await page.locator("#rg-addressee").fill("JOHN SMITH");
  await page.locator("#rg-address").fill("123 MAIN ST");
  await page.locator("#rg-body").fill("HAVE FUN AT THE MEETING\n73");
  await page.locator("#rg-signature").fill("ARRL HEADQUARTERS");

  // 检查自动计算：正文 "HAVE FUN AT THE MEETING"(5) + "73"(1) = 6。
  await expect(page.getByText("检查（正文词 / 组数）：6")).toBeVisible();
  // 报头行：编号 优先级 台站 检查 地点 日期。
  await expect(page.getByText("12 R W1AW 6 NEWINGTON CT JAN 15")).toBeVisible();
  // 正文夹在两个 BT 之间、签名在第二个 BT 之后；句号才是字母 X，不是行前缀，签名也没有 `=` 前缀。
  const pre = page.locator("section#radiogram pre");
  await expect(pre).toContainText("BT\nHAVE FUN AT THE MEETING\n73\nBT\nARRL HEADQUARTERS", {
    useInnerText: true,
  });
  await expect(pre).not.toContainText("X HAVE FUN");
  await expect(pre).not.toContainText("= ARRL");
});

test("NTS Radiogram 生成：紧急优先级在报头里拼写全称 EMERGENCY", async ({ page }) => {
  await page.goto("/tools#radiogram");
  await expect(page.locator("#rg-number")).toBeVisible();

  await page.locator("#rg-number").fill("3");
  await page.locator("#rg-station").fill("W1AW");
  await page.locator("#rg-place").fill("NEWINGTON CT");
  await page.locator("#rg-date2").fill("JAN 15");
  await page.locator("#rg-addressee").fill("JOHN SMITH");
  await page.locator("#rg-address").fill("123 MAIN ST");
  await page.locator("#rg-body").fill("SEND HELP");
  await page.locator("#rg-signature").fill("JANE");
  await pickOption(page, "优先级", "紧急 EMERGENCY");

  // ARRL FSD-3：紧急永远拼写全称，不写缩写 E。
  await expect(page.locator("section#radiogram pre")).toContainText(
    "3 EMERGENCY W1AW 2 NEWINGTON CT JAN 15",
  );
});

test("ICS-213 生成：切换到 ICS-213 填表生成字段式报文", async ({ page }) => {
  await page.goto("/tools#radiogram");
  await page.locator("#radiogram-format").click();
  await page.getByRole("option", { name: "ICS-213" }).click();

  await page.locator("#rg-incident").fill("FLOOD");
  await page.locator("#rg-to").fill("NET CONTROL");
  await page.locator("#rg-from").fill("FIELD STATION 2");
  await page.locator("#rg-subject").fill("WATER LEVEL");
  await page.locator("#rg-date").fill("2026-10-17");
  await page.locator("#rg-message").fill("Water rising fast.");
  await page.locator("#rg-approved").fill("INCIDENT COMMANDER");

  await expect(page.getByText("TO: NET CONTROL")).toBeVisible();
  await expect(page.getByText("FROM: FIELD STATION 2")).toBeVisible();
  await expect(page.getByText("SUBJECT: WATER LEVEL")).toBeVisible();
  await expect(page.getByText("APPROVED BY: INCIDENT COMMANDER")).toBeVisible();
});

test("ICS-213 生成：缺必填字段时不输出报文、复制按钮禁用", async ({ page }) => {
  await page.goto("/tools#radiogram");
  await page.locator("#radiogram-format").click();
  await page.getByRole("option", { name: "ICS-213" }).click();

  // 只填主题与正文，缺收件人 / 发件人 / 日期。
  await page.locator("#rg-subject").fill("WATER LEVEL");
  await page.locator("#rg-message").fill("Water rising fast.");

  await expect(page.getByText(/请先填写必填字段/)).toBeVisible();
  await expect(page.getByText("SUBJECT: WATER LEVEL")).toHaveCount(0);
  await expect(page.getByRole("button", { name: "复制" })).toBeDisabled();
});

test("专题页：NTS Radiogram / ICS-213 有核心概念、优先级与处理指令", async ({ page }) => {
  await page.goto("/radiogram");
  await expect(page.getByRole("heading", { name: "NTS Radiogram / ICS-213 报文" })).toBeVisible();
  await expect(page.getByText("报头 Preamble")).toBeVisible();
  await expect(page.getByText("检查 Check")).toBeVisible();
  // 优先级表：四档缩写（精确匹配，避免命中概念描述里「紧急 EMERGENCY / 优先 P …」那一句）。
  await expect(page.getByText("紧急 EMERGENCY", { exact: true })).toBeVisible();
  await expect(page.getByText("常规 ROUTINE")).toBeVisible();
  // 正文夹在两个 BT 之间；句号写 X、X 算一组且不能是最后一组。
  await expect(page.getByText(/正文前后各有一个 BT/)).toBeVisible();
  await expect(page.getByText(/句号写成字母 X/)).toBeVisible();
  // 处理指令 HX 表（精确匹配，避免命中概念描述里的 "HXA–HXG"）。
  await expect(page.getByText("HXA", { exact: true })).toBeVisible();
  await expect(page.getByText("HXG", { exact: true })).toBeVisible();
});
