//! 由 `cargo make pixel-sprites`（`crates/tools/src/pixel/sprites.rs`）生成，请勿手改。
//!
//! 图标来源：pixelarticons（MIT License，Copyright (c) 2019 Gerrit Halfmann，
//! https://github.com/halfmage/pixelarticons）；其余图标与全部精灵为本项目手绘。

/// 一个调色板色块：颜色与它覆盖的像素路径。
pub struct SpriteLayer {
  pub fill: &'static str,
  pub d: &'static str,
}

/// 一张精灵：边长（像素格数）与分色图层。
pub struct SpriteData {
  pub size: u8,
  pub layers: &'static [SpriteLayer],
}

const HERO_LAYERS: &[SpriteLayer] = &[
  SpriteLayer {
    fill: "var(--px-sprite-ink,#1a1c2c)",
    d: r#"M5 1h6v1h-6zM4 2h1v1h-1zM11 2h1v1h-1zM3 3h1v1h-1zM12 3h1v1h-1zM2 4h3v1h-3zM11 4h3v1h-3zM2 5h1v2h-1zM4 5h1v2h-1zM6 5h1v1h-1zM9 5h1v1h-1zM11 5h1v2h-1zM13 5h1v2h-1zM2 7h3v1h-3zM11 7h3v1h-3zM4 8h2v1h-2zM10 8h2v1h-2zM3 9h1v1h-1zM12 9h1v1h-1zM2 10h1v2h-1zM13 10h1v2h-1zM4 11h1v1h-1zM11 11h1v1h-1zM3 12h1v1h-1zM5 12h1v1h-1zM10 12h1v1h-1zM12 12h1v1h-1zM4 13h1v1h-1zM7 13h2v1h-2zM11 13h1v1h-1zM4 14h3v1h-3zM9 14h3v1h-3z"#,
  },
  SpriteLayer {
    fill: "#3b5dc9",
    d: r#"M5 2h6v1h-6zM4 3h8v1h-8zM5 13h2v1h-2zM9 13h2v1h-2z"#,
  },
  SpriteLayer {
    fill: "#f4c9a0",
    d: r#"M5 4h6v1h-6zM5 5h1v1h-1zM7 5h2v1h-2zM10 5h1v1h-1zM5 6h6v1h-6zM5 7h2v1h-2zM9 7h2v1h-2zM6 8h4v1h-4zM3 11h1v1h-1zM12 11h1v1h-1z"#,
  },
  SpriteLayer {
    fill: "#94b0c2",
    d: r#"M3 5h1v2h-1zM12 5h1v2h-1z"#,
  },
  SpriteLayer {
    fill: "#b13e53",
    d: r#"M7 7h2v1h-2z"#,
  },
  SpriteLayer {
    fill: "#41a6f6",
    d: r#"M4 9h8v1h-8zM3 10h2v1h-2zM6 10h4v1h-4zM11 10h2v1h-2zM5 11h6v1h-6zM6 12h4v1h-4z"#,
  },
  SpriteLayer {
    fill: "#f4f4f4",
    d: r#"M5 10h1v1h-1zM10 10h1v1h-1z"#,
  },
];
pub const HERO: SpriteData = SpriteData {
  size: 16,
  layers: HERO_LAYERS,
};

const MON_STATIC_LAYERS: &[SpriteLayer] = &[
  SpriteLayer {
    fill: "var(--px-sprite-ink,#1a1c2c)",
    d: r#"M4 1h1v1h-1zM7 1h2v2h-2zM11 1h1v1h-1zM3 2h1v1h-1zM5 2h1v1h-1zM10 2h1v1h-1zM12 2h1v1h-1zM2 3h1v1h-1zM6 3h1v1h-1zM9 3h1v1h-1zM13 3h1v7h-1zM1 4h1v4h-1zM5 5h1v1h-1zM9 5h1v1h-1zM4 6h2v1h-2zM9 6h2v1h-2zM2 8h1v2h-1zM7 9h2v1h-2zM3 10h1v1h-1zM12 10h1v1h-1zM2 11h1v1h-1zM4 11h2v1h-2zM9 11h3v1h-3zM13 11h1v1h-1zM1 12h1v1h-1zM4 12h1v1h-1zM6 12h4v1h-4zM11 12h1v1h-1zM14 12h1v1h-1z"#,
  },
  SpriteLayer {
    fill: "#94b0c2",
    d: r#"M4 2h1v1h-1zM11 2h1v1h-1zM3 3h3v1h-3zM7 3h2v1h-2zM10 3h3v1h-3zM2 4h11v1h-11zM2 5h2v2h-2zM6 5h3v2h-3zM11 5h2v2h-2zM2 7h11v1h-11zM3 8h2v1h-2zM11 8h2v1h-2zM3 9h3v1h-3zM10 9h3v1h-3zM4 10h3v1h-3zM9 10h3v1h-3zM6 11h3v1h-3z"#,
  },
  SpriteLayer {
    fill: "#f4f4f4",
    d: r#"M4 5h1v1h-1zM10 5h1v1h-1z"#,
  },
  SpriteLayer {
    fill: "#b13e53",
    d: r#"M5 8h6v1h-6zM6 9h1v1h-1zM9 9h1v1h-1zM7 10h2v1h-2z"#,
  },
];
pub const MON_STATIC: SpriteData = SpriteData {
  size: 16,
  layers: MON_STATIC_LAYERS,
};

const MON_SWR_LAYERS: &[SpriteLayer] = &[
  SpriteLayer {
    fill: "var(--px-sprite-ink,#1a1c2c)",
    d: r#"M2 1h2v1h-2zM12 1h2v1h-2zM1 2h1v1h-1zM4 2h1v1h-1zM7 2h2v1h-2zM11 2h1v1h-1zM14 2h1v1h-1zM0 3h1v2h-1zM5 3h2v1h-2zM9 3h2v1h-2zM15 3h1v2h-1zM1 5h1v1h-1zM5 5h2v1h-2zM9 5h2v1h-2zM14 5h1v1h-1zM2 6h3v1h-3zM6 6h1v5h-1zM9 6h1v5h-1zM11 6h3v1h-3zM2 9h2v1h-2zM12 9h2v1h-2zM1 10h1v1h-1zM4 10h1v1h-1zM11 10h1v1h-1zM14 10h1v1h-1zM0 11h1v2h-1zM5 11h2v1h-2zM9 11h2v1h-2zM15 11h1v2h-1zM1 13h1v1h-1zM5 13h6v1h-6zM14 13h1v1h-1zM2 14h3v1h-3zM11 14h3v1h-3z"#,
  },
  SpriteLayer {
    fill: "#b13e53",
    d: r#"M2 2h2v1h-2zM12 2h2v1h-2zM1 3h1v2h-1zM4 3h1v1h-1zM7 3h2v1h-2zM11 3h1v1h-1zM14 3h1v2h-1zM4 4h8v1h-8zM2 5h3v1h-3zM7 5h2v7h-2zM11 5h3v1h-3zM2 10h2v1h-2zM12 10h2v1h-2zM1 11h1v2h-1zM4 11h1v1h-1zM11 11h1v1h-1zM14 11h1v2h-1zM4 12h8v1h-8zM2 13h3v1h-3zM11 13h3v1h-3z"#,
  },
  SpriteLayer {
    fill: "#ef7d57",
    d: r#"M2 3h2v1h-2zM12 3h2v1h-2zM2 4h1v1h-1zM13 4h1v1h-1zM2 11h2v2h-2zM12 11h2v2h-2z"#,
  },
  SpriteLayer {
    fill: "#f4f4f4",
    d: r#"M3 4h1v1h-1zM12 4h1v1h-1z"#,
  },
];
pub const MON_SWR: SpriteData = SpriteData {
  size: 16,
  layers: MON_SWR_LAYERS,
};

const MON_NOISE_LAYERS: &[SpriteLayer] = &[
  SpriteLayer {
    fill: "var(--px-sprite-ink,#1a1c2c)",
    d: r#"M6 2h4v1h-4zM4 3h2v1h-2zM10 3h2v1h-2zM3 4h1v1h-1zM12 4h1v1h-1zM2 5h1v2h-1zM13 5h1v2h-1zM1 7h1v5h-1zM5 7h1v1h-1zM10 7h1v1h-1zM14 7h1v5h-1zM4 8h2v1h-2zM10 8h2v1h-2zM6 10h4v1h-4zM2 12h1v1h-1zM13 12h1v1h-1zM3 13h2v1h-2zM11 13h2v1h-2zM5 14h6v1h-6z"#,
  },
  SpriteLayer {
    fill: "#38b764",
    d: r#"M6 3h4v1h-4zM4 4h2v1h-2zM7 4h5v1h-5zM3 5h2v1h-2zM6 5h4v1h-4zM11 5h2v1h-2zM3 6h10v1h-10zM2 7h2v2h-2zM6 7h4v2h-4zM12 7h2v2h-2zM2 9h12v1h-12zM2 10h4v1h-4zM10 10h4v1h-4zM2 11h12v1h-12zM3 12h10v1h-10zM5 13h6v1h-6z"#,
  },
  SpriteLayer {
    fill: "#a7f070",
    d: r#"M6 4h1v1h-1zM5 5h1v1h-1z"#,
  },
  SpriteLayer {
    fill: "#f4f4f4",
    d: r#"M10 5h1v1h-1zM4 7h1v1h-1zM11 7h1v1h-1z"#,
  },
];
pub const MON_NOISE: SpriteData = SpriteData {
  size: 16,
  layers: MON_NOISE_LAYERS,
};

const MON_SPUR_LAYERS: &[SpriteLayer] = &[
  SpriteLayer {
    fill: "var(--px-sprite-ink,#1a1c2c)",
    d: r#"M0 1h1v1h-1zM13 1h1v1h-1zM0 2h2v1h-2zM12 2h2v1h-2zM0 3h1v3h-1zM2 3h1v1h-1zM5 3h1v1h-1zM8 3h1v1h-1zM11 3h1v1h-1zM13 3h1v3h-1zM3 4h2v1h-2zM6 4h2v1h-2zM9 4h2v1h-2zM1 6h1v2h-1zM4 6h1v1h-1zM9 6h1v1h-1zM12 6h1v2h-1zM2 8h1v1h-1zM11 8h1v1h-1zM3 9h1v1h-1zM6 9h2v1h-2zM10 9h1v1h-1zM4 10h2v1h-2zM8 10h2v1h-2zM5 11h4v1h-4z"#,
  },
  SpriteLayer {
    fill: "#5d275d",
    d: r#"M1 3h1v1h-1zM12 3h1v1h-1zM1 4h2v1h-2zM5 4h1v1h-1zM8 4h1v1h-1zM11 4h2v1h-2zM1 5h12v1h-12zM2 6h2v1h-2zM6 6h2v1h-2zM10 6h2v1h-2zM2 7h10v1h-10zM3 8h3v1h-3zM8 8h3v1h-3zM4 9h2v1h-2zM8 9h2v1h-2zM6 10h2v1h-2z"#,
  },
  SpriteLayer {
    fill: "#f4f4f4",
    d: r#"M5 6h1v1h-1zM8 6h1v1h-1z"#,
  },
  SpriteLayer {
    fill: "#b13e53",
    d: r#"M6 8h2v1h-2z"#,
  },
];
pub const MON_SPUR: SpriteData = SpriteData {
  size: 16,
  layers: MON_SPUR_LAYERS,
};

const MON_FADE_LAYERS: &[SpriteLayer] = &[
  SpriteLayer {
    fill: "var(--px-sprite-ink,#1a1c2c)",
    d: r#"M4 1h6v1h-6zM3 2h1v1h-1zM10 2h1v1h-1zM2 3h1v9h-1zM11 3h1v8h-1zM4 4h2v1h-2zM8 4h2v1h-2zM4 5h1v1h-1zM8 5h1v1h-1zM6 7h2v1h-2zM4 11h1v1h-1zM7 11h1v1h-1zM10 11h2v1h-2zM2 12h2v1h-2zM5 12h3v1h-3zM9 12h2v1h-2z"#,
  },
  SpriteLayer {
    fill: "#f4f4f4",
    d: r#"M4 2h6v1h-6zM3 3h8v1h-8zM3 4h1v2h-1zM6 4h2v2h-2zM10 4h1v2h-1zM3 6h8v1h-8zM3 7h3v1h-3zM8 7h3v1h-3zM3 8h8v3h-8zM3 11h1v1h-1zM5 11h2v1h-2zM8 11h2v1h-2z"#,
  },
  SpriteLayer {
    fill: "#3b5dc9",
    d: r#"M5 5h1v1h-1zM9 5h1v1h-1z"#,
  },
];
pub const MON_FADE: SpriteData = SpriteData {
  size: 16,
  layers: MON_FADE_LAYERS,
};

const BOSS_LAYERS: &[SpriteLayer] = &[
  SpriteLayer {
    fill: "var(--px-sprite-ink,#1a1c2c)",
    d: r#"M2 0h1v1h-1zM13 0h1v1h-1zM1 1h1v2h-1zM3 1h1v1h-1zM12 1h1v1h-1zM14 1h1v2h-1zM4 2h1v1h-1zM11 2h1v1h-1zM2 3h1v2h-1zM5 3h6v1h-6zM13 3h1v2h-1zM1 5h1v6h-1zM14 5h1v6h-1zM6 6h1v1h-1zM9 6h1v1h-1zM5 7h2v1h-2zM9 7h2v1h-2zM4 9h1v2h-1zM11 9h1v1h-1zM6 10h1v1h-1zM8 10h1v1h-1zM10 10h1v1h-1zM12 10h1v1h-1zM2 11h1v1h-1zM5 11h6v1h-6zM13 11h1v1h-1zM3 12h1v1h-1zM12 12h1v1h-1zM2 13h3v1h-3zM11 13h3v1h-3zM1 14h1v1h-1zM4 14h2v1h-2zM10 14h2v1h-2zM14 14h1v1h-1zM2 15h3v1h-3zM6 15h4v1h-4zM11 15h3v1h-3z"#,
  },
  SpriteLayer {
    fill: "#b13e53",
    d: r#"M2 1h1v1h-1zM13 1h1v1h-1zM2 2h2v1h-2zM12 2h2v1h-2zM3 3h2v1h-2zM11 3h2v1h-2zM3 4h10v1h-10zM2 5h12v1h-12zM2 6h2v2h-2zM7 6h2v2h-2zM12 6h2v2h-2zM2 8h12v1h-12zM2 9h2v2h-2zM5 9h6v1h-6zM12 9h2v1h-2zM13 10h1v1h-1zM3 11h2v1h-2zM11 11h2v1h-2zM4 12h8v1h-8zM5 13h6v1h-6zM6 14h4v1h-4z"#,
  },
  SpriteLayer {
    fill: "#f4f4f4",
    d: r#"M4 6h2v1h-2zM10 6h2v1h-2zM4 7h1v1h-1zM11 7h1v1h-1zM5 10h1v1h-1zM7 10h1v1h-1zM9 10h1v1h-1zM11 10h1v1h-1z"#,
  },
  SpriteLayer {
    fill: "#ffcd75",
    d: r#"M2 14h2v1h-2zM12 14h2v1h-2z"#,
  },
];
pub const BOSS: SpriteData = SpriteData {
  size: 16,
  layers: BOSS_LAYERS,
};

const BADGE_STAR_LAYERS: &[SpriteLayer] = &[
  SpriteLayer {
    fill: "var(--px-sprite-ink,#1a1c2c)",
    d: r#"M7 1h2v2h-2zM6 3h1v1h-1zM9 3h1v1h-1zM0 4h7v1h-7zM9 4h7v1h-7zM1 5h1v1h-1zM14 5h1v1h-1zM2 6h1v1h-1zM13 6h1v1h-1zM3 7h1v2h-1zM12 7h1v2h-1zM2 9h1v2h-1zM13 9h1v2h-1zM6 10h4v1h-4zM1 11h1v2h-1zM5 11h1v1h-1zM10 11h1v1h-1zM14 11h1v2h-1zM4 12h1v1h-1zM11 12h1v1h-1zM2 13h2v1h-2zM12 13h2v1h-2z"#,
  },
  SpriteLayer {
    fill: "#ffcd75",
    d: r#"M7 3h2v2h-2zM2 5h12v1h-12zM3 6h10v1h-10zM4 7h2v1h-2zM7 7h5v1h-5zM4 8h8v1h-8zM3 9h10v1h-10zM3 10h3v1h-3zM10 10h3v1h-3zM2 11h3v1h-3zM11 11h3v1h-3zM2 12h2v1h-2zM12 12h2v1h-2z"#,
  },
  SpriteLayer {
    fill: "#f4f4f4",
    d: r#"M6 7h1v1h-1z"#,
  },
];
pub const BADGE_STAR: SpriteData = SpriteData {
  size: 16,
  layers: BADGE_STAR_LAYERS,
};

const BADGE_MEDAL_LAYERS: &[SpriteLayer] = &[
  SpriteLayer {
    fill: "var(--px-sprite-ink,#1a1c2c)",
    d: r#"M3 1h1v2h-1zM6 1h1v2h-1zM9 1h1v2h-1zM12 1h1v2h-1zM4 3h1v1h-1zM7 3h2v1h-2zM11 3h1v1h-1zM5 4h1v1h-1zM10 4h1v1h-1zM6 5h4v1h-4zM4 6h2v1h-2zM10 6h2v1h-2zM3 7h1v1h-1zM12 7h1v1h-1zM2 8h1v4h-1zM13 8h1v4h-1zM8 9h2v1h-2zM7 10h1v1h-1zM10 10h1v2h-1zM8 11h1v1h-1zM3 12h1v1h-1zM12 12h1v1h-1zM4 13h2v1h-2zM10 13h2v1h-2zM6 14h4v1h-4z"#,
  },
  SpriteLayer {
    fill: "#3b5dc9",
    d: r#"M4 1h2v2h-2zM5 3h2v1h-2zM6 4h2v1h-2z"#,
  },
  SpriteLayer {
    fill: "#b13e53",
    d: r#"M10 1h2v2h-2zM9 3h2v1h-2zM8 4h2v1h-2z"#,
  },
  SpriteLayer {
    fill: "#ffcd75",
    d: r#"M6 6h4v1h-4zM4 7h2v1h-2zM8 7h4v1h-4zM3 8h2v2h-2zM6 8h7v1h-7zM6 9h2v1h-2zM10 9h3v1h-3zM3 10h4v1h-4zM8 10h2v1h-2zM11 10h2v2h-2zM3 11h5v1h-5zM9 11h1v1h-1zM4 12h8v1h-8zM6 13h4v1h-4z"#,
  },
  SpriteLayer {
    fill: "#ffe9a8",
    d: r#"M6 7h2v1h-2zM5 8h1v2h-1z"#,
  },
];
pub const BADGE_MEDAL: SpriteData = SpriteData {
  size: 16,
  layers: BADGE_MEDAL_LAYERS,
};

const BADGE_TROPHY_LAYERS: &[SpriteLayer] = &[
  SpriteLayer {
    fill: "var(--px-sprite-ink,#1a1c2c)",
    d: r#"M2 1h12v1h-12zM1 2h1v1h-1zM14 2h1v1h-1zM0 3h2v1h-2zM14 3h2v1h-2zM0 4h1v2h-1zM2 4h1v2h-1zM13 4h1v2h-1zM15 4h1v2h-1zM1 6h2v1h-2zM13 6h2v1h-2zM2 7h2v1h-2zM12 7h2v1h-2zM3 8h2v1h-2zM11 8h2v1h-2zM5 9h2v1h-2zM9 9h2v1h-2zM6 10h1v2h-1zM9 10h1v2h-1zM4 12h2v1h-2zM10 12h2v1h-2zM3 13h1v1h-1zM12 13h1v1h-1zM3 14h10v1h-10z"#,
  },
  SpriteLayer {
    fill: "#ffcd75",
    d: r#"M2 2h12v1h-12zM2 3h1v1h-1zM4 3h10v1h-10zM3 4h1v2h-1zM5 4h8v2h-8zM3 6h10v1h-10zM4 7h8v1h-8zM5 8h6v1h-6zM7 9h2v3h-2zM6 12h4v1h-4zM4 13h8v1h-8z"#,
  },
  SpriteLayer {
    fill: "#ffe9a8",
    d: r#"M3 3h1v1h-1zM4 4h1v2h-1z"#,
  },
];
pub const BADGE_TROPHY: SpriteData = SpriteData {
  size: 16,
  layers: BADGE_TROPHY_LAYERS,
};

const BADGE_FLAME_LAYERS: &[SpriteLayer] = &[
  SpriteLayer {
    fill: "var(--px-sprite-ink,#1a1c2c)",
    d: r#"M7 1h2v1h-2zM6 2h1v2h-1zM9 2h1v1h-1zM10 3h1v2h-1zM5 4h1v1h-1zM4 5h1v1h-1zM11 5h1v1h-1zM3 6h1v2h-1zM12 6h1v2h-1zM2 8h1v4h-1zM13 8h1v4h-1zM3 12h1v1h-1zM12 12h1v1h-1zM4 13h2v1h-2zM10 13h2v1h-2zM6 14h4v1h-4z"#,
  },
  SpriteLayer {
    fill: "#b13e53",
    d: r#"M7 2h2v1h-2zM7 3h3v1h-3zM6 4h2v1h-2zM9 4h1v1h-1zM5 5h2v1h-2zM9 5h2v1h-2zM4 6h2v1h-2zM10 6h2v1h-2zM4 7h1v1h-1zM11 7h1v1h-1zM3 8h2v3h-2zM11 8h2v3h-2zM3 11h3v1h-3zM10 11h3v1h-3zM4 12h3v1h-3zM9 12h3v1h-3zM6 13h4v1h-4z"#,
  },
  SpriteLayer {
    fill: "#ef7d57",
    d: r#"M8 4h1v1h-1zM7 5h2v1h-2zM6 6h2v1h-2zM9 6h1v1h-1zM5 7h2v1h-2zM10 7h1v4h-1zM5 8h1v3h-1zM6 11h1v1h-1zM9 11h1v1h-1zM7 12h2v1h-2z"#,
  },
  SpriteLayer {
    fill: "#ffcd75",
    d: r#"M8 6h1v1h-1zM7 7h3v1h-3zM6 8h4v1h-4zM6 9h2v1h-2zM9 9h1v1h-1zM6 10h4v1h-4zM7 11h2v1h-2z"#,
  },
  SpriteLayer {
    fill: "#f4f4f4",
    d: r#"M8 9h1v1h-1z"#,
  },
];
pub const BADGE_FLAME: SpriteData = SpriteData {
  size: 16,
  layers: BADGE_FLAME_LAYERS,
};

const BADGE_ANTENNA_LAYERS: &[SpriteLayer] = &[
  SpriteLayer {
    fill: "var(--px-sprite-ink,#1a1c2c)",
    d: r#"M2 1h1v1h-1zM13 1h1v1h-1zM1 2h1v1h-1zM3 2h1v1h-1zM12 2h1v1h-1zM14 2h1v1h-1zM0 3h1v2h-1zM2 3h1v2h-1zM4 3h1v2h-1zM7 3h2v4h-2zM11 3h1v2h-1zM13 3h1v2h-1zM15 3h1v2h-1zM1 5h1v1h-1zM3 5h1v1h-1zM12 5h1v1h-1zM14 5h1v1h-1zM2 6h1v1h-1zM13 6h1v1h-1zM6 7h1v3h-1zM9 7h1v3h-1zM5 10h1v1h-1zM10 10h1v1h-1zM4 11h1v1h-1zM11 11h1v1h-1zM3 12h1v1h-1zM12 12h1v1h-1zM2 13h12v1h-12z"#,
  },
  SpriteLayer {
    fill: "#566c86",
    d: r#"M7 7h2v3h-2zM6 10h4v1h-4zM5 11h6v1h-6zM4 12h8v1h-8z"#,
  },
];
pub const BADGE_ANTENNA: SpriteData = SpriteData {
  size: 16,
  layers: BADGE_ANTENNA_LAYERS,
};

const BADGE_GLOBE_LAYERS: &[SpriteLayer] = &[
  SpriteLayer {
    fill: "var(--px-sprite-ink,#1a1c2c)",
    d: r#"M4 1h8v1h-8zM3 2h1v1h-1zM12 2h1v1h-1zM2 3h1v1h-1zM13 3h1v1h-1zM1 4h1v7h-1zM14 4h1v7h-1zM2 11h1v1h-1zM13 11h1v1h-1zM3 12h1v1h-1zM12 12h1v1h-1zM4 13h8v1h-8z"#,
  },
  SpriteLayer {
    fill: "#3b5dc9",
    d: r#"M4 2h2v1h-2zM8 2h4v1h-4zM3 3h1v1h-1zM8 3h5v1h-5zM2 4h2v1h-2zM7 4h3v1h-3zM12 4h2v3h-2zM2 5h3v1h-3zM7 5h2v1h-2zM2 6h7v1h-7zM2 7h1v2h-1zM5 7h4v1h-4zM11 7h3v1h-3zM6 8h8v1h-8zM2 9h2v1h-2zM6 9h4v1h-4zM12 9h2v2h-2zM2 10h7v1h-7zM3 11h6v1h-6zM11 11h2v1h-2zM4 12h8v1h-8z"#,
  },
  SpriteLayer {
    fill: "#38b764",
    d: r#"M6 2h2v1h-2zM4 3h4v1h-4zM4 4h3v1h-3zM10 4h2v1h-2zM5 5h2v1h-2zM9 5h3v2h-3zM3 7h2v1h-2zM9 7h2v1h-2zM3 8h3v1h-3zM4 9h2v1h-2zM10 9h2v1h-2zM9 10h3v1h-3zM9 11h2v1h-2z"#,
  },
];
pub const BADGE_GLOBE: SpriteData = SpriteData {
  size: 16,
  layers: BADGE_GLOBE_LAYERS,
};

const BADGE_BOOK_LAYERS: &[SpriteLayer] = &[
  SpriteLayer {
    fill: "var(--px-sprite-ink,#1a1c2c)",
    d: r#"M2 2h6v1h-6zM9 2h6v1h-6zM1 3h1v7h-1zM7 3h2v7h-2zM14 3h1v7h-1zM1 10h14v1h-14z"#,
  },
  SpriteLayer {
    fill: "#3b5dc9",
    d: r#"M2 3h5v1h-5zM9 3h5v1h-5zM2 4h1v1h-1zM6 4h1v1h-1zM9 4h1v1h-1zM13 4h1v1h-1zM2 5h5v1h-5zM9 5h5v1h-5zM2 6h1v1h-1zM6 6h1v1h-1zM9 6h1v1h-1zM13 6h1v1h-1zM2 7h5v1h-5zM9 7h5v1h-5zM2 8h1v1h-1zM6 8h1v1h-1zM9 8h1v1h-1zM13 8h1v1h-1zM2 9h5v1h-5zM9 9h5v1h-5z"#,
  },
  SpriteLayer {
    fill: "#f4f4f4",
    d: r#"M3 4h3v1h-3zM10 4h3v1h-3zM3 6h3v1h-3zM10 6h3v1h-3zM3 8h3v1h-3zM10 8h3v1h-3z"#,
  },
];
pub const BADGE_BOOK: SpriteData = SpriteData {
  size: 16,
  layers: BADGE_BOOK_LAYERS,
};

const BADGE_LOG_LAYERS: &[SpriteLayer] = &[
  SpriteLayer {
    fill: "var(--px-sprite-ink,#1a1c2c)",
    d: r#"M2 1h10v1h-10zM2 2h1v9h-1zM11 2h2v1h-2zM4 3h6v1h-6zM11 3h1v1h-1zM13 3h1v1h-1zM11 4h2v1h-2zM14 4h1v8h-1zM4 5h6v1h-6zM11 5h1v6h-1zM4 7h4v1h-4zM4 9h6v1h-6zM2 11h10v1h-10zM7 12h1v1h-1zM13 12h1v1h-1zM8 13h5v1h-5z"#,
  },
  SpriteLayer {
    fill: "#f4f4f4",
    d: r#"M3 2h8v1h-8zM3 3h1v1h-1zM10 3h1v1h-1zM12 3h1v1h-1zM3 4h8v1h-8zM13 4h1v1h-1zM3 5h1v1h-1zM10 5h1v1h-1zM12 5h2v7h-2zM3 6h8v1h-8zM3 7h1v1h-1zM8 7h3v1h-3zM3 8h8v1h-8zM3 9h1v1h-1zM10 9h1v1h-1zM3 10h8v1h-8z"#,
  },
  SpriteLayer {
    fill: "#8b5a3c",
    d: r#"M8 12h5v1h-5z"#,
  },
];
pub const BADGE_LOG: SpriteData = SpriteData {
  size: 16,
  layers: BADGE_LOG_LAYERS,
};

const BADGE_LOCK_LAYERS: &[SpriteLayer] = &[
  SpriteLayer {
    fill: "var(--px-sprite-ink,#1a1c2c)",
    d: r#"M6 1h4v1h-4zM5 2h1v1h-1zM4 3h1v3h-1zM6 3h4v1h-4zM11 3h1v3h-1zM6 4h1v2h-1zM9 4h1v2h-1zM2 6h12v1h-12zM2 7h1v5h-1zM13 7h1v5h-1zM7 8h2v2h-2zM8 10h1v1h-1zM2 12h12v1h-12z"#,
  },
  SpriteLayer {
    fill: "#566c86",
    d: r#"M6 2h5v1h-5zM5 3h1v3h-1zM10 3h1v3h-1z"#,
  },
  SpriteLayer {
    fill: "#ffcd75",
    d: r#"M3 7h10v1h-10zM3 8h4v2h-4zM9 8h4v3h-4zM3 10h5v1h-5zM3 11h10v1h-10z"#,
  },
];
pub const BADGE_LOCK: SpriteData = SpriteData {
  size: 16,
  layers: BADGE_LOCK_LAYERS,
};

const NODE_FLAG_LAYERS: &[SpriteLayer] = &[
  SpriteLayer {
    fill: "var(--px-sprite-ink,#1a1c2c)",
    d: r#"M2 1h2v1h-2zM2 2h1v7h-1zM4 2h2v1h-2zM6 3h2v1h-2zM8 4h2v1h-2zM10 5h1v1h-1zM8 6h2v1h-2zM6 7h2v1h-2zM4 8h2v1h-2zM2 9h2v1h-2zM2 10h1v3h-1zM1 13h2v1h-2zM4 13h2v1h-2z"#,
  },
  SpriteLayer {
    fill: "#b13e53",
    d: r#"M3 2h1v1h-1zM3 3h3v1h-3zM3 4h5v1h-5zM3 5h7v1h-7zM3 6h5v1h-5zM3 7h3v1h-3zM3 8h1v1h-1z"#,
  },
  SpriteLayer {
    fill: "#8b5a3c",
    d: r#"M3 10h1v4h-1z"#,
  },
];
pub const NODE_FLAG: SpriteData = SpriteData {
  size: 16,
  layers: NODE_FLAG_LAYERS,
};

const NODE_HEART_LAYERS: &[SpriteLayer] = &[
  SpriteLayer {
    fill: "var(--px-sprite-ink,#1a1c2c)",
    d: r#"M2 2h4v1h-4zM8 2h4v1h-4zM1 3h1v1h-1zM6 3h2v1h-2zM12 3h1v1h-1zM0 4h1v3h-1zM13 4h1v3h-1zM1 7h1v1h-1zM12 7h1v1h-1zM2 8h1v1h-1zM11 8h1v1h-1zM3 9h1v1h-1zM10 9h1v1h-1zM4 10h1v1h-1zM9 10h1v1h-1zM5 11h1v1h-1zM8 11h1v1h-1zM6 12h2v1h-2z"#,
  },
  SpriteLayer {
    fill: "#b13e53",
    d: r#"M2 3h4v1h-4zM8 3h4v1h-4zM1 4h2v2h-2zM5 4h8v1h-8zM4 5h9v1h-9zM1 6h12v1h-12zM2 7h10v1h-10zM3 8h8v1h-8zM4 9h6v1h-6zM5 10h4v1h-4zM6 11h2v1h-2z"#,
  },
  SpriteLayer {
    fill: "#f4f4f4",
    d: r#"M3 4h2v1h-2zM3 5h1v1h-1z"#,
  },
];
pub const NODE_HEART: SpriteData = SpriteData {
  size: 16,
  layers: NODE_HEART_LAYERS,
};

const NODE_COIN_LAYERS: &[SpriteLayer] = &[
  SpriteLayer {
    fill: "var(--px-sprite-ink,#1a1c2c)",
    d: r#"M5 2h6v1h-6zM3 3h2v1h-2zM11 3h2v1h-2zM2 4h1v2h-1zM13 4h1v2h-1zM7 5h2v1h-2zM1 6h1v4h-1zM6 6h1v3h-1zM9 6h1v3h-1zM14 6h1v4h-1zM6 9h4v1h-4zM2 10h1v2h-1zM7 10h1v1h-1zM13 10h1v2h-1zM3 12h2v1h-2zM11 12h2v1h-2zM5 13h6v1h-6z"#,
  },
  SpriteLayer {
    fill: "#ffcd75",
    d: r#"M5 3h6v1h-6zM3 4h2v1h-2zM7 4h6v1h-6zM3 5h1v1h-1zM5 5h2v1h-2zM9 5h4v1h-4zM2 6h2v1h-2zM5 6h1v1h-1zM7 6h2v3h-2zM10 6h4v4h-4zM2 7h4v3h-4zM3 10h4v1h-4zM8 10h5v1h-5zM3 11h10v1h-10zM5 12h6v1h-6z"#,
  },
  SpriteLayer {
    fill: "#ffe9a8",
    d: r#"M5 4h2v1h-2zM4 5h1v2h-1z"#,
  },
];
pub const NODE_COIN: SpriteData = SpriteData {
  size: 16,
  layers: NODE_COIN_LAYERS,
};

/// 按名称查找精灵（页面里用字符串选精灵，如成就 id → 徽章）。
pub fn sprite_by_name(name: &str) -> Option<&'static SpriteData> {
  match name {
    "hero" => Some(&HERO),
    "mon_static" => Some(&MON_STATIC),
    "mon_swr" => Some(&MON_SWR),
    "mon_noise" => Some(&MON_NOISE),
    "mon_spur" => Some(&MON_SPUR),
    "mon_fade" => Some(&MON_FADE),
    "boss" => Some(&BOSS),
    "badge_star" => Some(&BADGE_STAR),
    "badge_medal" => Some(&BADGE_MEDAL),
    "badge_trophy" => Some(&BADGE_TROPHY),
    "badge_flame" => Some(&BADGE_FLAME),
    "badge_antenna" => Some(&BADGE_ANTENNA),
    "badge_globe" => Some(&BADGE_GLOBE),
    "badge_book" => Some(&BADGE_BOOK),
    "badge_log" => Some(&BADGE_LOG),
    "badge_lock" => Some(&BADGE_LOCK),
    "node_flag" => Some(&NODE_FLAG),
    "node_heart" => Some(&NODE_HEART),
    "node_coin" => Some(&NODE_COIN),
    _ => None,
  }
}
