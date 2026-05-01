use anyhow::Result;
use std::fs;
use std::path::PathBuf;
use std::io::{self, Write};
use crate::config::Config;
use crate::adapters::types::{ChannelAccount, ChannelType};

#[derive(Debug, Clone)]
pub enum SetupMode {
    QuickStart,
    Manual,
    Import,
}

#[derive(Debug, Clone)]
pub enum ConfigAction {
    Keep,
    Modify,
    Reset,
}

#[derive(Debug, Clone)]
pub enum ResetScope {
    ConfigOnly,
    ConfigAndCreds,
    Full,
}

pub struct SetupWizard {
    config_path: PathBuf,
    mode: SetupMode,
    existing_config: Option<Config>,
}

impl SetupWizard {
    pub fn new(config_path: &str) -> Self {
        Self {
            config_path: PathBuf::from(config_path),
            mode: SetupMode::QuickStart,
            existing_config: None,
        }
    }

    pub fn with_mode(mut self, mode: SetupMode) -> Self {
        self.mode = mode;
        self
    }

    pub fn run(&mut self) -> Result<Config> {
        println!("============================================================");
        println!("  AIO Agent 配置引导向导 v1.0.0");
        println!("============================================================\n");

        self.load_existing_config()?;
        self.select_setup_mode()?;
        self.handle_existing_config()?;

        let config = match self.mode {
            SetupMode::QuickStart => self.quickstart_flow()?,
            SetupMode::Manual => self.manual_flow()?,
            SetupMode::Import => self.import_flow()?,
        };

        self.save_config(&config)?;
        self.print_completion_message(&config)?;

        Ok(config)
    }

    fn load_existing_config(&mut self) -> Result<()> {
        if self.config_path.exists() {
            match Config::from_file(self.config_path.to_str().unwrap()) {
                Ok(config) => {
                    self.existing_config = Some(config);
                    println!("检测到现有配置文件: {}", self.config_path.display());
                }
                Err(e) => {
                    println!("现有配置文件加载失败: {}", e);
                }
            }
        }
        Ok(())
    }

    fn select_setup_mode(&mut self) -> Result<()> {
        println!("选择配置模式:");
        println!("  [1] QuickStart - 快速配置，使用默认值");
        println!("  [2] Manual     - 手动配置所有选项");
        println!("  [3] Import     - 从环境变量/现有配置导入\n");

        let choice = prompt_input("选择 (1-3)", "1");
        self.mode = match choice.as_str() {
            "1" => SetupMode::QuickStart,
            "2" => SetupMode::Manual,
            "3" => SetupMode::Import,
            _ => SetupMode::QuickStart,
        };
        Ok(())
    }

    fn handle_existing_config(&mut self) -> Result<()> {
        if let Some(existing) = &self.existing_config {
            println!("\n现有配置摘要:");
            println!("  模型: {}", existing.agent.model);
            println!("  最大迭代: {}", existing.agent.max_iterations);
            println!("  API地址: {}", existing.llm.base_url);

            println!("\n如何处理现有配置:");
            println!("  [1] 保留现有值");
            println!("  [2] 更新现有值");
            println!("  [3] 重置配置\n");

            let action = prompt_input("选择 (1-3)", "2");
            match action.as_str() {
                "1" => {
                    println!("使用现有配置");
                    return Ok(());
                }
                "3" => {
                    println!("\n重置范围:");
                    println!("  [1] 仅配置");
                    println!("  [2] 配置 + 凭据");
                    println!("  [3] 完全重置 (配置 + 凭据 + 会话 + 工作区)\n");

                    let scope = prompt_input("选择 (1-3)", "1");
                    self.perform_reset(&match scope.as_str() {
                        "1" => ResetScope::ConfigOnly,
                        "2" => ResetScope::ConfigAndCreds,
                        _ => ResetScope::Full,
                    })?;
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn perform_reset(&mut self, scope: &ResetScope) -> Result<()> {
        match scope {
            ResetScope::ConfigOnly => {
                if self.config_path.exists() {
                    fs::remove_file(&self.config_path)?;
                    println!("配置文件已删除");
                }
            }
            ResetScope::ConfigAndCreds => {
                if self.config_path.exists() {
                    fs::remove_file(&self.config_path)?;
                }
                let home = dirs_next::home_dir().unwrap_or_else(|| PathBuf::from("."));
                let aio_dir = home.join(".aio-agent");
                if aio_dir.exists() {
                    fs::remove_dir_all(&aio_dir)?;
                }
                println!("配置和凭据已删除");
            }
            ResetScope::Full => {
                if self.config_path.exists() {
                    fs::remove_file(&self.config_path)?;
                }
                let home = dirs_next::home_dir().unwrap_or_else(|| PathBuf::from("."));
                let aio_dir = home.join(".aio-agent");
                if aio_dir.exists() {
                    fs::remove_dir_all(&aio_dir)?;
                }
                let workspace = home.join("aio-agent-workspace");
                if workspace.exists() {
                    fs::remove_dir_all(&workspace)?;
                }
                println!("完全重置完成");
            }
        }
        self.existing_config = None;
        Ok(())
    }

    fn quickstart_flow(&self) -> Result<Config> {
        println!("\n============================================================");
        println!("QuickStart 配置 - 使用推荐的默认值");
        println!("============================================================\n");

        let mut config = Config::default();

        let api_key_env = std::env::var("AIO_AGENT_API_KEY").ok();
        let api_url_env = std::env::var("AIO_AGENT_API_URL").ok();
        let api_model_env = std::env::var("AIO_AGENT_MODEL").ok();

        config.llm.api_key = api_key_env.unwrap_or_else(|| {
            prompt_input("API密钥", "sk-")
        });

        config.llm.base_url = api_url_env.unwrap_or_else(|| {
            prompt_input("API基础地址", "https://api.openai.com/v1")
        });

        config.agent.model = api_model_env.unwrap_or_else(|| {
            prompt_input("默认模型", "gpt-4")
        });

        config.agent.max_iterations = 10;
        config.agent.timeout_seconds = 300;

        let port = prompt_input("网关端口", "3000");
        let gateway_port: u16 = port.parse().unwrap_or(3000);

        println!("\n网关认证:");
        println!("  [1] Token (推荐)");
        println!("  [2] Password\n");
        let auth_choice = prompt_input("选择 (1-2)", "1");

        let token = if auth_choice == "1" {
            let token = generate_random_token();
            println!("\n已生成网关Token: {}", mask_secret(&token));
            Some(token)
        } else {
            let password = prompt_input("设置网关密码", "password");
            Some(password)
        };

        config.channels.insert("default".to_string(), crate::config::ChannelConfig {
            enabled: true,
            host: "127.0.0.1".to_string(),
            port: gateway_port,
            auth_token: token,
        });

        config.memory.path = "~/.aio-agent/memory.db".to_string();
        config.memory.max_sessions = 100;
        config.tools.enabled = vec![
            "web_search".to_string(),
            "file_read".to_string(),
            "file_write".to_string(),
            "terminal".to_string(),
            "web_fetch".to_string(),
        ];

        Ok(config)
    }

    fn manual_flow(&self) -> Result<Config> {
        let mut config = self.existing_config.clone().unwrap_or_else(Config::default);

        println!("\n============================================================");
        println!("Manual 配置 - 手动配置所有选项");
        println!("============================================================");

        println!("\n[1/7] LLM 提供商配置");
        println!("-------------------");
        config.llm.api_key = prompt_input("API密钥", &config.llm.api_key);
        config.llm.base_url = prompt_input("API基础地址", &config.llm.base_url);
        config.agent.model = prompt_input("默认模型", &config.agent.model);

        println!("\n[2/7] Agent 配置");
        println!("----------------");
        let max_iter = prompt_input("最大迭代次数", &config.agent.max_iterations.to_string());
        config.agent.max_iterations = max_iter.parse().unwrap_or(10);

        let timeout = prompt_input("超时时间（秒）", &config.agent.timeout_seconds.to_string());
        config.agent.timeout_seconds = timeout.parse().unwrap_or(300);

        println!("\n[3/7] 网关配置");
        println!("--------------");
        let host = prompt_input("网关绑定地址", &config.channels.get("default")
            .map(|c| c.host.clone()).unwrap_or_else(|| "127.0.0.1".to_string()));

        let port_str = prompt_input("网关端口", &config.channels.get("default")
            .map(|c| c.port.to_string()).unwrap_or_else(|| "3000".to_string()));
        let port: u16 = port_str.parse().unwrap_or(3000);

        println!("\n网关绑定模式:");
        println!("  [1] Loopback (127.0.0.1) - 仅本地访问");
        println!("  [2] LAN (0.0.0.0)        - 局域网访问");
        println!("  [3] Custom IP            - 自定义IP\n");
        let bind_choice = prompt_input("选择 (1-3)", "1");
        let bind_host = match bind_choice.as_str() {
            "2" => "0.0.0.0".to_string(),
            "3" => prompt_input("自定义IP", "192.168.1.100"),
            _ => "127.0.0.1".to_string(),
        };

        println!("\n网关认证模式:");
        println!("  [1] Token (推荐)");
        println!("  [2] Password");
        println!("  [3] None (仅本地开发)\n");
        let auth_choice = prompt_input("选择 (1-3)", "1");
        let auth_token = match auth_choice.as_str() {
            "1" => {
                let existing = config.channels.get("default").and_then(|c| c.auth_token.clone());
                match existing {
                    Some(token) => {
                        println!("使用现有Token: {}", mask_secret(&token));
                        Some(token)
                    }
                    None => {
                        let generate = prompt_input("生成新Token? (Y/n)", "Y");
                        if generate.to_lowercase().starts_with("n") {
                            Some(prompt_input("输入Token", "your-token"))
                        } else {
                            let token = generate_random_token();
                            println!("已生成: {}", mask_secret(&token));
                            Some(token)
                        }
                    }
                }
            }
            "2" => Some(prompt_input("设置密码", "password")),
            _ => None,
        };

        config.channels.insert("default".to_string(), crate::config::ChannelConfig {
            enabled: true,
            host: if bind_host != "127.0.0.1" { bind_host } else { host },
            port,
            auth_token,
        });

        println!("\n[4/7] 通道配置");
        println!("--------------");
        let add_channel = prompt_input("添加聊天通道? (y/N)", "N");
        if add_channel.to_lowercase().starts_with("y") {
            println!("\n可用通道类型:");
            println!("  [1] Telegram");
            println!("  [2] Discord");
            println!("  [3] Slack");
            println!("  [4] Webhook\n");
            let channel_type = prompt_input("选择 (1-4)", "1");
            let token = prompt_input("通道Token/Bot Token", "");
            let channel_id = prompt_input("通道ID", "default");

            let ctype = match channel_type.as_str() {
                "1" => ChannelType::Telegram,
                "2" => ChannelType::Discord,
                "3" => ChannelType::Slack,
                _ => ChannelType::Webhook,
            };

            let channel_account = ChannelAccount::new(&channel_id, ctype)
                .with_token(token)
                .enabled();

            println!("通道 {} 已添加 ({})", channel_account.id, channel_account.channel_type);
        }

        println!("\n[5/7] 记忆存储配置");
        println!("------------------");
        config.memory.path = prompt_input("记忆数据库路径", &config.memory.path);
        let max_sessions = prompt_input("最大会话数 (0=无限制)", "100");
        config.memory.max_sessions = max_sessions.parse().unwrap_or(100);

        println!("\n[6/7] 工具配置");
        println!("--------------");
        println!("可用工具:");
        let available_tools = [
            "web_search", "file_read", "file_write", "terminal", "web_fetch",
            "search_files", "list_dir", "copy_file", "move_file", "delete_file",
        ];
        for (i, tool) in available_tools.iter().enumerate() {
            let enabled = config.tools.enabled.contains(&tool.to_string());
            println!("  [{}] {} {}", i + 1, tool, if enabled { "✓" } else { " " });
        }
        println!("\n启用的工具 (逗号分隔，留空=全部启用)");
        let tools_input = prompt_input("工具列表", "all");
        if tools_input.to_lowercase() == "all" {
            config.tools.enabled = available_tools.iter().map(|s| s.to_string()).collect();
        } else {
            config.tools.enabled = tools_input
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
        }

        println!("\n[7/7] 权限配置");
        println!("--------------");
        println!("允许模式 (正则表达式，每行一个):");
        let allow_input = prompt_input("允许模式", "read_file(.*),write_to_file(.*),web_search(.*)");
        config.permissions.allow = allow_input.split(',').map(|s| s.trim().to_string()).collect();

        let deny_input = prompt_input("拒绝模式", "execute_code(rm -rf /)");
        config.permissions.deny = deny_input.split(',').map(|s| s.trim().to_string()).collect();

        Ok(config)
    }

    fn import_flow(&self) -> Result<Config> {
        println!("\n============================================================");
        println!("Import 配置 - 从环境变量和现有配置导入");
        println!("============================================================\n");

        let mut config = Config::default();

        if let Ok(api_key) = std::env::var("AIO_AGENT_API_KEY") {
            println!("从环境变量导入 API Key");
            config.llm.api_key = api_key;
        }
        if let Ok(api_url) = std::env::var("AIO_AGENT_API_URL") {
            println!("从环境变量导入 API URL: {}", api_url);
            config.llm.base_url = api_url;
        }
        if let Ok(model) = std::env::var("AIO_AGENT_MODEL") {
            println!("从环境变量导入 Model: {}", model);
            config.agent.model = model;
        }
        if let Ok(max_iter) = std::env::var("AIO_AGENT_MAX_ITERATIONS") {
            if let Ok(val) = max_iter.parse() {
                config.agent.max_iterations = val;
            }
        }
        if let Ok(gateway_port) = std::env::var("AIO_AGENT_GATEWAY_PORT") {
            if let Ok(val) = gateway_port.parse() {
                config.channels.insert("default".to_string(), crate::config::ChannelConfig {
                    port: val,
                    ..Default::default()
                });
            }
        }

        println!("\n导入完成，共导入 {} 项配置", {
            let mut count = 0;
            if !config.llm.api_key.is_empty() { count += 1; }
            if !config.llm.base_url.is_empty() { count += 1; }
            if !config.agent.model.is_empty() { count += 1; }
            if config.agent.max_iterations > 0 { count += 1; }
            count
        });

        if config.llm.api_key.is_empty() {
            config.llm.api_key = prompt_input("API密钥 (未从环境变量找到)", "sk-your-api-key");
        }

        Ok(config)
    }

    fn save_config(&self, config: &Config) -> Result<()> {
        if self.config_path.exists() {
            let overwrite = prompt_input("配置文件已存在，是否覆盖？(y/N)", "N");
            if !overwrite.to_lowercase().starts_with('y') {
                println!("\n跳过配置保存");
                return Ok(());
            }
        }

        config.save_to_file(self.config_path.to_str().unwrap())?;
        println!("\n配置已保存到: {}", self.config_path.display());
        Ok(())
    }

    fn print_completion_message(&self, config: &Config) -> Result<()> {
        println!("\n============================================================");
        println!("配置完成！配置摘要:");
        println!("============================================================");
        println!("  模型: {}", config.agent.model);
        println!("  最大迭代: {}", config.agent.max_iterations);
        println!("  API地址: {}", config.llm.base_url);
        println!("  网关端口: {}", config.channels.get("default").map(|c| c.port).unwrap_or(3000));
        println!("  记忆路径: {}", config.memory.path);
        println!("  启用工具: {} 个", config.tools.enabled.len());
        println!("\n下一步:");
        println!("  运行 'aio-agent'           - 启动交互模式");
        println!("  运行 'aio-agent serve'     - 启动API服务器");
        println!("  运行 'aio-agent query <消息>' - 单次查询");
        println!("  运行 'aio-agent test'      - 运行测试");
        println!("============================================================");

        Ok(())
    }
}

fn prompt_input(prompt: &str, default: &str) -> String {
    print!("{} [{}]: ", prompt, default);
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();

    let input = input.trim();
    if input.is_empty() {
        default.to_string()
    } else {
        input.to_string()
    }
}

fn generate_random_token() -> String {
    use std::time::SystemTime;
    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let hash = format!("{:x}", timestamp);
    format!("aio-token-{}-{}", &hash[..8], &hash[8..16])
}

fn mask_secret(secret: &str) -> String {
    if secret.len() <= 8 {
        "********".to_string()
    } else {
        let visible = &secret[..4];
        format!("{}...{}", visible, &secret[secret.len() - 4..])
    }
}
