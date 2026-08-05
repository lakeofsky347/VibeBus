# CLI 与 MCP 协议

所有时间均为 Unix 毫秒。CLI 响应为 JSON。MCP 工具以文本内容返回格式化 JSON，使两个接口上可见相同的领域模型。

## 身份与检查

| 能力 | CLI | MCP |
| --- | --- | --- |
| 初始化项目 | `init` | 刻意仅限 CLI |
| 注册智能体 | `register` | `vibebus_register` |
| 恢复智能体 | `agent recover` | `vibebus_agent_recover` |
| 轮换/建档恢复密钥 | `agent provision-recovery` | `vibebus_recovery_provision` |
| 检查凭据保险库 | `credential status` | `vibebus_credential_status` |
| 删除凭据条目 | `credential delete` | `vibebus_credential_delete` |
| 检查操作员就绪状态 | `operator status` | 仅项目状态 |
| 初始化操作员凭据 | `operator init` | 刻意不可用 |
| 轮换操作员凭据 | `operator rotate` | 刻意不可用 |
| 恢复操作员保险库条目 | `operator restore-credential` | 刻意不可用 |
| 删除操作员保险库条目 | `operator delete-credential` | 刻意不可用 |
| 列出智能体 | `agents` | `vibebus_agents` |
| 项目快照 | `status` | `vibebus_status` |
| 完整性检查 | `doctor` | `vibebus_doctor` |
| 一致性备份 | `backup` | `vibebus_backup` |

注册默认以明文返回一次持有者令牌与恢复密钥。CLI `--store-credentials` 或 MCP `storeCredentials=true` 将这对写入当前用户的 OS 保险库，成功后返回 `secretsRedacted=true` 的元数据而非任一密钥。Windows 使用 Generic Credential；macOS 使用账号为 `VibeBus` 的 Keychain 通用密码。服务/目标为 `VibeBus:<project-id>:<agent>`，绝不进入仓库或 SQLite 数据库。

恢复接受显式恢复密钥或从匹配的保险库条目加载，撤销两个旧密钥，递增 `tokenGeneration`，并生成全新的一对。建档需要当前持有者令牌，并在不改变持有者令牌的情况下撤销先前的恢复密钥。若恢复或建档使用了保险库密钥，VibeBus 自动替换存储的一对；调用方也可以显式请求存储。轮换后的保险库写入失败会返回新的一对并附 `secretsRedacted=false` 与 `credentialStorageError`，因为隐藏它会使身份永久孤立。

变更或私有读取需要智能体身份。CLI 持有者解析顺序为 `--token`、`VIBEBUS_AGENT_TOKEN`、当前用户保险库。MCP 解析顺序为显式 `token`、保险库。因此仅当正确的项目/智能体保险库条目存在时，令牌字段才是可选的。`credential status` 绝不返回密钥材料。`credential delete` 仅移除 OS 条目；它不删除或撤销智能体，后续无令牌调用在凭据再次存储前都会失败。MCP 删除额外要求 `confirm=true`。同用户进程共享 Windows 凭据管理器或 macOS 钥匙串的信任边界。

项目操作员是破坏性维护的独立程序性能力，而非智能体角色。其数据库行仅包含 SHA-256 摘要与代数；密钥存储在 `VibeBusOperator:<project-id>` 下，不会与 `VibeBus:<project-id>:<agent>` 冲突。操作员变更命令首先要求真实终端与精确键入确认。它们在 MCP 中不存在。成功存储保险库会脱敏密钥。若初始化或轮换后的保险库写入失败，交互式响应返回唯一可用密钥加 `credentialStorageError`；维护者保护该密钥后，用 `operator restore-credential` 修复条目（无回显读取密钥）。`operator status` 以 `ready` 报告数据库/保险库代数一致，而不暴露密钥材料。`operator delete-credential` 需要精确的 `delete:<project-id>` 确认，仅移除 OS 保险库条目；数据库摘要与代数保持已配置状态，因此删除不能伪装成撤销或项目重置。若无当前密钥的另一份安全副本，删除对该数据库而言刻意不可逆。

## API 注入

API 提供方让插件向派生智能体 CLI（`codex`、`claude`、`gemini`、...）注入 Base URL、API 密钥与可选模型，而无需编辑任何用户全局配置。该设计镜像 cc-switch 的 provider 模型，但将密钥挡在文件之外：provider 元数据存于 `.vibebus/api/api.json`（项目级），API 密钥只存于 `VibeBusApi:<project-id>:<provider>` 下的 OS 凭据保险库。

| 能力 | CLI | MCP |
| --- | --- | --- |
| 添加 provider（+ 密钥存入保险库） | `api provider add --name X --base-url URL --format anthropic\|openai-chat\|openai-responses\|gemini [--model M] [--key K]` | — |
| 列出 provider | `api provider list` | `vibebus_api_provider_list` |
| 激活 provider | `api use --name X [--force] [--write-config]` | `vibebus_api_use` |
| 停用 | `api unuse` | `vibebus_api_unuse` |
| 恢复备份 | `api restore --n N`（1 = 最新，保留 10 份） | — |
| 本地健康检查 | `api health --name X` | `vibebus_api_health` |
| 网络探测 | `api health --name X --check-network`（TCP，5s 超时） | 同一工具，`checkNetwork=true` |
| 以注入环境启动 CLI | `api run -- <cli> [args...]` | — |
| 为环境文件解析密钥 | `api key get --name X`（仅 stdout） | — |
| 以注入环境派生 | `agent spawn --api X`（否则 profile `api`，否则当前激活项） | `vibebus_agent_spawn` |
| TUI 环境导出文件 | `api use` 时生成（`.vibebus/api/env.sh` / `env.ps1`） | `vibebus_api_use` 也会写入 |

注入语义：

- **默认零注入**：未配置 provider 或没有激活 provider 时，`agent spawn` 向子进程传递的环境与功能引入前完全相同（`api` 响应字段缺失）。
- **子进程中的优先级**：注入的 API 环境 → 显式 `profile.env`（胜出）→ `CI=1`（总是最后，抑制交互式提示）。
- **密钥永不落盘**：`api.json`、CLI 响应与日志只报告变量*名*与密钥*存在性*，绝不报告密钥值。环境导出文件将密钥替换为 `$(vibebus api key get --name X)`（PowerShell：`(& vibebus api key get --name X)[0]`），使密钥在源码解析时从保险库解析。
- **冲突守卫**：若进程环境已将某个注入变量设置为不同值，`api use` 以 `conflict` 失败；`--force` 覆盖。
- **写前备份**：每次 `use`/`unuse` 都将配置快照到 `.vibebus/api/backups/`（最新 10 份）；`restore --n 1` 还原到先前状态。`unuse` 将密钥保留在保险库并删除环境导出文件；`restore` 重写它们以匹配恢复的激活 provider。
- **可选配置文件注入**：`api use --write-config` 额外将注入环境深合并进 Claude Code 的项目级 `.claude/settings.json`（原文件备份到 `.vibebus/api/config-backups/`）；`unuse` 还原。默认关闭，因此除非被要求，VibeBus 绝不编辑用户全局配置。
- **Base URL 规范化**：去除尾部斜杠且只接受 `http(s)`，避免手动配置中出现的 `//` 路径拼接失败。
- **本地健康检查 + 可选网络探测**：`api health` 无网络流量地校验配置 + 保险库密钥存在性 + URL 形状；`--check-network` 额外向 Base URL 主机打开 TCP 连接（标准 `TcpStream`，5s 连接超时，无 HTTP 客户端依赖）。
- **审计轨迹**：`api use`/`api unuse`/`api restore` 向总线追加 `api.provider_activated` / `api.provider_deactivated` / `api.config_restored` 事件（载荷仅携带变量名）。MCP 工具通过共享处理器执行相同操作。
- **`api run`**：在激活 provider 的环境下以 `<cli> [args...]` 替换当前进程（启动时从保险库解析密钥）。无激活 provider 时以校验错误失败。

## 智能体派生

派生桥将外部智能体 CLI 派生为可追踪、有界的会话。

| 能力 | CLI | MCP |
| --- | --- | --- |
| 派生智能体会话 | `agent spawn --profile P --agent A [--api X] --task "..." [--role R] [--timeout-secs N]` | `vibebus_agent_spawn` |
| 回复澄清问题 | —（仅 MCP） | `vibebus_agent_reply` |
| 终止会话 | —（仅 MCP） | `vibebus_agent_kill` |
| 列出活跃会话 | —（仅 MCP） | `vibebus_agent_status` |

Profile 来自内置默认（`codex`、`claude`、`gemini`、`demo`），由项目根目录的 `.vibebus/agents.json` 叠加。派生创建任务、绑定会话线程并运行子进程；在终态结果（`[RESULT]` 标记）下桥记录自消息并完成任务，而 `[QUESTION]` 使会话保持存活以供稍后 `vibebus_agent_reply`，`[ERROR]`/超时终止它。派生子进程接收调用方环境加上任何 API 提供方注入与末尾的 `CI=1`；会话输出被摘要为事实，绝不完整转录。

## 能力注册表

| 能力 | CLI | MCP |
| --- | --- | --- |
| 从 SKILL.md 注册或更新技能 | `skill register --id <id> --name <name> --file <path> [--agents a,b] [--deps d1,d2]` | — |
| 列出已注册技能 | `skill list` | `vibebus_skill_list` |
| 为智能体 CLI 安装技能 | `skill install --id <id> --agent <cli>` | `vibebus_skill_install` |
| 列出已注册插件 | `plugin list` | `vibebus_plugin_list` |
| 将 `api.json` 中的 provider 同步进总线 | `api sync-providers` | — |

注册表（`providers`、`skills`、`plugins`）是项目级且声明式的：`skill register` 存储 id/version/name/agents-supported/dependencies 与内容；`skill install` 将 SKILL.md 复制到目标智能体的技能目录（例如 `.claude/skills/`）；`api sync-providers` 将 `.vibebus/api.json` provider upsert。注册是可查询事实，因此一个 provider 或技能定义可在每个受支持编码智能体间复用，而无需复制密钥。

## 消息

| 能力 | CLI | MCP |
| --- | --- | --- |
| 发送 | `send` | `vibebus_send` |
| 收件箱 | `inbox` | `vibebus_inbox` |
| 标记已读 | `read` | `vibebus_read` |
| ACK | `ack` | `vibebus_ack` |
| 关闭 | `close` | `vibebus_close` |

优先级为 `low`、`normal`、`high` 与 `urgent`。`requiresAck` 记录发送方意图；回执状态仍由收件人显式更新。

普通收件箱读取隐藏已关闭消息。CLI 调用方可组合 `inbox --all --include-closed`；MCP 调用方设置 `unreadOnly=false` 与 `includeClosed=true` 以检查回执历史。关闭仅限收件人，将消息标记为已读，且是可重试的终态动作。`requiresAck=true` 的消息必须先在关闭前 ACK。关闭后，read 与 ACK 变更冲突。

`send --idempotency-key <key>` / `idempotencyKey` 使外部重试返回原始消息。同一键用于不同请求则冲突。

## 任务

| 能力 | CLI | MCP |
| --- | --- | --- |
| 创建 | `task create` | `vibebus_task_create` |
| 认领 | `task claim` | `vibebus_task_claim` |
| 更新 | `task update` | `vibebus_task_update` |
| 完成 | `task complete` | `vibebus_task_complete` |
| 查看 | `task show` | `vibebus_task_show` |
| 列出 | `task list` | `vibebus_task_list` |

状态转换：

```text
pending --dependencies complete--> ready --claim--> claimed
claimed --> working | blocked | completed | abandoned
working --> review | blocked | completed | abandoned
review --> working | blocked | completed | abandoned
blocked --> working | abandoned
completed, abandoned --> terminal
```

每次更新都包含 `expectedVersion`。冲突需要重新读取并协调。

## 任务/线程绑定

| 能力 | CLI | MCP |
| --- | --- | --- |
| 绑定活跃任务 | `thread bind` | `vibebus_thread_bind` |
| 解绑任务 | `thread unbind` | `vibebus_thread_unbind` |
| 列出活跃/历史 | `thread list` | `vibebus_thread_list` |

只有当前任务所有者可以绑定或解绑。绑定时任务必须是非终态，且每任务至多一个活跃绑定。重复同一绑定或解绑返回既有记录；不同的活跃线程冲突。`thread list` 默认返回活跃绑定；CLI `--all` 或 MCP `activeOnly=false` 包含历史。完成或放弃任务会自动将其活跃绑定时间戳标记为已解绑。

线程 ID 是调用方提供的不透明标识符，1-128 个 ASCII 字母、数字、`-`、`_`、`.`、`:` 或 `/`。VibeBus 记录该关联，但不会创建、导航、唤醒或归档原生 Codex 任务。

## 路径预留

| 能力 | CLI | MCP |
| --- | --- | --- |
| 获取 | `reserve add` | `vibebus_reserve` |
| 续期自有租约 | `reserve renew` | `vibebus_reservation_renew` |
| 释放 | `reserve release` | `vibebus_release` |
| 列出活跃 | `reserve list` | `vibebus_reservations` |

TTL 为 1 至 86,400 秒。路径使用 `/` 规范化的项目相对语法。预留表达意图与冲突检测；它不是操作系统文件锁。

获取接受可选 `--task`/`taskId` 加幂等键。提供任务时，它必须是非终态，且请求路径必须通过该精确任务与智能体的角色策略或活跃覆盖。续期保留原始任务关联，要求已认证所有者加一个活跃、未过期的预留；新到期时间从续期时间计算。

## 责任策略与任务事实

| 能力 | CLI | MCP |
| --- | --- | --- |
| 检查生效角色策略 | `responsibility inspect` | `vibebus_responsibility_inspect` |
| 授予任务级覆盖 | `responsibility override` | `vibebus_responsibility_override` |
| 记录 Git 提交事实 | `fact git-commit` | `vibebus_git_commit_record` |
| 记录测试结果事实 | `fact test-result` | `vibebus_test_result_record` |

`.vibebus/responsibility.json` 版本 1 将角色名映射到 `allowedPaths`，并可提供 `defaultAllowedPaths`。每个模式是精确的项目相对路径、终态 `/**` 子树或全局 `**`；绝对路径、`..`、空段与其他通配符位置无效。文档限制为 64 KiB、128 个角色与每角色 256 个模式。文件缺失时，传统项目保持允许一切。文件存在但无效时，受保护操作失败即关闭。

覆盖只能由非终态任务的已认证所有者授予既有智能体。它将一个经校验的路径模式、受让人、原因与 60–86,400 秒 TTL 绑定到该任务。幂等键使精确重试安全，变更载荷冲突。覆盖仅授权责任：它不转移任务所有权、不抑制预留冲突、不授予操作系统访问权。

Git 事实将 40 或 64 十六进制提交 SHA、1–512 字节摘要与至多 200 个唯一规范化变更路径绑定到一个已拥有的非终态任务。每个路径必须通过责任授权。测试事实绑定稳定结果键、套件、`passed|failed|skipped|error` 结果、有界摘要/命令与可选报告制品（项目级或属于同一任务）。任务/提交与任务/结果键唯一性比通用幂等清理存活更久；精确语义重试返回原始事实，漂移冲突。

## 制品

| 能力 | CLI | MCP |
| --- | --- | --- |
| 发布 | `artifact publish` | `vibebus_artifact_publish` |
| 列出 | `artifact list` | `vibebus_artifact_list` |

发布要求项目内存在一个常规文件。VibeBus 存储 SHA-256 摘要、类型、摘要、可选任务 ID 与任意 JSON 元数据。任务级制品必须通过发布者与任务的责任授权；项目级制品保留既有项目边界行为。
Windows CLI 调用中，`--metadata-file <path>` 是复杂 JSON 的稳定形式，因为原生参数引用会改变内嵌引号。MCP 直接接受元数据对象。

制品发布接受幂等键。请求身份包含当前文件 SHA-256，因此更改文件并复用键会冲突而非返回过期制品。

## 已确认决策与智能体上下文

| 能力 | CLI | MCP |
| --- | --- | --- |
| 确认不可变决策 | `decision confirm` | `vibebus_decision_confirm` |
| 同步限定上下文 | `context sync` | `vibebus_context_sync` |

`decision confirm` 要求已认证所有者、非终态任务、项目级语义 `key`、1–1,024 UTF-8 字节摘要与可选制品 ID。任务级制品必须属于同一任务；项目级制品可被引用而无需复制。首次确认追加一个 `decision_confirmed` 事件。即使没有幂等键，重复精确键/载荷也返回原始决策，而更改任务、作者、摘要或规范化制品集会冲突。幂等键额外保护歧义首次响应。

`context sync` 返回一个已认证确定性投影，仅包含：

1. 智能体的活跃已拥有任务；
2. 这些任务的直接依赖；
3. 定向给智能体的未读消息；
4. 与已拥有/依赖任务范围相关的已确认决策；以及
5. 与同一范围相关的制品；
6. 同一范围的 Git 提交事实；以及
7. 同一范围的测试结果事实。

无关任务事实与已确认、已读或已关闭的消息被排除。制品暴露身份、任务、类型、路径、SHA-256 与有界摘要；VibeBus 绝不将制品内容读入上下文。Git 事实仅暴露提交身份、摘要与有界变更路径预览。测试事实暴露有界摘要/命令与报告引用，绝不日志。长任务/消息字段是带显式截断标志的有界预览。

CLI 默认 `--item-limit 100 --byte-budget 65536`；MCP 使用 `itemLimit` 与 `byteBudget`。条目上限接受 1–500，字节预算接受 4,096–1,048,576。`bytesUsed` 是序列化上下文条目的精确总和，而非响应信封。截断页返回 `hasMore=true` 与不透明 `nextCursor`；以 `--cursor`/`cursor` 传入以继续。游标编码最后确定性事实键而非 SQL 偏移，因此未变化的投影分页不会重复或遗漏。游标不是原子数据库快照：当并发任务/消息状态必须一致反映时，从头重启。

## 事件与订阅

| 能力 | CLI | MCP |
| --- | --- | --- |
| 查询事件 | `event list` | `vibebus_events` |
| 创建订阅 | `subscription create` | `vibebus_subscription_create` |
| 列出自有订阅 | `subscription list` | `vibebus_subscription_list` |
| 窥视可重放交付 | `subscription peek` | `vibebus_subscription_peek` |
| ACK 可重放交付 | `subscription ack` | `vibebus_subscription_ack` |
| 轮询并前进（传统） | `subscription poll` | `vibebus_subscription_poll` |

事件使用项目级单调递增 `sequence`。以 `afterSequence`/`--after` 查询并保留最后返回的序号。空事件类型过滤器表示全部类型；非空过滤器接受至多 32 个精确事件名。消息事件包含路由元数据，而非消息主题或正文。留存推进历史下限后，更旧的游标冲突而非静默返回部分历史；检查 `retention status` 并从 `eventsPrunedThroughSequence` 继续。

订阅属于一个已认证智能体，并按智能体与名称唯一。省略 `fromSequence` 从当前项目尾部开始；`0` 仅在留存历史下限仍为零时重放匹配历史。显式游标早于该下限则冲突。订阅视图暴露已提交游标、可选待投递与最近已确认投递 ID。

`peek` 至多创建一个包含至多 500 个匹配事件的待投递，而不推进已提交游标。重复 peek 返回同一投递与完整批次，即使新请求指定更小的上限或已有更新事件到达。投递在表示扫描的非匹配项目事件范围时可以包含零个匹配事件；它仍必须被 ACK 才能越过该范围前进。

`ack` 接受待投递 `deliveryId`，将已提交游标推进穿过投递范围，并清除待处理状态。重试最近成功的 ACK 返回原始游标与时间戳并附 `replayed=true`。错误或过期 ID 冲突。这提供对批次的至少一次访问，而非恰好一次处理；消费方必须使副作用幂等，并仅在完整批次成功后 ACK。

传统 `poll` 为兼容性保留。它返回至多 500 个事件并立即提交，因此提交后丢失的响应不会重放。它拒绝在可重放投递待处理时运行，因此不会静默越过未确认批次。

## 有界留存

| 能力 | CLI | MCP |
| --- | --- | --- |
| 预览候选 | `retention plan` | `vibebus_retention_plan` |
| 批准精确计划 | `operator approve-retention` | 刻意不可用 |
| 应用已确认计划 | `retention apply` | `vibebus_retention_apply` |
| 检查历史下限 | `retention status` | `vibebus_retention_status` |

预览与应用都要求已认证智能体。应用额外要求单独认证的操作员批准。默认：事件 90 天、始终保留 1,000 条近期事件、幂等记录与已关闭消息 30 天、终态任务/线程绑定历史 90 天。每个年龄接受 1–3,650 天；近期尾部接受 1–1,000,000 事件。`closedMessageMaxAgeDays` 必须至少为 `idempotencyMaxAgeDays`，使缓存的发送重试不能引用已删除消息。

预览是只读的，返回策略、订阅保护细节、精确候选数与 `planId`。本地维护者随后运行 `operator approve-retention`，重复任何自定义策略标志，审阅打印到终端的完整当前计划，并键入完整计划 ID。批准默认为 600 秒，接受 60–3,600 秒，并绑定精确计划与当前操作员代数。它记录在 `retention_approvals` 中而不追加领域事件，因此批准不会使它所授权的计划失效。

应用必须重复相同自定义策略值并提供该 ID。在一个 `BEGIN IMMEDIATE` 事务中，它重新计算计划、从当前操作员代数选择一个未过期/未消费批准、执行有界删除、消费该批准、追加审计事件并存储报告。任何介入的领域事件、游标变更、先前清理、候选变更、批准过期或操作员轮换都会阻止新的应用。并发尝试只能消费一次批准；胜出者正常完成，败者返回存储报告并附 `replayed=true`。已成功运行的后续重试不需要新批准，也不能删除两次。

事件候选形成一个连续前缀：足够旧、位于或低于最慢订阅游标、且位于近期尾部之外。待投递的可重放交付保持其已提交游标不变，因此其完整范围保持受保护。应用也移除过期幂等记录、旧已关闭回执、产生的无回执消息与属于终态任务的旧未绑定历史。它保留活跃状态并追加 `retention_applied` 新审计事件。该操作不运行 `VACUUM`。

## 离线压缩

| 能力 | CLI | MCP |
| --- | --- | --- |
| 压缩既有数据库 | `maintenance compact --backup <new-path>` | 刻意不可用 |

压缩与逻辑留存分离，且不是智能体变更。先停止所有项目用户，确认 Operator `ready=true`，选择不存在的备份路径，并从真实终端调用命令。CLI 打印目标项目，并在读取保险库支持的 Operator 密钥前要求精确文本 `compact:<project-id>`。重定向输入在通过维护路径打开数据库前失败。

该操作绝不创建或迁移数据库。它要求当前模式、WAL、匹配项目行、零非终态任务、零活跃任务/线程绑定、零未过期预留、快速失败检查点与排他锁、已验证的 `VACUUM` 前备份以及数据库卷上的保守空闲空间。成功恢复并检查点 WAL、验证完整性/外键/模式、写入有界 `compaction_started`/`compaction_completed` 事件，并返回哈希、大小、页/空闲页计数、回收字节、备份身份、Operator 代数与 `verified=true`。后续前置条件或压缩步骤失败时，有效备份可能保留；重试时绝不覆盖它。

## 结构化交接

| 能力 | CLI | MCP |
| --- | --- | --- |
| 发送交接 | `handoff send` | `vibebus_handoff_send` |
| 恢复快照 | `handoff snapshot` | `vibebus_handoff_snapshot` |
| 构建仅审阅提案 | `handoff propose` | `vibebus_handoff_propose` |

交接是包含 `summary`、可选 `taskId`、`decisions`、`artifacts`、`blockers` 与 `nextActions` JSON 正文的定向消息。VibeBus 强制 `high` 优先级与 `requiresAck=true`，验证引用的任务与制品，并通过幂等键支持重试去重。收件人应阅读正文、按其行动并调用 `ack`。

已认证快照组合未读消息、非终态已拥有任务、其活跃任务/线程绑定、活跃自有预留、智能体的近期制品、给定序号后的近期可用事件、最新事件序号与留存状态。它将过期提供的序号钳制到留存历史下限，使恢复保持可用。它是操作恢复视图。确定性、任务级、预算化的依赖、决策、制品与生命周期事实投影请使用 `context sync`。

`handoff propose` 是单个任务的所有者认证读模型。它返回任务、一个有界摘要与限尾 Git 事实、测试结果、决策与制品。它不创建消息、收件人、ACK 义务或副作用。Stop Hook 将此提案写入插件数据以供审阅；人类或任务必须仍然显式调用 `handoff send`。

## 幂等规则

幂等键按项目、已认证智能体与操作限定。有效键为 1-128 个 ASCII 字母、数字、`-`、`_`、`.` 或 `:`。它们可用于消息/交接发送、预留获取/续期、制品发布、决策确认、责任覆盖、Git 事实与测试结果事实。同一键加同一有效请求返回存储响应；同一键加不同请求在记录仍在配置的保留窗口内时返回冲突。已确认决策、Git 提交与测试结果也通过不可变领域键保留语义去重。任务创建已有稳定的调用方选择任务 ID，而任务认领与更新依赖原子状态/版本检查。

## MCP root 规则

使用捆绑插件时，每次调用都传入绝对仓库根目录作为 `root`。直接 `vibebus mcp --root <path>` 启动可以省略它，因为进程已有显式默认根目录。
