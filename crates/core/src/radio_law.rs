//! 法规原文库：条文结构化 + 题目 ↔ 条款双向关联。
//!
//! 数据文件是 `data/radio-law.txt`（**原文照录**：出处、令号、施行日期、核对日期都在文件头）。
//! 本模块提供三件事：
//!
//! 1. 按条号取条文（页面渲染、条款级检索）；
//! 2. [`find_refs`]：在一段文字里找出「《业余无线电台管理办法》第N条」引用 —— 解析卡片据此
//!    给引用加原文链接。**只认本库收录的法律**：别的法律（如《民法典》）或缩写（《办法》）
//!    一律保持原文，不瞎链；
//! 3. [`ARTICLE_QUESTIONS`]：人工维护的「条文 → 题目」映射（反向链接）。每道题都能在题库里
//!    查到（app 侧有单测钉住 id 必须存在），页面据此列出「引用这条的题目」。
//!
//! # 为什么不翻译条文
//!
//! 法律文本只有官方发布版有规范效力，任何翻译都是非官方转述 —— 所以条文**不译不缩写**，
//! 界面语言切换时条文仍是中文原文（页面上写明这一点）。

use std::sync::LazyLock;

/// 内嵌的条文数据文件（含出处与取数规则）。
const DATA: &str = include_str!("../data/radio-law.txt");

/// 一部法律（目前收录《业余无线电台管理办法》与《中华人民共和国无线电管理条例》）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Law {
  /// 稳定 key（条文行里引用它）。
  pub key: &'static str,
  /// 书名（`《业余无线电台管理办法》`，含书名号）。
  pub title: &'static str,
  /// 发布令号。
  pub order: &'static str,
  /// 施行日期（`YYYY-MM-DD`）。
  pub effective: &'static str,
  /// 官方来源地址。
  pub source_url: &'static str,
}

/// 一条条文。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LawArticle {
  /// 所属法律 key（[`Law::key`]）。
  pub law: &'static str,
  /// 条号（从 1 起）。
  pub number: u16,
  /// 章标题（含「第X章」）。
  pub chapter: &'static str,
  /// 条文（款与款之间是真实换行；原文照录，不译不缩写）。
  pub text: String,
}

/// 解析数据文件：跳过注释，`|` 分字段，条文里的字面 `\n` 还原成换行。
fn parse(data: &'static str) -> (Vec<Law>, Vec<LawArticle>) {
  let mut laws = Vec::new();
  let mut articles = Vec::new();
  for line in data.lines() {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') {
      continue;
    }
    let mut f = line.split('|');
    match (f.next(), f.next(), f.next(), f.next()) {
      (Some("LAW"), Some(key), Some(title), Some(order)) => {
        let effective = f.next().unwrap_or("");
        let source_url = f.next().unwrap_or("");
        laws.push(Law {
          key,
          title,
          order,
          effective,
          source_url,
        });
      }
      (Some(law), Some(number), Some(chapter), Some(text)) => {
        let Ok(number) = number.trim().parse::<u16>() else {
          continue;
        };
        articles.push(LawArticle {
          law,
          number,
          chapter,
          text: text.replace("\\n", "\n"),
        });
      }
      _ => {}
    }
  }
  (laws, articles)
}

static TABLE: LazyLock<(Vec<Law>, Vec<LawArticle>)> = LazyLock::new(|| parse(DATA));

/// 全部已收录的法律。
#[must_use]
pub fn laws() -> &'static [Law] {
  &TABLE.0
}

/// 按稳定 key 取法律。
#[must_use]
pub fn law(key: &str) -> Option<&'static Law> {
  TABLE.0.iter().find(|l| l.key == key)
}

/// 按「书名（含书名号）」取法律。
#[must_use]
pub fn law_by_title(title: &str) -> Option<&'static Law> {
  TABLE.0.iter().find(|l| l.title == title)
}

/// 取一条条文。
#[must_use]
pub fn article(law: &str, number: u16) -> Option<&'static LawArticle> {
  TABLE.1.iter().find(|a| a.law == law && a.number == number)
}

/// 某部法律的全部条文（按条号升序）。
#[must_use]
pub fn articles(law: &str) -> &'static [LawArticle] {
  // 数据文件按条号顺序书写，直接切片即可；单测钉住这一点。
  let all = TABLE.1.as_slice();
  let start = all.iter().position(|a| a.law == law).unwrap_or(all.len());
  let end = all
    .iter()
    .rposition(|a| a.law == law)
    .map_or(all.len(), |i| i + 1);
  &all[start..end]
}

/// 某部法律的章：`(章标题, 章内条号)`，按出现顺序。
#[must_use]
pub fn chapters(law: &str) -> Vec<(&'static str, Vec<u16>)> {
  let mut out: Vec<(&'static str, Vec<u16>)> = Vec::new();
  for a in articles(law) {
    match out.last_mut() {
      Some((chapter, nums)) if *chapter == a.chapter => nums.push(a.number),
      _ => out.push((a.chapter, vec![a.number])),
    }
  }
  out
}

/// 一段文字里的一处条文引用。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArticleRef {
  /// 法律 key（[`Law::key`]）。
  pub law: &'static str,
  /// 条号。
  pub number: u16,
  /// 在原文里的字节起点（含书名号）。
  pub start: usize,
  /// 字节终点（不含后随字符）。
  pub end: usize,
}

/// 单个中文数字字（`零`–`九`）→ 数值。
fn zh_digit(c: char) -> Option<u16> {
  "零一二三四五六七八九"
    .chars()
    .position(|d| d == c)
    .and_then(|i| u16::try_from(i).ok())
}

/// 非零的中文数字字（`一`–`九`）：十位、百位、个位在「十」「百」前后都不能是 0。
fn zh_nonzero_digit(c: char) -> Option<u16> {
  zh_digit(c).filter(|&d| d > 0)
}

/// 中文数字（法条写法，支持到「九百九十九」）→ 数值。
///
/// 只认法条里会出现的规范写法：`五`、`十`、`十三`、`二十`、`三十七`、`一百`、`一百零三`、
/// `一百二十三`；`三三`、`十十`、`一百三`（口语）这类不成数的组合一律返回 `None`，
/// 以免把别的文字误当成条号。
#[must_use]
pub fn zh_number(s: &str) -> Option<u16> {
  let chars: Vec<char> = s.chars().collect();
  let mut rest = chars.as_slice();
  let mut total: u16 = 0;
  // 百位：`一百` / `一百零三` / `一百二十`。
  if let [h, '百', tail @ ..] = rest {
    total = zh_nonzero_digit(*h)? * 100;
    rest = tail;
    if rest.is_empty() {
      return Some(total);
    }
    if let ['零', unit] = rest {
      return Some(total + zh_nonzero_digit(*unit)?);
    }
  }
  // 百位以下：个位 / `十` / `十三` / `二十` / `三十七`。
  let below = match rest {
    ['十'] => 10,
    [c] if total == 0 => zh_digit(*c)?,
    ['十', u] => 10 + zh_nonzero_digit(*u)?,
    [t, '十'] => zh_nonzero_digit(*t)? * 10,
    [t, '十', u] => zh_nonzero_digit(*t)? * 10 + zh_nonzero_digit(*u)?,
    _ => return None,
  };
  Some(total + below)
}

/// 数值（1–999）→ 中文数字，[`zh_number`] 的逆运算（法条写法：`十三`、`三十七`、`一百零三`）。
///
/// 站内搜索据此给每条条文补上「第三十七条」的写法 —— 法律原文用中文数字，用户也这么搜。
#[must_use]
pub fn to_zh_number(n: u16) -> Option<String> {
  if !(1..=999).contains(&n) {
    return None;
  }
  const DIGITS: [char; 10] = ['零', '一', '二', '三', '四', '五', '六', '七', '八', '九'];
  let digit = |d: u16| DIGITS[usize::from(d)];
  let (h, t, u) = (n / 100, n / 10 % 10, n % 10);
  let mut out = String::new();
  if h > 0 {
    out.push(digit(h));
    out.push('百');
  }
  match (t, u) {
    (0, 0) => {}
    (0, u) => {
      if h > 0 {
        out.push('零');
      }
      out.push(digit(u));
    }
    // 十几：`十三`（没有百位时省略「一」）；有百位时写全，如 `一百一十三`。
    (1, u) if h == 0 => {
      out.push('十');
      if u > 0 {
        out.push(digit(u));
      }
    }
    (t, u) => {
      out.push(digit(t));
      out.push('十');
      if u > 0 {
        out.push(digit(u));
      }
    }
  }
  Some(out)
}

/// 在一段文字里找「《本库收录的法律》第N条」引用（阿拉伯或中文数字都认）。
///
/// 只返回**条号在本库存在**的引用；不收录的法律（如《民法典》）与缩写（《办法》）保持原文。
#[must_use]
pub fn find_refs(text: &str) -> Vec<ArticleRef> {
  let mut out = Vec::new();
  for law in laws() {
    let mut from = 0usize;
    while let Some(pos) = text[from..].find(law.title) {
      let title_start = from + pos;
      let after_title = title_start + law.title.len();
      // 书名号之后必须紧跟「第N条」。
      let Some(tail) = text[after_title..].strip_prefix('第') else {
        from = after_title;
        continue;
      };
      let head = after_title + "第".len();
      // 阿拉伯数字与中文数字两条路，各自记下条号与「数字部分占了多少字节」
      // （字节跨度用来算引用的终点，不能拿数字的十进制长度去数中文的字节）。
      // 函数体里 `?` 不能直接用（外层返回 Vec），包一个立刻执行的闭包拿 Option 语义。
      let parsed: Option<(u16, usize)> = (|| {
        if tail.starts_with(|c: char| c.is_ascii_digit()) {
          let digits: String = tail.chars().take_while(|c| c.is_ascii_digit()).collect();
          let n: u16 = digits.parse().ok()?;
          (tail[digits.len()..].starts_with('条') && n > 0).then_some((n, digits.len()))
        } else {
          let digits: String = tail
            .chars()
            .take_while(|c| {
              *c != '条' && *c != '，' && *c != '。' && *c != '和' && *c != '、' && *c != '；'
            })
            .collect();
          zh_number(&digits).and_then(|n| {
            tail[digits.len()..]
              .starts_with('条')
              .then_some((n, digits.len()))
          })
        }
      })();
      let Some((number, span)) = parsed else {
        from = after_title;
        continue;
      };
      if article(law.key, number).is_some() {
        out.push(ArticleRef {
          law: law.key,
          number,
          start: title_start,
          end: head + span + "条".len(),
        });
      }
      from = after_title;
    }
  }
  // 按法律逐部扫描，不同法律的引用会交错出现在文本里：调用方（解析卡片）要按文本顺序切片，
  // 必须按起点升序，否则「条例在前、办法在后」时游标会回退、文字重复。
  out.sort_by_key(|r| r.start);
  out
}

/// 映射到一条条文的一道题。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MappedQuestion {
  /// 题库 id（A 卷）。
  pub id: &'static str,
  /// 知识点 code（反向链接到 `/practice?sub=<code>`）。
  pub p_code: &'static str,
  /// 题干（照抄题库；app 侧有单测逐字节比对，防止两边漂移）。
  pub stem: &'static str,
}

/// 人工维护的「条文 → 题目」映射。
///
/// 对应关系是人工校对的（题目没有结构化引用字段），题干与知识点**照抄题库**并把一致性
/// 交给 app 侧单测（逐字节比对）—— 题目改了这里不跟，检查会红。
/// 条目带法律 key：办法与条例都有「第1条」，只有条号会撞。
pub struct ArticleMapping {
  /// 法律 key（[`Law::key`]）。
  pub law: &'static str,
  /// 条号。
  pub number: u16,
  /// 引用这条的题目。
  pub questions: &'static [MappedQuestion],
}

pub const ARTICLE_QUESTIONS: &[ArticleMapping] = &[
  ArticleMapping {
    law: "办法",
    number: 3, // 不得用于商业利益（出租车载客、商业促销）
    questions: &[
      MappedQuestion {
        id: "A-119",
        p_code: "1.5.2",
        stem: "出租车安装业余电台并用来传递有关载客的信息，这种行为的性质是：",
      },
      MappedQuestion {
        id: "A-120",
        p_code: "1.5.2",
        stem: "利用业余无线电台通信来促销业余无线电产品或者推动与业余无线电活动有关的其他商业性活动，对这类行为的态度应该是：",
      },
    ],
  },
  ArticleMapping {
    law: "办法",
    number: 6, // 应急临时设台、与非业余台通信
    questions: &[
      MappedQuestion {
        id: "A-133",
        p_code: "1.5.3",
        stem: "关于业余无线电台的应急通信，正确的叙述是：",
      },
      MappedQuestion {
        id: "A-134",
        p_code: "1.5.3",
        stem: "业余无线电台允许与非业余无线电台通信的条件是：",
      },
    ],
  },
  ArticleMapping {
    law: "办法",
    number: 8, // 未成年人设台
    questions: &[MappedQuestion {
      id: "A-67",
      p_code: "1.3.2",
      stem: "关于未成年人设置、使用业余无线电台，下列哪些选项为正确：",
    }],
  },
  ArticleMapping {
    law: "办法",
    number: 20, // 执照有效期、届满更换
    questions: &[
      MappedQuestion {
        id: "A-33",
        p_code: "1.2.1",
        stem: "业余无线电台执照的有效期不超过：",
      },
      MappedQuestion {
        id: "A-41",
        p_code: "1.2.2",
        stem: "业余无线电台执照有效期届满后需要继续使用的，应当在下列期限内向作出许可决定的无线电管理机构申请更换业余无线电台执照：",
      },
    ],
  },
  ArticleMapping {
    law: "办法",
    number: 30, // A / B / C 类的频段与功率
    questions: &[
      MappedQuestion {
        id: "A-57",
        p_code: "1.3.2",
        stem: "A 类业余无线电台允许工作的频率范围和最大发射功率为：",
      },
      MappedQuestion {
        id: "A-58",
        p_code: "1.3.2",
        stem: "取得 B 类业余无线电台操作技术能力验证证书的，可以申请设置、使用业余无线电台的工作频段和最大发射功率为：",
      },
      MappedQuestion {
        id: "A-59",
        p_code: "1.3.2",
        stem: "取得 C 类业余无线电台操作技术能力验证证书的，可以申请设置、使用业余无线电台的工作频段和最大发射功率为：",
      },
    ],
  },
  ArticleMapping {
    law: "办法",
    number: 36, // 日志记载与保留
    questions: &[
      MappedQuestion {
        id: "A-261",
        p_code: "2.2.3",
        stem: "法规和国际业余无线电惯例要求业余电台日志应记载的必要内容是：",
      },
      MappedQuestion {
        id: "A-262",
        p_code: "2.2.3",
        stem: "法规和国际业余无线电惯例要求业余电台日志应记载的必要内容是：",
      },
    ],
  },
  ArticleMapping {
    law: "办法",
    number: 37, // 呼号的正确使用
    questions: &[
      MappedQuestion {
        id: "A-70",
        p_code: "1.4.1",
        stem: "《业余无线电台管理办法》规定正确使用业余无线电台呼号的方法是：",
      },
      MappedQuestion {
        id: "A-85",
        p_code: "1.4.1",
        stem: "关于使用业余无线电台呼号正确的是：",
      },
    ],
  },
  ArticleMapping {
    law: "办法",
    number: 43, // 禁止行为（商业两条与第三条交叉引用）
    questions: &[
      MappedQuestion {
        id: "A-119",
        p_code: "1.5.2",
        stem: "出租车安装业余电台并用来传递有关载客的信息，这种行为的性质是：",
      },
      MappedQuestion {
        id: "A-120",
        p_code: "1.5.2",
        stem: "利用业余无线电台通信来促销业余无线电产品或者推动与业余无线电活动有关的其他商业性活动，对这类行为的态度应该是：",
      },
    ],
  },
  ArticleMapping {
    law: "办法",
    number: 1, // 制定依据与制定机构
    questions: &[MappedQuestion {
      id: "A-2",
      p_code: "1.1.1",
      stem: "我国专门针对业余无线电台的管理文件及其制定机构分别是：",
    }],
  },
  ArticleMapping {
    law: "办法",
    number: 47, // 未经许可设台的罚则指向条例第七十条
    questions: &[MappedQuestion {
      id: "A-149",
      p_code: "1.6.2",
      stem: "对擅自设置、使用业余无线电台的，无线电管理机构可以根据其具体情况给予下列处罚：",
    }],
  },
  // ── 《中华人民共和国无线电管理条例》────────────────────────
  ArticleMapping {
    law: "条例",
    number: 1, // 制定机构
    questions: &[MappedQuestion {
      id: "A-1",
      p_code: "1.1.1",
      stem: "我国专门针对无线电管理的行政法规及其制定机构是：",
    }],
  },
  ArticleMapping {
    law: "条例",
    number: 70, // 擅自设台的罚则
    questions: &[
      MappedQuestion {
        id: "A-148",
        p_code: "1.6.2",
        stem: "对未经许可擅自使用无线电频率的，无线电管理机构可以根据其具体情况给予下列处罚：",
      },
      MappedQuestion {
        id: "A-149",
        p_code: "1.6.2",
        stem: "对擅自设置、使用业余无线电台的，无线电管理机构可以根据其具体情况给予下列处罚：",
      },
      MappedQuestion {
        id: "A-150",
        p_code: "1.6.2",
        stem: "擅自设置、使用无线电台（站）从事诈骗等违法活动，可以根据其具体情况给予下列处罚：",
      },
    ],
  },
];

/// 引用某一条的题目（见 [`ARTICLE_QUESTIONS`]）。
#[must_use]
pub fn questions_for(law: &str, number: u16) -> &'static [MappedQuestion] {
  ARTICLE_QUESTIONS
    .iter()
    .find(|m| m.law == law && m.number == number)
    .map_or(&[], |m| m.questions)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn the_file_states_its_provenance() {
    // 条文是法律文本：出处、令号、施行日期与核对日期必须写在数据文件里。
    for needle in [
      "工业和信息化部官网",
      "第67号",
      "2024-03-01",
      "2026-10-09",
      "原文照录",
    ] {
      assert!(DATA.contains(needle), "数据文件里少了「{needle}」");
    }
  }

  #[test]
  fn all_articles_parse_with_chapters_and_text() {
    assert_eq!(laws().len(), 2, "办法与条例都已收录");
    // 办法：58 条、七章。
    let method = law("办法").expect("办法");
    assert!(method.title.contains("业余无线电台管理办法"));
    let all = articles("办法");
    assert_eq!(all.len(), 58, "办法共 58 条");
    for (i, a) in all.iter().enumerate() {
      assert_eq!(a.number as usize, i + 1, "条号连续从 1 开始");
      assert!(
        !a.chapter.is_empty() && !a.text.trim().is_empty(),
        "第{}条",
        a.number
      );
    }
    let chs = chapters("办法");
    assert_eq!(chs.len(), 7, "办法共七章");
    assert_eq!(chs[0].0, "第一章 总则");
    assert_eq!(chs[0].1[0], 1);
    assert_eq!(chs[6].0, "第七章 附则");
    // 条例：85 条、九章；罚则是第八章（第七十条 = 擅自设台）。
    let reg = law("条例").expect("条例");
    assert!(reg.title.contains("无线电管理条例"));
    let all = articles("条例");
    assert_eq!(all.len(), 85, "条例共 85 条");
    for (i, a) in all.iter().enumerate() {
      assert_eq!(a.number as usize, i + 1, "条号连续从 1 开始");
      assert!(
        !a.chapter.is_empty() && !a.text.trim().is_empty(),
        "第{}条",
        a.number
      );
    }
    let chs = chapters("条例");
    assert_eq!(chs.len(), 9, "条例共九章");
    assert_eq!(chs[7].0, "第八章 法律责任");
    let a70 = article("条例", 70).expect("第七十条");
    assert!(a70.text.contains("5万元以上20万元以下"), "{}", a70.text);
  }

  #[test]
  fn paragraph_breaks_survive_parsing() {
    // 第二条有两款：换行必须还原（否则「本办法所称业余无线电台」会与第一款挤成一行）。
    let a = article("办法", 2).expect("第二条");
    assert!(a.text.contains('\n'), "款的换行要还原");
    assert!(a.text.contains("本办法所称业余无线电台"));
    // 单款条文没有换行。
    assert!(!article("办法", 8).expect("第八条").text.contains('\n'));
  }

  #[test]
  fn find_refs_links_only_laws_we_host() {
    let text = "依据《业余无线电台管理办法》第三十条，B类……另见《业余无线电台管理办法》第36条。";
    let refs = find_refs(text);
    assert_eq!(refs.len(), 2);
    assert_eq!((refs[0].number, refs[1].number), (30, 36));
    assert_eq!(
      &text[refs[0].start..refs[0].end],
      "《业余无线电台管理办法》第三十条"
    );
    // 不在库里的法律（《民法典》）与缩写（《办法》）保持原文，不瞎链。
    let foreign = "《中华人民共和国民法典》第二百五十二条";
    assert!(find_refs(foreign).is_empty());
    assert!(find_refs("《办法》第三十条").is_empty());
    // 引用不存在的条号也不链（页面锚点会落空）。
    assert!(find_refs("《业余无线电台管理办法》第九十九条").is_empty());
    // 第二部法律也认得出来（两部混在一起也各自归位）。
    let both = "《业余无线电台管理办法》第四十七条与《中华人民共和国无线电管理条例》第七十条";
    let refs = find_refs(both);
    assert_eq!(refs.len(), 2);
    assert_eq!((refs[0].law, refs[0].number), ("办法", 47));
    assert_eq!((refs[1].law, refs[1].number), ("条例", 70));
    assert_eq!(
      &both[refs[1].start..refs[1].end],
      "《中华人民共和国无线电管理条例》第七十条"
    );
  }

  #[test]
  fn chinese_and_arabic_article_numbers_both_resolve() {
    let text = "《业余无线电台管理办法》第三十七条和《业余无线电台管理办法》第8条";
    let refs = find_refs(text);
    assert_eq!(refs.len(), 2);
    assert_eq!((refs[0].number, refs[1].number), (37, 8));
    // 中文数字换算本身也要对。
    assert_eq!(zh_number("三十七"), Some(37));
    assert_eq!(zh_number("十"), Some(10));
    assert_eq!(zh_number("二十"), Some(20));
    assert_eq!(zh_number("一百零三"), Some(103));
    assert_eq!(zh_number(""), None);
    assert_eq!(zh_number("五十八"), Some(58));
    assert_eq!(zh_number("一百"), Some(100));
    assert_eq!(zh_number("一百二十三"), Some(123));
    assert_eq!(zh_number("十三"), Some(13));
  }

  #[test]
  fn zh_number_rejects_malformed_combinations() {
    // 不成数的组合不能被「宽容」解析成某个数 —— 否则别的文字会被误当成条号。
    for bad in [
      "三三",
      "十十",
      "二二十",
      "三十七八",
      "百",
      "零三",
      "一百三",
      "一百零",
      "一百零零三",
      "三十零",
      "七十十",
      "三条",
      "abc",
    ] {
      assert_eq!(zh_number(bad), None, "「{bad}」不该被解析");
    }
  }

  #[test]
  fn to_zh_number_round_trips_with_zh_number() {
    assert_eq!(to_zh_number(7).as_deref(), Some("七"));
    assert_eq!(to_zh_number(10).as_deref(), Some("十"));
    assert_eq!(to_zh_number(13).as_deref(), Some("十三"));
    assert_eq!(to_zh_number(20).as_deref(), Some("二十"));
    assert_eq!(to_zh_number(37).as_deref(), Some("三十七"));
    assert_eq!(to_zh_number(70).as_deref(), Some("七十"));
    assert_eq!(to_zh_number(100).as_deref(), Some("一百"));
    assert_eq!(to_zh_number(103).as_deref(), Some("一百零三"));
    assert_eq!(to_zh_number(113).as_deref(), Some("一百一十三"));
    assert_eq!(to_zh_number(0), None);
    assert_eq!(to_zh_number(1000), None);
    for n in 1..=999u16 {
      let zh = to_zh_number(n).expect("1–999 都能写");
      assert_eq!(zh_number(&zh), Some(n), "{n} → {zh} 往返不一致");
    }
  }

  #[test]
  fn refs_come_back_in_text_order_even_across_laws() {
    // 「条例在前、办法在后」：按法律扫描的原始顺序是 办法 → 条例，必须按文本位置重排。
    let text = "《中华人民共和国无线电管理条例》第七十条，另见《业余无线电台管理办法》第四十七条。";
    let refs = find_refs(text);
    assert_eq!(refs.len(), 2);
    assert_eq!((refs[0].law, refs[0].number), ("条例", 70));
    assert_eq!((refs[1].law, refs[1].number), ("办法", 47));
    assert!(refs[0].end <= refs[1].start, "引用区间不能重叠 / 回退");
    assert!(refs.windows(2).all(|w| w[0].start < w[1].start));
  }

  #[test]
  fn every_mapped_question_has_an_article_and_vice_versa() {
    for m in ARTICLE_QUESTIONS {
      assert!(
        article(m.law, m.number).is_some(),
        "{} 第{}条不在条文库里",
        m.law,
        m.number
      );
      assert!(
        !m.questions.is_empty(),
        "{} 第{}条的映射是空的",
        m.law,
        m.number
      );
      assert_eq!(questions_for(m.law, m.number), m.questions);
      for q in m.questions.iter() {
        assert!(
          !q.id.is_empty() && !q.p_code.is_empty() && !q.stem.is_empty(),
          "{} 第{}条",
          m.law,
          m.number
        );
      }
    }
    // 办法与条例的第 1 条都有映射，但各自只取自己的（条号会撞，法律维度必须生效）。
    assert_eq!(questions_for("办法", 1).len(), 1);
    assert_eq!(questions_for("条例", 1).len(), 1);
    assert_ne!(
      questions_for("办法", 1)[0].id,
      questions_for("条例", 1)[0].id
    );
    // 没映射的条返回空（页面据此不显示「题目」那一栏）。
    assert!(questions_for("办法", 2).is_empty());
  }
}
