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
                        self.handle_question(question);
                    } else if input.starts_with("/") {
                        let parts: Vec<&str> = input.splitn(2, ' ').collect();
                        self.handle_command(parts[0], parts.get(1).unwrap_or(&""));
                    } else {
                        self.handle_question(input);
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

    fn handle_question(&mut self, question: &str) {
        println!("处理中: {}...\n", question);
    }

    fn handle_command(&self, cmd: &str, args: &str) {
        match cmd {
            "/tools" => {
                println!("可用工具:");
                println!("  - web_search: 搜索网络获取最新信息");
                println!("  - file_read: 读取文件内容");
                println!("  - file_write: 写入文件内容");
                println!("  - terminal: 执行终端命令");
                println!("  - web_fetch: 抓取网页内容");
                println!("  - search_files: 搜索文件");
                println!("  - list_dir: 列出目录");
            }
            "/config" => {
                println!("当前配置:");
                println!("  模型: {}", self.agent.config.agent.model);
                println!("  最大迭代: {}", self.agent.config.agent.max_iterations);
                println!("  超时: {}s", self.agent.config.agent.timeout_seconds);
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
        println!("  /ask <问题> 或 /q <问题> - 向Agent提问");
        println!("  /tools                   - 查看可用工具");
        println!("  /config                  - 查看当前配置");
        println!("  /memory                  - 查看记忆会话");
        println!("  /skills                  - 查看Skills");
        println!("  status                   - 显示Agent状态");
        println!("  clear                    - 清屏");
        println!("  help                     - 显示此帮助");
        println!("  exit/quit                - 退出");
        println!("\n或者直接输入问题与Agent对话\n");
    }

    fn show_status(&self) {
        println!("\nAgent状态:");
        println!("  模型: {}", self.agent.config.agent.model);
        println!("  会话ID: {}", self.agent.session_id);
        println!("  消息数: {}", self.agent.messages.len());
        println!("  工具数: {}", self.agent.tools.list_tools().len());
    }
}
