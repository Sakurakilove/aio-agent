use anyhow::Result;
use std::io::{self, Write};
use crate::agent_engine::AioAgent;
use crate::config::Config;

pub struct CliApp {
    agent: AioAgent,
    interactive: bool,
}

impl CliApp {
    pub fn new(config: Config) -> Result<Self> {
        let agent = AioAgent::new(config)?;
        Ok(Self {
            agent,
            interactive: false,
        })
    }

    pub fn interactive_mode(&mut self) -> Result<()> {
        self.interactive = true;
        println!("============================================================");
        println!("AIO Agent 交互式终端");
        println!("输入 'exit' 或 'quit' 退出");
        println!("输入 'help' 查看帮助");
        println!("============================================================");

        let mut rt = tokio::runtime::Runtime::new()?;

        loop {
            print!("\nAIO Agent> ");
            io::stdout().flush()?;

            let mut input = String::new();
            match io::stdin().read_line(&mut input) {
                Ok(_) => {}
                Err(e) => {
                    eprintln!("读取输入失败: {}", e);
                    continue;
                }
            }

            let input = input.trim();
            if input.is_empty() {
                continue;
            }

            match input.to_lowercase().as_str() {
                "exit" | "quit" => {
                    println!("再见！");
                    break;
                }
                "help" => {
                    self.show_help();
                }
                "status" => {
                    self.show_status();
                }
                "clear" => {
                    print!("\x1B[2J\x1B[1;1H");
                }
                _ => {
                    if input.starts_with("/ask ") || input.starts_with("/q ") {
                        let question = input.trim_start_matches("/ask ").trim_start_matches("/q ");
                        self.handle_question(&mut rt, question);
                    } else if input.starts_with("/") {
                        let parts: Vec<&str> = input.splitn(2, ' ').collect();
                        self.handle_command(parts[0], parts.get(1).unwrap_or(&""));
                    } else {
                        self.handle_question(&mut rt, input);
                    }
                }
            }
        }

        Ok(())
    }

    pub async fn run_single_query(&mut self, query: &str) -> Result<()> {
        let result = self.agent.run_conversation(query).await?;
        println!("\n{}", result.final_response);
        println!("\n[迭代次数: {}]", result.iterations);
        Ok(())
    }

    fn handle_question(&mut self, rt: &mut tokio::runtime::Runtime, question: &str) {
        println!("处理中: {}...\n", question);
        match rt.block_on(self.agent.run_conversation(question)) {
            Ok(result) => {
                println!("{}", result.final_response);
                println!("\n[迭代: {}] [工具调用: {}]", result.iterations, self.agent.stats.total_tool_calls);
            }
            Err(e) => {
                eprintln!("错误: {}", e);
            }
        }
    }

    /// 使用流式模式处理问题
    fn handle_question_stream(&mut self, rt: &mut tokio::runtime::Runtime, question: &str) {
        println!("处理中(流式): {}...\n\n", question);
        match rt.block_on(self.agent.stream_conversation(question)) {
            Ok(response) => {
                println!("\n\n[响应长度: {} 字符]", response.len());
            }
            Err(e) => {
                eprintln!("流式错误: {}", e);
            }
        }
    }

    fn handle_command(&mut self, cmd: &str, args: &str) {
        match cmd {
            "/tools" => {
                println!("可用工具:");
                for tool in self.agent.tools.list_tools() {
                    println!("  - {}", tool);
                }
            }
            "/config" => {
                println!("当前配置:");
                println!("  活跃提供商: {}", self.agent.config.providers.active);
                println!("  模型: {}", self.agent.llm_provider.default_model);
                println!("  API地址: {}", self.agent.llm_provider.base_url);
                println!("  API密钥: {}", self.agent.llm_provider.masked_api_key());
                println!("  最大迭代: {}", self.agent.config.agent.max_iterations);
                println!("  超时: {}s", self.agent.config.agent.timeout_seconds);
                println!("  工具数: {}", self.agent.tools.list_tools().len());
            }
            "/memory" => {
                match self.agent.memory.list_sessions() {
                    Ok(sessions) => {
                        println!("会话记忆: {} 个", sessions.len());
                        for s in &sessions {
                            println!("  - {}", s);
                        }
                    }
                    Err(e) => eprintln!("获取会话失败: {}", e),
                }
            }
            "/provider" => {
                if args.is_empty() {
                    println!("当前提供商: {}", self.agent.config.providers.active);
                    println!("可用提供商:");
                    for p in &self.agent.config.providers.providers {
                        let marker = if p.name == self.agent.config.providers.active { " ★" } else { "" };
                        println!("  - {} ({}){}", p.name, if p.enabled { "启用" } else { "禁用" }, marker);
                    }
                } else {
                    match self.agent.switch_provider(args.trim()) {
                        Ok(()) => println!("✓ 已切换到提供商: {}", args.trim()),
                        Err(e) => eprintln!("✗ 切换失败: {}", e),
                    }
                }
            }
            "/skills" => {
                if args.is_empty() {
                    let skills = self.agent.list_skills();
                    if skills.is_empty() {
                        println!("暂无已注册的Skills");
                    } else {
                        println!("已注册Skills ({} 个):", skills.len());
                        for skill in &skills {
                            println!("  - {}", skill);
                        }
                    }
                    println!("\n用法: /skills <search_query>  - 搜索Skills");
                } else {
                    let results = self.agent.search_skills(args);
                    if results.is_empty() {
                        println!("未找到匹配 '{}' 的Skills", args);
                    } else {
                        println!("搜索结果 ({} 个):", results.len());
                        for skill in &results {
                            println!("  - {}", skill);
                        }
                    }
                }
            }
            "/handoff" => {
                if args.is_empty() {
                    if self.agent.handoff_manager.is_none() {
                        self.agent.enable_handoff();
                    }
                    println!("可用Agent:");
                    for agent in self.agent.list_handoff_agents() {
                        println!("  - {} (工具: {})", agent.name, agent.tools.join(", "));
                    }
                    println!("\n用法: /handoff <agent_name> [reason]");
                } else {
                    let parts: Vec<&str> = args.splitn(2, ' ').collect();
                    let agent_name = parts[0];
                    let reason = parts.get(1).unwrap_or(&"用户请求转交");
                    if self.agent.handoff_manager.is_none() {
                        self.agent.enable_handoff();
                    }
                    let mut rt = tokio::runtime::Runtime::new().unwrap();
                    match rt.block_on(self.agent.handoff_to(agent_name, reason)) {
                        Ok(result) => {
                            if result.accepted {
                                println!("✓ 已转交给Agent '{}': {}", result.agent_name, result.response);
                            } else {
                                println!("✗ 转交失败: {}", result.response);
                            }
                        }
                        Err(e) => eprintln!("✗ 转交错误: {}", e),
                    }
                }
            }
            "/parse" => {
                if args.is_empty() {
                    println!("用法: /parse <text>  - 解析输出为结构化格式");
                } else {
                    let result = self.agent.parse_output(args);
                    println!("格式: {:?}", result.format);
                    println!("成功: {}", result.parse_success);
                    if let Some(parsed) = result.parsed {
                        println!("解析结果: {}", serde_json::to_string_pretty(&parsed).unwrap_or_default());
                    }
                    if let Some(error) = result.error {
                        println!("错误: {}", error);
                    }
                }
            }
            "/stream" => {
                if args.is_empty() {
                    println!("用法: /stream <question>  - 流式模式对话");
                } else {
                    let mut rt2 = tokio::runtime::Runtime::new().unwrap();
                    self.handle_question_stream(&mut rt2, args);
                }
            }
            "/crew" => {
                if args.is_empty() {
                    println!("用法: /crew <task_description>  - 使用Crew多Agent协作");
                    println!("将创建研究员+分析师+写作者的Crew来完成任务");
                } else {
                    let crew_agents = vec![
                        crate::agents::Agent::new("1", "研究员", "搜索和分析信息", "专业研究员"),
                        crate::agents::Agent::new("2", "分析师", "分析数据", "数据分析专家"),
                        crate::agents::Agent::new("3", "写作者", "撰写报告", "专业写作者"),
                    ];
                    let tasks = vec![
                        crate::tasks::Task::new("t1", &format!("研究: {}", args)),
                        crate::tasks::Task::new("t2", &format!("分析: {}", args)),
                        crate::tasks::Task::new("t3", &format!("撰写关于'{}'的报告", args)),
                    ];
                    let mut rt2 = tokio::runtime::Runtime::new().unwrap();
                    match rt2.block_on(self.agent.run_crew(crew_agents, tasks, crate::agents::Process::Sequential)) {
                        Ok(results) => {
                            println!("✓ Crew任务完成 ({} 个任务):", results.len());
                            for (id, result) in &results {
                                println!("\n--- 任务 {} ---\n{}", id, result.chars().take(500).collect::<String>());
                            }
                        }
                        Err(e) => eprintln!("✗ Crew执行失败: {}", e),
                    }
                }
            }
            _ => {
                println!("未知命令: {}", cmd);
                println!("输入 'help' 查看可用命令");
            }
        }
    }

    fn show_help(&self) {
        println!("\n可用命令:");
        println!("  /ask <问题> 或 /q <问题>  - 向Agent提问");
        println!("  /stream <问题>            - 流式模式对话");
        println!("  /tools                    - 查看可用工具");
        println!("  /config                   - 查看当前配置");
        println!("  /memory                   - 查看记忆会话");
        println!("  /provider [名称]          - 查看/切换提供商");
        println!("  /handoff [agent] [reason] - 查看/执行Agent转交");
        println!("  /crew <task>              - Crew多Agent协作");
        println!("  /parse <text>             - 解析输出为结构化格式");
        println!("  /skills [query]           - 查看/搜索Skills");
        println!("  status                    - 显示Agent状态");
        println!("  clear                     - 清屏");
        println!("  help                      - 显示此帮助");
        println!("  exit/quit                 - 退出");
        println!("\n或者直接输入问题与Agent对话\n");
    }

    fn show_status(&self) {
        println!("\nAgent状态:");
        println!("  活跃提供商: {}", self.agent.config.providers.active);
        println!("  模型: {}", self.agent.llm_provider.default_model);
        println!("  会话ID: {}", self.agent.session_id);
        println!("  消息数: {}", self.agent.messages.len());
        println!("  工具数: {}", self.agent.tools.list_tools().len());
        println!("  提供商数: {}", self.agent.config.providers.providers.len());
    }
}
