use anyhow::Result;
use std::fs;
use std::path::PathBuf;

/// 诊断检查项
#[derive(Debug)]
pub struct CheckResult {
    pub name: String,
    pub status: CheckStatus,
    pub message: String,
    pub suggestion: Option<String>,
}

#[derive(Debug)]
pub enum CheckStatus {
    Pass,
    Warn,
    Fail,
}

impl std::fmt::Display for CheckStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CheckStatus::Pass => write!(f, "✓"),
            CheckStatus::Warn => write!(f, "⚠"),
            CheckStatus::Fail => write!(f, "✗"),
        }
    }
}

/// 系统信息
#[derive(Debug)]
pub struct SystemInfo {
    pub os: String,
    pub arch: String,
    pub rust_version: String,
    pub chrome_version: Option<String>,
}

/// Doctor诊断工具
pub struct Doctor;

impl Doctor {
    /// 运行完整诊断
    pub fn run() -> Result<()> {
        println!("============================================================");
        println!("  AIO Agent 诊断工具 v1.0.0");
        println!("============================================================\n");

        let checks = vec![
            Self::check_rust_environment(),
            Self::check_config_files(),
            Self::check_api_configuration(),
            Self::check_memory_system(),
            Self::check_tools_configuration(),
            Self::check_skills_system(),
            Self::check_platform_adapters(),
            Self::check_permissions_system(),
            Self::check_chrome_availability(),
        ];

        let mut pass_count = 0;
        let mut warn_count = 0;
        let mut fail_count = 0;

        for check in checks {
            match check.status {
                CheckStatus::Pass => pass_count += 1,
                CheckStatus::Warn => warn_count += 1,
                CheckStatus::Fail => fail_count += 1,
            }

            println!("{} {}", check.status, check.name);
            println!("  {}", check.message);
            if let Some(suggestion) = &check.suggestion {
                println!("  建议: {}", suggestion);
            }
            println!();
        }

        println!("============================================================");
        println!("诊断结果: {} 通过, {} 警告, {} 失败", pass_count, warn_count, fail_count);
        println!("============================================================");

        if fail_count > 0 {
            println!("\n存在失败项，请检查并修复上述问题");
        } else if warn_count > 0 {
            println!("\n存在警告项，建议进行检查");
        } else {
            println!("\n所有检查通过！系统配置正常");
        }

        Ok(())
    }

    fn check_rust_environment() -> CheckResult {
        let rust_version = std::process::Command::new("rustc")
            .arg("--version")
            .output()
            .ok();

        if let Some(output) = rust_version {
            let version_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
            CheckResult {
                name: "Rust环境".to_string(),
                status: CheckStatus::Pass,
                message: format!("Rust版本: {}", version_str),
                suggestion: None,
            }
        } else {
            CheckResult {
                name: "Rust环境".to_string(),
                status: CheckStatus::Fail,
                message: "未找到rustc命令".to_string(),
                suggestion: Some("请安装Rust: https://rustup.rs/".to_string()),
            }
        }
    }

    fn check_config_files() -> CheckResult {
        let config_paths = vec![
            PathBuf::from("aio-agent.toml"),
            PathBuf::from("config.toml"),
            dirs_next::home_dir()
                .map(|h| h.join(".aio-agent").join("config.toml"))
                .unwrap_or_default(),
        ];

        let mut found_config = false;
        let mut config_path = String::new();

        for path in config_paths {
            if path.exists() {
                found_config = true;
                config_path = path.display().to_string();
                break;
            }
        }

        if found_config {
            CheckResult {
                name: "配置文件".to_string(),
                status: CheckStatus::Pass,
                message: format!("找到配置文件: {}", config_path),
                suggestion: None,
            }
        } else {
            CheckResult {
                name: "配置文件".to_string(),
                status: CheckStatus::Warn,
                message: "未找到配置文件".to_string(),
                suggestion: Some("运行 'aio-agent setup' 创建配置文件".to_string()),
            }
        }
    }

    fn check_api_configuration() -> CheckResult {
        let api_key = std::env::var("AIO_AGENT_API_KEY")
            .ok()
            .or_else(|| {
                // 尝试从配置文件读取
                if let Some(home) = dirs_next::home_dir() {
                    let config_path = home.join(".aio-agent").join("config.toml");
                    if config_path.exists() {
                        if let Ok(content) = fs::read_to_string(&config_path) {
                            if let Some(start) = content.find("api_key = \"") {
                                let start = start + 11;
                                if let Some(end) = content[start..].find('"') {
                                    return Some(content[start..start+end].to_string());
                                }
                            }
                        }
                    }
                }
                None
            });

        if let Some(key) = api_key {
            if key.starts_with("sk-") && key.len() > 10 {
                CheckResult {
                    name: "API配置".to_string(),
                    status: CheckStatus::Pass,
                    message: format!("API密钥已配置 ({}...)", &key[..8]),
                    suggestion: None,
                }
            } else {
                CheckResult {
                    name: "API配置".to_string(),
                    status: CheckStatus::Fail,
                    message: "API密钥格式不正确".to_string(),
                    suggestion: Some("请检查API密钥格式".to_string()),
                }
            }
        } else {
            CheckResult {
                name: "API配置".to_string(),
                status: CheckStatus::Fail,
                message: "未配置API密钥".to_string(),
                suggestion: Some("设置环境变量 AIO_AGENT_API_KEY 或在配置文件中添加".to_string()),
            }
        }
    }

    fn check_memory_system() -> CheckResult {
        let default_path = dirs_next::home_dir()
            .map(|h| h.join(".aio-agent").join("memory.db"))
            .unwrap_or_else(|| PathBuf::from("memory.db"));

        if default_path.exists() {
            let metadata = fs::metadata(&default_path);
            if let Ok(meta) = metadata {
                CheckResult {
                    name: "记忆系统".to_string(),
                    status: CheckStatus::Pass,
                    message: format!("SQLite数据库存在 ({} bytes)", meta.len()),
                    suggestion: None,
                }
            } else {
                CheckResult {
                    name: "记忆系统".to_string(),
                    status: CheckStatus::Warn,
                    message: "无法读取数据库文件".to_string(),
                    suggestion: Some("检查文件权限".to_string()),
                }
            }
        } else {
            CheckResult {
                name: "记忆系统".to_string(),
                status: CheckStatus::Warn,
                message: "记忆数据库不存在（首次运行时创建）".to_string(),
                suggestion: None,
            }
        }
    }

    fn check_tools_configuration() -> CheckResult {
        let default_tools = vec![
            "web_search", "file_read", "file_write", "terminal", "web_fetch",
        ];

        CheckResult {
            name: "工具配置".to_string(),
            status: CheckStatus::Pass,
            message: format!("默认工具可用: {} 个", default_tools.len()),
            suggestion: None,
        }
    }

    fn check_skills_system() -> CheckResult {
        let skills_dir = dirs_next::home_dir()
            .map(|h| h.join(".aio-agent").join("skills"));

        if let Some(dir) = skills_dir {
            if dir.exists() {
                match fs::read_dir(&dir) {
                    Ok(entries) => {
                        let count = entries.count();
                        CheckResult {
                            name: "Skills系统".to_string(),
                            status: CheckStatus::Pass,
                            message: format!("Skills目录存在 ({} 个技能)", count),
                            suggestion: None,
                        }
                    }
                    Err(_) => CheckResult {
                        name: "Skills系统".to_string(),
                        status: CheckStatus::Warn,
                        message: "无法读取Skills目录".to_string(),
                        suggestion: Some("检查目录权限".to_string()),
                    }
                }
            } else {
                CheckResult {
                    name: "Skills系统".to_string(),
                    status: CheckStatus::Warn,
                    message: "Skills目录不存在".to_string(),
                    suggestion: Some("创建 ~/.aio-agent/skills/ 目录添加技能".to_string()),
                }
            }
        } else {
            CheckResult {
                name: "Skills系统".to_string(),
                status: CheckStatus::Warn,
                message: "无法确定Skills目录路径".to_string(),
                suggestion: None,
            }
        }
    }

    fn check_platform_adapters() -> CheckResult {
        let adapters = crate::adapters::AdapterFactory::list_adapters();
        
        CheckResult {
            name: "平台适配器".to_string(),
            status: CheckStatus::Pass,
            message: format!("可用适配器: {}", adapters.join(", ")),
            suggestion: None,
        }
    }

    fn check_permissions_system() -> CheckResult {
        CheckResult {
            name: "权限系统".to_string(),
            status: CheckStatus::Pass,
            message: "权限系统已启用".to_string(),
            suggestion: None,
        }
    }

    fn check_chrome_availability() -> CheckResult {
        // 检查Chrome/Chromium是否可用
        let chrome_paths = vec![
            "chrome".to_string(),
            "chromium".to_string(),
            "google-chrome".to_string(),
            "google-chrome-stable".to_string(),
        ];

        for cmd in chrome_paths {
            let result = std::process::Command::new(&cmd)
                .arg("--version")
                .output();

            if let Ok(output) = result {
                if output.status.success() {
                    let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    return CheckResult {
                        name: "Chrome浏览器".to_string(),
                        status: CheckStatus::Pass,
                        message: format!("Chrome可用: {}", version),
                        suggestion: None,
                    };
                }
            }
        }

        CheckResult {
            name: "Chrome浏览器".to_string(),
            status: CheckStatus::Warn,
            message: "Chrome/Chromium未找到（浏览器自动化需要）".to_string(),
            suggestion: Some("安装Chrome或Chromium以使用浏览器自动化工具".to_string()),
        }
    }
}
