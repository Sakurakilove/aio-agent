# Rust-Agent 项目联动性和组合性修复计划

## 当前状态

### 已完成的修复

1. **agent_engine 与 LLM 的实际调用联动** ✓
   - 在 `AioAgent` 结构体中添加了 `llm_provider: LlmProvider` 字段
   - `new()` 方法现在会从配置中读取活跃的 provider 并初始化 LLM 客户端
   - `run_conversation()` 方法现在会：
     - 构建 LLM 消息（转换内部 Message 为 LlmChatMessage）
     - 构建工具定义（从 ToolRegistry 提取 schema 转为 ToolDefinition）
     - 调用 LLM API 获取响应
     - 解析 tool_calls 并执行对应工具
     - 将工具结果反馈给 LLM 进行多轮对话
   - 添加了 `switch_provider()` 方法支持运行时切换 provider
   - 修复了 `ChatMessage.tool_calls` 字段类型从 `Option<ToolCall>` 改为 `Option<Vec<ToolCall>>`

2. **工具注册完整性** ✓
   - 从原来的 4 个工具增加到 22+ 个工具：
     - WebSearchTool, FileReadTool, FileWriteTool, TerminalTool
     - WebFetchTool, SearchFilesTool, PatchFileTool, ListDirTool
     - JsonTool, UrlTool, TextTool, DateTimeTool
     - FileInfoTool, MkdirTool, RemoveTool, CopyTool, MoveTool
     - EnvTool, SystemInfoTool, CalculatorTool, Base64Tool, HashTool, RegexTool

3. **模型切换与 agent 的联动** ✓
   - `AioAgent::switch_provider()` 方法支持运行时切换
   - CLI 的 `model switch` 命令会更新配置文件
   - HTTP API 添加了 `/providers/switch` 端点

4. **HTTP API 与 agent 的联动** ✓
   - 增强了 `/status` 端点，显示当前 provider 信息
   - 新增 `/providers` - 列出所有提供商
   - 新增 `/providers/switch` - 切换提供商
   - 新增 `/providers/stats` - 提供商统计信息
   - 新增 `/memory/sessions` - 列出记忆会话
   - 保持 `/health`, `/chat`, `/tools` 端点正常工作

5. **记忆系统与 agent 的联动** ✓
   - `run_conversation()` 结束时自动保存会话到 SQLite
   - MemoryManager 支持：
     - save_session / load_session
     - search_semantic_memory
     - list_sessions
     - cleanup

### 编译状态
- ✓ `cargo check` 通过（仅警告，无错误）
- ✓ `cargo build --release` 成功

## 功能联动性说明

### 数据流
```
用户输入 (CLI/HTTP)
  ↓
AioAgent.run_conversation()
  ↓
构建 LLM 消息 + 工具定义
  ↓
调用 LLM API (通过当前配置的 provider)
  ↓
解析响应
  ├─ 如果有 tool_calls → 执行工具 → 将结果添加到消息 → 继续循环
  └─ 如果没有 tool_calls → 返回最终响应
  ↓
保存会话到记忆系统 (SQLite)
  ↓
返回结果给用户
```

### 配置联动
- 配置文件 (TOML/JSON/YAML) → Config → AioAgent
- providers.active → 选择哪个 LlmProvider
- providers.providers[] → 可用的提供商列表
- tools.enabled → 启用的工具（在注册时参考）

### 运行时切换
- CLI: `aio-agent model switch openai` → 更新配置 → 下次启动生效
- HTTP: `POST /providers/switch {"name": "openai"}` → 立即生效
- Agent: `agent.switch_provider("openai")` → 立即生效

## 后续优化建议

1. **流式输出集成** - streaming 模块已与 provider 解耦，可考虑集成到 agent 对话循环
2. **工具选择优化** - 当前将所有工具 schema 发送给 LLM，可考虑动态选择相关工具
3. **记忆检索增强** - 在对话开始时自动加载历史会话
4. **错误恢复** - LLM API 失败时的降级策略
5. **性能优化** - 工具执行并发化、上下文缓存等
