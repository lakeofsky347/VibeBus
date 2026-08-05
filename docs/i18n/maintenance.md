# 双语文档维护规范 / Bilingual Documentation Maintenance

> 本规范定义 VibeBus 仓库的双语文档体系如何维护：目录约定、语言路由、新增文档流程、翻译质量门禁与同步义务。适用对象：所有向仓库提交文档改动的贡献者与维护者。

## 1. 目录结构与语言路由

### 1.1 目录约定

```text
docs/
├── README.md                  # 语言路由入口（双语索引表）
├── en/                        # 英文规范源（canonical source）
│   ├── README.md
│   ├── architecture.md
│   ├── protocol.md
│   └── ...                    # 全部核心文档
├── zh-CN/                     # 简体中文翻译
│   ├── README.md
│   ├── architecture.md
│   ├── protocol.md
│   └── ...                    # 与 en/ 一一对应
└── i18n/
    ├── glossary.md            # 术语表
    ├── translation-matrix.md  # 翻译对照表
    └── maintenance.md         # 本文件
```

### 1.2 语言选择机制

**按路径路由，无运行时配置。** 目录路径本身就是语言选择器：

- `docs/en/<file>.md` → 英文
- `docs/zh-CN/<file>.md` → 简体中文
- 仓库根 `README.md` 顶部提供语言切换链接；`docs/README.md` 提供完整双语索引。

**链接约定**（保证双向切换可用）：

| 位置 | 指向英文 | 指向中文 |
| --- | --- | --- |
| `docs/en/x.md` 内 | `x.md`（同目录） | `../zh-CN/x.md` |
| `docs/zh-CN/x.md` 内 | `../en/x.md` | `x.md`（同目录） |
| 根 `README.md` | `docs/en/README.md` | `docs/zh-CN/README.md` |

**根级文件例外**：`CHANGELOG.md`、`CONTRIBUTING.md`、`SECURITY.md`、`CODE_OF_CONDUCT.md` 的英文原版留在仓库根目录（GitHub、发布工作流与工具链依赖这些位置），不复制进 `docs/en/`，避免双份英文源漂移。中文翻译位于 `docs/zh-CN/` 同名文件。

## 2. 术语与翻译标准

### 2.1 术语表是强制基准

翻译必须使用 [glossary.md](glossary.md) 中的标准译法。核心规则：

- **永不翻译**：命令名（`vibebus_*`、`api use`）、工具名、表名（`providers`）、文件路径（`.vibebus/api.json`）、版本号（`v0.11.0`）、哈希值、`MCP`/`Hook`/`CLI`/`SQLite WAL` 等专有词、代码块内容。
- **首次出现格式**：核心术语首次出现用「中文（English）」，如「智能体（Agent）」「路径预留（reservation）」，后续保持一致。
- **新增术语**：遇到术语表未覆盖的词，按字面准确翻译，并回填 `glossary.md`（见 §4.4）。

### 2.2 结构一致性（强制）

中文译文必须与英文源保持：

- **标题层级一致**：`#`/`##`/`###` 数量与顺序完全一致（锚点可用）；
- **表格结构一致**：行数、列数一致；只翻译"说明/用途"类文本列，命令列/标识符列保持原文；
- **代码块一致**：围栏（` ``` `）数量一致，代码块内容逐字不变；
- **链接一致**：相对路径按 §1.2 规则改写，锚点目标保持一致。

**结构校验**（提交前运行）：

```sh
for f in architecture protocol multi-tool-adapters design-research backup-restore release macos container HANDOFF cross-agent-collaboration; do
  en_h=$(grep -c '^#' "docs/en/$f.md"); zh_h=$(grep -c '^#' "docs/zh-CN/$f.md");
  en_b=$(grep -c '^```' "docs/en/$f.md"); zh_b=$(grep -c '^```' "docs/zh-CN/$f.md");
  [ "$en_h" = "$zh_h" ] && [ "$en_b" = "$zh_b" ] || echo "MISMATCH: $f";
done
echo "structure check done"
```

### 2.3 写作风格

- 简洁、专业的中文技术写作，主动语态；
- 第二人称（"你"）或无人称均可，全文保持一致；
- 不添加、删除或重排英文源的任何章节；
- 不把一段长英文拆成多段、也不把多段合并成一段（保持段落粒度便于 diff 对照）。

## 3. 新增文档流程

新增任何面向用户的文档（README、how-to、参考、架构说明）时：

### 3.1 新建（双语双份）

1. **先写英文源**到 `docs/en/<name>.md`（规范源优先）。
2. **再写中文翻译**到 `docs/zh-CN/<name>.md`，遵循 §2 标准。
3. **更新索引**：
   - `docs/README.md` 的 Document index 表加一行；
   - `docs/i18n/translation-matrix.md` 的对照表加一行；
   - 若文档是根级文件（CHANGELOG 等），英文放根目录、中文放 `docs/zh-CN/`。
4. **自检**：运行 §2.2 的结构校验脚本；检查相对链接（用 `docs/README.md` 的链接约定核对）。

### 3.2 删除

- 从 `docs/en/` 与 `docs/zh-CN/` **同时**删除对应文件（可用 `git rm`，历史保留在 git 中）；
- 同步更新 `docs/README.md` 与 `translation-matrix.md`；
- 若删除根级英文文件，同时删除 `docs/zh-CN/` 对应中文翻译。

### 3.3 重命名/移动

- 双语同步移动，保持相对链接正确；
- 更新所有指向该文件的链接（`grep -rn "old-name" --include="*.md" .` 检查）。

## 4. 双语同步流程

英文源是唯一事实来源。任何对英文内容的修改**必须在同一 PR 中**同步中文翻译。

### 4.1 触发同步的情形

- 修改代码行为、CLI 命令、MCP 工具、数据库表、Hook、配置字段；
- 修改英文文档正文（即使只是措辞）；
- 版本号、schema 版本、工具数量等事实变更；
- 术语表更新导致既有译文需要回改。

### 4.2 同步步骤

1. 修改 `docs/en/<file>.md`；
2. 对照修改 `docs/zh-CN/<file>.md` 的对应段落；
3. 若英文增删了章节，中文同步增删（保持 §2.2 结构一致）；
4. 更新 `translation-matrix.md` 的状态列（如需）；
5. 运行结构校验脚本。

### 4.3 变更后的对照表维护

`translation-matrix.md` 中每份文档有行数列。提交后若 `en` 与 `zh-CN` 行数出现明显差异（超过合理翻译损耗，一般 <5%），说明结构可能漂移，需要人工确认。

### 4.4 术语表回填

翻译/修改过程中遇到术语表未覆盖的新术语时，**同 PR** 内：

1. 在 `glossary.md` 对应分类下添加一行（English / 简体中文 / Notes）；
2. 若是对既有术语译法的修正，更新术语表并全文搜索旧译法统一替换。

## 5. 质量门禁（CI 建议）

以下检查建议加入 CI（当前未配置，见 [translation-matrix.md](translation-matrix.md) 待办）：

1. **结构一致性**：对每对 `docs/en/*.md` / `docs/zh-CN/*.md` 比较标题数与代码块围栏数；
2. **链接有效性**：检查所有相对链接目标存在（仓库内）；
3. **术语检查**：基于 `glossary.md` 的术语表扫描中文译文，发现不一致即告警；
4. **运行时文件保护**：`plugins/*/skills/vibebus-coordination/SKILL.md` 五份副本必须保持逐字一致（md5 校验），且不得被翻译。

## 6. 运行时文件边界

以下文件**刻意不翻译**，不属于双语文档体系：

| 文件 | 原因 |
| --- | --- |
| `plugins/*/skills/vibebus-coordination/SKILL.md` | MCP/工具按路径加载，运行契约必须稳定；保持英文 |
| `plugins/opencode/AGENTS.md` | OpenCode 运行时指令 |
| `plugins/cursor/.cursor/skills/vibebus-coordination/SKILL.md` | 运行时文件 |
| 代码内 `///` doc 注释 | 跟随代码语言（当前英文），不纳入翻译 |

若未来需要为 SKILL.md 提供多语言，必须采用"英文为主 + 可选多语言副本经配置加载"的方式，并先通过插件契约评审。

## 7. 速查清单（PR 提交前）

- [ ] 改动的每个英文文件都有对应中文翻译同步更新；
- [ ] 中文译文标题层级、表格、代码块与英文一致（运行过结构校验）；
- [ ] 代码块/命令/表名/路径/版本号未被翻译；
- [ ] `docs/README.md` 与 `translation-matrix.md` 已同步；
- [ ] 新术语已回填 `glossary.md`；
- [ ] SKILL.md / AGENTS.md 等运行时文件未被改动；
- [ ] 相对链接双向可达（en ↔ zh-CN ↔ 根 README）。
