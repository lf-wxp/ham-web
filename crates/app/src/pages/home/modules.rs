//! 首页「模块入口」的数据：三张卡片的标题、图标、序号与条目链接。
//!
//! 只有像素版首页用它（现代版的入口分组是平铺磁贴 + 三列链接，组织方式不同）。
//! 数据放在单独文件而不是页面里：页面本身被拆成「像素版 / 现代版」两份实现，
//! 共用数据再留在页面文件里就会变成两份拷贝。

use crate::icons::IconKind;

/// 首页模块入口。
pub(super) struct ModuleCard {
  pub title: &'static str,
  pub desc: &'static str,
  pub href: &'static str,
  pub icon: IconKind,
  /// 标题行左侧的序号（`01` / `02` / `03`）。
  pub index: &'static str,
  /// 桌面端的纵向错位（阶梯式下沉，形成一条斜向的视线）。
  ///
  /// 写成完整的类名字面量放在数据里：Tailwind 只扫描源码里的完整类名，
  /// 运行时拼接 `md:mt-{n}` 会被当成不存在而不生成样式。
  pub offset: &'static str,
  pub links: &'static [(&'static str, &'static str)],
}

pub(super) const MODULES: &[ModuleCard] = &[
  ModuleCard {
    title: "考试中心",
    desc: "A/B/C 类题库 · 练习 · 模拟考试",
    href: "/practice",
    icon: IconKind::Timer,
    index: "01",
    offset: "lg:mt-0",
    links: &[
      ("练习", "/practice"),
      ("模拟考试", "/exam"),
      ("每日挑战", "/daily-challenge"),
      ("分类浏览", "/browse"),
      ("闪卡刷题", "/flashcards"),
      ("错题集", "/mistakes"),
      ("学习周报", "/weekly"),
    ],
  },
  ModuleCard {
    title: "知识库",
    desc: "80+ 专题 · 呼号 · 术语 · 模式 · 传播 · 天线 · 通联",
    href: "/reference",
    icon: IconKind::BookOpen,
    index: "02",
    offset: "lg:mt-6",
    links: &[
      ("考试速查", "/reference"),
      ("呼号前缀", "/prefixes"),
      ("术语表", "/glossary"),
      ("莫尔斯电码", "/morse"),
      ("数字模式", "/modes"),
      ("通联实务", "/operating"),
    ],
  },
  ModuleCard {
    title: "工具",
    desc: "计算器 · 通联日志 · 实时数据",
    href: "/tools",
    icon: IconKind::Calculator,
    index: "03",
    offset: "lg:mt-12",
    links: &[
      ("小工具", "/tools"),
      ("通联日志", "/log"),
      ("呼号抄收", "/callsign-copy"),
      ("太阳活动", "/solar"),
      ("业余卫星", "/satellites"),
      ("通知中心", "/notifications"),
    ],
  },
];
