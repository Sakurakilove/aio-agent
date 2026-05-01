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

#[derive(Parser)]
#[command(name = "aio-agent")]
#[command(version = "1.0.0")]
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
        let provider = streaming::StreamingLlmProvider::new(
            &config.llm.api_key,
            &config.llm.base_url,
            &config.agent.model,
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
    let delegation_id = agent.delegate_task(task, max_iterations).await?;
    println!("委派ID: {}", delegation_id);

    let stats = agent.get_stats();
    println!("委派成功: {} 个委派, {} 个成功",
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

    println!("\n平台适配器:");
    for adapter in adapters::AdapterFactory::list_adapters() {
        println!("  - {}", adapter);
    }

    println!("\n提供商:");
    let mut pm = providers::ProviderManager::new();
    pm.add_provider(providers::ProviderInfo::openai(""));
    pm.add_provider(providers::ProviderInfo::ollama("http://localhost:11434"));
    for p in pm.list_providers() {
        println!("  - {} ({} 模型)", p.name, p.models.len());
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
        Ok(config::Config::default())
    }
}
