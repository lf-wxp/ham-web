-- 账号名下的同步数据：云端备份拆成的多张表。
--
-- 约定（新增表照这个来）：
--   · 全表 STRICT：列类型是硬约束，写错类型当场报错，而不是悄悄存成别的东西；
--   · 第一列是 user_id，外键级联到 users：注销账号一次清干净；
--   · 能当主键的自然键就当主键；有先后顺序、顺序本身是数据的列表，用 pos 保序；
--   · 子表用复合外键级联到父表；
--   · 枚举列的 CHECK 只收「解析阶段一定会产出」的取值——约束比解析更严，就会让一份合法的备份
--     在落库时才失败（整次导入返回 500），所以写约束前先对照 core 里的解析；
--   · 布尔存 0 / 1；u64 按位存进 INTEGER（i64），读回时还原；
--   · 唯一的 JSON 列：练习 / 考试的断点进度里按题位对齐的数组（整体读写、与别的行没有关系，
--     拆成几千行没有任何查询上的好处）。其余结构化数据都拆成行。
--
-- 哪些 key 用哪张表见 `relational/` 里各个 `Table` 的 `KEY`；没有专表的 key 在 user_kv。

-- 云端备份的概况（内容在下面各表里）。
CREATE TABLE backup_meta (
  user_id     INTEGER PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
  -- 内容指纹（core::delta::manifest_fingerprint）：对云端全部同步 key 的 (key, 摘要) 清单求的值，
  -- 也是前端的同步基线与 If-Match 的比较对象。存十进制文本：u64 超出 SQLite INTEGER（i64）的范围。
  fingerprint TEXT    NOT NULL,
  size        INTEGER NOT NULL,                  -- 全部 key 的规范文本字节数之和
  updated_at  INTEGER NOT NULL
) STRICT;

-- 目录：账号的每个同步 key 恰好一行。
--   inline：值就在 value 里（设置、没有专表的 key、解析不了的内容……）；
--   table ：值在专表里，value 为 NULL（没有这一行，`[]` 与「没有这个 key」就分不开）。
-- digest 是**规范文本**的 SHA-256（core::delta::Digest）：没变的 key 不重写；它也是云端清单里这个 key 的摘要，
-- 客户端用同一个函数对本机内容求，两边直接可比。bytes 是规范文本的字节数，账号总量的上限靠它核。
CREATE TABLE user_kv (
  user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  key     TEXT    NOT NULL,
  value   TEXT,
  storage TEXT    NOT NULL CHECK (storage IN ('inline', 'table')),
  digest  BLOB    NOT NULL CHECK (length(digest) = 32),
  bytes   INTEGER NOT NULL CHECK (bytes >= 0),
  PRIMARY KEY (user_id, key),
  CHECK ((storage = 'inline') = (value IS NOT NULL))
) STRICT, WITHOUT ROWID;

-- ───────────────────────── 通联与台站 ─────────────────────────

-- 通联日志（key = logbook）。id 是日志自己的自增编号，账号内唯一；pos 保留数组顺序。
CREATE TABLE qsos (
  user_id          INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  id               INTEGER NOT NULL,
  pos              INTEGER NOT NULL,
  operator         TEXT    NOT NULL,
  station_id       INTEGER NOT NULL,             -- 指向 station_profiles.id；0 或悬空 = 当前台站，故不设外键
  date             TEXT    NOT NULL,             -- YYYY-MM-DD（解析不了的原样留着，宁可留着怪写法也不改记录）
  time             TEXT    NOT NULL,             -- HH:MM
  time_off         TEXT    NOT NULL,
  freq             TEXT    NOT NULL,             -- MHz，文本（保留用户写法）
  band             TEXT    NOT NULL,
  mode             TEXT    NOT NULL,
  callsign         TEXT    NOT NULL,
  rst              TEXT    NOT NULL,
  rst_sent         TEXT    NOT NULL,
  rst_rcvd         TEXT    NOT NULL,
  tx_pwr           TEXT    NOT NULL,
  gridsquare       TEXT    NOT NULL,
  name             TEXT    NOT NULL,
  qth              TEXT    NOT NULL,
  prop_mode        TEXT    NOT NULL,
  sat_name         TEXT    NOT NULL,
  my_sota_ref      TEXT    NOT NULL,
  my_pota_ref      TEXT    NOT NULL,
  contact_sota_ref TEXT    NOT NULL,
  contact_pota_ref TEXT    NOT NULL,
  contest_id       TEXT    NOT NULL,
  stx              TEXT    NOT NULL,
  srx              TEXT    NOT NULL,
  dxcc             TEXT    NOT NULL,
  cqz              TEXT    NOT NULL,
  ituz             TEXT    NOT NULL,
  state            TEXT    NOT NULL,
  iota             TEXT    NOT NULL,
  remark           TEXT    NOT NULL,
  qsl_sent         INTEGER NOT NULL CHECK (qsl_sent IN (0, 1)),
  qsl_sent_via     TEXT    NOT NULL CHECK (qsl_sent_via IN ('none', 'bureau', 'direct')),
  qsl_rcvd         INTEGER NOT NULL CHECK (qsl_rcvd IN (0, 1)),
  lotw_sent        INTEGER NOT NULL CHECK (lotw_sent IN (0, 1)),
  lotw_rcvd        INTEGER NOT NULL CHECK (lotw_rcvd IN (0, 1)),
  eqsl_sent        INTEGER NOT NULL CHECK (eqsl_sent IN (0, 1)),
  eqsl_rcvd        INTEGER NOT NULL CHECK (eqsl_rcvd IN (0, 1)),
  qrz_rcvd         INTEGER NOT NULL CHECK (qrz_rcvd IN (0, 1)),
  PRIMARY KEY (user_id, id)
) STRICT;
-- 整份读取按 `pos` 取行：没有这个索引，每次都要把该账号的全部行读出来再临时排序。
CREATE INDEX qsos_user_pos ON qsos(user_id, pos);

-- 台站档案（key = station-book）：册头一行，档案若干行。
CREATE TABLE station_books (
  user_id   INTEGER PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
  next_id   INTEGER NOT NULL,
  active_id INTEGER NOT NULL
) STRICT;
CREATE TABLE station_profiles (
  user_id    INTEGER NOT NULL REFERENCES station_books(user_id) ON DELETE CASCADE,
  id         INTEGER NOT NULL,
  pos        INTEGER NOT NULL,
  label      TEXT    NOT NULL,
  callsign   TEXT    NOT NULL,
  operator   TEXT    NOT NULL,
  gridsquare TEXT    NOT NULL,
  rig        TEXT    NOT NULL,
  antenna    TEXT    NOT NULL,
  PRIMARY KEY (user_id, id)
) STRICT;

-- QSL 卡片影像（key = qsl-image:<通联编号>）：类型 + 二进制，比 base64 文本小 1/3。
CREATE TABLE qsl_images (
  user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  qso_id  INTEGER NOT NULL,                      -- 指向 qsos.id（日志可能在 user_kv 里，故不设外键）
  mime    TEXT    NOT NULL CHECK (mime IN ('image/jpeg', 'image/png', 'image/webp')),
  data    BLOB    NOT NULL,
  PRIMARY KEY (user_id, qso_id)
) STRICT;
-- 存了影像的通联编号（key = qsl-image-index）。
CREATE TABLE qsl_image_index (
  user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  pos     INTEGER NOT NULL,
  qso_id  INTEGER NOT NULL,
  PRIMARY KEY (user_id, pos)
) STRICT;

-- ───────────────────────── 错题与考试 ─────────────────────────

-- 错题本（key = mistake-book）。key 是题目内容指纹，与题号、题库无关；
-- 题目快照拆成列 + 子表：题面、选项、标准答案、我的作答、错过的题库。
CREATE TABLE mistakes (
  user_id        INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  key            TEXT    NOT NULL,
  pos            INTEGER NOT NULL,
  q_id           TEXT,                           -- 题库内题目 ID，如 A-1
  code_j         TEXT,                           -- 官方题号
  code_p         TEXT,                           -- 官方分类码
  question       TEXT    NOT NULL,
  kind           TEXT    NOT NULL CHECK (kind IN ('single', 'multiple')),
  image_url      TEXT,
  explanation    TEXT,
  has_pages      INTEGER NOT NULL CHECK (has_pages IN (0, 1)),  -- 历史字段 pages：有没有、起止页
  pages_start    INTEGER,
  pages_end      INTEGER,
  wrong_count    INTEGER NOT NULL,
  streak         INTEGER NOT NULL,
  last_wrong_ms  INTEGER NOT NULL,
  due_ms         INTEGER NOT NULL,
  ease           REAL    NOT NULL,
  interval_days  REAL    NOT NULL,
  last_review_ms INTEGER NOT NULL,
  cause          TEXT,                           -- 错因（用户自评），NULL = 未标注
  PRIMARY KEY (user_id, key)
) STRICT;
CREATE TABLE mistake_options (
  user_id INTEGER NOT NULL,
  key     TEXT    NOT NULL,
  pos     INTEGER NOT NULL,
  opt_key TEXT    NOT NULL,                      -- 选项字母
  text    TEXT    NOT NULL,
  PRIMARY KEY (user_id, key, pos),
  FOREIGN KEY (user_id, key) REFERENCES mistakes(user_id, key) ON DELETE CASCADE
) STRICT, WITHOUT ROWID;
-- 标准答案（answer）与我的作答（mine）。
CREATE TABLE mistake_choices (
  user_id INTEGER NOT NULL,
  key     TEXT    NOT NULL,
  kind    TEXT    NOT NULL CHECK (kind IN ('answer', 'mine')),
  pos     INTEGER NOT NULL,
  value   TEXT    NOT NULL,
  PRIMARY KEY (user_id, key, kind, pos),
  FOREIGN KEY (user_id, key) REFERENCES mistakes(user_id, key) ON DELETE CASCADE
) STRICT, WITHOUT ROWID;
CREATE TABLE mistake_banks (
  user_id INTEGER NOT NULL,
  key     TEXT    NOT NULL,
  bank    TEXT    NOT NULL CHECK (bank IN ('A', 'B', 'C')),
  PRIMARY KEY (user_id, key, bank),
  FOREIGN KEY (user_id, key) REFERENCES mistakes(user_id, key) ON DELETE CASCADE
) STRICT, WITHOUT ROWID;

-- 模拟考试成绩（key = exam-history）。没有天然唯一键，按 pos 排。
CREATE TABLE exam_results (
  user_id   INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  pos       INTEGER NOT NULL,
  bank      TEXT    NOT NULL,
  correct   INTEGER NOT NULL,
  total     INTEGER NOT NULL,
  timestamp INTEGER NOT NULL,                    -- 交卷时间，毫秒
  weak      INTEGER NOT NULL CHECK (weak IN (0, 1)),
  PRIMARY KEY (user_id, pos)
) STRICT;

-- 最近一次考试的逐题复盘（key = exam:last-review）。
CREATE TABLE exam_reviews (
  user_id        INTEGER PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
  bank           TEXT    NOT NULL,
  finished_at_ms INTEGER NOT NULL
) STRICT;
CREATE TABLE exam_review_items (
  user_id     INTEGER NOT NULL REFERENCES exam_reviews(user_id) ON DELETE CASCADE,
  pos         INTEGER NOT NULL,
  id          TEXT    NOT NULL,
  code        TEXT    NOT NULL,
  question    TEXT    NOT NULL,
  explanation TEXT    NOT NULL,
  category    TEXT    NOT NULL,
  PRIMARY KEY (user_id, pos)
) STRICT;
CREATE TABLE exam_review_options (
  user_id INTEGER NOT NULL,
  pos     INTEGER NOT NULL,
  opt_pos INTEGER NOT NULL,
  text    TEXT    NOT NULL,
  PRIMARY KEY (user_id, pos, opt_pos),
  FOREIGN KEY (user_id, pos) REFERENCES exam_review_items(user_id, pos) ON DELETE CASCADE
) STRICT, WITHOUT ROWID;
-- 标准答案（answer）与用户作答（given）。
CREATE TABLE exam_review_choices (
  user_id    INTEGER NOT NULL,
  pos        INTEGER NOT NULL,
  kind       TEXT    NOT NULL CHECK (kind IN ('answer', 'given')),
  choice_pos INTEGER NOT NULL,
  value      TEXT    NOT NULL,
  PRIMARY KEY (user_id, pos, kind, choice_pos),
  FOREIGN KEY (user_id, pos) REFERENCES exam_review_items(user_id, pos) ON DELETE CASCADE
) STRICT, WITHOUT ROWID;

-- 练习 / 考试的断点进度：key = practice:{版本}:{题库} / exam:savedState:{版本}:{题库}，一个 key 一行。
-- 按题位的数组（题序、作答、标记）长度等于题数，整体读写，存 JSON 文本——全库唯一的 JSON 列。
CREATE TABLE practice_progress (
  user_id          INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  key              TEXT    NOT NULL,
  version          INTEGER NOT NULL,
  bank             TEXT    NOT NULL CHECK (bank IN ('A', 'B', 'C')),
  version_id       TEXT,
  timestamp        INTEGER NOT NULL,
  current_index    INTEGER NOT NULL,
  order_mode       TEXT    NOT NULL,
  show_answer      INTEGER NOT NULL CHECK (show_answer IN (0, 1)),
  show_explanation INTEGER CHECK (show_explanation IN (0, 1)),
  total            INTEGER NOT NULL,
  order_indices    TEXT    NOT NULL CHECK (json_valid(order_indices)),
  answers          TEXT    NOT NULL CHECK (json_valid(answers)),
  PRIMARY KEY (user_id, key)
) STRICT;
CREATE TABLE exam_progress (
  user_id            INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  key                TEXT    NOT NULL,
  version            INTEGER NOT NULL,
  bank               TEXT    NOT NULL CHECK (bank IN ('A', 'B', 'C')),
  version_id         TEXT,
  timestamp          INTEGER NOT NULL,
  end_at_ms          INTEGER NOT NULL,
  current_index      INTEGER NOT NULL,
  answers            TEXT    NOT NULL CHECK (json_valid(answers)),
  flags              TEXT    NOT NULL CHECK (json_valid(flags)),
  total              INTEGER NOT NULL,
  question_ids       TEXT    CHECK (json_valid(question_ids)),        -- NULL = 原文里没有这个字段
  questions_snapshot TEXT    CHECK (json_valid(questions_snapshot)),  -- 同上
  PRIMARY KEY (user_id, key)
) STRICT;

-- ───────────────────────── 学习统计 ─────────────────────────

-- 学习统计（key = study-stats）：范围（bank 为空 = 全部，否则 A / B / C）→ 分项与做过的题。
CREATE TABLE study_scopes (
  user_id  INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  bank     TEXT    NOT NULL CHECK (bank IN ('', 'A', 'B', 'C')),
  answered INTEGER NOT NULL,
  correct  INTEGER NOT NULL,
  PRIMARY KEY (user_id, bank)
) STRICT;
CREATE TABLE study_tallies (
  user_id  INTEGER NOT NULL,
  bank     TEXT    NOT NULL,
  kind     TEXT    NOT NULL CHECK (kind IN ('category', 'sub')),  -- 一级分类 / 官方分类码
  key      TEXT    NOT NULL,
  answered INTEGER NOT NULL,
  correct  INTEGER NOT NULL,
  PRIMARY KEY (user_id, bank, kind, key),
  FOREIGN KEY (user_id, bank) REFERENCES study_scopes(user_id, bank) ON DELETE CASCADE
) STRICT;
CREATE TABLE study_seen (
  user_id      INTEGER NOT NULL,
  bank         TEXT    NOT NULL,
  question_key TEXT    NOT NULL,                 -- 题目内容指纹
  PRIMARY KEY (user_id, bank, question_key),
  FOREIGN KEY (user_id, bank) REFERENCES study_scopes(user_id, bank) ON DELETE CASCADE
) STRICT, WITHOUT ROWID;

-- 单题统计（key = question-stats）。
CREATE TABLE question_stats (
  user_id      INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  question_key TEXT    NOT NULL,
  attempts     INTEGER NOT NULL,
  correct      INTEGER NOT NULL,
  last_ms      INTEGER NOT NULL,
  duration_ms  INTEGER NOT NULL,
  PRIMARY KEY (user_id, question_key)
) STRICT;

-- 按日期的记录：每日作答量（key = study-daily）、每日挑战成绩（key = daily-challenge）。
CREATE TABLE study_daily (
  user_id     INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  day         TEXT    NOT NULL,                  -- YYYY-MM-DD
  answered    INTEGER NOT NULL,
  new_count   INTEGER NOT NULL,                  -- 其中第一次做的题数
  duration_ms INTEGER NOT NULL,
  PRIMARY KEY (user_id, day)
) STRICT;
CREATE TABLE daily_challenge (
  user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  day     TEXT    NOT NULL,
  correct INTEGER NOT NULL,
  total   INTEGER NOT NULL,
  PRIMARY KEY (user_id, day)
) STRICT;

-- 收藏题目（key = bookmarks）：集合，顺序是前端写出的顺序，原样保留。
CREATE TABLE bookmarks (
  user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  pos     INTEGER NOT NULL,
  value   TEXT    NOT NULL,
  PRIMARY KEY (user_id, pos)
) STRICT;
-- 收藏分组（key = bookmark-groups）：分组与分组下的题目都按 pos 保序。
CREATE TABLE bookmark_groups (
  user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  pos     INTEGER NOT NULL,
  name    TEXT    NOT NULL,
  PRIMARY KEY (user_id, pos)
) STRICT;
CREATE TABLE bookmark_group_items (
  user_id   INTEGER NOT NULL,
  group_pos INTEGER NOT NULL,
  pos       INTEGER NOT NULL,
  stable_id TEXT    NOT NULL,
  PRIMARY KEY (user_id, group_pos, pos),
  FOREIGN KEY (user_id, group_pos) REFERENCES bookmark_groups(user_id, pos) ON DELETE CASCADE
) STRICT, WITHOUT ROWID;

-- 题目私人笔记（key = question-notes）：pos 保住前端写出的键序。
CREATE TABLE question_notes (
  user_id   INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  pos       INTEGER NOT NULL,
  stable_id TEXT    NOT NULL,
  note      TEXT    NOT NULL,
  PRIMARY KEY (user_id, pos),
  UNIQUE (user_id, stable_id)
) STRICT;

-- 知识卡片复习（key = card-review）：进度头 + 每张卡 + 今日各卡组新卡数。
CREATE TABLE card_schedule (
  user_id INTEGER PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
  new_day TEXT    NOT NULL                       -- 「今天」是哪天（本地日期）
) STRICT;
CREATE TABLE card_states (
  user_id       INTEGER NOT NULL REFERENCES card_schedule(user_id) ON DELETE CASCADE,
  id            TEXT    NOT NULL,                -- 形如 q:QRM，前缀是卡组
  ease          REAL    NOT NULL,
  interval_days REAL    NOT NULL,
  due_ms        INTEGER NOT NULL,
  last_ms       INTEGER NOT NULL,
  reps          INTEGER NOT NULL,
  lapses        INTEGER NOT NULL,
  PRIMARY KEY (user_id, id)
) STRICT;
CREATE TABLE card_new_today (
  user_id INTEGER NOT NULL REFERENCES card_schedule(user_id) ON DELETE CASCADE,
  deck    TEXT    NOT NULL CHECK (deck IN ('qcode', 'abbrev', 'glossary', 'phonetic', 'morse')),
  count   INTEGER NOT NULL,
  PRIMARY KEY (user_id, deck)
) STRICT;

-- 闯关：关卡历史最佳星级（key = rpg-stars）、怪物图鉴（key = rpg-bestiary）。
CREATE TABLE stage_stars (
  user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  stage   TEXT    NOT NULL,                      -- {题库}:{分类key}
  stars   INTEGER NOT NULL CHECK (stars BETWEEN 0 AND 255),
  PRIMARY KEY (user_id, stage)
) STRICT;
CREATE TABLE bestiary (
  user_id      INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  question_key TEXT    NOT NULL,
  PRIMARY KEY (user_id, question_key)
) STRICT, WITHOUT ROWID;

-- 已提示过的成就解锁 id（key = achievements-seen）。
CREATE TABLE achievements_seen (
  user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  pos     INTEGER NOT NULL,
  value   TEXT    NOT NULL,
  PRIMARY KEY (user_id, pos)
) STRICT;

-- ───────────────────────── 莫尔斯与电台 ─────────────────────────

-- Koch 抄收进度（key = morse-koch）、CW 呼号抄收成绩（key = morse-runner）。
-- 按字符的计数，字符就是 JSON 对象的键；库里存成一个字符的文本，读回时校验。
CREATE TABLE koch_progress (
  user_id       INTEGER PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
  level         INTEGER NOT NULL,
  char_wpm      INTEGER NOT NULL,
  effective_wpm INTEGER NOT NULL,
  level_sent    INTEGER NOT NULL,
  level_correct INTEGER NOT NULL
) STRICT;
CREATE TABLE koch_chars (
  user_id INTEGER NOT NULL REFERENCES koch_progress(user_id) ON DELETE CASCADE,
  ch      TEXT    NOT NULL,
  sent    INTEGER NOT NULL,
  correct INTEGER NOT NULL,
  PRIMARY KEY (user_id, ch)
) STRICT;
CREATE TABLE runner_stats (
  user_id     INTEGER PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
  wpm         INTEGER NOT NULL,
  adaptive    INTEGER NOT NULL CHECK (adaptive IN (0, 1)),
  cut_numbers INTEGER NOT NULL CHECK (cut_numbers IN (0, 1)),
  pileup      INTEGER NOT NULL CHECK (pileup IN (0, 1)),
  sessions    INTEGER NOT NULL,
  qsos        INTEGER NOT NULL,
  calls_ok    INTEGER NOT NULL,
  best_score  INTEGER NOT NULL,
  top_wpm     INTEGER NOT NULL
) STRICT;
CREATE TABLE runner_missed (
  user_id INTEGER NOT NULL REFERENCES runner_stats(user_id) ON DELETE CASCADE,
  ch      TEXT    NOT NULL,
  missed  INTEGER NOT NULL,
  PRIMARY KEY (user_id, ch)
) STRICT;

-- 已通联的 DXCC 稀有前缀（key = dxcc_wanted_done），按点击先后。
CREATE TABLE dxcc_wanted_done (
  user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  pos     INTEGER NOT NULL,
  value   TEXT    NOT NULL,
  PRIMARY KEY (user_id, pos)
) STRICT;

-- ───────────────────────── 设置 ─────────────────────────

-- 备考计划（key = learning-plan）、DX 热点提醒（key = dx-alerts）、卫星过境提醒（key = sat-watch）。
CREATE TABLE study_plan (
  user_id       INTEGER PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
  target_date   TEXT    NOT NULL,
  bank          TEXT    CHECK (bank IN ('A', 'B', 'C')),   -- NULL = 取作答最多的题库
  reminder_time TEXT    NOT NULL
) STRICT;
CREATE TABLE dx_alerts (
  user_id  INTEGER PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
  new_dxcc INTEGER NOT NULL CHECK (new_dxcc IN (0, 1)),
  new_band INTEGER NOT NULL CHECK (new_band IN (0, 1))
) STRICT;
CREATE TABLE dx_alert_calls (
  user_id INTEGER NOT NULL REFERENCES dx_alerts(user_id) ON DELETE CASCADE,
  pos     INTEGER NOT NULL,
  pattern TEXT    NOT NULL,
  PRIMARY KEY (user_id, pos)
) STRICT;
CREATE TABLE sat_watch (
  user_id   INTEGER PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
  lat       REAL    NOT NULL,
  lon       REAL    NOT NULL,
  min_elev  REAL    NOT NULL,
  alerts    INTEGER NOT NULL CHECK (alerts IN (0, 1)),
  lead_min  INTEGER NOT NULL,
  apt_alert INTEGER NOT NULL CHECK (apt_alert IN (0, 1))
) STRICT;
CREATE TABLE sat_favorites (
  user_id INTEGER NOT NULL REFERENCES sat_watch(user_id) ON DELETE CASCADE,
  pos     INTEGER NOT NULL,
  norad   INTEGER NOT NULL,
  name    TEXT    NOT NULL,
  PRIMARY KEY (user_id, pos)
) STRICT;

-- ───────────────────── 前端页面自己的小文档 ─────────────────────
-- 这些类型原来只定义在页面里，现在搬进了 core（账号同步的服务端要存它们，前后端必须是同一份定义）。

-- 倒计时（key = countdowns）。id 是本机自增编号，列表内唯一。
CREATE TABLE countdown_events (
  user_id   INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  id        INTEGER NOT NULL,
  pos       INTEGER NOT NULL,
  title     TEXT    NOT NULL,
  target_ms INTEGER NOT NULL,                    -- 目标时间，毫秒时间戳
  PRIMARY KEY (user_id, id)
) STRICT;

-- 首页每日打卡（key = daily-checkin）。
CREATE TABLE daily_checkin (
  user_id   INTEGER PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
  last_date TEXT    NOT NULL,                    -- YYYYMMDD（UTC）
  streak    INTEGER NOT NULL
) STRICT;

-- 摩尔斯：解码练习统计（key = morse-stats）、解码练习设置（key = morse-trainer）、
-- 发报统计（key = morse-send）、播放设置（key = morse-settings）。
-- 枚举列的取值就是 serde 写出的变体名；core 里有测试逐个变体核对。
CREATE TABLE morse_stats (
  user_id       INTEGER PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
  correct       INTEGER NOT NULL,
  wrong         INTEGER NOT NULL,
  best_streak   INTEGER NOT NULL,
  top_wpm       REAL    NOT NULL,
  today         TEXT    NOT NULL,                -- YYYY-MM-DD
  today_correct INTEGER NOT NULL,
  today_wrong   INTEGER NOT NULL
) STRICT;
CREATE TABLE morse_mistakes (
  user_id INTEGER NOT NULL REFERENCES morse_stats(user_id) ON DELETE CASCADE,
  ch      TEXT    NOT NULL,                      -- 答案文本（单字符或一个词）
  count   INTEGER NOT NULL,
  PRIMARY KEY (user_id, ch)
) STRICT;
CREATE TABLE morse_daily (
  user_id INTEGER NOT NULL REFERENCES morse_stats(user_id) ON DELETE CASCADE,
  day     TEXT    NOT NULL,
  correct INTEGER NOT NULL,
  wrong   INTEGER NOT NULL,
  PRIMARY KEY (user_id, day)
) STRICT;
CREATE TABLE morse_trainer (
  user_id  INTEGER PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
  mode     TEXT    NOT NULL CHECK (mode IN ('Listen', 'Read', 'Choice')),
  scope    TEXT    NOT NULL CHECK (scope IN ('Letters', 'Digits', 'Punctuation', 'Both', 'All', 'Weak')),
  target   TEXT    NOT NULL CHECK (target IN ('Single', 'Word', 'CallSign', 'Sentence')),
  wpm      REAL    NOT NULL,
  eff_wpm  REAL    NOT NULL,
  adaptive INTEGER NOT NULL CHECK (adaptive IN (0, 1))
) STRICT;
CREATE TABLE morse_send (
  user_id     INTEGER PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
  correct     INTEGER NOT NULL,
  wrong       INTEGER NOT NULL,
  best_streak INTEGER NOT NULL
) STRICT;
CREATE TABLE morse_settings (
  user_id INTEGER PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
  tone_hz INTEGER NOT NULL,
  volume  REAL    NOT NULL                       -- f32 按位无损地放进 REAL
) STRICT;

-- 竞赛录入当前场次（key = contest-session）。`end` 是 SQL 关键字，列名用 end_at。
CREATE TABLE contest_session (
  user_id         INTEGER PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
  contest_id      TEXT    NOT NULL,
  start           TEXT    NOT NULL,              -- UTC，YYYY-MM-DD HH:MM
  end_at          TEXT    NOT NULL,              -- 空 = 未限定结束时间
  station_id      INTEGER NOT NULL,
  my_exch         TEXT    NOT NULL,
  freq            TEXT    NOT NULL,
  mode            TEXT    NOT NULL,
  power           TEXT    NOT NULL,
  target_qsos     INTEGER NOT NULL,
  planned_minutes INTEGER NOT NULL
) STRICT;

-- 网格地图筛选（key = grid-map-filters）。
CREATE TABLE grid_filters (
  user_id  INTEGER PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
  band     TEXT,
  mode     TEXT,
  dxcc     TEXT,
  year     TEXT,
  callsign TEXT    NOT NULL,
  qsl      TEXT    CHECK (qsl IN ('Confirmed', 'SentPending', 'NotSent')),
  paths    INTEGER NOT NULL CHECK (paths IN (0, 1))
) STRICT;

-- 听题模式设置（key = listen-settings）：各题源上次听到的位置是子表。
CREATE TABLE listen_settings (
  user_id    INTEGER PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
  source     TEXT    NOT NULL CHECK (source IN ('a', 'b', 'c', 'mistakes', 'bookmarks')),
  rate       REAL    NOT NULL,
  think_secs INTEGER NOT NULL,
  explain    INTEGER NOT NULL CHECK (explain IN (0, 1)),
  shuffle    INTEGER NOT NULL CHECK (shuffle IN (0, 1))
) STRICT;
CREATE TABLE listen_positions (
  user_id  INTEGER NOT NULL REFERENCES listen_settings(user_id) ON DELETE CASCADE,
  source   TEXT    NOT NULL CHECK (source IN ('a', 'b', 'c', 'mistakes', 'bookmarks')),
  position INTEGER NOT NULL,
  PRIMARY KEY (user_id, source)
) STRICT;

-- ───────────────────── 核心里本来就有类型、但之前没建表的 ─────────────────────

-- 本台信息（key = station-info）：当前台站的镜像。
CREATE TABLE station_info (
  user_id    INTEGER PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
  callsign   TEXT    NOT NULL,
  operator   TEXT    NOT NULL,
  gridsquare TEXT    NOT NULL,
  rig        TEXT    NOT NULL,
  antenna    TEXT    NOT NULL
) STRICT;

-- 器材实测值（key = gear-measurements）：机型 id → 实测。数值都可以缺（NULL）。
CREATE TABLE gear_measurements (
  user_id             INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  gear_id             TEXT    NOT NULL,
  current_a           REAL,
  noise_floor_dbm     REAL,
  rmdr_db             REAL,
  phase_noise_10k_dbc REAL,
  measured_on         TEXT    NOT NULL,          -- YYYY-MM-DD；空 = 老数据或没记
  PRIMARY KEY (user_id, gear_id)
) STRICT;

-- DIY 项目制作进度（key = diy-progress）：项目名 → 是否完成、已采购的材料。
CREATE TABLE diy_progress (
  user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  project TEXT    NOT NULL,
  done    INTEGER NOT NULL CHECK (done IN (0, 1)),
  PRIMARY KEY (user_id, project)
) STRICT;
CREATE TABLE diy_bought (
  user_id INTEGER NOT NULL,
  project TEXT    NOT NULL,
  item    TEXT    NOT NULL,                      -- BOM 材料名
  PRIMARY KEY (user_id, project, item),
  FOREIGN KEY (user_id, project) REFERENCES diy_progress(user_id, project) ON DELETE CASCADE
) STRICT, WITHOUT ROWID;

-- 噪声本底记录（key = noise-floor-samples）：按记录顺序。
CREATE TABLE noise_samples (
  user_id        INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  pos            INTEGER NOT NULL,
  label          TEXT    NOT NULL,
  at_ms          REAL    NOT NULL,               -- Unix 毫秒（前端写成浮点数）
  floor_db       REAL    NOT NULL,               -- f32 按位无损地放进 REAL
  occupied_ratio REAL    NOT NULL,
  PRIMARY KEY (user_id, pos)
) STRICT;

-- NEC 天线设计存档（key = nec-designs）：一份设计 = 一行 + 导线 / 馈电 / 负载 / 传输线四张子表。
-- 数组下标（pos）是数据的一部分：馈电、负载、传输线用导线的下标引用导线。
CREATE TABLE nec_designs (
  user_id      INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  id           INTEGER NOT NULL,                 -- 本机自增 id，存档册内唯一
  pos          INTEGER NOT NULL,                 -- 最新的在前
  name         TEXT    NOT NULL,
  saved_at     TEXT    NOT NULL,
  freq_hz      REAL    NOT NULL,
  ground       TEXT    NOT NULL CHECK (ground IN ('free', 'perfect', 'lossy')),
  eps_r        REAL,                             -- 仅 lossy
  sigma        REAL,                             -- 仅 lossy
  conductivity REAL,                             -- 导线电导率（S/m）；NULL = 理想导体
  PRIMARY KEY (user_id, id),
  CHECK ((ground = 'lossy') = (eps_r IS NOT NULL AND sigma IS NOT NULL)),
  CHECK (ground = 'lossy' OR (eps_r IS NULL AND sigma IS NULL))
) STRICT;
CREATE TABLE nec_wires (
  user_id   INTEGER NOT NULL,
  design_id INTEGER NOT NULL,
  pos       INTEGER NOT NULL,
  ax REAL NOT NULL, ay REAL NOT NULL, az REAL NOT NULL,
  bx REAL NOT NULL, by REAL NOT NULL, bz REAL NOT NULL,
  radius    REAL    NOT NULL,
  segments  INTEGER NOT NULL,
  PRIMARY KEY (user_id, design_id, pos),
  FOREIGN KEY (user_id, design_id) REFERENCES nec_designs(user_id, id) ON DELETE CASCADE
) STRICT, WITHOUT ROWID;
CREATE TABLE nec_feeds (
  user_id   INTEGER NOT NULL,
  design_id INTEGER NOT NULL,
  pos       INTEGER NOT NULL,
  wire      INTEGER NOT NULL,
  at        REAL    NOT NULL,
  volts     REAL    NOT NULL,
  PRIMARY KEY (user_id, design_id, pos),
  FOREIGN KEY (user_id, design_id) REFERENCES nec_designs(user_id, id) ON DELETE CASCADE
) STRICT, WITHOUT ROWID;
CREATE TABLE nec_loads (
  user_id   INTEGER NOT NULL,
  design_id INTEGER NOT NULL,
  pos       INTEGER NOT NULL,
  wire      INTEGER NOT NULL,
  at        REAL    NOT NULL,
  r_ohm     REAL    NOT NULL,
  l_uh      REAL    NOT NULL,
  c_pf      REAL    NOT NULL,
  PRIMARY KEY (user_id, design_id, pos),
  FOREIGN KEY (user_id, design_id) REFERENCES nec_designs(user_id, id) ON DELETE CASCADE
) STRICT, WITHOUT ROWID;
CREATE TABLE nec_lines (
  user_id         INTEGER NOT NULL,
  design_id       INTEGER NOT NULL,
  pos             INTEGER NOT NULL,
  wire_a          INTEGER NOT NULL,
  at_a            REAL    NOT NULL,
  wire_b          INTEGER NOT NULL,
  at_b            REAL    NOT NULL,
  z0              REAL    NOT NULL,
  length_m        REAL    NOT NULL,
  velocity_factor REAL    NOT NULL,
  PRIMARY KEY (user_id, design_id, pos),
  FOREIGN KEY (user_id, design_id) REFERENCES nec_designs(user_id, id) ON DELETE CASCADE
) STRICT, WITHOUT ROWID;

-- 模拟考试交卷后暂存的「本次错题」（key = exam:wrong:{A|B|C}）：每个题库一份，每份是若干道题的快照。
-- 题面拆成列 + 子表（选项、标准答案），与错题本里的题目快照同一个形状。
CREATE TABLE wrong_questions (
  user_id     INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  bank        TEXT    NOT NULL CHECK (bank IN ('A', 'B', 'C')),
  pos         INTEGER NOT NULL,
  q_id        TEXT,
  code_j      TEXT,
  code_p      TEXT,
  question    TEXT    NOT NULL,
  kind        TEXT    NOT NULL CHECK (kind IN ('single', 'multiple')),
  image_url   TEXT,
  explanation TEXT,
  has_pages   INTEGER NOT NULL CHECK (has_pages IN (0, 1)),
  pages_start INTEGER,
  pages_end   INTEGER,
  PRIMARY KEY (user_id, bank, pos)
) STRICT;
CREATE TABLE wrong_question_options (
  user_id INTEGER NOT NULL,
  bank    TEXT    NOT NULL,
  pos     INTEGER NOT NULL,
  opt_pos INTEGER NOT NULL,
  opt_key TEXT    NOT NULL,
  text    TEXT    NOT NULL,
  PRIMARY KEY (user_id, bank, pos, opt_pos),
  FOREIGN KEY (user_id, bank, pos) REFERENCES wrong_questions(user_id, bank, pos) ON DELETE CASCADE
) STRICT, WITHOUT ROWID;
CREATE TABLE wrong_question_answers (
  user_id INTEGER NOT NULL,
  bank    TEXT    NOT NULL,
  pos     INTEGER NOT NULL,
  ans_pos INTEGER NOT NULL,
  value   TEXT    NOT NULL,
  PRIMARY KEY (user_id, bank, pos, ans_pos),
  FOREIGN KEY (user_id, bank, pos) REFERENCES wrong_questions(user_id, bank, pos) ON DELETE CASCADE
) STRICT, WITHOUT ROWID;
