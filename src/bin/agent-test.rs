mod agent_modules {
    pub use aio_agent::tools::{ToolRegistry, WebSearchTool, FileReadTool, FileWriteTool, TerminalTool, WebFetchTool, ToolResult};
    pub use aio_agent::memory::MemoryManager;
    pub use aio_agent::permissions::PermissionChecker;
    pub use aio_agent::tasks::{TaskLoop, Task, TaskStatus};
    pub use aio_agent::agents::{Crew, Agent, Process};
    pub use aio_agent::workflows::Sop;
    pub use aio_agent::gateway::{GatewayBuilder, ChannelAccount, ChannelType, GatewayMessage};
    pub use aio_agent::skills::{SkillManager, SkillMetadata, SkillEntry, HermesMetadata};
    pub use aio_agent::lanes::Lane;
    pub use aio_agent::messaging::{Message, Role, ToolCall};
}

use agent_modules::*;
use std::collections::HashMap;
use std::sync::Arc;

#[tokio::main]
async fn main() {
    println!("============================================================");
    println!("AIO Agent 核心功能测试套件");
    println!("============================================================");
    
    let mut passed = 0;
    let mut failed = 0;
    
    match test_tool_system().await {
        Ok(_) => { passed += 1; println!("[通过] 第1轮: 工具系统测试"); }
        Err(e) => { failed += 1; println!("[失败] 第1轮: 工具系统测试 - {}", e); }
    }
    
    match test_memory_system() {
        Ok(_) => { passed += 1; println!("[通过] 第2轮: 记忆系统测试"); }
        Err(e) => { failed += 1; println!("[失败] 第2轮: 记忆系统测试 - {}", e); }
    }
    
    match test_permission_system() {
        Ok(_) => { passed += 1; println!("[通过] 第3轮: 权限系统测试"); }
        Err(e) => { failed += 1; println!("[失败] 第3轮: 权限系统测试 - {}", e); }
    }
    
    match test_task_system().await {
        Ok(_) => { passed += 1; println!("[通过] 第4轮: 任务系统测试"); }
        Err(e) => { failed += 1; println!("[失败] 第4轮: 任务系统测试 - {}", e); }
    }
    
    match test_crew_system().await {
        Ok(_) => { passed += 1; println!("[通过] 第5轮: 多Agent协作测试"); }
        Err(e) => { failed += 1; println!("[失败] 第5轮: 多Agent协作测试 - {}", e); }
    }
    
    match test_sop_system().await {
        Ok(_) => { passed += 1; println!("[通过] 第6轮: SOP流程测试"); }
        Err(e) => { failed += 1; println!("[失败] 第6轮: SOP流程测试 - {}", e); }
    }
    
    match test_gateway_system() {
        Ok(_) => { passed += 1; println!("[通过] 第7轮: 网关系统测试"); }
        Err(e) => { failed += 1; println!("[失败] 第7轮: 网关系统测试 - {}", e); }
    }
    
    match test_skills_system() {
        Ok(_) => { passed += 1; println!("[通过] 第8轮: Skills系统测试"); }
        Err(e) => { failed += 1; println!("[失败] 第8轮: Skills系统测试 - {}", e); }
    }
    
    match test_lane_system() {
        Ok(_) => { passed += 1; println!("[通过] 第9轮: 通道隔离测试"); }
        Err(e) => { failed += 1; println!("[失败] 第9轮: 通道隔离测试 - {}", e); }
    }
    
    match test_message_system() {
        Ok(_) => { passed += 1; println!("[通过] 第10轮: 消息系统测试"); }
        Err(e) => { failed += 1; println!("[失败] 第10轮: 消息系统测试 - {}", e); }
    }
    
    match test_web_fetch().await {
        Ok(_) => { passed += 1; println!("[通过] 第11轮: 网页爬取测试"); }
        Err(e) => { failed += 1; println!("[失败] 第11轮: 网页爬取测试 - {}", e); }
    }
    
    println!("\n============================================================");
    println!("测试完成: 通过 {} 个, 失败 {} 个", passed, failed);
    println!("============================================================");
}

async fn test_tool_system() -> anyhow::Result<()> {
    println!("\n=== 第1轮: 工具系统测试 ===");
    
    let mut registry = ToolRegistry::new();
    
    registry.register(Arc::new(WebSearchTool));
    registry.register(Arc::new(FileReadTool));
    registry.register(Arc::new(FileWriteTool));
    registry.register(Arc::new(TerminalTool));
    
    println!("工具注册: {} 个工具", registry.list_tools().len());
    assert_eq!(registry.list_tools().len(), 4, "工具注册数量应为4");
    
    println!("工具列表: {:?}", registry.list_tools());
    
    let schema = registry.get_schema("web_search");
    assert!(schema.is_some(), "web_search schema应存在");
    println!("web_search schema: {:?}", schema);
    
    let result = registry.execute("web_search", serde_json::json!({"query": "Rust Agent"})).await?;
    assert!(result.success, "web_search应成功执行");
    println!("web_search结果: {:?}", result);
    
    let result = registry.execute("terminal", serde_json::json!({"command": "echo hello"})).await?;
    assert!(result.success, "terminal应成功执行");
    println!("terminal结果: {:?}", result);
    
    let result = registry.execute("nonexistent", serde_json::json!({})).await?;
    assert!(!result.success, "不存在的工具应返回失败");
    println!("不存在工具: {:?}", result);
    
    Ok(())
}

fn test_memory_system() -> anyhow::Result<()> {
    println!("\n=== 第2轮: 记忆系统测试 ===");
    
    let db_path = "./test_memory.db";
    let mut memory = MemoryManager::new(db_path)?;
    println!("记忆数据库创建成功: {}", db_path);
    
    let messages = vec![
        serde_json::json!({"role": "user", "content": "你好"}),
        serde_json::json!({"role": "assistant", "content": "你好！有什么可以帮助你的？"}),
    ];
    
    memory.save_session("test-session-1", &messages)?;
    println!("会话保存成功");
    
    let session = memory.load_session("test-session-1")?;
    assert!(session.is_some(), "会话应存在");
    let session = session.unwrap();
    assert_eq!(session.messages.len(), 2, "会话消息数应为2");
    println!("会话加载成功: {} 条消息", session.messages.len());
    
    let mut metadata = HashMap::new();
    metadata.insert("source".to_string(), "user_input".to_string());
    metadata.insert("type".to_string(), "fact".to_string());
    
    let id = memory.add_semantic_memory("Rust是一门安全的系统编程语言", &metadata)?;
    println!("语义记忆添加成功: {}", id);
    
    let results = memory.search_semantic_memory("Rust")?;
    assert!(!results.is_empty(), "应找到语义记忆");
    println!("语义记忆搜索结果: {} 条", results.len());
    println!("第一条: {}", results[0].content);
    
    let sessions = memory.list_sessions()?;
    assert!(!sessions.is_empty(), "会话列表应不为空");
    println!("会话列表: {:?}", sessions);
    
    std::fs::remove_file(db_path).ok();
    
    Ok(())
}

fn test_permission_system() -> anyhow::Result<()> {
    println!("\n=== 第3轮: 权限系统测试 ===");
    
    let checker = PermissionChecker::new(
        vec![
            "read_file".to_string(),
            "write_to_file".to_string(),
            "search_web".to_string(),
        ],
        vec![
            "rm -rf /".to_string(),
            "sudo rm -rf".to_string(),
        ],
    );
    println!("权限检查器创建成功");
    
    assert!(checker.check("read_file", "/etc/passwd"), "读取文件应被允许");
    println!("✓ read_file 允许");
    
    assert!(checker.check("write_to_file", "~/.aio-agent/config.yaml"), "写入应被允许");
    println!("✓ write_to_file 允许");
    
    assert!(checker.check("search_web", "anything"), "搜索应被允许");
    println!("✓ search_web 允许");
    
    assert!(!checker.check("rm -rf /", "dangerous"), "危险命令应被拒绝");
    println!("✓ rm -rf / 拒绝");
    
    assert!(!checker.check("sudo rm -rf", "dangerous"), "sudo rm应被拒绝");
    println!("✓ sudo rm -rf 拒绝");
    
    assert!(!checker.check("admin_action", "anything"), "未定义操作应被拒绝");
    println!("✓ 未定义操作 拒绝");
    
    Ok(())
}

async fn test_task_system() -> anyhow::Result<()> {
    println!("\n=== 第4轮: 任务系统测试 ===");
    
    let mut task_loop = TaskLoop::new("构建RAG问答系统", 5);
    println!("任务循环创建: 目标={}", task_loop.goal);
    
    let subtasks = task_loop.decompose();
    println!("任务分解: {} 个子任务", subtasks.len());
    assert!(!subtasks.is_empty(), "任务分解应产生子任务");
    
    for task in &subtasks {
        println!("  - {} [{}]", task.description, task.id);
    }
    
    let completed = task_loop.run().await?;
    println!("任务执行完成: {} 个任务", completed.len());
    assert!(!completed.is_empty(), "应有完成的任务");
    
    for task in &completed {
        assert_eq!(task.status, TaskStatus::Completed, "任务状态应为已完成");
        println!("  [{}] {} - {:?}", task.id, task.description, task.status);
    }
    
    Ok(())
}

async fn test_crew_system() -> anyhow::Result<()> {
    println!("\n=== 第5轮: 多Agent协作测试 ===");
    
    let agents = vec![
        Agent::new("agent-1", "研究员", "搜索和分析信息", "专业研究员，擅长信息检索"),
        Agent::new("agent-2", "分析师", "分析数据和生成报告", "数据分析专家"),
        Agent::new("agent-3", "写手", "撰写最终文档", "专业写作人员"),
    ];
    println!("创建 {} 个Agent", agents.len());
    
    let tasks = vec![
        Task::new("task-1", "搜索最新AI Agent框架"),
        Task::new("task-2", "分析框架优缺点"),
        Task::new("task-3", "撰写对比报告"),
    ];
    println!("创建 {} 个任务", tasks.len());
    
    let crew_seq = Crew::new(agents.clone(), tasks.clone(), Process::Sequential);
    let results_seq = crew_seq.kickoff().await?;
    println!("顺序执行结果: {} 个任务", results_seq.len());
    for (task_id, result) in &results_seq {
        println!("  {}: {}", task_id, result);
    }
    
    let crew_hier = Crew::new(agents, tasks, Process::Hierarchical);
    let results_hier = crew_hier.kickoff().await?;
    println!("层级执行结果: {} 个任务", results_hier.len());
    for (task_id, result) in &results_hier {
        println!("  {}: {}", task_id, result);
    }
    
    Ok(())
}

async fn test_sop_system() -> anyhow::Result<()> {
    println!("\n=== 第6轮: SOP流程测试 ===");
    
    let mut sop = Sop::new();
    sop.add_step("需求分析", "analyze_requirements", "分析师", None);
    sop.add_step("方案设计", "design_solution", "架构师", None);
    sop.add_step("编码实现", "implement_code", "开发工程师", None);
    sop.add_step("测试验证", "test_and_verify", "测试工程师", None);
    sop.add_step("部署上线", "deploy_to_production", "运维工程师", None);
    println!("SOP创建: 5 个步骤");
    
    let mut context = HashMap::new();
    context.insert("project".to_string(), "AIO Agent".to_string());
    context.insert("version".to_string(), "1.0.0".to_string());
    
    let results = sop.execute(&context).await?;
    println!("SOP执行完成: {} 个步骤", results.len());
    assert_eq!(results.len(), 5, "SOP应有5个步骤");
    
    for (step_name, result) in &results {
        assert!(result.success, "SOP步骤应成功执行");
        println!("  {}: 成功", step_name);
    }
    
    Ok(())
}

fn test_gateway_system() -> anyhow::Result<()> {
    println!("\n=== 第7轮: 网关系统测试 ===");
    
    let gateway = GatewayBuilder::new()
        .host("127.0.0.1")
        .port(3000)
        .auth_token("secret-token")
        .max_connections(500)
        .add_channel(
            ChannelAccount::new("telegram-1", ChannelType::Telegram)
                .with_token("bot-token-123".to_string())
                .enabled()
        )
        .add_channel(
            ChannelAccount::new("discord-1", ChannelType::Discord)
                .with_token("discord-token-456".to_string())
                .enabled()
        )
        .add_channel(
            ChannelAccount::new("slack-1", ChannelType::Slack)
                .with_webhook("https://hooks.slack.com/test".to_string())
                .enabled()
        )
        .build();
    
    println!("网关创建成功: {}:{}", gateway.config.host, gateway.config.port);
    assert_eq!(gateway.config.port, 3000, "端口应为3000");
    assert_eq!(gateway.config.auth_token, Some("secret-token".to_string()), "auth_token应匹配");
    assert_eq!(gateway.config.max_connections, 500, "最大连接数应为500");
    println!("通道数量: {}", gateway.config.channels.len());
    assert_eq!(gateway.config.channels.len(), 3, "应有3个通道");
    
    for channel in &gateway.config.channels {
        println!("  - {} ({}) [enabled={}, configured={}]", 
            channel.id, 
            channel.channel_type, 
            channel.enabled,
            channel.configured
        );
        assert!(channel.enabled, "通道应启用");
    }
    
    let msg = GatewayMessage::new(
        ChannelType::Telegram,
        "telegram-1".to_string(),
        "user-123".to_string(),
        "Hello from Telegram!".to_string(),
    );
    println!("网关消息创建成功: id={}, sender={}, content={}", msg.id, msg.sender_id, msg.content);
    assert_eq!(msg.channel_type, ChannelType::Telegram, "消息通道类型应匹配");
    assert_eq!(msg.content, "Hello from Telegram!", "消息内容应匹配");
    
    Ok(())
}

fn test_skills_system() -> anyhow::Result<()> {
    println!("\n=== 第8轮: Skills系统测试 ===");
    
    let unique_id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    
    let skills_dir = dirs_next::home_dir()
        .unwrap_or_default()
        .join(".aio-agent")
        .join("skills");
    if skills_dir.exists() {
        let _ = std::fs::remove_dir_all(&skills_dir);
    }
    std::fs::create_dir_all(&skills_dir)?;
    
    let mut skill_manager = SkillManager::new()?;
    println!("Skills管理器创建成功");
    
    let all_skills = skill_manager.list_skills();
    println!("已加载技能: {} 个", all_skills.len());
    
    let search_results = skill_manager.search_skills("github");
    println!("搜索'github'结果: {} 个", search_results.len());
    
    let skill_name = format!("test-skill-{}", unique_id);
    let skill_path = skill_manager.create_skill(
        &skill_name,
        "A unique test skill",
        "# Test Skill\n\nThis is a unique test.",
        "testing",
        Some(vec!["test".to_string(), "unique".to_string()]),
    )?;
    println!("创建技能成功: {}", skill_path.display());
    assert!(skill_path.exists(), "技能文件应存在");
    
    skill_manager.load_skills()?;
    
    let skills = skill_manager.list_skills();
    println!("当前技能总数: {}", skills.len());
    assert!(!skills.is_empty(), "至少应有1个技能");
    
    let status = skill_manager.get_status();
    println!("技能状态:");
    println!("  总数: {}", status.total_skills);
    println!("  类别: {:?}", status.categories);
    
    Ok(())
}

fn test_lane_system() -> anyhow::Result<()> {
    println!("\n=== 第9轮: 通道隔离测试 ===");
    
    let main_lane = Lane::Main;
    let nested_lane = Lane::resolve_nested(Some("nested-1"));
    let cron_lane = Lane::resolve_cron(Some("cron-1"));
    let subagent_lane = Lane::Subagent;
    
    println!("主通道: {:?}", main_lane);
    println!("嵌套通道: {:?}", nested_lane);
    println!("定时通道: {:?}", cron_lane);
    println!("子Agent通道: {:?}", subagent_lane);
    
    assert_eq!(main_lane, Lane::Main, "主通道应匹配");
    assert_eq!(nested_lane, Lane::Nested, "嵌套通道应为Nested");
    assert_eq!(cron_lane, Lane::Nested, "定时嵌套通道应为Nested");
    assert!(!subagent_lane.is_nested(), "子Agent通道不是嵌套通道");
    
    let resolved = Lane::resolve_nested(Some("custom-lane"));
    println!("自定义通道解析: {:?}", resolved);
    assert!(resolved.is_nested(), "自定义通道应解析为嵌套");
    
    Ok(())
}

fn test_message_system() -> anyhow::Result<()> {
    println!("\n=== 第10轮: 消息系统测试 ===");
    
    let user_msg = Message::user("你好，帮我搜索一下信息".to_string());
    assert_eq!(user_msg.role, Role::User, "角色应为用户");
    println!("用户消息: role={:?}, content={}", user_msg.role, user_msg.content);
    
    let assistant_msg = Message::assistant("好的，我来帮你搜索".to_string());
    assert_eq!(assistant_msg.role, Role::Assistant, "角色应为助手");
    println!("助手消息: role={:?}, content={}", assistant_msg.role, assistant_msg.content);
    
    let system_msg = Message::system("你是一个AI助手".to_string());
    assert_eq!(system_msg.role, Role::System, "角色应为系统");
    println!("系统消息: role={:?}, content={}", system_msg.role, system_msg.content);
    
    let tool_calls = vec![
        ToolCall {
            id: "call-1".to_string(),
            name: "web_search".to_string(),
            arguments: serde_json::json!({"query": "Rust Agent"}),
        },
    ];
    
    let tool_msg = Message::with_tool_calls(
        "让我搜索一下相关信息".to_string(),
        tool_calls,
    );
    assert_eq!(tool_msg.role, Role::Assistant, "工具调用消息角色应为助手");
    assert!(tool_msg.tool_calls.is_some(), "工具调用应存在");
    assert_eq!(tool_msg.tool_calls.as_ref().unwrap().len(), 1, "应有1个工具调用");
    println!("工具调用消息: {} 个工具调用", tool_msg.tool_calls.as_ref().unwrap().len());
    
    let result_msg = Message::with_tool_result(
        "搜索完成，找到3条结果".to_string(),
        serde_json::json!({"results": ["result1", "result2", "result3"]}),
    );
    assert_eq!(result_msg.role, Role::Tool, "工具结果消息角色应为Tool");
    assert!(result_msg.tool_result.is_some(), "工具结果应存在");
    println!("工具结果消息: 结果={}", result_msg.content);
    
    let serialized = serde_json::to_string(&user_msg)?;
    println!("消息序列化成功: {} 字符", serialized.len());
    
    let deserialized: Message = serde_json::from_str(&serialized)?;
    assert_eq!(deserialized.role, Role::User, "反序列化后角色应匹配");
    println!("消息反序列化成功");
    
    Ok(())
}

async fn test_web_fetch() -> anyhow::Result<()> {
    println!("\n=== 第11轮: 网页爬取测试 ===");
    
    let mut registry = ToolRegistry::new();
    registry.register(Arc::new(WebFetchTool));
    println!("WebFetchTool注册成功");
    
    let schema = registry.get_schema("web_fetch");
    assert!(schema.is_some(), "web_fetch schema应存在");
    println!("web_fetch schema存在");
    
    println!("\n--- 测试1: 抓取example.com（text模式） ---");
    let result = registry.execute("web_fetch", serde_json::json!({
        "url": "https://www.example.com",
        "format": "text"
    })).await?;
    if result.success {
        if let Some(data) = &result.data {
            let status = data["status"].as_u64().unwrap_or(0);
            let length = data["length"].as_u64().unwrap_or(0);
            let content = data["content"].as_str().unwrap_or("");
            let format = data["format"].as_str().unwrap_or("");
            println!("状态码: {}", status);
            println!("内容长度: {} 字符", length);
            println!("格式: {}", format);
            let preview: String = content.chars().take(200).collect();
            println!("内容预览: {}", preview);
            assert!(!content.is_empty(), "内容不应为空");
            assert_eq!(format, "text", "格式应为text");
            assert!(!content.contains('<'), "text模式不应包含HTML标签");
        }
    } else {
        println!("网页抓取失败（可能是网络问题）: {:?}", result.error);
        println!("提示: 请检查网络连接是否正常");
    }
    
    println!("\n--- 测试2: 抓取example.com（html模式） ---");
    let result = registry.execute("web_fetch", serde_json::json!({
        "url": "https://www.example.com",
        "format": "html"
    })).await?;
    if result.success {
        if let Some(data) = &result.data {
            let status = data["status"].as_u64().unwrap_or(0);
            let length = data["length"].as_u64().unwrap_or(0);
            let content = data["content"].as_str().unwrap_or("");
            let format = data["format"].as_str().unwrap_or("");
            println!("状态码: {}", status);
            println!("内容长度: {} 字符", length);
            println!("格式: {}", format);
            let preview: String = content.chars().take(200).collect();
            println!("内容预览: {}", preview);
            assert!(!content.is_empty(), "内容不应为空");
            assert_eq!(format, "html", "格式应为html");
            assert!(content.contains('<'), "html模式应包含HTML标签");
        }
    } else {
        println!("网页抓取失败（可能是网络问题）: {:?}", result.error);
    }
    
    println!("\n--- 测试3: 无效URL（错误处理） ---");
    let result = registry.execute("web_fetch", serde_json::json!({
        "url": "invalid-url"
    })).await?;
    assert!(!result.success, "无效URL应返回失败");
    println!("无效URL错误: {:?}", result.error);
    assert!(result.error.as_ref().unwrap().contains("只支持HTTP/HTTPS协议"), "应提示协议错误");
    
    println!("\n--- 测试4: 空URL（错误处理） ---");
    let result = registry.execute("web_fetch", serde_json::json!({})).await?;
    assert!(!result.success, "空URL应返回失败");
    println!("空URL错误: {:?}", result.error);
    assert!(result.error.as_ref().unwrap().contains("URL不能为空"), "应提示URL为空");
    
    println!("\n--- 测试5: 不存在的域名（错误处理） ---");
    let result = registry.execute("web_fetch", serde_json::json!({
        "url": "https://this-domain-definitely-does-not-exist-12345.com"
    })).await?;
    assert!(!result.success, "不存在的域名应返回失败");
    println!("不存在域名错误: {:?}", result.error);
    
    println!("\n--- 测试6: 工具注册集成 ---");
    registry.register(Arc::new(WebSearchTool));
    registry.register(Arc::new(TerminalTool));
    let tools = registry.list_tools();
    println!("工具列表: {:?}", tools);
    assert!(tools.contains(&"web_fetch"), "工具列表应包含web_fetch");
    
    println!("\n--- 测试7: HTML标签剥离功能 ---");
    let html = "<html><body><h1>Hello World</h1><p>This is a <b>test</b>.</p></body></html>";
    let stripped = aio_agent::tools::strip_html_tags(html);
    println!("原始HTML: {}", html);
    println!("剥离后: {}", stripped);
    assert!(!stripped.contains('<'), "不应包含HTML标签");
    assert!(stripped.contains("Hello World"), "应包含标题文本");
    assert!(stripped.contains("This is a test."), "应包含段落文本");
    
    Ok(())
}
