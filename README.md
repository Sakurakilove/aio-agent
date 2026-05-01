# AIO Agent - All In One AI Agent 解决方案

[![Version](https://img.shields.io/badge/version-1.0.0-blue.svg)](https://github.com/aio-agent/aio-agent/releases)
[![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20Linux%20%7C%20macOS-lightgrey.svg)]()
[![Build Status](https://img.shields.io/badge/build-passing-brightgreen.svg)]()
[![Tests](https://img.shields.io/badge/tests-9%2F9%20passing-brightgreen.svg)]()

**AIO Agent** 是一个用Rust编写的高性能、模块化AI Agent框架，集成了OpenClaw网关系统、Hermes Agent Skills机制、多Agent协作、预算控制、上下文管理等企业级特性。

## 快速开始

```bash
# 克隆项目
git clone https://github.com/aio-agent/aio-agent.git
cd aio-agent

# 编译
cargo build --release

# 运行配置引导
./target/release/aio-agent setup

# 或直接运行
./target/release/aio-agent
```

## 特性概览

### 核心架构
- **27个独立模块**: 每个模块职责单一，易于扩展
- **异步运行时**: 基于tokio全功能异步运行时，支持高并发
- **类型安全**: 充分利用Rust类型系统，编译期捕获错误

### AI能力
- **多提供商支持**: OpenAI兼容API，支持OpenAI、Ollama、自定义提供商
- **流式输出**: SSE流式响应，实时显示生成内容
- **工具调用**: 22+预设工具，支持Web搜索、文件操作、终端执行、网页爬取、浏览器自动化
- **Skills系统**: Hermes Agent风格的Skills，YAML frontmatter定义，权限控制
- **多Agent协作**: Crew模式，支持顺序/并行/层级处理

### 平台集成
- **12个聊天平台**: Telegram、Discord、Slack、WhatsApp、Signal、Matrix、Teams、Webhook、QQ机器人、企业微信、飞书、钉钉
- **浏览器自动化**: 6个工具（导航、截图、点击、填表、获取内容、执行JS）
- **HTTP API服务器**: Axum框架，CORS支持，RESTful端点
- **CLI终端**: 9个命令，交互式CLI，配置引导向导

### 企业特性
- **预算控制**: 迭代预算、工具预算、动态调整
- **上下文管理**: 上下文压缩、敏感信息清理
- **错误处理**: 错误分类、自动重试、指数退避
- **记忆系统**: SQLite持久化，支持会话管理、全文搜索、记忆预取
- **委派系统**: 子Agent委派，限制迭代和工具权限
- **调度器**: Cron风格定时任务，支持every/hourly/daily
- **网关系统**: OpenClaw风格网关，支持多通道管理

### 安全特性
- **权限系统**: 正则表达式模式匹配，允许/拒绝规则
- **配置引导**: 交互式setup向导，支持QuickStart/Manual/Import模式
- **敏感信息清理**: 上下文scrubber，防止API密钥泄露
- **Doctor诊断**: 9项系统检查，确保配置正确

## CLI命令

```bash
# 启动交互模式
./target/release/aio-agent

# 启动HTTP API服务器
./target/release/aio-agent serve --host 127.0.0.1 --port 3000

# 单次查询（支持流式输出）
./target/release/aio-agent query "搜索最新AI框架信息" --stream

# 配置引导
./target/release/aio-agent setup

# 委派任务
./target/release/aio-agent delegate "分析这个数据集" --max-iterations 20

# 添加定时任务
./target/release/aio-agent schedule "每日报告" "echo report" --schedule-type daily

# 列出平台适配器
./target/release/aio-agent adapters

# 运行测试套件
./target/release/aio-agent test

# 查看状态
./target/release/aio-agent status

# 运行系统诊断
./target/release/aio-agent doctor

# 查看帮助
./target/release/aio-agent --help
```

## HTTP API

启动服务器后，可以使用REST API：

```bash
# 健康检查
curl http://localhost:3000/health

# 查看状态
curl http://localhost:3000/status

# 发送聊天请求
curl -X POST http://localhost:3000/chat \
  -H "Content-Type: application/json" \
  -d '{"message": "你好，请介绍一下自己"}'

# 列出可用工具
curl http://localhost:3000/tools
```

## 架构设计

```
src/
├── agent_engine/     # Agent核心引擎
├── adapters/         # 12个平台适配器
│   ├── telegram.rs
│   ├── discord.rs
│   ├── slack.rs
│   ├── whatsapp.rs
│   ├── signal.rs
│   ├── matrix.rs
│   ├── teams.rs
│   ├── webhook.rs
│   ├── qqbot.rs
│   ├── wecom.rs
│   ├── feishu.rs
│   └── dingtalk.rs
├── api/              # HTTP API服务器
├── cli/              # CLI交互终端
├── config/           # 配置管理
├── doctor/           # 系统诊断
├── streaming/        # 流式输出
├── setup/            # 配置引导
├── tools/            # 22+工具（含浏览器自动化）
└── ...               # 其他15个模块
```

## 贡献指南

欢迎提交Issue和Pull Request！

1. Fork本项目
2. 创建特性分支 (`git checkout -b feature/amazing-feature`)
3. 提交更改 (`git commit -m 'Add amazing feature'`)
4. 推送到分支 (`git push origin feature/amazing-feature`)
5. 提交Pull Request

## 许可证

本项目采用MIT许可证。详见[LICENSE](LICENSE)文件。

## 致谢

本项目参考和整合了以下优秀开源项目的理念：

- **OpenClaw**: 网关系统和通道管理
- **Hermes Agent**: Skills机制和元数据系统
- **AutoGPT**: 自主Agent循环
- **LangChain**: 工具链设计
- **CrewAI**: 多Agent协作

感谢所有开源贡献者！
