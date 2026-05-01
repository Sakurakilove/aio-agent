<div align="center">

<img src="images/logo.png" alt="AIO Agent Logo" width="200" height="auto">

# 🤖 AIO Agent

**Production-Grade AI Agent Framework in Rust**

[![Version](https://img.shields.io/badge/version-1.4.1-blue.svg)](https://github.com/Sakurakilove/aio-agent/releases)
[![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20Linux%20%7C%20macOS-lightgrey.svg)]()
[![Build](https://img.shields.io/badge/build-passing-brightgreen.svg)]()
[![Tests](https://img.shields.io/badge/tests-passing-brightgreen.svg)]()

高性能 · 模块化 · 类型安全 · 生产就绪

对标 LangGraph · CrewAI · AutoGen · OpenAI Swarm

[🚀 快速开始](#-快速开始) · [📖 文档](#-使用方式) · [🏗️ 架构](#️-架构) · [📊 对比](#-与主流框架对比) · [⬇️ 下载](https://github.com/Sakurakilove/aio-agent/releases)

</div>

---

## ✨ 核心特性

<table>
<tr>
<td width="50%">

🦀 **Rust 原生性能**
零成本抽象，内存安全，编译期保证

🧠 **LLM 驱动 ReAct 循环**
完整的多轮工具调用，OpenAI API 完全兼容

🛡️ **Guardrails 验证**
长度/关键词/正则三重输入输出防护

</td>
<td width="50%">

👤 **Human-in-the-Loop**
控制台交互审批，4 级风险等级控制

🔄 **Agent Handoff**
5 个预置专业角色，LLM 驱动多 Agent 转交

💾 **持久化检查点**
SQLite 状态快照，断点恢复

</td>
</tr>
<tr>
<td width="50%">

📊 **可观测性**
回调钩子系统，13 种事件类型追踪

🔧 **22+ 内置工具**
文件/网络/终端/数据处理全覆盖

</td>
<td width="50%">

🏢 **多 Agent 协作**
Crew 模式，Sequential/Hierarchical 编排

🔐 **多提供商配置**
OpenAI/Ollama/自定义，运行时热切换

</td>
</tr>
</table>

## 📦 快速开始

### 预编译版本（推荐）

从 [GitHub Releases](https://github.com/Sakurakilove/aio-agent/releases) 下载：

```bash
# Windows
aio-agent.exe setup

# Linux/macOS
chmod +x aio-agent && ./aio-agent setup
```

### 从源码编译

```bash
git clone https://github.com/Sakurakilove/aio-agent.git
cd aio-agent
cargo build --release
./target/release/aio-agent setup
```

## 🚀 使用方式

### CLI 交互模式

```bash
aio-agent                                    # 交互式终端
aio-agent query "分析这个项目"                  # 单次提问
aio-agent query "搜索AI框架" --stream          # 流式输出
```

<details>
<summary>📋 交互模式内置命令</summary>

| 命令 | 说明 |
|------|------|
| `/ask <问题>` | 向 Agent 提问 |
| `/tools` | 查看所有可用工具 |
| `/config` | 查看当前配置 |
| `/provider [名称]` | 查看/切换 LLM 提供商 |
| `/memory` | 查看记忆会话 |
| `status` | 显示 Agent 状态 |
| `help` | 显示帮助 |
| `exit` | 退出 |

</details>

### HTTP API 模式

```bash
aio-agent serve --host 127.0.0.1 --port 3000
```

```bash
curl http://localhost:3000/health                          # 健康检查
curl http://localhost:3000/status                          # 查看状态
curl -X POST http://localhost:3000/chat -H "Content-Type: application/json" \
  -d '{"message": "你好"}'                                 # 聊天
curl http://localhost:3000/providers                       # 列出提供商
curl -X POST http://localhost:3000/providers/switch \
  -H "Content-Type: application/json" -d '{"name":"ollama"}'  # 切换提供商
curl http://localhost:3000/tools                           # 列出工具
curl http://localhost:3000/memory/sessions                 # 记忆会话
```

### 模型管理

```bash
aio-agent model list                                        # 列出提供商
aio-agent model switch openai                               # 切换提供商
aio-agent model add my-provider --api-key sk-xxx \
  --base-url https://api.example.com/v1 --model gpt-4      # 添加提供商
aio-agent model test                                        # 测试连接
```

### 其他命令

```bash
aio-agent status                                            # 查看状态
aio-agent doctor                                            # 系统诊断
aio-agent setup                                             # 配置引导
aio-agent delegate "分析数据" --max-iterations 20            # 委派任务
aio-agent schedule "每日报告" "echo report" --schedule-type daily  # 定时任务
```

## 🏗️ 架构

### 33 个模块

<details>
<summary>📁 查看完整模块列表</summary>

```
src/
├── agent_engine/      # Agent 核心引擎（ReAct 循环 + LLM 集成）
├── providers/         # LLM 提供商（OpenAI 兼容 API）
├── config/            # 配置管理（TOML/JSON/YAML + 多提供商）
├── tools/             # 22+ 工具集
├── memory/            # 记忆系统（SQLite 持久化）
├── messaging/         # 消息系统（tool_call_id 完整支持）
├── permissions/       # 权限系统
├── budget/            # 预算控制（迭代 + 工具）
├── context/           # 上下文压缩与清理
├── errors/            # 错误分类与重试
├── delegation/        # 任务委派（LLM 驱动子 Agent）
├── callbacks/         # 回调钩子系统（13 种事件）
├── guardrails/        # Guardrails 输入输出验证
├── output_parser/     # 结构化输出解析
├── human_in_loop/     # Human-in-the-Loop 人机协作
├── handoff/           # Agent Handoff 转交机制
├── checkpoint/        # 持久化检查点（SQLite）
├── agents/            # Crew 多 Agent 协作（LLM 驱动）
├── streaming/         # 流式输出（SSE）
├── cli/               # CLI 交互终端
├── api/               # HTTP API 服务器（Axum）
├── adapters/          # 12 个平台适配器
├── workflows/         # 工作流引擎
├── skills/            # Skills 系统（YAML 定义）
├── lanes/             # Lane 管理
├── gateway/           # 网关系统
├── setup/             # 配置引导向导
├── doctor/            # 系统诊断
├── logging/           # 日志系统
├── scheduler/         # 定时任务
├── interrupt/         # 中断处理
└── tasks/             # 任务管理
```

</details>

### Agent 执行流程

```
用户输入 ──→ Guardrails 输入验证 ──→ AioAgent.run_conversation()
                                         │
                                    构建 LLM 消息 + 工具定义
                                         │
                                    LlmProvider.chat_completion()
                                         │
                                    ┌────┴────┐
                                    │         │
                               tool_calls   纯文本 ──→ 返回响应
                                    │
                               HITL 审批
                                    │
                               执行工具 ──→ Tool 消息 ──→ 继续循环
                                         │
                                    Guardrails 输出验证
                                         │
                                    保存检查点 + 记忆
                                         │
                                    回调事件（AgentEnd）
```

## 🛡️ Guardrails

对标 OpenAI Swarm / LangGraph 的输入输出验证：

```rust
LengthGuardrail::new(100_000, 50_000);           // 长度限制

KeywordGuardrail::new(
    vec!["rm -rf /", "drop table"],               // 禁止关键词
    vec!["password", "api_key"],                   // 警告关键词
);

RegexGuardrail::new("no_sql", r"DROP\s+TABLE", true)?;  // 正则匹配
```

## 👤 Human-in-the-Loop

对标 AutoGen / LangGraph 的人机协作：

| Handler | 说明 | 适用场景 |
|---------|------|---------|
| `ConsoleApprovalHandler` | 控制台交互：批准/拒绝/修改 | 开发调试 |
| `AutoApprovalHandler` | 按风险等级自动审批 | 生产部署 |

风险等级：`Low` → `Medium` → `High` → `Critical`

高风险工具（terminal、file_write、remove）自动触发审批。

## 🔄 Agent Handoff

对标 OpenAI Swarm 的多 Agent 转交：

| Agent | 角色 | 可转交给 |
|-------|------|---------|
| `default` | 通用助手 | 所有 Agent |
| `researcher` | 专业研究员 | writer, analyst |
| `writer` | 专业写作者 | reviewer |
| `analyst` | 数据分析师 | writer |
| `coder` | 编程专家 | reviewer |
| `reviewer` | 质量审核员 | writer, coder |

## 🧰 22+ 内置工具

| 类别 | 工具 |
|------|------|
| 🌐 网络 | `web_search` `web_fetch` |
| 📄 文件 | `file_read` `file_write` `patch_file` `search_files` `list_dir` `file_info` |
| 💻 系统 | `terminal` `env` `system_info` |
| 📁 目录 | `mkdir` `remove` `copy` `move` |
| 🔢 数据 | `json_tool` `url_tool` `text_tool` `datetime_tool` |
| 🧮 计算 | `calculator` `base64_tool` `hash_tool` `regex_tool` |

## ⚙️ 配置

支持 TOML / JSON / YAML，默认位置 `~/.aio-agent/aio-agent.toml`：

```toml
[agent]
model = "gpt-4"
max_iterations = 10
timeout_seconds = 300

[providers]
active = "openai"

[[providers.providers]]
name = "openai"
api_key = ""                    # 或设置 AIO_AGENT_API_KEY 环境变量
base_url = "https://api.openai.com/v1"
default_model = "gpt-4"
models = ["gpt-4", "gpt-4o", "gpt-3.5-turbo"]
enabled = true

[[providers.providers]]
name = "ollama"
api_key = ""
base_url = "http://localhost:11434/v1"
default_model = "llama3"
models = ["llama3", "mistral", "codellama"]
enabled = true

[memory]
path = "~/.aio-agent/memory.db"

[permissions]
allow = [".*"]
deny = []
```

**环境变量：**

| 变量 | 说明 | 默认值 |
|------|------|--------|
| `AIO_AGENT_API_KEY` | API 密钥 | 配置文件值 |
| `AIO_AGENT_API_URL` | API 地址 | `https://api.openai.com/v1` |
| `AIO_AGENT_MODEL` | 默认模型 | `gpt-4` |

## 📊 与主流框架对比

| 特性 | AIO Agent | LangGraph | CrewAI | AutoGen | Swarm |
|:-----|:---------:|:---------:|:------:|:-------:|:-----:|
| 语言 | **Rust** | Python | Python | Python | Python |
| Agent 循环 | ReAct | 图状态机 | ReAct | 对话 | Routine |
| 多 Agent | Crew+Handoff | 图编排 | 角色 | 对话 | Handoff |
| Guardrails | ✅ 3种 | ✅ | ❌ | ❌ | ✅ |
| HITL | ✅ 2种 | ✅ | ❌ | ✅ | ❌ |
| 持久化 | ✅ SQLite | ✅ | ❌ | ❌ | ❌ |
| 可观测性 | ✅ 13事件 | ✅ | ❌ | ❌ | ✅ |
| 结构化输出 | ✅ 5格式 | ✅ | ✅ | ❌ | ✅ |
| 流式输出 | ✅ SSE | ✅ | ✅ | ✅ | ✅ |
| 模型无关 | ✅ | ✅ | ✅ | ✅ | ❌ |
| 性能 | 🚀 原生 | 🐍 | 🐍 | 🐍 | 🐍 |

## 🤝 贡献

欢迎提交 Issue 和 Pull Request！

1. Fork → 2. Branch → 3. Commit → 4. Push → 5. PR

## 📄 许可证

[MIT](LICENSE)

## 🙏 致谢

- **LangGraph** - 图编排、持久化状态、可观测性
- **CrewAI** - 多 Agent 角色协作
- **AutoGen** - Human-in-the-Loop、对话驱动
- **OpenAI Swarm** - Agent Handoff、Guardrails
- **PydanticAI** - 结构化输出、类型安全
- **Semantic Kernel** - 企业级连接器
- **Smolagents** - Code Agent 范式

---

<div align="center">
  <img src="images/avatar.png" alt="Sakurakilove" width="40" height="40">
  <p><a href="https://github.com/Sakurakilove">Sakurakilove</a></p>
</div>
