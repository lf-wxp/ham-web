//! `exam:wrong:{A|B|C}`：模拟考试交卷后暂存的「本次错题」，每个题库一份，每份是若干道题的快照。
//!
//! 认领一族 key（[`Key::Prefix`]）。题库就是 key 的后缀，表里存成 `bank` 列。
//! 题面拆成列 + 子表（选项、标准答案），与错题本里的题目快照同一个形状。

use ham_web_core::bank::Bank;
use ham_web_core::question::{Codes, QuestionItem, QuestionOption, QuestionType};
use rusqlite::{Connection, Transaction, params};

use super::{Key, Table, corrupt, enum_from_text, enum_text, pages_cols, pages_of};

const PREFIX: &str = "exam:wrong:";

pub(super) struct WrongQuestions {
  bank: Bank,
  items: Vec<QuestionItem>,
}

/// key 后缀里的题库。`Bank` 只认大写的 `A` / `B` / `C`，所以每个题库恰好对应一个 key，
/// 不会有两个 key 映射到同一批行；其余写法（小写、别的字母、多余的后缀）认不出，留在目录里原样存放。
fn bank_of(key: &str) -> Option<Bank> {
  key.strip_prefix(PREFIX)?.parse().ok()
}

impl Table for WrongQuestions {
  const KEY: Key = Key::Prefix(PREFIX);
  const TABLES: &'static [&'static str] = &[
    "wrong_question_answers",
    "wrong_question_options",
    "wrong_questions",
  ];

  fn decode(key: &str, raw: &str) -> Option<Self> {
    Some(Self {
      bank: bank_of(key)?,
      items: serde_json::from_str::<Vec<QuestionItem>>(raw).ok()?,
    })
  }

  fn encode(&self) -> Option<String> {
    serde_json::to_string(&self.items).ok()
  }

  fn insert(&self, tx: &Transaction<'_>, user: i64, _key: &str) -> rusqlite::Result<()> {
    let bank = enum_text(&self.bank)?;
    let mut head = tx.prepare_cached(
      "INSERT INTO wrong_questions
         (user_id, bank, pos, q_id, code_j, code_p, question, kind, image_url, explanation,
          has_pages, pages_start, pages_end)
       VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
    )?;
    let mut option = tx.prepare_cached(
      "INSERT INTO wrong_question_options (user_id, bank, pos, opt_pos, opt_key, text)
       VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
    )?;
    let mut answer = tx.prepare_cached(
      "INSERT INTO wrong_question_answers (user_id, bank, pos, ans_pos, value)
       VALUES (?1, ?2, ?3, ?4, ?5)",
    )?;
    for (pos, q) in self.items.iter().enumerate() {
      // 解构不带 `..`：给 `QuestionItem` 加了字段而忘了建列，会直接编译失败而不是悄悄丢数据。
      let QuestionItem {
        id,
        codes,
        question,
        options,
        answer_keys,
        kind,
        pages,
        image_url,
        explanation,
      } = q;
      let pos = pos as i64;
      let (has_pages, pages_start, pages_end) = pages_cols(*pages);
      head.execute(params![
        user,
        bank,
        pos,
        id,
        codes.j,
        codes.p,
        question,
        enum_text(kind)?,
        image_url,
        explanation,
        has_pages,
        pages_start,
        pages_end
      ])?;
      for (i, o) in options.iter().enumerate() {
        option.execute(params![user, bank, pos, i as i64, o.key, o.text])?;
      }
      for (i, v) in answer_keys.iter().enumerate() {
        answer.execute(params![user, bank, pos, i as i64, v])?;
      }
    }
    Ok(())
  }

  fn load(conn: &Connection, user: i64, key: &str) -> rusqlite::Result<Self> {
    let bank = bank_of(key).ok_or_else(|| corrupt(format!("{key} 不是错题暂存的 key")))?;
    let bank_text = enum_text(&bank)?;

    let mut items: Vec<QuestionItem> = conn
      .prepare_cached(
        "SELECT q_id, code_j, code_p, question, kind, image_url, explanation,
                has_pages, pages_start, pages_end
         FROM wrong_questions WHERE user_id = ?1 AND bank = ?2 ORDER BY pos",
      )?
      .query_map(params![user, bank_text], |r| {
        Ok(QuestionItem {
          id: r.get(0)?,
          codes: Codes {
            j: r.get(1)?,
            p: r.get(2)?,
          },
          question: r.get(3)?,
          options: Vec::new(),
          answer_keys: Vec::new(),
          kind: enum_from_text::<QuestionType>(&r.get::<_, String>(4)?)?,
          pages: pages_of(r.get(7)?, r.get(8)?, r.get(9)?),
          image_url: r.get(5)?,
          explanation: r.get(6)?,
        })
      })?
      .collect::<rusqlite::Result<_>>()?;

    // 子表按 (pos, 序号) 读出来直接追加到对应题上：顺序就是原顺序。
    for (pos, o) in conn
      .prepare_cached(
        "SELECT pos, opt_key, text FROM wrong_question_options
         WHERE user_id = ?1 AND bank = ?2 ORDER BY pos, opt_pos",
      )?
      .query_map(params![user, bank_text], |r| {
        Ok((
          r.get::<_, i64>(0)?,
          QuestionOption {
            key: r.get(1)?,
            text: r.get(2)?,
          },
        ))
      })?
      .collect::<rusqlite::Result<Vec<_>>>()?
    {
      if let Some(q) = usize::try_from(pos).ok().and_then(|i| items.get_mut(i)) {
        q.options.push(o);
      }
    }
    for (pos, v) in conn
      .prepare_cached(
        "SELECT pos, value FROM wrong_question_answers
         WHERE user_id = ?1 AND bank = ?2 ORDER BY pos, ans_pos",
      )?
      .query_map(params![user, bank_text], |r| {
        Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?))
      })?
      .collect::<rusqlite::Result<Vec<_>>>()?
    {
      if let Some(q) = usize::try_from(pos).ok().and_then(|i| items.get_mut(i)) {
        q.answer_keys.push(v);
      }
    }
    Ok(Self { bank, items })
  }

  /// 只删这一个题库的行：同一族里别的题库不动。
  fn clear(conn: &Connection, user: i64, key: &str) -> rusqlite::Result<()> {
    let Some(bank) = bank_of(key) else {
      return Ok(());
    };
    conn
      .execute(
        "DELETE FROM wrong_questions WHERE user_id = ?1 AND bank = ?2",
        params![user, enum_text(&bank)?],
      )
      .map(|_| ())
  }
}
