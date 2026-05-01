# AIO Agent - All In One AI Agent 框架

[![Version](https://img.shields.io/badge/version-1.3.0-blue.svg)](https://github.com/Sakurakilove/aio-agent/releases)
[![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20Linux%20%7C%20macOS-lightgrey.svg)]()
[![Build Status](https://img.shields.io/badge/build-passing-brightgreen.svg)]()
[![Tests](https://img.shields.io/badge/tests-passing-brightgreen.svg)]()

**AIO Agent** 是一个用 Rust 编写的高性能、模块化 AI Agent 框架，对标 LangGraph、CrewAI、AutoGen、OpenAI Swarm 等主流框架，提供完整的生产级 Agent 能力。

## ✨ 核心亮点

- 🦀 **Rust 原生性能** - 零成本抽象，内存安全，编译期保证
- 🧠 **LLM 驱动的 Agent 循环** - 完整的 ReAct 循环，支持多轮工具调用
- 🛡️ **Guardrails 输入输出验证** - 长度/关键词/正则三重防护
- 👤 **Human-in-the-Loop** - 人机协作，工具执行前人工审批
- 🔄 **Agent Handoff** - 多 Agent 转交机制，5 个预置专业角色
- 💾 **持久化检查点** - SQLite 状态快照，断点恢复
- 📊 **可观测性** - 回调钩子系统，13 种事件追踪
- 🔧 **22+ 内置工具** - 文件/网络/终端/数据处理全覆盖
- 🏢 **多 Agent 协作** - Crew 模式，Sequential/Hierarchical 编排
- 🔐 **多提供商配置** - OpenAI/Ollama/自定义，运行时切换

## 📦 快速安装

### 预编译版本（推荐）

从 [GitHub Releases](https://github.com/Sakurakilove/aio-agent/releases) 下载对应平台的预编译二进制文件。

```bash
# Windows
aio-agent.exe setup

# Linux/macOS
chmod +x aio-agent
./aio-agent setup
```

### 从源码编译

```bash
git clone https://github.com/Sakurakilove/aio-agent.git
cd aio-agent
cargo build --release

# 运行配置引导
./target/release/aio-agent setup
```

## 🚀 使用方式

### CLI 交互模式

```bash
# 启动交互式终端
aio-agent

# 直接提问
aio-agent query "帮我分析这个项目的架构"

# 流式输出
aio-agent query "搜索最新AI框架" --stream
```

交互模式内置命令：

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

### HTTP API 模式

```bash
# 启动 API 服务器
aio-agent serve --host 127.0.0.1 --port 3000
```

```bash
# 健康检查
curl http://localhost:3000/health

# 查看状态（含当前提供商信息）
curl http://localhost:3000/status

# 发送聊天请求
curl -X POST http://localhost:3000/chat \
  -H "Content-Type: application/json" \
  -d '{"message": "你好，请介绍一下自己"}'

# 列出可用工具
curl http://localhost:3000/tools

# 列出所有提供商
curl http://localhost:3000/providers

# 切换提供商
curl -X POST http://localhost:3000/providers/switch \
  -H "Content-Type: application/json" \
  -d '{"name": "ollama"}'

# 查看提供商统计
curl http://localhost:3000/providers/stats

# 查看记忆会话
curl http://localhost:3000/memory/sessions
```

### 模型管理

```bash
# 列出所有模型提供商
aio-agent model list

# 切换提供商
aio-agent model switch openai

# 添加新提供商
aio-agent model add my-provider --api-key sk-xxx --base-url https://api.example.com/v1 --model gpt-4

# 移除提供商
aio-agent model remove my-provider

# 测试当前模型连接
aio-agent model test
```

### 其他命令

```bash
# 查看状态
aio-agent status

# 系统诊断
aio-agent doctor

# 配置引导
aio-agent setup

# 委派任务
aio-agent delegate "分析这个数据集" --max-iterations 20

# 添加定时任务
aio-agent schedule "每日报告" "echo report" --schedule-type daily

# 列出平台适配器
aio-agent adapters
```

## 🏗️ 架构设计

### 模块总览（33 个模块）

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

### Agent 执行流程

```
用户输入 (CLI / HTTP API)
    ↓
Guardrails 输入验证
    ↓
AioAgent.run_conversation()
    ├─ 添加 System 提示消息
    ├─ 构建 LLM 消息（含 tool_calls / tool_call_id）
    ├─ 构建工具定义（22+ 工具 schema）
    ↓
LlmProvider.chat_completion() → OpenAI 兼容 API
    ↓
解析 LLM 响应
    ├─ tool_calls → HITL 审批 → 执行工具 → Tool 消息（含 tool_call_id）→ 继续循环
    └─ 纯文本 → 返回最终响应
    ↓
Guardrails 输出验证
    ↓
保存检查点（Checkpoint）
    ↓
保存会话到记忆系统（SQLite）
    ↓
触发回调事件（AgentEnd）
    ↓
返回 AgentResult
```

## 🛡️ Guardrails 系统

对标 OpenAI Swarm / LangGraph 的输入输出验证：

```rust
// 长度限制
LengthGuardrail::new(100000, 50000)

// 关键词过滤
KeywordGuardrail::new(
    vec!["rm -rf /", "drop table"],  // 禁止
    vec!["password", "api_key"],      // 警告
)

// 正则匹配
RegexGuardrail::new("no_sql", r"DROP\s+TABLE", true)
```

## 👤 Human-in-the-Loop

对标 AutoGen / LangGraph 的人机协作：

- **ConsoleApprovalHandler** - 控制台交互审批（批准/拒绝/修改）
- **AutoApprovalHandler** - 按风险等级自动审批
- 4 级风险等级：Low / Medium / High / Critical
- 高风险工具（terminal、file_write、remove）自动触发审批

## 🔄 Agent Handoff

对标 OpenAI Swarm 的多 Agent 转交：

| Agent | 角色 | 可转交给 |
|-------|------|---------|
| default | 通用助手 | 所有 Agent |
| researcher | 专业研究员 | writer, analyst |
| writer | 专业写作者 | reviewer |
| analyst | 数据分析师 | writer |
| coder | 编程专家 | reviewer |
| reviewer | 质量审核员 | writer, coder |

## 🧰 22+ 内置工具

| 类别 | 工具 |
|------|------|
| **网络** | web_search, web_fetch |
| **文件** | file_read, file_write, patch_file, search_files, list_dir, file_info |
| **系统** | terminal, env, system_info |
| **目录** | mkdir, remove, copy, move |
| **数据** | json_tool, url_tool, text_tool, datetime_tool |
| **计算** | calculator, base64_tool, hash_tool, regex_tool |

## ⚙️ 配置文件

支持 TOML / JSON / YAML 格式，默认位置 `~/.aio-agent/aio-agent.toml`：

```toml
[agent]
model = "gpt-4"
max_iterations = 10
timeout_seconds = 300

[providers]
active = "openai"

[[providers.providers]]
name = "openai"
api_key = ""           # 或设置环境变量 AIO_AGENT_API_KEY
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

## 🌐 环境变量

| 变量 | 说明 | 默认值 |
|------|------|--------|
| `AIO_AGENT_API_KEY` | API 密钥 | 配置文件中的值 |
| `AIO_AGENT_API_URL` | API 地址 | https://api.openai.com/v1 |
| `AIO_AGENT_MODEL` | 默认模型 | gpt-4 |

## 📊 与主流框架对比

| 特性 | AIO Agent | LangGraph | CrewAI | AutoGen | OpenAI Swarm |
|------|-----------|-----------|--------|---------|--------------|
| 语言 | Rust | Python | Python | Python | Python |
| Agent 循环 | ReAct | 图状态机 | ReAct | 对话驱动 | Routine |
| 多 Agent | Crew + Handoff | 图编排 | 角色编排 | 对话编排 | Handoff |
| Guardrails | ✅ 3 种 | ✅ 条件边 | ❌ | ❌ | ✅ |
| HITL | ✅ 2 种 | ✅ 断点 | ❌ | ✅ 核心 | ❌ |
| 持久化 | ✅ SQLite | ✅ 多后端 | ❌ | ❌ | ❌ |
| 可观测性 | ✅ 13 事件 | ✅ LangSmith | ❌ | ❌ | ✅ Tracing |
| 结构化输出 | ✅ 5 格式 | ✅ | ✅ | ❌ | ✅ |
| 流式输出 | ✅ SSE | ✅ 最完善 | ✅ 基本 | ✅ | ✅ |
| 模型无关 | ✅ 多提供商 | ✅ | ✅ | ✅ | ❌ OpenAI |
| 性能 | 🚀 原生 | 🐍 解释型 | 🐍 | 🐍 | 🐍 |

## 🤝 贡献指南

欢迎提交 Issue 和 Pull Request！

1. Fork 本项目
2. 创建特性分支 (`git checkout -b feature/amazing-feature`)
3. 提交更改 (`git commit -m 'Add amazing feature'`)
4. 推送到分支 (`git push origin feature/amazing-feature`)
5. 提交 Pull Request

## 📄 许可证

本项目采用 MIT 许可证。详见 [LICENSE](LICENSE) 文件。

## 🙏 致谢

本项目参考和整合了以下优秀开源项目的理念：

- **LangGraph** - 图编排、持久化状态、可观测性
- **CrewAI** - 多 Agent 角色协作
- **AutoGen** - Human-in-the-Loop、对话驱动
- **OpenAI Swarm** - Agent Handoff、Guardrails
- **PydanticAI** - 结构化输出、类型安全
- **Semantic Kernel** - 企业级连接器
- **Smolagents** - Code Agent 范式
