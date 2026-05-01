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
                println!("Skills管理需要使用Skills模块");
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
        println!("  /tools                    - 查看可用工具");
        println!("  /config                   - 查看当前配置");
        println!("  /memory                   - 查看记忆会话");
        println!("  /provider [名称]          - 查看/切换提供商");
        println!("  /skills                   - 查看Skills");
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
