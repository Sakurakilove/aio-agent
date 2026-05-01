# Changelog

## v1.0.0 (2026-04-30)

### 核心功能
- 核心Agent引擎，支持OpenAI兼容API
- 22+预设工具（Web搜索、文件操作、终端执行、网页爬取等）
- SQLite记忆系统，支持会话管理和全文搜索
- Hermes Agent风格Skills系统，YAML frontmatter定义
- 多Agent协作（Crew模式），支持顺序/并行/层级处理
- 预算控制系统（迭代预算、工具预算）
- 上下文管理（压缩、敏感信息清理）
- 错误分类和自动重试机制
- 子Agent委派系统
- Cron风格调度器

### 平台和集成
- OpenClaw风格网关系统
- Telegram和Discord平台适配器
- HTTP REST API服务器（Axum框架）
- 交互式CLI终端
- SSE流式输出支持

### 配置和部署
- 配置引导向导（QuickStart/Manual/Import模式）
- 环境变量配置支持
- TOML配置文件支持
- 多提供商管理（OpenAI、Ollama、自定义）

### 安全和权限
- 基于正则表达式的工具权限系统
- 上下文敏感信息清理
- 输入验证和类型检查

### 日志和监控
- tracing日志系统
- 结构化日志输出
- 日志文件持久化
