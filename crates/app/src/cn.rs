//! 类名合并：`cn(&[...])`，语义对齐 shadcn/ui 使用的 `clsx + tailwind-merge`。
//!
//! 后出现的类会覆盖同一「冲突组」（且变体前缀相同）中先出现的类，例如
//! `cn(&["px-4 h-9", "h-7 px-2"])` → `"h-7 px-2"`。只覆盖本项目实际用到的 Tailwind 工具类分组。

use std::collections::HashSet;

/// 合并多段类名字符串。
pub fn cn(parts: &[&str]) -> String {
  let tokens: Vec<&str> = parts.iter().flat_map(|p| p.split_whitespace()).collect();
  let mut seen: HashSet<String> = HashSet::new();
  let mut kept: Vec<&str> = Vec::with_capacity(tokens.len());

  for &token in tokens.iter().rev() {
    let (modifiers, base) = split_modifiers(token);
    let base = base.strip_prefix('!').unwrap_or(base);
    let Some(group) = class_group(base) else {
      kept.push(token);
      continue;
    };
    let key = format!("{modifiers}|{group}");
    if seen.contains(&key) {
      continue;
    }
    seen.insert(key);
    for c in conflicts(group) {
      seen.insert(format!("{modifiers}|{c}"));
    }
    kept.push(token);
  }
  kept.reverse();
  kept.dedup();
  kept.join(" ")
}

/// 拆分变体前缀（方括号内的 `:` 不作为分隔符）。
fn split_modifiers(token: &str) -> (&str, &str) {
  let mut depth = 0i32;
  let mut last = None;
  for (i, c) in token.char_indices() {
    match c {
      '[' | '(' => depth += 1,
      ']' | ')' => depth -= 1,
      ':' if depth == 0 => last = Some(i),
      _ => {}
    }
  }
  match last {
    Some(i) => (&token[..i], &token[i + 1..]),
    None => ("", token),
  }
}

fn conflicts(group: &str) -> &'static [&'static str] {
  match group {
    "p" => &["px", "py", "pt", "pr", "pb", "pl", "ps", "pe"],
    "px" => &["pl", "pr", "ps", "pe"],
    "py" => &["pt", "pb"],
    "m" => &["mx", "my", "mt", "mr", "mb", "ml", "ms", "me"],
    "mx" => &["ml", "mr", "ms", "me"],
    "my" => &["mt", "mb"],
    "size" => &["w", "h"],
    "gap" => &["gap-x", "gap-y"],
    "inset" => &["inset-x", "inset-y", "top", "right", "bottom", "left"],
    "inset-x" => &["left", "right"],
    "inset-y" => &["top", "bottom"],
    "border-w" => &[
      "border-w-x",
      "border-w-y",
      "border-w-t",
      "border-w-r",
      "border-w-b",
      "border-w-l",
    ],
    "rounded" => &["rounded-t", "rounded-r", "rounded-b", "rounded-l"],
    "overflow" => &["overflow-x", "overflow-y"],
    _ => &[],
  }
}

fn is_arbitrary(v: &str) -> bool {
  v.starts_with('[') && v.ends_with(']')
}

fn is_number_like(v: &str) -> bool {
  !v.is_empty()
    && v
      .chars()
      .all(|c| c.is_ascii_digit() || c == '.' || c == '/')
}

fn is_length_arbitrary(v: &str) -> bool {
  is_arbitrary(v) && {
    let inner = &v[1..v.len() - 1];
    ["px", "rem", "em", "%", "vh", "vw", "svh", "dvh", "ch"]
      .iter()
      .any(|u| inner.ends_with(u))
      || inner.starts_with("calc(")
      || inner.starts_with("length:")
  }
}

const TEXT_SIZES: &[&str] = &[
  "xs", "sm", "base", "lg", "xl", "2xl", "3xl", "4xl", "5xl", "6xl", "7xl", "8xl", "9xl",
];
const FONT_WEIGHTS: &[&str] = &[
  "thin",
  "extralight",
  "light",
  "normal",
  "medium",
  "semibold",
  "bold",
  "extrabold",
  "black",
];
const BORDER_STYLES: &[&str] = &["solid", "dashed", "dotted", "double", "hidden", "none"];

fn value_after<'a>(base: &'a str, prefix: &str) -> Option<&'a str> {
  if base == prefix {
    return Some("");
  }
  base.strip_prefix(prefix)?.strip_prefix('-')
}

/// 返回工具类所属的冲突组。
fn class_group(base: &str) -> Option<&'static str> {
  let base = base.strip_prefix('-').unwrap_or(base);
  let exact = match base {
    "block" | "inline-block" | "inline" | "flex" | "inline-flex" | "grid" | "inline-grid"
    | "contents" | "hidden" | "table" | "flow-root" | "list-item" => Some("display"),
    "static" | "fixed" | "absolute" | "relative" | "sticky" => Some("position"),
    "visible" | "invisible" | "collapse" => Some("visibility"),
    "truncate" | "text-ellipsis" | "text-clip" => Some("text-overflow"),
    "shrink" => Some("shrink"),
    "grow" => Some("grow"),
    "flex-row" | "flex-row-reverse" | "flex-col" | "flex-col-reverse" => Some("flex-direction"),
    "flex-wrap" | "flex-wrap-reverse" | "flex-nowrap" => Some("flex-wrap"),
    "outline-none" | "outline-hidden" | "outline-dashed" | "outline-dotted" | "outline-double" => {
      Some("outline-style")
    }
    "outline" => Some("outline-w"),
    "ring" => Some("ring-w"),
    "shadow" => Some("shadow"),
    "rounded" => Some("rounded"),
    "border" => Some("border-w"),
    "transition" => Some("transition"),
    "underline" | "no-underline" | "line-through" => Some("text-decoration"),
    "italic" | "not-italic" => Some("font-style"),
    "sr-only" | "not-sr-only" => Some("sr"),
    _ => None,
  };
  if exact.is_some() {
    return exact;
  }

  let prefixed: &[(&str, &'static str)] = &[
    ("px", "px"),
    ("py", "py"),
    ("pt", "pt"),
    ("pr", "pr"),
    ("pb", "pb"),
    ("pl", "pl"),
    ("ps", "ps"),
    ("pe", "pe"),
    ("p", "p"),
    ("mx", "mx"),
    ("my", "my"),
    ("mt", "mt"),
    ("mr", "mr"),
    ("mb", "mb"),
    ("ml", "ml"),
    ("ms", "ms"),
    ("me", "me"),
    ("m", "m"),
    ("min-w", "min-w"),
    ("max-w", "max-w"),
    ("min-h", "min-h"),
    ("max-h", "max-h"),
    ("w", "w"),
    ("h", "h"),
    ("size", "size"),
    ("gap-x", "gap-x"),
    ("gap-y", "gap-y"),
    ("gap", "gap"),
    ("space-x", "space-x"),
    ("space-y", "space-y"),
    ("inset-x", "inset-x"),
    ("inset-y", "inset-y"),
    ("inset", "inset"),
    ("top", "top"),
    ("right", "right"),
    ("bottom", "bottom"),
    ("left", "left"),
    ("z", "z"),
    ("opacity", "opacity"),
    ("cursor", "cursor"),
    ("items", "items"),
    ("justify", "justify"),
    ("self", "self"),
    ("content", "content"),
    ("overflow-x", "overflow-x"),
    ("overflow-y", "overflow-y"),
    ("overflow", "overflow"),
    ("leading", "leading"),
    ("tracking", "tracking"),
    ("whitespace", "whitespace"),
    ("duration", "duration"),
    ("ease", "ease"),
    ("delay", "delay"),
    ("animate", "animate"),
    ("translate-x", "translate-x"),
    ("translate-y", "translate-y"),
    ("rotate", "rotate"),
    ("scale", "scale"),
    ("grid-cols", "grid-cols"),
    ("grid-rows", "grid-rows"),
    ("col-span", "col-span"),
    ("col-start", "col-start"),
    ("row-span", "row-span"),
    ("row-start", "row-start"),
    ("auto-rows", "auto-rows"),
    ("object", "object"),
    ("aspect", "aspect"),
    ("select", "select"),
    ("pointer-events", "pointer-events"),
    ("underline-offset", "underline-offset"),
    ("line-clamp", "line-clamp"),
    ("ring-offset", "ring-offset"),
    ("fill", "fill"),
    ("stroke", "stroke"),
    ("basis", "basis"),
    ("order", "order"),
    ("backdrop-blur", "backdrop-blur"),
    ("rounded-t", "rounded-t"),
    ("rounded-r", "rounded-r"),
    ("rounded-b", "rounded-b"),
    ("rounded-l", "rounded-l"),
  ];
  for &(prefix, group) in prefixed {
    if value_after(base, prefix).is_some() {
      return Some(group);
    }
  }

  if let Some(v) = value_after(base, "flex") {
    return (!v.is_empty()).then_some("flex");
  }
  if let Some(v) = value_after(base, "shrink") {
    return (!v.is_empty()).then_some("shrink");
  }
  if value_after(base, "grow").is_some() {
    return Some("grow");
  }
  if value_after(base, "rounded").is_some() {
    return Some("rounded");
  }
  if value_after(base, "shadow").is_some() {
    return Some("shadow");
  }
  if value_after(base, "transition").is_some() {
    return Some("transition");
  }
  if let Some(v) = value_after(base, "text") {
    return Some(match v {
      "left" | "center" | "right" | "justify" | "start" | "end" => "text-align",
      v if TEXT_SIZES.contains(&v) || is_length_arbitrary(v) => "text-size",
      _ => "text-color",
    });
  }
  if let Some(v) = value_after(base, "font") {
    return Some(if FONT_WEIGHTS.contains(&v) {
      "font-weight"
    } else {
      "font-family"
    });
  }
  if let Some(v) = value_after(base, "bg") {
    return Some(match v {
      "fixed" | "local" | "scroll" => "bg-attachment",
      v if v.starts_with("clip") => "bg-clip",
      v if v.starts_with("gradient") || v.starts_with("linear") => "bg-image",
      _ => "bg-color",
    });
  }
  for (side, group) in [
    ("border-x", "border-w-x"),
    ("border-y", "border-w-y"),
    ("border-t", "border-w-t"),
    ("border-r", "border-w-r"),
    ("border-b", "border-w-b"),
    ("border-l", "border-w-l"),
  ] {
    if let Some(v) = value_after(base, side) {
      return Some(
        if v.is_empty() || is_number_like(v) || is_length_arbitrary(v) {
          group
        } else {
          "border-color-side"
        },
      );
    }
  }
  if let Some(v) = value_after(base, "border") {
    return Some(if is_number_like(v) || is_length_arbitrary(v) {
      "border-w"
    } else if BORDER_STYLES.contains(&v) {
      "border-style"
    } else {
      "border-color"
    });
  }
  if let Some(v) = value_after(base, "ring") {
    return Some(if is_number_like(v) || is_length_arbitrary(v) {
      "ring-w"
    } else {
      "ring-color"
    });
  }
  if let Some(v) = value_after(base, "outline") {
    return Some(if is_number_like(v) {
      "outline-w"
    } else {
      "outline-color"
    });
  }
  None
}

#[cfg(test)]
mod tests {
  use super::cn;

  #[test]
  fn later_classes_win_within_group() {
    assert_eq!(
      cn(&["h-8 px-3 rounded-md", "h-7 px-2"]),
      "rounded-md h-7 px-2"
    );
    assert_eq!(
      cn(&["inline-flex gap-2", "flex items-center gap-2"]),
      "flex items-center gap-2"
    );
    assert_eq!(
      cn(&["transition-all", "active:scale-[0.98] transition-transform"]),
      "active:scale-[0.98] transition-transform"
    );
    assert_eq!(
      cn(&["max-w-[calc(100%-2rem)] sm:max-w-lg", "sm:max-w-[520px]"]),
      "max-w-[calc(100%-2rem)] sm:max-w-[520px]"
    );
  }

  #[test]
  fn keeps_unrelated_and_variant_classes() {
    assert_eq!(
      cn(&["text-sm text-muted-foreground", "text-xs"]),
      "text-muted-foreground text-xs"
    );
    assert_eq!(
      cn(&["border bg-background", "border-green-200 bg-green-50"]),
      "border border-green-200 bg-green-50"
    );
    assert_eq!(cn(&["outline-none", "outline"]), "outline-none outline");
    assert_eq!(
      cn(&["hover:bg-accent", "hover:bg-green-100"]),
      "hover:bg-green-100"
    );
    assert_eq!(cn(&["p-4", "px-2"]), "p-4 px-2");
    assert_eq!(cn(&["px-2", "p-4"]), "p-4");
    assert_eq!(
      cn(&["[&_svg:not([class*='size-'])]:size-4", "size-9"]),
      "[&_svg:not([class*='size-'])]:size-4 size-9"
    );
  }
}
