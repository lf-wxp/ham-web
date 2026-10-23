# 题库数据：构建与换版

> 题库 JSON 已提交入库，日常开发不需要重建；换数据源或官方换版时看这份。

## 数据集构建

题库 JSON 与图片已提交在 `public/questions/`，日常开发无需重新构建。需要更新题库时：

```bash
cargo make dataset                                   # 从默认远程仓库拉取
DATASET_DIR=/path/to/csv-repo cargo make dataset     # 优先使用本地目录
DATASET_REMOTE=https://raw.githubusercontent.com/<you>/<repo>/main cargo make dataset
```

默认数据源为 [`xiedada05/crac-amateur-radio-exam-questions-2025-csv`](https://github.com/xiedada05/crac-amateur-radio-exam-questions-2025-csv)。数据源需包含：`class_a.csv`、`class_b.csv`、`class_c.csv`、`full.csv`、`images.csv` 与 `images_2/{题号}.jpg`。CSV 列：`J`（题号）、`P`（分类码）、`Q`（题干）、`T`（答案）、`A`–`D`（选项），可选解析列（`Explanation` / `解析` / `analysis`）。

建库时会统一题面排版（中文与数字 / 英文之间的空格、全角字母数字转半角等），让同一道题在各题库里逐字节一致、不再分叉出多个内容指纹，规则见 [`docs/typography.md`](typography.md)。已有数据可用 `cargo make questions-normalize` 原地迁移（不重新下载 CSV）。

输出：

- `public/questions/{A,B,C,full}.json`：题目列表，ID 形如 `A-1`（按题库内顺序编号）
- `public/questions/images/N.jpg`：题目附图
- `public/questions/search-index.json`：全站搜索用的精简索引（题干 + 解析，A/B/C 三库），随题库一起刷新

解析来源优先级：`data/explanations.json`（按内容指纹匹配） > CSV 解析列。


## 新增题库版本

1. 把新版本题库 JSON 放到 `public/questions/`（当前版本直接覆盖 `A/B/C.json`）；
2. 编辑 `public/questions/config.json`，在 `versions` 中添加版本并更新顶层 `version`（前端检测到版本号变化会清空缓存）：
   ```json
   {
     "id": "2026-04", "name": "2026年4月版本", "description": "…", "isLatest": true,
     "banks": { "A": { "path": "/questions/A.json", "description": "A类题库" }, "B": { … }, "C": { … } },
     "updatedAt": "2026-04-01T00:00:00Z"
   }
   ```
3. 旧版本的 `isLatest` 改为 `false`。`/questions/<版本>/A.json` 形式的路径会被映射为 `/questions/A.json`。

