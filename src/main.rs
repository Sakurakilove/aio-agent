use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing_subscriber;

mod agent_engine;
mod config;
mod tools;
mod memory;
mod permissions;
mod tasks;
mod agents;
mod workflows;
mod messaging;
mod gateway;
mod skills;
mod lanes;
mod providers;
mod streaming;
mod cli;
mod api;
mod setup;
mod doctor;
mod budget;
mod logging;
mod context;
mod errors;
mod scheduler;
mod delegation;
mod adapters;
mod interrupt;
mod callbacks;
mod guardrails;
mod output_parser;
mod human_in_loop;
mod handoff;
mod checkpoint;

#[derive(Parser)]
#[command(name = "aio-agent")]
#[command(version = "1.4.0")]
#[command(about = "All In One Agent - 集成化一站式AI Agent解决方案", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    #[arg(short, long, help = "配置文件路径")]
    config: Option<String>,

    #[arg(long, help = "启用流式输出")]
    stream: bool,

    #[arg(long, help = "启用详细日志")]
    verbose: bool,

    #[arg(long, help = "API服务器主机地址", default_value = "127.0.0.1")]
    host: Option<String>,

    #[arg(long, help = "API服务器端口", default_value_t = 3000)]
    port: u16,
}

#[derive(Subcommand)]
enum Commands {
    #[command(about = "启动配置引导向导")]
    Setup {
        #[arg(short, long, help = "配置文件路径")]
        config: Option<String>,
    },

    #[command(about = "启动HTTP API服务器")]
    Serve {
        #[arg(long, help = "主机地址", default_value = "127.0.0.1")]
        host: String,

        #[arg(long, help = "端口", default_value_t = 3000)]
        port: u16,
    },

    #[command(about = "单次查询模式")]
    Query {
        #[arg(help = "查询内容")]
        message: String,
    },

    #[command(about = "委派任务给子Agent")]
    Delegate {
        #[arg(help = "任务描述")]
        task: String,

        #[arg(long, help = "最大迭代次数", default_value_t = 50)]
        max_iterations: usize,
    },

    #[command(about = "添加定时任务")]
    Schedule {
        #[arg(help = "任务名称")]
        name: String,

        #[arg(help = "任务命令")]
        command: String,

        #[arg(long, help = "调度类型: every/hourly/daily", default_value = "hourly")]
        schedule_type: String,

        #[arg(long, help = "间隔秒数(用于every类型)", default_value_t = 60)]
        seconds: u64,
    },

    #[command(about = "列出可用的聊天平台适配器")]
    Adapters,

    #[command(about = "运行完整测试套件")]
    Test,

    #[command(about = "显示Agent状态")]
    Status,

    #[command(about = "运行系统诊断")]
    Doctor,

    #[command(about = "模型管理", subcommand)]
    Model(ModelCommand),
}

#[derive(Subcommand)]
enum ModelCommand {
    #[command(about = "列出所有模型提供商")]
    List,

    #[command(about = "切换当前使用的模型提供商")]
    Switch {
        #[arg(help = "提供商名称 (openai/anthropic/ollama/custom)")]
        name: String,
    },

    #[command(about = "添加新的模型提供商")]
    Add {
        #[arg(long, help = "提供商名称")]
        name: String,

        #[arg(long, help = "API密钥")]
        api_key: Option<String>,

        #[arg(long, help = "API基础地址", default_value = "https://api.openai.com/v1")]
        base_url: String,

        #[arg(long, help = "可用模型列表，逗号分隔")]
        models: Option<String>,

        #[arg(long, help = "默认模型")]
        default_model: Option<String>,
    },

    #[command(about = "移除模型提供商")]
    Remove {
        #[arg(help = "提供商名称")]
        name: String,
    },

    #[command(about = "测试当前模型连接")]
    Test,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let _log_manager = logging::LogManager::init_simple()?;

    match cli.command {
        Some(Commands::Setup { config }) => {
            run_setup(&config.unwrap_or_else(|| "aio-agent.toml".to_string()))?;
        }

        Some(Commands::Serve { host, port }) => {
            run_api_server(&host, port, cli.config.as_deref()).await?;
        }

        Some(Commands::Query { message }) => {
            run_single_query(&message, cli.config.as_deref(), cli.stream).await?;
        }

        Some(Commands::Delegate { task, max_iterations }) => {
            run_delegate(&task, max_iterations, cli.config.as_deref()).await?;
        }

        Some(Commands::Schedule { name, command, schedule_type, seconds }) => {
            run_schedule(&name, &command, &schedule_type, seconds)?;
        }

        Some(Commands::Adapters) => {
            run_list_adapters();
        }

        Some(Commands::Test) => {
            run_tests().await?;
        }

        Some(Commands::Status) => {
            run_status(cli.config.as_deref()).await?;
        }

        Some(Commands::Doctor) => {
            run_doctor()?;
        }

        Some(Commands::Model(model_cmd)) => {
            run_model_command(model_cmd, cli.config.as_deref()).await?;
        }

        None => {
            run_interactive(cli.config.as_deref(), cli.stream).await?;
        }
    }

    Ok(())
}

fn run_setup(config_path: &str) -> Result<()> {
    let mut wizard = setup::SetupWizard::new(config_path);
    wizard.run()?;
    Ok(())
}

async fn run_api_server(host: &str, port: u16, config_path: Option<&str>) -> Result<()> {
    let config = load_config(config_path)?;
    api::start_server(config, host, port).await?;
    Ok(())
}

async fn run_single_query(message: &str, config_path: Option<&str>, stream: bool) -> Result<()> {
    let config = load_config(config_path)?;

    if stream {
        let active_provider = config.providers.providers.iter()
            .find(|p| p.name == config.providers.active && p.enabled)
            .or_else(|| config.providers.providers.iter().find(|p| p.enabled));

        let (api_key, base_url, model) = if let Some(provider) = active_provider {
            (provider.api_key.clone(), provider.base_url.clone(), provider.default_model.clone())
        } else {
            (
                std::env::var("AIO_AGENT_API_KEY").unwrap_or_default(),
                std::env::var("AIO_AGENT_API_URL").unwrap_or_else(|_| "https://api.openai.com/v1".to_string()),
                std::env::var("AIO_AGENT_MODEL").unwrap_or_else(|_| "gpt-4".to_string()),
            )
        };

        let provider = streaming::StreamingLlmProvider::new(
            &api_key,
            &base_url,
            &model,
        );

        let messages = vec![
            streaming::StreamingMessage::system("你是一个有用的AI助手"),
            streaming::StreamingMessage::user(message),
        ];

        let stream = provider.stream_chat_simple(messages).await?;
        let response = streaming::print_stream(stream).await?;
        println!("\n\n[完整响应长度: {} 字符]", response.len());
    } else {
        let mut agent = agent_engine::AioAgent::new(config)?;
        let result = agent.run_conversation(message).await?;
        println!("{}", result.final_response);
        println!("\n[迭代次数: {}] [上下文压缩: {}] [委派数: {}]",
            result.iterations, result.context_compressed, result.delegation_count);
    }

    Ok(())
}

async fn run_delegate(task: &str, max_iterations: usize, config_path: Option<&str>) -> Result<()> {
    let config = load_config(config_path)?;
    let mut agent = agent_engine::AioAgent::new(config)?;

    println!("正在委派任务: {}", task);
    let result = agent.delegate_task(task, max_iterations).await?;
    println!("委派结果: {}", result);

    let stats = agent.get_stats();
    println!("委派统计: {} 个委派创建, {} 个成功",
        stats.delegations_created, stats.delegations_succeeded);

    Ok(())
}

fn run_schedule(name: &str, command: &str, schedule_type: &str, seconds: u64) -> Result<()> {
    use scheduler::{CronJob, CronSchedule, CronScheduler};

    let mut scheduler = CronScheduler::new();

    let schedule = match schedule_type {
        "every" => CronSchedule::Every { seconds },
        "hourly" => CronSchedule::Hourly,
        "daily" => CronSchedule::Daily { hour: 9, minute: 0 },
        _ => CronSchedule::Interval { minutes: 60 },
    };

    let job = CronJob::new("job-1", name, schedule, command);
    scheduler.add_job(job);

    println!("定时任务已添加: {}", name);
    println!("调度类型: {}", schedule_type);
    println!("总任务数: {}", scheduler.job_count());

    if let Some(next) = scheduler.get_next_run() {
        println!("下次执行: {}", next);
    }

    Ok(())
}

fn run_list_adapters() {
    println!("可用的聊天平台适配器:");
    for adapter in adapters::AdapterFactory::list_adapters() {
        println!("  - {}", adapter);
    }
}

fn run_doctor() -> Result<()> {
    doctor::Doctor::run()?;
    Ok(())
}

async fn run_status(config_path: Option<&str>) -> Result<()> {
    let config = load_config(config_path)?;
    let agent = agent_engine::AioAgent::new(config)?;

    println!("============================================================");
    println!("AIO Agent 状态");
    println!("============================================================");
    println!("{}", agent.get_context_info());

    let stats = agent.get_stats();
    println!("\n统计信息:");
    println!("  总迭代: {}", stats.total_iterations);
    println!("  工具调用: {}", stats.total_tool_calls);
    println!("  错误数: {}", stats.total_errors);
    println!("  上下文压缩: {}", stats.context_compressions);
    println!("  委派创建: {}", stats.delegations_created);
    println!("  委派成功: {}", stats.delegations_succeeded);

    println!("\n当前提供商: {} (模型: {})", agent.config.providers.active, agent.llm_provider.default_model);
    println!("  API地址: {}", agent.llm_provider.base_url);

    println!("\n已配置提供商:");
    for p in &agent.config.providers.providers {
        let marker = if p.name == agent.config.providers.active { " ★" } else { "" };
        let status = if p.enabled { "启用" } else { "禁用" };
        println!("  - {} ({}) - 模型: {} 个{}", p.name, status, p.models.len(), marker);
    }

    println!("\n已注册工具: {} 个", agent.tools.list_tools().len());
    for tool in agent.tools.list_tools() {
        println!("  - {}", tool);
    }

    println!("\n平台适配器:");
    for adapter in adapters::AdapterFactory::list_adapters() {
        println!("  - {}", adapter);
    }

    Ok(())
}

async fn run_interactive(config_path: Option<&str>, _stream: bool) -> Result<()> {
    let config = load_config(config_path)?;
    let mut app = cli::CliApp::new(config)?;
    app.interactive_mode()?;
    Ok(())
}

async fn run_tests() -> Result<()> {
    println!("运行AIO Agent测试套件...\n");

    let config = config::Config::default();
    let mut aio_agent = agent_engine::AioAgent::new(config)?;

    println!("[测试 1/9] Agent对话测试...");
    let result = aio_agent.run_conversation("请帮我搜索最新AI Agent框架信息").await?;
    println!("  ✓ 完成 (迭代: {}, 压缩: {})", result.iterations, result.context_compressed);

    println!("\n[测试 2/9] 多Agent协作测试...");
    let agents = vec![
        agents::Agent::new("1", "研究员", "搜索信息", "专业研究员"),
        agents::Agent::new("2", "分析师", "分析数据", "数据分析专家"),
    ];
    let tasks = vec![
        tasks::Task::new("t1", "搜索最新AI框架"),
        tasks::Task::new("t2", "分析框架优缺点"),
    ];
    let crew = agents::Crew::new(agents, tasks, agents::Process::Sequential);
    let results = crew.kickoff().await?;
    println!("  ✓ 完成 ({} 个任务)", results.len());

    println!("\n[测试 3/9] 任务循环测试...");
    let mut task_loop = tasks::TaskLoop::new("构建RAG系统", 10);
    let completed = task_loop.run().await?;
    println!("  ✓ 完成 ({} 个任务)", completed.len());

    println!("\n[测试 4/9] SOP流程测试...");
    let mut sop = workflows::Sop::new();
    sop.add_step("搜索", "web_search", "研究员", None);
    sop.add_step("分析", "web_search", "分析师", None);
    let sop_results = sop.execute(&std::collections::HashMap::new()).await?;
    println!("  ✓ 完成 ({} 个步骤)", sop_results.len());

    println!("\n[测试 5/9] 网关系统测试...");
    let gateway_config = gateway::GatewayBuilder::new()
        .host("127.0.0.1")
        .port(3000)
        .auth_token("secret-token")
        .max_connections(1000)
        .add_channel(
            gateway::ChannelAccount::new("telegram-1", gateway::ChannelType::Telegram)
                .with_token("bot-token-123".to_string())
                .enabled()
        )
        .build();
    println!("  ✓ 完成 ({} 个通道)", gateway_config.config.channels.len());

    println!("\n[测试 6/9] Skills系统测试...");
    match skills::SkillManager::new() {
        Ok(mut skill_manager) => {
            let all_skills = skill_manager.list_skills();
            println!("  ✓ 完成 ({} 个技能)", all_skills.len());
        }
        Err(e) => {
            println!("  ⚠ Skills初始化警告: {}", e);
        }
    }

    println!("\n[测试 7/9] 预算系统测试...");
    let budget = budget::IterationBudget::new(5);
    assert!(budget.consume());
    assert!(budget.consume());
    println!("  预算使用: {}/{}", budget.used(), budget.remaining());
    budget.refund();
    println!("  退款后: {}/{}", budget.used(), budget.remaining());
    let tool_budget = budget::ToolBudget::new(10, 60);
    println!("  工具预算: 剩余{}次, 剩余{}秒", tool_budget.remaining(), tool_budget.time_remaining());
    println!("  ✓ 完成");

    println!("\n[测试 8/9] 调度器测试...");
    let mut scheduler = scheduler::CronScheduler::new();
    let job = scheduler::CronJob::new("test-1", "测试任务", scheduler::CronSchedule::Hourly, "echo test");
    scheduler.add_job(job);
    println!("  任务数: {}", scheduler.job_count());
    println!("  下次执行: {:?}", scheduler.get_next_run());
    println!("  ✓ 完成");

    println!("\n[测试 9/9] 委派系统测试...");
    let policy = delegation::DelegationPolicy::default();
    let mut dm = delegation::DelegationManager::new(policy);
    assert!(dm.can_delegate());
    let req = delegation::SubAgentRequest {
        task: "测试委派任务".to_string(),
        max_iterations: 10,
        allowed_tools: None,
        context: std::collections::HashMap::new(),
    };
    let id = dm.create_delegation(req);
    dm.complete_delegation(&id, true, "委派完成".to_string(), 3);
    println!("  委派: {}, 状态: 成功", id);
    println!("  ✓ 完成");

    println!("\n============================================================");
    println!("所有测试完成！");
    println!("============================================================");

    Ok(())
}

fn load_config(config_path: Option<&str>) -> Result<config::Config> {
    if let Some(path) = config_path {
        config::Config::from_file(path)
    } else {
        let paths = vec![
            "aio-agent.toml".to_string(),
            "config.toml".to_string(),
            dirs_next::home_dir()
                .map(|h| h.join(".aio-agent").join("config.toml"))
                .unwrap_or_default()
                .display()
                .to_string(),
        ];

        for path in paths {
            if std::path::Path::new(&path).exists() {
                if let Ok(config) = config::Config::from_file(&path) {
                    println!("[信息] 使用配置文件: {}", path);
                    return Ok(config);
                }
            }
        }

        Ok(config::Config::default())
    }
}

async fn run_model_command(cmd: ModelCommand, config_path: Option<&str>) -> Result<()> {
    let config_path_str = config_path.unwrap_or("aio-agent.toml");
    let mut config = load_config(config_path)?;

    match cmd {
        ModelCommand::List => {
            println!("\n可用的模型提供商:");
            println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            for provider in &config.providers.providers {
                let active = if provider.name == config.providers.active {
                    " ★"
                } else {
                    "   "
                };
                let status = if provider.enabled { "启用" } else { "禁用" };
                println!("{} {} ({}) - 模型: {} 个", active, provider.name, status, provider.models.len());
                if provider.name == config.providers.active {
                    println!("     默认模型: {}", provider.default_model);
                    println!("     API地址: {}", provider.base_url);
                }
            }
            println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            println!("当前活跃提供商: {}", config.providers.active);
        }

        ModelCommand::Switch { name } => {
            let provider_exists = config.providers.providers.iter().any(|p| p.name == name);
            if !provider_exists {
                return Err(anyhow::anyhow!("提供商 '{}' 不存在，请先使用 'model add' 添加", name));
            }

            if let Some(provider) = config.providers.providers.iter().find(|p| p.name == name) {
                if !provider.enabled {
                    return Err(anyhow::anyhow!("提供商 '{}' 已禁用，请先启用", name));
                }
            }

            config.providers.active = name.clone();
            config.agent.model = config.providers.providers.iter()
                .find(|p| p.name == name)
                .map(|p| p.default_model.clone())
                .unwrap_or_else(|| "gpt-4".to_string());

            config.save_to_file(config_path_str)?;
            println!("✓ 已切换到模型提供商: {}", name);
            println!("  默认模型: {}", config.agent.model);
        }

        ModelCommand::Add { name, api_key, base_url, models, default_model } => {
            let provider_exists = config.providers.providers.iter().any(|p| p.name == name);
            if provider_exists {
                return Err(anyhow::anyhow!("提供商 '{}' 已存在，请先移除再添加", name));
            }

            let key = api_key.unwrap_or_else(|| std::env::var("AIO_AGENT_API_KEY").unwrap_or_default());
            let models_list = models.map(|m| m.split(',').map(|s| s.trim().to_string()).collect::<Vec<_>>())
                .unwrap_or_else(|| vec!["gpt-4".to_string()]);
            let default = default_model.unwrap_or_else(|| models_list.first().cloned().unwrap_or("gpt-4".to_string()));

            let provider = config::ProviderConfig::custom(&name, &key, &base_url, models_list.clone());
            config.providers.providers.push(provider);
            config.save_to_file(config_path_str)?;

            println!("✓ 已添加模型提供商: {}", name);
            println!("  API地址: {}", base_url);
            println!("  可用模型: {}", models_list.join(", "));
            println!("  默认模型: {}", default);
        }

        ModelCommand::Remove { name } => {
            let initial_len = config.providers.providers.len();
            config.providers.providers.retain(|p| p.name != name);

            if config.providers.providers.len() == initial_len {
                return Err(anyhow::anyhow!("提供商 '{}' 不存在", name));
            }

            if config.providers.active == name {
                if let Some(first) = config.providers.providers.first() {
                    config.providers.active = first.name.clone();
                }
            }

            config.save_to_file(config_path_str)?;
            println!("✓ 已移除模型提供商: {}", name);
        }

        ModelCommand::Test => {
            let active_provider = config.providers.providers.iter()
                .find(|p| p.name == config.providers.active);

            match active_provider {
                Some(provider) => {
                    println!("正在测试模型连接: {} ({})", provider.name, provider.default_model);

                    let client = reqwest::Client::new();
                    let url = format!("{}/chat/completions", provider.base_url);

                    let body = serde_json::json!({
                        "model": provider.default_model,
                        "messages": [{"role": "user", "content": "Hello"}],
                        "max_tokens": 10,
                    });

                    match client.post(&url)
                        .header("Authorization", format!("Bearer {}", provider.api_key))
                        .header("Content-Type", "application/json")
                        .json(&body)
                        .timeout(std::time::Duration::from_secs(10))
                        .send()
                        .await
                    {
                        Ok(response) => {
                            let status = response.status();
                            if status.is_success() {
                                println!("✓ 模型连接成功!");
                                println!("  提供商: {}", provider.name);
                                println!("  模型: {}", provider.default_model);
                                println!("  API地址: {}", provider.base_url);
                            } else {
                                let error_text = response.text().await.unwrap_or_default();
                                println!("✗ 模型连接失败 (HTTP {})", status);
                                println!("  错误: {}", error_text.chars().take(200).collect::<String>());
                            }
                        }
                        Err(e) => {
                            println!("✗ 模型连接失败");
                            println!("  错误: {}", e);
                        }
                    }
                }
                None => {
                    println!("✗ 没有找到活跃的模型提供商");
                    println!("  请先使用 'model add' 添加提供商");
                }
            }
        }
    }

    Ok(())
}
