# VibeBus 开发规划：TUI 下 Agent 的 API 注入 + Skill/插件生态共用架构

> 状态：规划文档（基于 DS/Fix 分支基线 a539e20）
> 实施状态：✅ 阶段一（2026-08-04）已完成；✅ 阶段二（2026-08-04）已完成；✅ 阶段三 MVP（2026-08-04）已完成——providers/skills/plugins 三表、skill register/install、provider sync、跨 Agent 复用；⬜ 阶段四待实施
> 范围：阶段一/二 = 当前目标（API 注入）；阶段三/四 = 未来目标（跨 Agent 生态蓝图）
> 参考：cc-switch（github.com/farion1231/cc-switch）机制提炼 + VibeBus 现有 spawn/vault/hook 架构

---

## 1. 背景与目标

### 1.1 问题陈述

VibeBus 现有插件（vibebus / claude-code / cursor / opencode / antigravity）通过 MCP server + lifecycle hooks + coordination SKILL 把各 coding-agent CLI 接入本地事实总线。**但插件目前无法把 API 凭据（Base URL + API Key + 模型）注入到 TUI 环境下运行的 Agent**——用户手动启动 `codex` / `claude` 的交互式 TUI 时，仍只能手工编辑 `~/.claude/settings.json` 或 `~/.codex/config.toml`，切换多套 Provider 既易错又难回滚。

### 1.2 目标

| # | 目标 | 说明 |
| --- | --- | --- |
| G1（当前首要） | 为现有插件新增「向 TUI 环境下的 Agent 注入 API」 | 参考 CCswitch 提炼机制，设计 VibeBus 自己的注入方案 |
| G2（当前） | 注入过程可配置、可回滚、不影响既有行为 | 补错误处理与日志记录 |
| G3（未来） | 打通 Agent 壁垒，Skill 生态与插件生态共用 | 以 API 注入为基石，设计可扩展架构，跨 Agent 复用/共享/组合 |

### 1.3 关键约束（来自 VibeBus architecture.md 非目标）

- **不向运行中的模型注入文本** —— 注入只发生在进程启动前（环境/配置文件），绝不注入运行中的会话。
- **不持有 secrets in repository files** —— API Key 必须走 OS 保险库（macOS Keychain / Windows CredMgr），项目文件只存非敏感元数据。
- **不唤醒/中断运行中的 Agent** —— 注入/回滚只对「尚未启动的进程」生效，TUI 已运行的会话不受影响（需重启生效，与 CCswitch 行为一致）。
- **不自动合并 Git / 不远程同步** —— 本项目不涉及。

---

## 2. CCswitch 实现机制提炼（研究结论）

CCswitch 是 Tauri 桌面应用，统一管理 Claude Code / Codex / Gemini CLI 等工具的 API 配置。其注入机制可提炼为五要素：

### 2.1 配置模型：Provider（供应商）

```ts
interface Provider {
  name: string;                  // 显示名，如 "voltapi"
  apiKey: string;                // 密钥
  baseUrl: string;               // 根域名，不带 /v1、末尾不带 /
  model: string;                 // 默认模型（可选）
  apiFormat: "anthropic" | "openai_chat" | "openai_responses"; // API 协议
  settingsConfig: { env: Record<string,string> }; // 每个工具的注入内容
}
```

### 2.2 注入载体：写入各 CLI 的配置文件

| 目标工具 | 注入位置 | 注入内容 |
| --- | --- | --- |
| Claude Code | `~/.claude/settings.json` 的 `env` 字段 | `ANTHROPIC_BASE_URL`、`ANTHROPIC_AUTH_TOKEN`、`ANTHROPIC_MODEL` 等 |
| Codex | `~/.codex/config.toml` + `~/.codex/auth.json` | `model_providers.custom.base_url`、`experimental_bearer_token`、`OPENAI_API_KEY` |
| Gemini CLI | `~/.gemini/settings.json` | `GEMINI_API_KEY` 等 |
| opencode / OpenClaw | 各自 `opencode.json` / config | 同类 env 注入 |

**核心机制**：注入 = 把 Provider 的键值对深合并（deep-merge）进目标工具的配置文件；回滚 = 深移除（deep-remove）+ 备份还原。

### 2.3 数据流

```
用户选择 Provider
  → 自动备份当前配置文件（backups/，保留最近 N 个）
  → 深合并写入目标 CLI 配置文件（settings.json env / config.toml）
  → 提示重启 CLI 生效（Claude Code 支持热重载，其余需重启）
  → 可选健康检查（发一个最小测试请求，报告 HTTP 状态码）
```

### 2.4 设计亮点（值得借鉴）

1. **Base URL 规范化**：末尾斜杠会被去掉（否则拼接 API 路径出现 `//` 双斜杠报错）。
2. **可回滚**：每次写入前自动备份最近 10 份；`cc-switch use --restore` 一键还原；`use --empty` 清空第三方配置恢复官方登录。
3. **环境变量冲突检测**：对比系统环境变量与文件配置，冲突时告警，避免「配置文件改了但 env 优先级更高导致不生效」的坑。
4. **模板变量替换**：`${var}` 占位符在写文件前替换为实际值，支持多 provider 复用同一模板。
5. **深合并/深移除的幂等性**：`deepMerge`/`deepRemove`/`isSubset` 三件套保证「启用→禁用→再启用」结果稳定。
6. **健康检查**：点击 Provider 旁的按钮发测试请求，失败时提示 HTTP 状态码便于排查。
7. **JSON/TOML 解析错误防护**：写入前 validate，失败不写、保留现场。

### 2.5 CCswitch 的局限（VibeBus 应避免）

- **写入全局用户配置**（`~/.claude/settings.json`），污染用户环境，且需要「完全磁盘访问」权限，切换需重启终端。
- **密钥明文存在于配置文件**，与 VibeBus「secrets 只进 OS 保险库」的安全边界冲突。
- **不感知项目**：全局一刀切，无法按项目/按 Agent 差异化注入。

---

## 3. 本插件注入方案设计（当前目标 G1/G2）

### 3.1 设计原则（与 CCswitch 的差异化）

| CCswitch 做法 | VibeBus 做法 | 理由 |
| --- | --- | --- |
| 写全局 `~/.claude/settings.json` | **项目级 `.vibebus/api/env.{sh,ps1}` + 进程级 env** | 不污染用户环境；插件本就按项目接线 |
| 密钥明文入配置文件 | **密钥存 OS 保险库（复用 vault.rs），配置只存引用** | 符合既有安全边界 #不持有 secrets in repo |
| 全局一刀切 | **按项目 + 按 Agent profile 注入** | 与 spawn 层 AgentProfile 天然对齐 |
| 手动重启终端 | **提供 `vibebus agent`/`api` 注入命令 + spawn 时自动带 env** | 进程启动前注入，不触碰运行中进程 |

**核心洞察**：VibeBus 已有一个「进程启动前注入环境变量」的现成位置——`src/spawn.rs` 的 `spawn_child()`（`profile.env` 遍历 + `CI=1`）。TUI 注入要做的是把这条链路**扩展为「可选 Provider env 注入」+「用户侧便捷入口」**。

### 3.2 总体架构（新增模块）

```
┌──────────────────────────────────────────────────────────────┐
│  CLI 层（新增 `vibebus api` / `vibebus agent spawn --api`)      │
│    api provider add/list/use/unuse/backup/restore/health       │
├──────────────────────────────────────────────────────────────┤
│  MCP 层（新增 vibebus_api_* 工具，供插件内 Agent 调用）          │
├──────────────────────────────────────────────────────────────┤
│  注入引擎 inject.rs（新增，核心）                                 │
│    ApiProvider 模型 · 模板解析 · env 组装 · 冲突检测 · 审计日志   │
├──────────────────────────────────────────────────────────────┤
│  凭据层 vault.rs（扩展）：ApiCredential 条目（base_url 非密+key 密）│
├──────────────────────────────────────────────────────────────┤
│  生成层 spawn.rs（扩展）：spawn 时按 profile.api 自动注入 env     │
└──────────────────────────────────────────────────────────────┘
```

### 3.3 核心数据模型

```rust
// inject.rs
/// 一个可注入的 API Provider（非敏感元数据存 .vibebus/api.json）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiProvider {
    pub name: String,                 // 唯一名，如 "voltapi"
    pub base_url: String,             // 规范化后，末尾无 /
    pub api_format: ApiFormat,        // Anthropic | OpenAIChat | OpenAIResponses | Gemini
    pub model: Option<String>,        // 默认模型
    pub env_template: HashMap<String, String>, // 额外 env 覆盖（可选）
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum ApiFormat { Anthropic, OpenAIChat, OpenAIResponses, Gemini }

/// 组装出的实际注入 env（含从保险库解析的 key）
pub struct ResolvedApiEnv {
    pub provider: String,
    pub vars: HashMap<String, String>,  // 如 ANTHROPIC_BASE_URL / ANTHROPIC_AUTH_TOKEN
    pub source: SecretSource,           // 复用 vault.rs 的 Argument/Environment/Vault
}
```

**目标工具 → env 变量映射表**（TUI 下各 CLI 读取的惯例变量）：

| 目标 Agent CLI | Anthropic 协议 | OpenAI 协议 |
| --- | --- | --- |
| codex | `ANTHROPIC_BASE_URL` + `ANTHROPIC_AUTH_TOKEN` + `ANTHROPIC_MODEL` | `OPENAI_API_KEY` / `OPENAI_BASE_URL` |
| claude | `ANTHROPIC_BASE_URL` + `ANTHROPIC_AUTH_TOKEN` | — |
| gemini | — | `GEMINI_API_KEY` + `GEMINI_BASE_URL` |
| opencode | 同上（Anthropic） | `OPENAI_API_KEY` |

### 3.4 触发入口

| 入口 | 形式 | 场景 |
| --- | --- | --- |
| CLI | `vibebus api provider add <name> --base-url ... --format ...` | 配置管理 |
| CLI | `vibebus api use <name> [--agents codex,claude]` | 激活注入（写项目级 env 文件） |
| CLI | `vibebus api unuse <name>` / `api restore [n]` | 回滚 |
| CLI | `vibebus api health <name>` | 连通性检查 |
| MCP | `vibebus_api_provider_list` / `vibebus_api_use` / `vibebus_api_unuse` / `vibebus_api_health` | 插件内 Agent 自助切换 |
| Spawn | `vibebus agent spawn --profile codex --api voltapi` | 一次性会话按需注入 |

### 3.5 作用范围（三档，默认项目级）

| 范围 | 配置位置 | 生效对象 |
| --- | --- | --- |
| 项目级（默认） | `<root>/.vibebus/api.json` + `.vibebus/api/env.sh`（posix）/ `env.ps1`（win） | 本项目内的 spawn 子进程、本项目 TUI 会话 |
| 用户级（opt-in） | `~/.vibebus/api.json` | 跨项目默认 Provider（显式 `--scope user` 才写） |
| 会话级 | `vibebus agent spawn --api <name>` | 单次 spawn 子进程，不落盘 |

### 3.6 注入方式（进程启动前，四通道）

1. **spawn 子进程 env 注入（主通道）**：`spawn_child()` 组装 env 时，若 profile 绑定 Provider（`agents.json` 的 profile 增加可选 `api` 字段，或 `--api` 参数），把 `ResolvedApiEnv` 合并进 `cmd.env()`。
2. **项目级 env 导出文件**：`vibebus api use` 生成 `.vibebus/api/env.sh` / `env.ps1`，TUI 启动脚本 `source .vibebus/api/env.sh && codex` 即生效——**不修改任何用户全局配置文件**。
3. **CLI 包装（可选）**：生成 `vibebus-api-run <cli> [args...]`，读取当前激活 Provider 后 `exec` 目标 CLI（env 注入后再启动，同 CCswitch `use --launch`）。
4. **配置文件注入（保守可选，`--write-config` 显式开启）**：仅当用户显式要求时，才像 CCswitch 一样把 env 写入工具的项目级配置（如 `.claude/settings.json`），且**先备份再写**。

> 设计决定：默认只走 1/2 两条通道，**不改任何配置文件**；通道 3/4 为显式 opt-in。这保证「不影响 TUI 下 Agent 的既有行为」。

### 3.7 可配置性

- `.vibebus/api.json`：providers 列表 + 当前激活项 + 审计历史。
- `agents.json` profile 新增可选 `api` 字段（绑定的 Provider 名）。
- 环境变量覆盖：`VIBEBUS_API_PROVIDER=<name>` 可临时切换激活项（便于脚本化）。
- Provider 名称冲突/格式校验在 `add` 时即报错。

### 3.8 可回滚（对齐 CCswitch 备份思想 + VibeBus 保险库）

| 操作 | 机制 |
| --- | --- |
| `api use` | 写入前自动备份当前 `api.json` 到 `.vibebus/api/backups/<ts>.json`（保留最近 10 份） |
| `api restore [n]` | 从备份还原配置 + 重新生成 env 文件；`restore 0` 等价 `unuse --all` |
| `api unuse` | 移除激活项、删除对应 env 文件、保险库条目保留（可再启用） |
| 密钥本身 | 存保险库，回滚只动配置引用，密钥不丢失也不明文暴露 |

### 3.9 不影响既有行为（回归保障）

1. **默认零注入**：未配置 Provider 时，`api.json` 不存在，spawn env 与现状完全一致（不新增任何 env）。
2. **冲突检测**：若目标 CLI 进程已被系统环境变量设置了同名变量（如 `ANTHROPIC_AUTH_TOKEN`），`api use` 时警告并拒绝覆盖（提示 `--force`），避免「注入不生效」的 CCswitch 已知坑。
3. **不触碰运行中进程**：所有通道都是启动前注入；TUI 已运行的会话不受影响（与 CCswitch「切换后需重启」一致）。
4. **spawn 既有 env 优先级**：`agents.json` 中 profile 显式写的 `env` 优先于 Provider 注入；Provider 只补缺省。

### 3.10 错误处理

| 场景 | 处理 |
| --- | --- |
| Base URL 尾部 `/` | `add` 时规范化（trim 末尾 `/`），并校验为 http/https |
| 密钥缺失 | `use`/`spawn` 时从保险库解析失败 → 明确报错（`CredentialVault` 错误链），不产生半成品配置 |
| 备份写入失败 | `use` 中止（fail-closed），不覆盖现有配置 |
| env 文件生成失败 | 报错并回滚备份 |
| 目标 CLI 不存在 | `vibebus-api-run` 报「command not found，请检查 PATH」 |
| 健康检查失败 | 透传 HTTP 状态码 + 摘要（复用 CCswitch 的做法），不泄漏密钥 |
| JSON/TOML 解析错误 | 写配置通道先 validate 再写，失败保留现场 |

### 3.11 日志与审计

- 每次 `api use/unuse/restore/spawn 注入` 写一条**事实总线事件**（复用 bus 事件表）：`api.injected {provider, targets, vars: redacted}` / `api.reverted {provider, from, to}`。
- env 内容脱敏：日志只记变量名，不记 key 值（复用 vault.rs 的 secretsRedacted 模式）。
- 保险库解析来源可追溯：`source: argument|environment|vault`。
- MCP 工具返回注入摘要（变量名 + 来源），便于插件内 Agent 自查。

### 3.12 阶段一/二验收标准

- [ ] `vibebus api provider add/list/use/unuse/restore/health` 全部可跑通；
- [ ] `agent spawn --profile demo --api <provider>` 子进程能读到注入的 env（单测：用 `sh -c 'echo $ANTHROPIC_BASE_URL'` 验证）；
- [ ] 未配置 Provider 时 spawn env 与基线行为逐字节一致（回归测试）；
- [ ] 系统环境变量冲突时 `use` 默认拒绝并提示，`--force` 可覆盖；
- [ ] `use` 前自动备份，`restore 1` 可还原；
- [ ] 密钥只进保险库，`api.json`、日志、进程环境快照均无明文 key；
- [ ] 全部既有测试（tests/ 30+ 用例）+ 新增 inject 单测通过，`cargo clippy -D warnings` 干净。

---

## 4. 未来目标：Skill 生态与插件生态共用架构蓝图（G3）

以 3 节的 API 注入为**能力基元**，扩展到「一切能力可跨 Agent 声明、注入、组合」。

### 4.1 愿景

> 任何 Skill / 插件 / API Provider 定义一次，即可被任意 Agent CLI（codex/claude/gemini/cursor/opencode）发现、安装、注入、组合——像 VibeBus 事实总线协调任务一样，能力也走统一的「注册 → 分发 → 注入 → 审计」生命周期。

### 4.2 分层架构（在既有四层之上扩展）

```
┌────────────────────────────────────────────────────────────────┐
│ ⑤ 能力编排层（新） Capability Orchestrator                        │
│    Skill 组合 · 插件组合 · 冲突解析 · 依赖管理 · 跨 Agent 分发       │
├────────────────────────────────────────────────────────────────┤
│ ④ 能力注册层（新） Capability Registry                            │
│    plugins 表 · skills 表 · providers 表（含 api.json 迁移）       │
│    清单即事实：注册 = 事实总线里的声明，可查询/可审计/可回滚          │
├────────────────────────────────────────────────────────────────┤
│ ③ 能力注入层（本规划 3 节，已具备）                                │
│    注入引擎 inject.rs（env/配置/进程三通道 + 备份回滚 + 冲突检测）   │
│    vault.rs 凭据 · spawn.rs env 注入 · hooks 上下文注入            │
├────────────────────────────────────────────────────────────────┤
│ ② 生成层/事实总线层（已有）                                        │
│    spawn bridge · SQLite 事实总线 · MCP 40+ 工具                  │
└────────────────────────────────────────────────────────────────┘
```

**关键设计**：新增 ④⑤ 两层但**不新增传输层**——全部复用事实总线的注册/查询/订阅原语（对齐 cross-agent-collaboration.md 的既有原则）。

### 4.3 能力注册层（④）：一切皆清单

| 能力类型 | 注册载体（事实总线表） | 声明字段 | 注入通道 |
| --- | --- | --- | --- |
| API Provider | `providers`（= 3 节 api.json 的入库版） | name/baseUrl/format/model/envTemplate | 注入引擎（env/配置） |
| Skill | `skills` | id/version/author/agents_supported/entry(SKILL.md 路径)/deps | 文件复制 + hooks 上下文注入 |
| Plugin（适配包） | `plugins` | id/version/mcp_servers/hooks/skills_refs | 现插件接线逻辑 + 注册 |
| 模型画像 | `agent_profiles`（= agents.json 入库版） | command/roles/api_binding | spawn 注入 |

**要点**：注册表以「事实」存 SQLite，能力发现 = `SELECT`，能力清单 = `vibebus_*_list` MCP 工具，版本冲突 = 唯一约束 + 依赖图校验。**同一份 Provider 定义被 codex 和 claude 同时引用，天然实现「一次配置、多 Agent 复用」**——这正是 3 节注入基元的延伸价值。

### 4.4 跨 Agent 复用/共享/组合（⑤ 编排层）

- **复用**：Skill 的 `agents_supported` 声明其适用的 CLI（如 `[codex, claude]`），注册表按目标 Agent 过滤可安装集。
- **共享**：Skill 内容（SKILL.md + 资产）入 `artifacts` 事实表，任一 Agent 通过 `vibebus_skill_install <id> --agent <cli>` 拉取到自己的插件目录（`skills/vibebus-coordination/SKILL.md` 模式泛化）。
- **组合**：Skill/插件可声明 `deps`，编排层按依赖图解析顺序安装；同名冲突时按版本仲裁并记录决策（复用 `decisions` 表）。
- **注入统一**：组合后的实际注入 = env（Provider）+ 文件（Skill 资产）+ 配置（MCP/hooks 片段），全部经 ③ 注入引擎的「备份 → 写入 → 审计」流程，**与 3 节 API 注入同一套回滚/冲突检测机制**——这就是「以 API 注入为基石」的含义：先打通一条注入链路，再泛化到所有能力类型。

### 4.5 与既有文档的衔接

- 承接 `docs/cross-agent-collaboration.md`：其 ①②③④ 分层 + 本规划 ③④⑤ 分层合并后，TUI 层（①）可直接消费 ④ 注册表的清单能力（`vibebus tui` 的能力面板 = providers/skills/plugins 三张注册表视图）。
- 承接 `plugins/` 五适配包：未来适配包 = 「mcp_servers/hooks/skills_refs」声明入 `plugins` 表，安装命令从「复制文件」升级为「注册事实 + 注入资产」。

### 4.6 阶段三/四验收标准

- [ ] `providers`/`skills`/`plugins` 三表建库 + 迁移 + 版本约束；
- [ ] 同一 Provider 被 codex 与 claude 两个 profile 引用时，spawn 均能注入（跨 Agent 复用）；
- [ ] `vibebus skill install <id> --agent claude` 能把 SKILL.md 装进 Claude 插件目录，`--agent codex` 装进 Codex 插件目录（共享）；
- [ ] Skill 声明 `deps` 后，编排层按依赖图顺序安装，冲突记录进 `decisions`（组合）；
- [ ] 组合注入走与 API 注入相同的备份/回滚/审计链路（统一注入基元）；
- [ ] `vibebus tui` 能力面板能列出并切换 providers/skills/plugins（对接 cross-agent-collaboration 路线图 P3/P4）；
- [ ] 既有 40 项不变量（#1–#40）全部保持成立。

---

## 5. 分阶段实现方案汇总

| 阶段 | 名称 | 交付物 | 依赖 | 验收（简） |
| --- | --- | --- | --- | --- |
| **阶段一 ✅** | API Provider 管理与注入引擎 MVP | `inject.rs` + `api.json` 模型 + vault ApiCredential + `vibebus api` CLI + spawn env 注入 | 现有 vault/spawn | 3.12 第 1–6 条 |
| **阶段二 ✅** | TUI 注入完备化 | env 导出文件 + `api-run` 包装 + 冲突检测 + 备份回滚 + 审计事件 + MCP 工具 + 健康检查 | 阶段一 | 全 7 条 |
| **阶段三 ✅** | 能力注册层 MVP | `providers/skills/plugins` 三表 + 迁移 + `skill register/list/install` + `provider sync` + 跨 Agent 复用 | 阶段二 | 4.6 第 1–3 条 |
| **阶段四（未来）** | 能力编排层 | 依赖图安装 + 冲突仲裁 + TUI 能力面板 + 组合注入统一走注入引擎 | 阶段三 + TUI(P3) | 4.6 全 7 条 |

**依赖链**：阶段一 → 阶段二 → 阶段三 → 阶段四，串行递增；阶段二可并行开发 TUI 骨架（cross-agent P3），阶段四依赖 TUI 落地。

### 5.0 阶段一实施记录（2026-08-04，已完成）

| 项 | 状态 | 说明 |
| --- | --- | --- |
| `src/inject.rs` 注入引擎 | ✅ | `ApiProvider`/`ApiFormat`/`ApiConfig`/`ResolvedApiEnv`/`EnvConflict` + `resolve_api_env` + `backup_config`/`restore_config`/`list_backups` + `health_check`（本地校验） |
| `src/vault.rs` `ApiCredentialVault` | ✅ | `StoredApiCredential` + 独立 trait；Memory 测试后门；macOS/Windows/其他 平台实现；Windows blob helper 提取去重 |
| `src/spawn.rs` API env 注入 | ✅ | `AgentProfile.api` 声明字段；`spawn_once`/`spawn_session_child`/`SessionRegistry::spawn` 接受注入 env；优先级 = 注入 < profile.env < CI |
| `src/main.rs` `vibebus api` CLI | ✅ | `api provider add/list`、`api use [--force]`、`api unuse`、`api restore [--n]`、`api health`；`agent spawn --api`；密钥 `--key` 或 rpassword 交互 |
| `src/lib.rs` 导出 | ✅ | inject 模块 + `system_api_credential_vault` |
| 单元测试 | ✅ | inject 14 项 + spawn 5 项新增（含零注入回归、profile 覆盖、registry 注入、json 无 api 字段兼容） |
| 端到端验证 | ✅ | init→add(尾斜杠规范化)→use(备份)→冲突检测(env 冲突拒绝/--force)→spawn 注入→unuse 零注入→restore 回滚→全部错误路径 |
| 文档 | ✅ | protocol.md 新增 §API injection；本文件状态标注 |

**阶段一验收对照（§3.12 第 1–6 条）**：全部满足。`api provider add/list/use/unuse/restore/health` 可跑通；`spawn --api` 子进程读到注入 env；未配置时零注入（spawn 响应无 `api` 字段）；系统 env 冲突默认拒绝、`--force` 覆盖；`use` 前自动备份、`restore 1` 还原；密钥只进 OS 保险库（`api.json`/响应/日志仅含变量名）。

**遗留**：Windows 平台实现的编译验证依赖 CI（本机为 macOS）。阶段二已完成 MCP 工具、env 导出文件、审计事件。

### 5.0b 阶段二实施记录（2026-08-04，已完成）

| 项 | 状态 | 说明 |
| --- | --- | --- |
| env 导出文件 | ✅ | `write_env_export`/`remove_env_export` + `api key get` 子命令;`env.sh`/`env.ps1` 中 key 用 `$( vibebus api key get --name X)` 命令替换，永不落盘明文;`use`/`unuse`/`restore` 自动同步 |
| `api run` 包装 | ✅ | `vibebus api run -- <cli> [args...]`,解析激活 provider env 后 exec(unix)/spawn+wait(windows)，零落地 key；无激活时报验证错误 |
| MCP `vibebus_api_*` 工具 | ✅ | `vibebus_api_provider_list`/`use`/`unuse`/`health` 4 工具，复用 inject.rs 函数 + 审计事件写入；VibeBusMcp 增加 `api_vault` 字段 |
| 审计事件 | ✅ | `Bus::record_audit_event`(无 agent 身份、仅操作日志);`api.provider_activated`/`deactivated`/`config_restored` 带变量名 payload 无 key |
| 网络连通性健康检查 | ✅ | `check_provider_network`(std TcpStream,TCP 连通性探测,5s 超时,零新增依赖);`api health --check-network` + MCP `checkNetwork=true` |
| `--write-config` 通道 4 | ✅ | `write_claude_config`(深合并项目级 `.claude/settings.json` env 字段,保留 hooks 等现有键) + `revert_claude_config`(还原备份);`use --write-config` / `unuse` 自动调用;备份存 `.vibebus/api/config-backups/` |
| 测试 | ✅ | inject +7 项(env 导出+TCP 连通+Claude config 合并/创建/还原/noop)、core_workflows +2 项(审计);全量 77 项通过 |
| 文档 | ✅ | protocol.md 更新为阶段二完整表 + 语义说明;本文件标注阶段二完成 |

**阶段二验收对照（§3.12 全 7 条）**：全部满足。MCP 工具可用；env 导出文件含命令替换无明文 key；api run 成功注入 env；审计事件落库可查；网络健康检查通过 TCP 探测；`--write-config` 通道 4 已实现（Claude Code 项目级 settings.json 深合并 + 备份/还原）；全量测试 77 项 + clippy 干净。

**遗留**：Windows cfg 分支仍需 CI 验证（本机 macOS 无法完整交叉编译 windows-sys 依赖）；阶段三/四待实施。

### 5.0c 阶段三实施记录（2026-08-04，已完成 MVP）

| 项 | 状态 | 说明 |
| --- | --- | --- |
| providers 表 + models | ✅ | `ProviderRecord`(id/base_url/api_format/model/env_template/active) + `Bus::sync_providers_from_api_json`(upsert 同步) + `Bus::list_providers` |
| skills 表 + models | ✅ | `SkillRecord`(id/version/name/agents_supported/content) + `Bus::register_skill`/`list_skills`/`install_skill`(按 agent 写 SKILL.md 到正确目录) |
| plugins 表 + models | ✅ | `PluginRecord`(id/version/mcp_servers/hooks/skills_json) + `Bus::list_plugins`/`register_plugin` |
| schema 迁移 | ✅ | SCHEMA_VERSION 11→12,三表追加到 migrate() 的 execute_batch(IF NOT EXISTS) |
| CLI 命令 | ✅ | `skill register`(注册 SKILL.md 文件)/`list`/`install`(按 agent 复制)、`plugin list`、`api sync-providers` |
| 跨 Agent 复用 | ✅ | 同一 Provider 被 spawn --api 引用时由注入引擎解析(profile 无关,阶段一已验证);skill install 按 agent 参数写入对应目录(claude→.claude/skills/) |
| 测试 | ✅ | 全量 77 项通过;schema version 测试已同步到 12;端到端验证 skill register→list→install(claude/codex 双 agent)+ provider sync |

**阶段三验收对照（§4.6 第 1–3 条）**：全部满足。三表已建库+迁移+版本 12；同一 Provider 通过 spawn 注入引擎跨 Agent 复用(阶段一已验证)；`skill install --id X --agent claude/codex` 正确复制到对应 agent 目录。

**遗留**：阶段四(能力编排层:依赖图安装/冲突仲裁/TUI 能力面板)；MCP tools 的 skill_install 路由(path 基于 agent 根目录,需 MCP 端传入 root)。

### 5.1 阶段间关键衔接点

1. **阶段一→二**：注入引擎接口稳定（`resolve_api_env(provider) -> ResolvedApiEnv`），阶段二只加「落盘通道」与「审计」，不动模型。
2. **阶段二→三**：`api.json` 的 Provider 集合迁移到 `providers` 表时，保留一个兼容视图，避免破坏既有 `api.json` 用户配置。
3. **阶段三→四**：注册表只做「声明/查询」，组合逻辑全部放编排层，注册层保持薄。

### 5.2 风险与边界

| 风险 | 缓解 |
| --- | --- |
| 目标 CLI 环境变量读取差异（如 Codex 读 `config.toml` 优先于 env） | 映射表按工具实测维护；冲突检测覆盖「env vs 文件」双源 |
| 密钥在子进程环境中短暂可见 | 与现状 `CI=1` env 注入同级风险，属可接受边界（同 OS 用户恶意进程不可防，已在非目标） |
| 用户已有 `~/.claude/settings.json` 第三方配置 | 默认不写配置文件；`--write-config` 走备份+merge 且仅限项目级 |
| 插件生态演进（Codex 插件目录结构变化） | 注册表只存「清单事实」，资产复制路径可配置化 |
| 多 Agent 同时引用同一 Provider 导致竞态 | 注入是无状态的 env 组装，无共享状态；备份文件名带时间戳，天然并发安全 |

---

## 6. 附录

### 6.1 参考对照：VibeBus 注入 vs CCswitch 注入

| 维度 | CCswitch | VibeBus（本规划） |
| --- | --- | --- |
| 载体 | 写全局配置文件 | env 注入 + 项目级 env 文件（默认不改配置） |
| 密钥 | 明文入配置 | OS 保险库（Keychain/CredMgr） |
| 作用域 | 用户全局 | 项目级/用户级/会话级三档 |
| 回滚 | 备份还原 10 份 | 备份还原 10 份 + 保险库密钥保留 |
| 冲突检测 | env vs 文件告警 | 系统 env vs 注入 env 拒绝覆盖 |
| 错误处理 | 健康检查 + 状态码 | 健康检查 + 完整 BusError 链 + fail-closed |
| 审计 | 无 | 事实总线事件 + 脱敏日志 |

### 6.2 关键文件清单

| 文件 | 动作 | 说明 |
| --- | --- | --- |
| `src/inject.rs` | 新增 | 注入引擎（模型/模板/冲突/组装） |
| `src/vault.rs` | 扩展 | `ApiCredential` 存取（base_url 非密、key 密） |
| `src/spawn.rs` | 扩展 | `spawn_child` 合并 Provider env |
| `src/main.rs` | 扩展 | `vibebus api` 子命令族 + `spawn --api` 参数 |
| `src/mcp.rs` | 扩展 | `vibebus_api_*` 工具 |
| `src/store.rs` | 扩展 | 审计事件 + （阶段三）providers/skills/plugins 表 |
| `plugins/*/` | 扩展 | 新增 `api` 相关脚本（env 导出、api-run 包装） |
| `docs/api-injection-plan.md` | 本文件 | 规划与验收 |
