use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::io::{self, Write};
use std::sync::Arc;
use tokio::sync::Mutex;

/// 人机审批结果：批准、拒绝（含原因）、修改后批准
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HumanApproval {
    Approved,
    Rejected(String),
    Modified(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalRequest {
    pub id: String,
    pub action_type: String,
    pub description: String,
    pub details: serde_json::Value,
    pub risk_level: RiskLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

pub trait ApprovalHandler: Send + Sync {
    fn request_approval(&self, request: &ApprovalRequest) -> HumanApproval;
}

pub struct ConsoleApprovalHandler {
    pub auto_approve_low_risk: bool,
}

impl ConsoleApprovalHandler {
    pub fn new(auto_approve_low_risk: bool) -> Self {
        Self { auto_approve_low_risk }
    }
}

impl ApprovalHandler for ConsoleApprovalHandler {
    fn request_approval(&self, request: &ApprovalRequest) -> HumanApproval {
        if self.auto_approve_low_risk {
            if matches!(request.risk_level, RiskLevel::Low) {
                return HumanApproval::Approved;
            }
        }

        println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("⚠️  需要人工审批");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("操作类型: {}", request.action_type);
        println!("描述: {}", request.description);
        println!("风险等级: {:?}", request.risk_level);
        println!("详情: {}", serde_json::to_string_pretty(&request.details).unwrap_or_default());
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("[y] 批准  [n] 拒绝  [m] 修改后批准");
        print!("请选择: ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        let choice = input.trim().to_lowercase();

        match choice.as_str() {
            "y" | "yes" | "是" => HumanApproval::Approved,
            "n" | "no" | "否" => HumanApproval::Rejected("用户拒绝操作".to_string()),
            "m" | "modify" => {
                print!("请输入修改后的内容: ");
                io::stdout().flush().unwrap();
                let mut modified = String::new();
                io::stdin().read_line(&mut modified).unwrap();
                HumanApproval::Modified(modified.trim().to_string())
            }
            _ => HumanApproval::Rejected("无效选择，默认拒绝".to_string()),
        }
    }
}

pub struct AutoApprovalHandler {
    pub approved_actions: Vec<String>,
    pub max_risk_level: RiskLevel,
}

impl AutoApprovalHandler {
    pub fn new(max_risk_level: RiskLevel) -> Self {
        Self {
            approved_actions: Vec::new(),
            max_risk_level,
        }
    }

    pub fn approve_action(&mut self, action: &str) {
        if !self.approved_actions.contains(&action.to_string()) {
            self.approved_actions.push(action.to_string());
        }
    }
}

impl ApprovalHandler for AutoApprovalHandler {
    fn request_approval(&self, request: &ApprovalRequest) -> HumanApproval {
        if self.approved_actions.contains(&request.action_type) {
            return HumanApproval::Approved;
        }

        let risk_order = |level: &RiskLevel| match level {
            RiskLevel::Low => 0,
            RiskLevel::Medium => 1,
            RiskLevel::High => 2,
            RiskLevel::Critical => 3,
        };

        if risk_order(&request.risk_level) <= risk_order(&self.max_risk_level) {
            HumanApproval::Approved
        } else {
            HumanApproval::Rejected(format!("风险等级 {:?} 超过自动批准阈值 {:?}", request.risk_level, self.max_risk_level))
        }
    }
}

/// 人机协作（HITL）管理器，控制哪些操作需要人工审批
pub struct HumanInTheLoop {
    pub handler: Arc<dyn ApprovalHandler>,
    pub approval_log: Arc<Mutex<Vec<(ApprovalRequest, HumanApproval)>>>,
    pub require_approval_for: Vec<String>,
    pub skip_approval_for: Vec<String>,
}

impl HumanInTheLoop {
    pub fn console(auto_approve_low_risk: bool) -> Self {
        Self {
            handler: Arc::new(ConsoleApprovalHandler::new(auto_approve_low_risk)),
            approval_log: Arc::new(Mutex::new(Vec::new())),
            require_approval_for: vec![
                "file_write".to_string(),
                "terminal".to_string(),
                "remove".to_string(),
                "move".to_string(),
            ],
            skip_approval_for: vec![
                "file_read".to_string(),
                "list_dir".to_string(),
                "web_search".to_string(),
                "web_fetch".to_string(),
            ],
        }
    }

    pub fn auto(max_risk_level: RiskLevel) -> Self {
        Self {
            handler: Arc::new(AutoApprovalHandler::new(max_risk_level)),
            approval_log: Arc::new(Mutex::new(Vec::new())),
            require_approval_for: Vec::new(),
            skip_approval_for: vec!["*".to_string()],
        }
    }

    pub fn needs_approval(&self, action: &str) -> bool {
        if self.skip_approval_for.contains(&"*".to_string()) {
            return false;
        }
        if self.skip_approval_for.contains(&action.to_string()) {
            return false;
        }
        if self.require_approval_for.contains(&action.to_string()) {
            return true;
        }
        !self.skip_approval_for.contains(&action.to_string())
    }

    pub fn request_approval(&self, request: ApprovalRequest) -> HumanApproval {
        let approval = self.handler.request_approval(&request);

        let log = self.approval_log.clone();
        let req = request.clone();
        let app = approval.clone();
        tokio::spawn(async move {
            let mut log = log.lock().await;
            log.push((req, app));
        });

        approval
    }

    pub async fn get_approval_log(&self) -> Vec<(ApprovalRequest, HumanApproval)> {
        self.approval_log.lock().await.clone()
    }
}
