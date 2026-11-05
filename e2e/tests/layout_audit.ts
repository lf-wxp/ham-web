// 页面内布局审计：`i18n_layout.spec.ts` 用它扫全站路由，`layout_audit.spec.ts` 用合成页面给它自测。
// 函数会被 Playwright 序列化后在浏览器里执行，所以必须自包含：不能引用本文件里的任何外部变量。

/**
 * 页面内布局审计：
 * 1. 文档横向滚动 —— 任何语言都不该出现（硬失败）
 * 2. 文本被裁掉 —— 自身内容宽于可视宽，且不是刻意省略（ellipsis / line-clamp）。
 *    溢出若**全部**由「有意的伸出」解释，也不算（见 `explainedByDeliberateOverflow`）：
 *    装饰层（`aria-hidden` 的氛围光、角标）与负外边距「出血」（悬停底色比文字宽一圈）。
 *    宿主自己的文字溢出、以及伸出量超过外边距的部分，照旧判定。
 * 3. 元素越过视口右边界 —— 排除地图 / 横向可滚容器
 * SVG 内部元素由 viewBox 决定坐标，不遵循 HTML 布局，仅统计不判定。
 */
export const AUDIT = () => {
  const doc = document.documentElement;
  const vw = window.innerWidth;
  const hScroll = Math.max(0, doc.scrollWidth - doc.clientWidth);
  const clipped: { sel: string; detail: string }[] = [];
  const overRight: { sel: string; detail: string }[] = [];
  const svgOver: string[] = [];

  const describe = (el: Element) => {
    const id = el.id ? `#${el.id}` : "";
    const cls = (el.getAttribute("class") || "").split(/\s+/).filter(Boolean).slice(0, 4).join(".");
    const text = (el.textContent || "").trim().replace(/\s+/g, " ").slice(0, 70);
    return `${el.tagName.toLowerCase()}${id}${cls ? "." + cls : ""} :: ${text}`;
  };

  // 横向可滚容器：`scrollWidth > clientWidth` 是它的正常工作状态，不是「文本被裁掉」
  const scrollableX = (el: Element) => {
    const cs = getComputedStyle(el);
    return cs.overflowX === "auto" || cs.overflowX === "scroll";
  };
  // 祖先是横向可滚容器时，后代超出视口右边界是合法的（宽表格本来就该横向滚），
  // 只判容器**自身**有没有越界 —— 否则 `min-w-[680px]` 这类表格会刷出几十条假阳性。
  const inScrollableX = (el: Element | null) => {
    for (let p = el; p; p = p.parentElement) {
      if (scrollableX(p)) return true;
    }
    return false;
  };

  // 溢出是否**完整地**由有意的伸出解释：宿主内至少有一个后代伸出了右缘，且每一个伸出的后代自己
  // （或它的某个祖先）是 ① `aria-hidden` 的装饰层，或 ② 用负的 `margin-right` 有意出血、
  // 且该祖先的伸出量不超过外边距（+1px 容差）。没有任何后代伸出去（说明是宿主自己的文字溢出）返回 false。
  const explainedByDeliberateOverflow = (host: Element) => {
    // `scrollWidth` 以 padding box 的右缘为界：border 之内才是可滚动区域。
    const edge = host.getBoundingClientRect().right - parseFloat(getComputedStyle(host).borderRightWidth);
    const explained = (d: Element) => {
      for (let n: Element | null = d; n && n !== host; n = n.parentElement) {
        if (n.getAttribute("aria-hidden") === "true") return true;
        const bleed = -parseFloat(getComputedStyle(n).marginRight);
        if (bleed > 0 && n.getBoundingClientRect().right - edge <= bleed + 1) return true;
      }
      return false;
    };
    let found = false;
    for (const d of Array.from(host.querySelectorAll("*"))) {
      const r = d.getBoundingClientRect();
      if (r.width === 0 || r.right - edge <= 2) continue; // 与 scrollWidth 判定同一个容差
      found = true;
      if (!explained(d)) return false;
    }
    return found;
  };

  // 视觉隐藏（`sr-only` / clip 技巧）的容器：自身被压到 1px 且溢出裁掉，
  // 里面的文字**对用户本来就不显示**，它的溢出不是「文本被裁掉」。
  // 典型就是 KaTeX 的 `.katex-mathml`（给读屏用的 MathML 副本，1px + `clip: rect(1px,1px,1px,1px)`）：
  // Chromium 会给里面那个 `<math>` 一个比内容窄的宽度，逐层报 `scrollWidth > clientWidth`。
  const inVisuallyHidden = (el: Element) => {
    for (let p = el.parentElement; p; p = p.parentElement) {
      if (p.clientWidth > 2 || p.clientHeight > 2) continue;
      const cs = getComputedStyle(p);
      if (cs.overflow !== "visible" || cs.clip !== "auto") return true;
    }
    return false;
  };

  for (const el of Array.from(document.querySelectorAll("main *"))) {
    if ((el as SVGElement).ownerSVGElement) {
      const r = el.getBoundingClientRect();
      if (r.width > 0 && r.right > vw + 1) svgOver.push(describe(el));
      continue;
    }
    const cs = getComputedStyle(el);
    if (cs.display === "none" || cs.visibility === "hidden") continue;
    const rect = el.getBoundingClientRect();
    if (rect.width === 0 || rect.height === 0) continue;
    if (!(el.textContent || "").trim()) continue;
    // sr-only / 1px 视觉隐藏元素（clip 技巧）不参与判定
    if (el.clientWidth <= 2 || el.clientHeight <= 2) continue;
    // 同上，但视觉隐藏发生在祖先上：容器被压到 1px 且裁掉溢出，后代再宽也看不见。
    if (inVisuallyHidden(el)) continue;

    // 刻意截断（ellipsis / line-clamp）属设计意图，不计入
    const lineClamp = (cs as unknown as { webkitLineClamp?: string }).webkitLineClamp;
    const intentional = cs.textOverflow === "ellipsis" || (!!lineClamp && lineClamp !== "none");

    if (
      !intentional &&
      !scrollableX(el) &&
      el.clientWidth > 0 &&
      el.scrollWidth > el.clientWidth + 2 &&
      !explainedByDeliberateOverflow(el)
    ) {
      clipped.push({
        sel: describe(el),
        detail: `scrollW=${el.scrollWidth} clientW=${el.clientWidth}`,
      });
    }

    if (
      rect.right > vw + 1 &&
      cs.position !== "fixed" &&
      cs.position !== "sticky" &&
      !inScrollableX(el.parentElement)
    ) {
      overRight.push({
        sel: describe(el),
        detail: `right=${Math.round(rect.right)} viewport=${vw}`,
      });
    }
  }
  // 未取整的浮点数（格式化丢失的典型特征）：文本里出现 6 位以上小数
  const longDecimals: string[] = [];
  for (const el of Array.from(document.querySelectorAll("main *"))) {
    if (el.children.length > 0) continue;
    // 测验选项是题库原文：`0.0000035×N（千瓦小时）` 就是那道题的选项，不是格式化丢失。
    // 而且测验每次加载随机抽题，不排除它会让这条断言随机失败。
    if (el.closest(".md-quiz-opt")) continue;
    const m = (el.textContent || "").trim().match(/\d+\.\d{6,}/);
    if (m) longDecimals.push(`${describe(el)} → ${m[0]}`);
  }

  return { hScroll, clipped, overRight, svgOver, longDecimals };
};
