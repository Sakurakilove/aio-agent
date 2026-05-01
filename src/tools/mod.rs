//! 工具系统 (来自Hermes toolsets.py + LangChain partners)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use anyhow::Result;
use async_trait::async_trait;

/// 工具执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub success: bool,
    pub data: Option<serde_json::Value>,
    pub error: Option<String>,
}

impl ToolResult {
    pub fn success(data: serde_json::Value) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn error(message: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message),
        }
    }
}

/// 工具Trait (异步)
#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn schema(&self) -> serde_json::Value;
    async fn execute(&self, args: serde_json::Value) -> Result<ToolResult>;
}

/// 工具注册器 (来自Hermes tools/registry.py)
pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
    schemas: HashMap<String, serde_json::Value>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
            schemas: HashMap::new(),
        }
    }

    /// 注册工具
    pub fn register(&mut self, tool: Arc<dyn Tool>) {
        let name = tool.name().to_string();
        let schema = tool.schema();
        self.schemas.insert(name.clone(), schema);
        self.tools.insert(name, tool);
    }

    /// 获取工具
    pub fn get(&self, name: &str) -> Option<&Arc<dyn Tool>> {
        self.tools.get(name)
    }

    /// 获取工具schema
    pub fn get_schema(&self, name: &str) -> Option<&serde_json::Value> {
        self.schemas.get(name)
    }

    /// 列出所有工具
    pub fn list_tools(&self) -> Vec<&str> {
        self.tools.keys().map(|k| k.as_str()).collect()
    }

    /// 执行工具
    pub async fn execute(&self, name: &str, args: serde_json::Value) -> Result<ToolResult> {
        match self.tools.get(name) {
            Some(tool) => tool.execute(args).await,
            None => Ok(ToolResult::error(format!("Tool '{}' not found", name))),
        }
    }

    /// 获取所有工具的schema列表
    pub fn get_all_schemas(&self) -> Vec<serde_json::Value> {
        self.schemas.values().cloned().collect()
    }
}

/// 内置工具实现 (来自Hermes tools/)

// 网络搜索工具
pub struct WebSearchTool;

#[async_trait]
impl Tool for WebSearchTool {
    fn name(&self) -> &str {
        "web_search"
    }

    fn description(&self) -> &str {
        "搜索网络获取最新信息"
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "name": "web_search",
            "description": self.description(),
            "parameters": {
                "type": "object",
                "properties": {
                    "query": {"type": "string", "description": "搜索查询"}
                },
                "required": ["query"]
            }
        })
    }

    async fn execute(&self, args: serde_json::Value) -> Result<ToolResult> {
        let query = args["query"].as_str().unwrap_or("");
        // 模拟搜索结果
        let result = serde_json::json!({
            "query": query,
            "results": vec![
                format!("搜索结果1: {}", query),
                format!("搜索结果2: {}", query),
            ]
        });
        Ok(ToolResult::success(result))
    }
}

// 文件读取工具
pub struct FileReadTool;

#[async_trait]
impl Tool for FileReadTool {
    fn name(&self) -> &str {
        "file_read"
    }

    fn description(&self) -> &str {
        "读取文件内容"
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "name": "file_read",
            "description": self.description(),
            "parameters": {
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "文件路径"}
                },
                "required": ["path"]
            }
        })
    }

    async fn execute(&self, args: serde_json::Value) -> Result<ToolResult> {
        let path = args["path"].as_str().unwrap_or("");
        match tokio::fs::read_to_string(path).await {
            Ok(content) => Ok(ToolResult::success(serde_json::json!({"content": content}))),
            Err(e) => Ok(ToolResult::error(format!("读取失败: {}", e))),
        }
    }
}

// 文件写入工具
pub struct FileWriteTool;

#[async_trait]
impl Tool for FileWriteTool {
    fn name(&self) -> &str {
        "file_write"
    }

    fn description(&self) -> &str {
        "写入文件内容"
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "name": "file_write",
            "description": self.description(),
            "parameters": {
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "文件路径"},
                    "content": {"type": "string", "description": "文件内容"}
                },
                "required": ["path", "content"]
            }
        })
    }

    async fn execute(&self, args: serde_json::Value) -> Result<ToolResult> {
        let path = args["path"].as_str().unwrap_or("");
        let content = args["content"].as_str().unwrap_or("");
        
        // 创建目录
        if let Some(parent) = std::path::Path::new(path).parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        
        match tokio::fs::write(path, content).await {
            Ok(_) => Ok(ToolResult::success(serde_json::json!({"message": format!("写入成功: {}", path)}))),
            Err(e) => Ok(ToolResult::error(format!("写入失败: {}", e))),
        }
    }
}

// 终端执行工具
pub struct TerminalTool;

#[async_trait]
impl Tool for TerminalTool {
    fn name(&self) -> &str {
        "terminal"
    }

    fn description(&self) -> &str {
        "执行终端命令"
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "name": "terminal",
            "description": self.description(),
            "parameters": {
                "type": "object",
                "properties": {
                    "command": {"type": "string", "description": "终端命令"}
                },
                "required": ["command"]
            }
        })
    }

    async fn execute(&self, args: serde_json::Value) -> Result<ToolResult> {
        let command = args["command"].as_str().unwrap_or("");
        
        // 安全检查：禁止危险命令
        if command.contains("rm -rf /") || command.contains("sudo rm -rf") {
            return Ok(ToolResult::error("禁止执行危险命令".to_string()));
        }

        #[cfg(unix)]
        {
            let output = tokio::process::Command::new("sh")
                .arg("-c")
                .arg(command)
                .output()
                .await?;
            
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            
            Ok(ToolResult::success(serde_json::json!({
                "stdout": stdout.to_string(),
                "stderr": stderr.to_string(),
                "exit_code": output.status.code().unwrap_or(-1),
            })))
        }
        
        #[cfg(windows)]
        {
            let output = tokio::process::Command::new("cmd")
                .arg("/c")
                .arg(command)
                .output()
                .await?;
            
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            
            Ok(ToolResult::success(serde_json::json!({
                "stdout": stdout.to_string(),
                "stderr": stderr.to_string(),
                "exit_code": output.status.code().unwrap_or(-1),
            })))
        }
    }
}

/// 网页爬取工具
pub struct WebFetchTool;

#[async_trait]
impl Tool for WebFetchTool {
    fn name(&self) -> &str {
        "web_fetch"
    }

    fn description(&self) -> &str {
        "抓取指定URL的网页内容，返回HTML或纯文本"
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "name": "web_fetch",
            "description": self.description(),
            "parameters": {
                "type": "object",
                "properties": {
                    "url": {"type": "string", "description": "要抓取的网页URL"},
                    "format": {"type": "string", "description": "返回格式: html(默认) 或 text", "enum": ["html", "text"]}
                },
                "required": ["url"]
            }
        })
    }

    async fn execute(&self, args: serde_json::Value) -> Result<ToolResult> {
        let url = args["url"].as_str().unwrap_or("");
        let format = args["format"].as_str().unwrap_or("html");

        if url.is_empty() {
            return Ok(ToolResult::error("URL不能为空".to_string()));
        }

        // 安全检查：只允许HTTP/HTTPS协议
        if !url.starts_with("http://") && !url.starts_with("https://") {
            return Ok(ToolResult::error("只支持HTTP/HTTPS协议".to_string()));
        }

        match reqwest::get(url).await {
            Ok(response) => {
                let status = response.status();
                let content_type = response
                    .headers()
                    .get(reqwest::header::CONTENT_TYPE)
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("unknown")
                    .to_string();

                let body = response.text().await?;

                if format == "text" {
                    // 简单HTML转纯文本（移除标签）
                    let text = strip_html_tags(&body);
                    Ok(ToolResult::success(serde_json::json!({
                        "url": url,
                        "status": status.as_u16(),
                        "content_type": content_type,
                        "format": "text",
                        "content": text,
                        "length": body.len(),
                    })))
                } else {
                    Ok(ToolResult::success(serde_json::json!({
                        "url": url,
                        "status": status.as_u16(),
                        "content_type": content_type,
                        "format": "html",
                        "content": body,
                        "length": body.len(),
                    })))
                }
            }
            Err(e) => Ok(ToolResult::error(format!("请求失败: {}", e))),
        }
    }
}

/// 简单HTML标签剥离函数
pub fn strip_html_tags(html: &str) -> String {
    let mut result = String::with_capacity(html.len());
    let mut in_tag = false;
    let mut last_was_space = false;

    for ch in html.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => {
                // 处理常见的空白字符
                if ch.is_whitespace() {
                    if !last_was_space {
                        result.push(' ');
                        last_was_space = true;
                    }
                } else {
                    result.push(ch);
                    last_was_space = false;
                }
            }
            _ => {}
        }
    }

    result.trim().to_string()
}

// ==================== 新增工具 ====================

/// 文件搜索工具（搜索文件和目录）
pub struct SearchFilesTool;

#[async_trait]
impl Tool for SearchFilesTool {
    fn name(&self) -> &str {
        "search_files"
    }

    fn description(&self) -> &str {
        "在指定目录中搜索文件，支持glob模式"
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "name": "search_files",
            "description": self.description(),
            "parameters": {
                "type": "object",
                "properties": {
                    "pattern": {"type": "string", "description": "搜索模式，如*.rs, *.md等"},
                    "path": {"type": "string", "description": "搜索根目录，默认为当前目录"}
                },
                "required": ["pattern"]
            }
        })
    }

    async fn execute(&self, args: serde_json::Value) -> Result<ToolResult> {
        let pattern = args["pattern"].as_str().unwrap_or("*");
        let search_path = args["path"].as_str().unwrap_or(".");
        
        let search_pattern = format!("{}/**/{}", search_path, pattern);
        
        match glob::glob(&search_pattern) {
            Ok(paths) => {
                let mut results = Vec::new();
                for entry in paths.take(50) {  // 限制结果数量
                    if let Ok(path) = entry {
                        results.push(path.to_string_lossy().to_string());
                    }
                }
                Ok(ToolResult::success(serde_json::json!({
                    "pattern": pattern,
                    "path": search_path,
                    "count": results.len(),
                    "files": results
                })))
            }
            Err(e) => Ok(ToolResult::error(format!("搜索失败: {}", e))),
        }
    }
}

/// 文件补丁工具（支持局部修改文件内容）
pub struct PatchFileTool;

#[async_trait]
impl Tool for PatchFileTool {
    fn name(&self) -> &str {
        "patch_file"
    }

    fn description(&self) -> &str {
        "对文件内容进行局部替换，类似patch操作"
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "name": "patch_file",
            "description": self.description(),
            "parameters": {
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "文件路径"},
                    "old_str": {"type": "string", "description": "要替换的原文"},
                    "new_str": {"type": "string", "description": "替换后的新内容"}
                },
                "required": ["path", "old_str", "new_str"]
            }
        })
    }

    async fn execute(&self, args: serde_json::Value) -> Result<ToolResult> {
        let path = args["path"].as_str().unwrap_or("");
        let old_str = args["old_str"].as_str().unwrap_or("");
        let new_str = args["new_str"].as_str().unwrap_or("");
        
        match tokio::fs::read_to_string(path).await {
            Ok(content) => {
                if !content.contains(old_str) {
                    return Ok(ToolResult::error(format!("未找到要替换的内容: {}", old_str.chars().take(50).collect::<String>())));
                }
                
                let new_content = content.replace(old_str, new_str);
                match tokio::fs::write(path, new_content).await {
                    Ok(_) => Ok(ToolResult::success(serde_json::json!({
                        "message": format!("补丁应用成功: {}", path),
                        "replacements": 1
                    }))),
                    Err(e) => Ok(ToolResult::error(format!("写入失败: {}", e))),
                }
            }
            Err(e) => Ok(ToolResult::error(format!("读取失败: {}", e))),
        }
    }
}

/// 目录列表工具
pub struct ListDirTool;

#[async_trait]
impl Tool for ListDirTool {
    fn name(&self) -> &str {
        "list_dir"
    }

    fn description(&self) -> &str {
        "列出目录中的文件和子目录"
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "name": "list_dir",
            "description": self.description(),
            "parameters": {
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "目录路径，默认为.表示当前目录"}
                },
                "required": ["path"]
            }
        })
    }

    async fn execute(&self, args: serde_json::Value) -> Result<ToolResult> {
        let path = args["path"].as_str().unwrap_or(".");
        
        match tokio::fs::read_dir(path).await {
            Ok(mut entries) => {
                let mut files = Vec::new();
                let mut dirs = Vec::new();
                
                while let Ok(Some(entry)) = entries.next_entry().await {
                    let name = entry.file_name().to_string_lossy().to_string();
                    if entry.file_type().await.map(|ft| ft.is_dir()).unwrap_or(false) {
                        dirs.push(name);
                    } else {
                        files.push(name);
                    }
                }
                
                Ok(ToolResult::success(serde_json::json!({
                    "path": path,
                    "files": files,
                    "directories": dirs,
                    "file_count": files.len(),
                    "dir_count": dirs.len()
                })))
            }
            Err(e) => Ok(ToolResult::error(format!("目录列表失败: {}", e))),
        }
    }
}

/// JSON处理工具
pub struct JsonTool;

#[async_trait]
impl Tool for JsonTool {
    fn name(&self) -> &str {
        "json_tool"
    }

    fn description(&self) -> &str {
        "处理JSON数据：格式化、解析、提取字段"
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "name": "json_tool",
            "description": self.description(),
            "parameters": {
                "type": "object",
                "properties": {
                    "action": {"type": "string", "description": "操作类型: format(格式化), parse(解析), extract(提取)", "enum": ["format", "parse", "extract"]},
                    "json_str": {"type": "string", "description": "要处理的JSON字符串"},
                    "field": {"type": "string", "description": "要提取的字段路径（仅extract操作需要）"}
                },
                "required": ["action", "json_str"]
            }
        })
    }

    async fn execute(&self, args: serde_json::Value) -> Result<ToolResult> {
        let action = args["action"].as_str().unwrap_or("format");
        let json_str = args["json_str"].as_str().unwrap_or("");
        
        match serde_json::from_str::<serde_json::Value>(json_str) {
            Ok(value) => {
                match action {
                    "format" => {
                        let formatted = serde_json::to_string_pretty(&value)?;
                        Ok(ToolResult::success(serde_json::json!({"formatted": formatted})))
                    }
                    "parse" => {
                        Ok(ToolResult::success(serde_json::json!({
                            "parsed": true,
                            "type": if value.is_object() { "object" } 
                                    else if value.is_array() { "array" } 
                                    else { "primitive" }
                        })))
                    }
                    "extract" => {
                        let field = args["field"].as_str().unwrap_or("");
                        if let Some(extracted) = value.get(field) {
                            Ok(ToolResult::success(serde_json::json!({"field": field, "value": extracted})))
                        } else {
                            Ok(ToolResult::error(format!("字段 '{}' 不存在", field)))
                        }
                    }
                    _ => Ok(ToolResult::error(format!("未知操作: {}", action))),
                }
            }
            Err(e) => Ok(ToolResult::error(format!("JSON解析失败: {}", e))),
        }
    }
}

/// URL处理工具
pub struct UrlTool;

#[async_trait]
impl Tool for UrlTool {
    fn name(&self) -> &str {
        "url_tool"
    }

    fn description(&self) -> &str {
        "处理URL：解析、构建、验证"
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "name": "url_tool",
            "description": self.description(),
            "parameters": {
                "type": "object",
                "properties": {
                    "action": {"type": "string", "description": "操作类型: parse(解析), validate(验证)", "enum": ["parse", "validate"]},
                    "url": {"type": "string", "description": "要处理的URL"}
                },
                "required": ["action", "url"]
            }
        })
    }

    async fn execute(&self, args: serde_json::Value) -> Result<ToolResult> {
        let action = args["action"].as_str().unwrap_or("parse");
        let url_str = args["url"].as_str().unwrap_or("");
        
        match url::Url::parse(url_str) {
            Ok(url) => {
                match action {
                    "parse" => {
                        Ok(ToolResult::success(serde_json::json!({
                            "scheme": url.scheme(),
                            "host": url.host_str().unwrap_or(""),
                            "port": url.port(),
                            "path": url.path(),
                            "query": url.query().unwrap_or(""),
                            "fragment": url.fragment().unwrap_or("")
                        })))
                    }
                    "validate" => {
                        Ok(ToolResult::success(serde_json::json!({
                            "valid": true,
                            "url": url_str
                        })))
                    }
                    _ => Ok(ToolResult::error(format!("未知操作: {}", action))),
                }
            }
            Err(e) => {
                if action == "validate" {
                    Ok(ToolResult::success(serde_json::json!({"valid": false, "error": e.to_string()})))
                } else {
                    Ok(ToolResult::error(format!("URL解析失败: {}", e)))
                }
            }
        }
    }
}

/// 文本处理工具（大小写转换、截断、统计等）
pub struct TextTool;

#[async_trait]
impl Tool for TextTool {
    fn name(&self) -> &str {
        "text_tool"
    }

    fn description(&self) -> &str {
        "文本处理：大小写转换、字数统计、截断、替换等"
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "name": "text_tool",
            "description": self.description(),
            "parameters": {
                "type": "object",
                "properties": {
                    "action": {"type": "string", "description": "操作类型", "enum": ["uppercase", "lowercase", "word_count", "char_count", "truncate", "replace"]},
                    "text": {"type": "string", "description": "要处理的文本"},
                    "limit": {"type": "integer", "description": "截断长度（仅truncate操作需要）"},
                    "old": {"type": "string", "description": "要替换的文本（仅replace操作需要）"},
                    "new": {"type": "string", "description": "替换后的文本（仅replace操作需要）"}
                },
                "required": ["action", "text"]
            }
        })
    }

    async fn execute(&self, args: serde_json::Value) -> Result<ToolResult> {
        let action = args["action"].as_str().unwrap_or("");
        let text = args["text"].as_str().unwrap_or("");
        
        match action {
            "uppercase" => Ok(ToolResult::success(serde_json::json!({"result": text.to_uppercase()}))),
            "lowercase" => Ok(ToolResult::success(serde_json::json!({"result": text.to_lowercase()}))),
            "word_count" => {
                let word_count = text.split_whitespace().count();
                let char_count = text.chars().count();
                Ok(ToolResult::success(serde_json::json!({
                    "words": word_count,
                    "chars": char_count,
                    "lines": text.lines().count()
                })))
            }
            "char_count" => Ok(ToolResult::success(serde_json::json!({"chars": text.chars().count()}))),
            "truncate" => {
                let limit = args["limit"].as_u64().unwrap_or(100) as usize;
                let truncated: String = text.chars().take(limit).collect();
                Ok(ToolResult::success(serde_json::json!({
                    "truncated": truncated,
                    "original_length": text.chars().count(),
                    "truncated_length": limit
                })))
            }
            "replace" => {
                let old = args["old"].as_str().unwrap_or("");
                let new = args["new"].as_str().unwrap_or("");
                let result = text.replace(old, new);
                Ok(ToolResult::success(serde_json::json!({"result": result})))
            }
            _ => Ok(ToolResult::error(format!("未知操作: {}", action))),
        }
    }
}

/// 时间/日期工具
pub struct DateTimeTool;

#[async_trait]
impl Tool for DateTimeTool {
    fn name(&self) -> &str {
        "datetime_tool"
    }

    fn description(&self) -> &str {
        "获取当前时间、日期、时区信息"
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "name": "datetime_tool",
            "description": self.description(),
            "parameters": {
                "type": "object",
                "properties": {
                    "action": {"type": "string", "description": "操作类型", "enum": ["now", "timestamp", "utc", "local"]},
                    "format": {"type": "string", "description": "时间格式，如'%Y-%m-%d %H:%M:%S'"}
                },
                "required": ["action"]
            }
        })
    }

    async fn execute(&self, args: serde_json::Value) -> Result<ToolResult> {
        use chrono::Local;
        use chrono::Utc;
        
        let action = args["action"].as_str().unwrap_or("now");
        let format = args["format"].as_str().unwrap_or("%Y-%m-%d %H:%M:%S");
        
        match action {
            "now" => {
                let now = Local::now();
                Ok(ToolResult::success(serde_json::json!({
                    "datetime": now.format(format).to_string(),
                    "timezone": "local"
                })))
            }
            "timestamp" => {
                let timestamp = chrono::Utc::now().timestamp();
                Ok(ToolResult::success(serde_json::json!({
                    "timestamp": timestamp,
                    "unit": "seconds"
                })))
            }
            "utc" => {
                let now = Utc::now();
                Ok(ToolResult::success(serde_json::json!({
                    "datetime": now.format(format).to_string(),
                    "timezone": "UTC"
                })))
            }
            "local" => {
                let now = Local::now();
                Ok(ToolResult::success(serde_json::json!({
                    "datetime": now.format(format).to_string(),
                    "timezone": now.offset().to_string()
                })))
            }
            _ => Ok(ToolResult::error(format!("未知操作: {}", action))),
        }
    }
}

/// 文件元信息工具
pub struct FileInfoTool;

#[async_trait]
impl Tool for FileInfoTool {
    fn name(&self) -> &str {
        "file_info"
    }

    fn description(&self) -> &str {
        "获取文件的元信息：大小、修改时间、类型等"
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "name": "file_info",
            "description": self.description(),
            "parameters": {
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "文件路径"}
                },
                "required": ["path"]
            }
        })
    }

    async fn execute(&self, args: serde_json::Value) -> Result<ToolResult> {
        let path = args["path"].as_str().unwrap_or("");
        
        match tokio::fs::metadata(path).await {
            Ok(metadata) => {
                let file_type = if metadata.is_dir() { "directory" } else { "file" };
                let size = metadata.len();
                let modified = metadata.modified()
                    .ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                
                Ok(ToolResult::success(serde_json::json!({
                    "path": path,
                    "type": file_type,
                    "size": size,
                    "size_human": format!("{:.2} KB", size as f64 / 1024.0),
                    "modified_timestamp": modified,
                    "is_dir": metadata.is_dir(),
                    "is_file": metadata.is_file()
                })))
            }
            Err(e) => Ok(ToolResult::error(format!("获取文件信息失败: {}", e))),
        }
    }
}

/// 创建目录工具
pub struct MkdirTool;

#[async_trait]
impl Tool for MkdirTool {
    fn name(&self) -> &str {
        "mkdir"
    }

    fn description(&self) -> &str {
        "创建目录（支持递归创建）"
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "name": "mkdir",
            "description": self.description(),
            "parameters": {
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "要创建的目录路径"},
                    "recursive": {"type": "boolean", "description": "是否递归创建父目录", "default": true}
                },
                "required": ["path"]
            }
        })
    }

    async fn execute(&self, args: serde_json::Value) -> Result<ToolResult> {
        let path = args["path"].as_str().unwrap_or("");
        
        match tokio::fs::create_dir_all(path).await {
            Ok(_) => Ok(ToolResult::success(serde_json::json!({
                "message": format!("目录创建成功: {}", path)
            }))),
            Err(e) => Ok(ToolResult::error(format!("目录创建失败: {}", e))),
        }
    }
}

/// 删除文件/目录工具
pub struct RemoveTool;

#[async_trait]
impl Tool for RemoveTool {
    fn name(&self) -> &str {
        "remove"
    }

    fn description(&self) -> &str {
        "删除文件或目录（带安全检查）"
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "name": "remove",
            "description": self.description(),
            "parameters": {
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "要删除的路径"},
                    "recursive": {"type": "boolean", "description": "是否递归删除目录", "default": false}
                },
                "required": ["path"]
            }
        })
    }

    async fn execute(&self, args: serde_json::Value) -> Result<ToolResult> {
        let path = args["path"].as_str().unwrap_or("");
        let recursive = args["recursive"].as_bool().unwrap_or(false);
        
        // 安全检查
        if path == "/" || path == "." || path.contains("..") {
            return Ok(ToolResult::error("禁止删除根目录、当前目录或包含..的路径".to_string()));
        }
        
        let path_obj = std::path::Path::new(path);
        
        if !path_obj.exists() {
            return Ok(ToolResult::error(format!("路径不存在: {}", path)));
        }
        
        if path_obj.is_dir() && !recursive {
            return Ok(ToolResult::error(format!("路径是目录，需要设置recursive=true: {}", path)));
        }
        
        match if path_obj.is_dir() {
            tokio::fs::remove_dir_all(path).await
        } else {
            tokio::fs::remove_file(path).await
        } {
            Ok(_) => Ok(ToolResult::success(serde_json::json!({
                "message": format!("删除成功: {}", path)
            }))),
            Err(e) => Ok(ToolResult::error(format!("删除失败: {}", e))),
        }
    }
}

/// 复制文件工具
pub struct CopyTool;

#[async_trait]
impl Tool for CopyTool {
    fn name(&self) -> &str {
        "copy"
    }

    fn description(&self) -> &str {
        "复制文件或目录"
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "name": "copy",
            "description": self.description(),
            "parameters": {
                "type": "object",
                "properties": {
                    "from": {"type": "string", "description": "源路径"},
                    "to": {"type": "string", "description": "目标路径"}
                },
                "required": ["from", "to"]
            }
        })
    }

    async fn execute(&self, args: serde_json::Value) -> Result<ToolResult> {
        let from = args["from"].as_str().unwrap_or("");
        let to = args["to"].as_str().unwrap_or("");
        
        match tokio::fs::copy(from, to).await {
            Ok(bytes) => Ok(ToolResult::success(serde_json::json!({
                "message": format!("复制成功: {} -> {}", from, to),
                "bytes_copied": bytes
            }))),
            Err(e) => Ok(ToolResult::error(format!("复制失败: {}", e))),
        }
    }
}

/// 移动/重命名工具
pub struct MoveTool;

#[async_trait]
impl Tool for MoveTool {
    fn name(&self) -> &str {
        "move"
    }

    fn description(&self) -> &str {
        "移动或重命名文件/目录"
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "name": "move",
            "description": self.description(),
            "parameters": {
                "type": "object",
                "properties": {
                    "from": {"type": "string", "description": "源路径"},
                    "to": {"type": "string", "description": "目标路径"}
                },
                "required": ["from", "to"]
            }
        })
    }

    async fn execute(&self, args: serde_json::Value) -> Result<ToolResult> {
        let from = args["from"].as_str().unwrap_or("");
        let to = args["to"].as_str().unwrap_or("");
        
        match tokio::fs::rename(from, to).await {
            Ok(_) => Ok(ToolResult::success(serde_json::json!({
                "message": format!("移动成功: {} -> {}", from, to)
            }))),
            Err(e) => Ok(ToolResult::error(format!("移动失败: {}", e))),
        }
    }
}

/// 环境变量工具
pub struct EnvTool;

#[async_trait]
impl Tool for EnvTool {
    fn name(&self) -> &str {
        "env"
    }

    fn description(&self) -> &str {
        "获取、设置环境变量"
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "name": "env",
            "description": self.description(),
            "parameters": {
                "type": "object",
                "properties": {
                    "action": {"type": "string", "description": "操作类型", "enum": ["get", "set", "list"]},
                    "name": {"type": "string", "description": "环境变量名"},
                    "value": {"type": "string", "description": "环境变量值（仅set操作需要）"}
                },
                "required": ["action"]
            }
        })
    }

    async fn execute(&self, args: serde_json::Value) -> Result<ToolResult> {
        let action = args["action"].as_str().unwrap_or("list");
        
        match action {
            "get" => {
                let name = args["name"].as_str().unwrap_or("");
                if name.is_empty() {
                    return Ok(ToolResult::error("环境变量名不能为空".to_string()));
                }
                match std::env::var(name) {
                    Ok(value) => Ok(ToolResult::success(serde_json::json!({"name": name, "value": value}))),
                    Err(_) => Ok(ToolResult::error(format!("环境变量 '{}' 不存在", name))),
                }
            }
            "set" => {
                let name = args["name"].as_str().unwrap_or("");
                let value = args["value"].as_str().unwrap_or("");
                if name.is_empty() {
                    return Ok(ToolResult::error("环境变量名不能为空".to_string()));
                }
                std::env::set_var(name, value);
                Ok(ToolResult::success(serde_json::json!({
                    "message": format!("环境变量设置成功: {}={}", name, value)
                })))
            }
            "list" => {
                let vars: HashMap<String, String> = std::env::vars().collect();
                Ok(ToolResult::success(serde_json::json!({
                    "count": vars.len(),
                    "variables": vars
                })))
            }
            _ => Ok(ToolResult::error(format!("未知操作: {}", action))),
        }
    }
}

/// 系统信息工具
pub struct SystemInfoTool;

#[async_trait]
impl Tool for SystemInfoTool {
    fn name(&self) -> &str {
        "system_info"
    }

    fn description(&self) -> &str {
        "获取系统信息：操作系统、架构、CPU数量、主机名等"
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "name": "system_info",
            "description": self.description(),
            "parameters": {
                "type": "object",
                "properties": {},
                "required": []
            }
        })
    }

    async fn execute(&self, _args: serde_json::Value) -> Result<ToolResult> {
        let os = std::env::consts::OS;
        let arch = std::env::consts::ARCH;
        let cpus = num_cpus::get();
        let hostname = hostname::get()
            .ok()
            .and_then(|h| h.into_string().ok())
            .unwrap_or_else(|| "unknown".to_string());
        
        Ok(ToolResult::success(serde_json::json!({
            "os": os,
            "arch": arch,
            "cpus": cpus,
            "hostname": hostname,
            "current_dir": std::env::current_dir()
                .ok()
                .and_then(|p| p.into_os_string().into_string().ok())
                .unwrap_or_else(|| "unknown".to_string())
        })))
    }
}

/// 计算工具
pub struct CalculatorTool;

#[async_trait]
impl Tool for CalculatorTool {
    fn name(&self) -> &str {
        "calculator"
    }

    fn description(&self) -> &str {
        "执行数学计算：加减乘除、幂运算、取模等"
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "name": "calculator",
            "description": self.description(),
            "parameters": {
                "type": "object",
                "properties": {
                    "operation": {"type": "string", "description": "运算类型", "enum": ["add", "sub", "mul", "div", "mod", "pow"]},
                    "a": {"type": "number", "description": "第一个操作数"},
                    "b": {"type": "number", "description": "第二个操作数"}
                },
                "required": ["operation", "a", "b"]
            }
        })
    }

    async fn execute(&self, args: serde_json::Value) -> Result<ToolResult> {
        let operation = args["operation"].as_str().unwrap_or("");
        let a = args["a"].as_f64().unwrap_or(0.0);
        let b = args["b"].as_f64().unwrap_or(0.0);
        
        let result = match operation {
            "add" => a + b,
            "sub" => a - b,
            "mul" => a * b,
            "div" => {
                if b == 0.0 {
                    return Ok(ToolResult::error("除数不能为零".to_string()));
                }
                a / b
            }
            "mod" => {
                if b == 0.0 {
                    return Ok(ToolResult::error("取模时除数不能为零".to_string()));
                }
                a % b
            }
            "pow" => a.powf(b),
            _ => return Ok(ToolResult::error(format!("未知运算: {}", operation))),
        };
        
        Ok(ToolResult::success(serde_json::json!({
            "operation": format!("{} {} {}", a, operation, b),
            "result": result
        })))
    }
}

/// Base64编解码工具
pub struct Base64Tool;

#[async_trait]
impl Tool for Base64Tool {
    fn name(&self) -> &str {
        "base64"
    }

    fn description(&self) -> &str {
        "Base64编码和解码"
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "name": "base64",
            "description": self.description(),
            "parameters": {
                "type": "object",
                "properties": {
                    "action": {"type": "string", "description": "操作类型", "enum": ["encode", "decode"]},
                    "data": {"type": "string", "description": "要编码或解码的数据"}
                },
                "required": ["action", "data"]
            }
        })
    }

    async fn execute(&self, args: serde_json::Value) -> Result<ToolResult> {
        use base64::{Engine as _, engine::general_purpose::STANDARD};
        
        let action = args["action"].as_str().unwrap_or("");
        let data = args["data"].as_str().unwrap_or("");
        
        match action {
            "encode" => {
                let encoded = STANDARD.encode(data);
                Ok(ToolResult::success(serde_json::json!({"encoded": encoded})))
            }
            "decode" => {
                match STANDARD.decode(data) {
                    Ok(decoded) => {
                        match String::from_utf8(decoded) {
                            Ok(s) => Ok(ToolResult::success(serde_json::json!({"decoded": s}))),
                            Err(_) => Ok(ToolResult::error("解码结果不是有效的UTF-8字符串".to_string())),
                        }
                    }
                    Err(e) => Ok(ToolResult::error(format!("Base64解码失败: {}", e))),
                }
            }
            _ => Ok(ToolResult::error(format!("未知操作: {}", action))),
        }
    }
}

/// 哈希计算工具
pub struct HashTool;

#[async_trait]
impl Tool for HashTool {
    fn name(&self) -> &str {
        "hash"
    }

    fn description(&self) -> &str {
        "计算数据的哈希值：MD5, SHA256等"
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "name": "hash",
            "description": self.description(),
            "parameters": {
                "type": "object",
                "properties": {
                    "algorithm": {"type": "string", "description": "哈希算法", "enum": ["sha256", "md5"]},
                    "data": {"type": "string", "description": "要计算哈希的数据"},
                    "file_path": {"type": "string", "description": "要计算哈希的文件路径（可选）"}
                },
                "required": ["algorithm"]
            }
        })
    }

    async fn execute(&self, args: serde_json::Value) -> Result<ToolResult> {
        use sha2::{Digest, Sha256};
        use md5::{Md5, Digest as Md5Digest};
        
        let algorithm = args["algorithm"].as_str().unwrap_or("sha256");
        
        let data = if let Some(file_path) = args.get("file_path").and_then(|v| v.as_str()) {
            tokio::fs::read(file_path).await?
        } else {
            args["data"].as_str().unwrap_or("").as_bytes().to_vec()
        };
        
        let hash = match algorithm {
            "sha256" => {
                let mut hasher = Sha256::new();
                hasher.update(&data);
                format!("{:x}", hasher.finalize())
            }
            "md5" => {
                let mut hasher = Md5::new();
                hasher.update(&data);
                format!("{:x}", hasher.finalize())
            }
            _ => return Ok(ToolResult::error(format!("不支持的哈希算法: {}", algorithm))),
        };
        
        Ok(ToolResult::success(serde_json::json!({
            "algorithm": algorithm,
            "hash": hash,
            "data_length": data.len()
        })))
    }
}

/// 正则表达式工具
pub struct RegexTool;

#[async_trait]
impl Tool for RegexTool {
    fn name(&self) -> &str {
        "regex"
    }

    fn description(&self) -> &str {
        "执行正则表达式匹配、搜索和替换"
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "name": "regex",
            "description": self.description(),
            "parameters": {
                "type": "object",
                "properties": {
                    "action": {"type": "string", "description": "操作类型", "enum": ["match", "search", "replace", "find_all"]},
                    "pattern": {"type": "string", "description": "正则表达式模式"},
                    "text": {"type": "string", "description": "要处理的文本"},
                    "replacement": {"type": "string", "description": "替换文本（仅replace操作需要）"}
                },
                "required": ["action", "pattern", "text"]
            }
        })
    }

    async fn execute(&self, args: serde_json::Value) -> Result<ToolResult> {
        let action = args["action"].as_str().unwrap_or("");
        let pattern = args["pattern"].as_str().unwrap_or("");
        let text = args["text"].as_str().unwrap_or("");
        
        match regex::Regex::new(pattern) {
            Ok(re) => {
                match action {
                    "match" => {
                        let is_match = re.is_match(text);
                        Ok(ToolResult::success(serde_json::json!({"match": is_match})))
                    }
                    "search" => {
                        if let Some(m) = re.find(text) {
                            Ok(ToolResult::success(serde_json::json!({
                                "found": true,
                                "match": m.as_str(),
                                "start": m.start(),
                                "end": m.end()
                            })))
                        } else {
                            Ok(ToolResult::success(serde_json::json!({"found": false})))
                        }
                    }
                    "replace" => {
                        let replacement = args["replacement"].as_str().unwrap_or("");
                        let result = re.replace_all(text, replacement);
                        Ok(ToolResult::success(serde_json::json!({"result": result.to_string()})))
                    }
                    "find_all" => {
                        let matches: Vec<String> = re.find_iter(text)
                            .map(|m| m.as_str().to_string())
                            .collect();
                        Ok(ToolResult::success(serde_json::json!({
                            "count": matches.len(),
                            "matches": matches
                        })))
                    }
                    _ => Ok(ToolResult::error(format!("未知操作: {}", action))),
                }
            }
            Err(e) => Ok(ToolResult::error(format!("正则表达式编译失败: {}", e))),
        }
    }
}

/// 浏览器导航工具
pub struct BrowserNavigateTool;

#[async_trait]
impl Tool for BrowserNavigateTool {
    fn name(&self) -> &str {
        "browser_navigate"
    }

    fn description(&self) -> &str {
        "导航到指定URL并在浏览器中打开"
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "name": "browser_navigate",
            "description": self.description(),
            "parameters": {
                "type": "object",
                "properties": {
                    "url": {
                        "type": "string",
                        "description": "要导航到的URL"
                    }
                },
                "required": ["url"]
            }
        })
    }

    async fn execute(&self, args: serde_json::Value) -> Result<ToolResult> {
        use headless_chrome::Browser;
        
        let url = match args["url"].as_str() {
            Some(url) => url,
            None => return Ok(ToolResult::error("缺少url参数".to_string())),
        };

        match Browser::default() {
            Ok(browser) => {
                match browser.new_tab() {
                    Ok(tab) => {
                        match tab.navigate_to(url) {
                            Ok(_) => {
                                match tab.wait_until_navigated() {
                                    Ok(_) => {
                                        let title = tab.get_title().unwrap_or_else(|_| String::new());
                                        let current_url = tab.get_url();
                                        Ok(ToolResult::success(serde_json::json!({
                                            "success": true,
                                            "title": title,
                                            "url": current_url,
                                            "message": "页面导航成功"
                                        })))
                                    }
                                    Err(e) => Ok(ToolResult::error(format!("等待页面加载失败: {}", e)))
                                }
                            }
                            Err(e) => Ok(ToolResult::error(format!("导航失败: {}", e)))
                        }
                    }
                    Err(e) => Ok(ToolResult::error(format!("创建新标签页失败: {}", e)))
                }
            }
            Err(e) => Ok(ToolResult::error(format!("启动浏览器失败: {}", e)))
        }
    }
}

/// 浏览器截图工具
pub struct BrowserScreenshotTool;

#[async_trait]
impl Tool for BrowserScreenshotTool {
    fn name(&self) -> &str {
        "browser_screenshot"
    }

    fn description(&self) -> &str {
        "对当前浏览器窗口进行截图"
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "name": "browser_screenshot",
            "description": self.description(),
            "parameters": {
                "type": "object",
                "properties": {
                    "url": {
                        "type": "string",
                        "description": "要截图的页面URL"
                    },
                    "full_page": {
                        "type": "boolean",
                        "description": "是否截取完整页面",
                        "default": true
                    },
                    "save_path": {
                        "type": "string",
                        "description": "截图保存路径"
                    }
                },
                "required": ["url"]
            }
        })
    }

    async fn execute(&self, args: serde_json::Value) -> Result<ToolResult> {
        use headless_chrome::Browser;
        use std::path::Path;
        
        let url = match args["url"].as_str() {
            Some(url) => url,
            None => return Ok(ToolResult::error("缺少url参数".to_string())),
        };

        let full_page = args["full_page"].as_bool().unwrap_or(true);
        let save_path = args["save_path"].as_str().unwrap_or("screenshot.png");

        match Browser::default() {
            Ok(browser) => {
                match browser.new_tab() {
                    Ok(tab) => {
                        if let Err(e) = tab.navigate_to(url) {
                            return Ok(ToolResult::error(format!("导航失败: {}", e)));
                        }

                        if let Err(e) = tab.wait_until_navigated() {
                            return Ok(ToolResult::error(format!("等待页面加载失败: {}", e)));
                        }

                        let png_data = match tab.capture_screenshot(headless_chrome::protocol::cdp::Page::CaptureScreenshotFormatOption::Png, None, None, true) {
                            Ok(data) => data,
                            Err(e) => return Ok(ToolResult::error(format!("截图失败: {}", e)))
                        };

                        let path = Path::new(save_path);
                        if let Some(parent) = path.parent() {
                            if !parent.exists() {
                                if let Err(e) = std::fs::create_dir_all(parent) {
                                    return Ok(ToolResult::error(format!("创建目录失败: {}", e)));
                                }
                            }
                        }

                        match std::fs::write(path, png_data) {
                            Ok(_) => Ok(ToolResult::success(serde_json::json!({
                                "success": true,
                                "path": save_path,
                                "message": "截图已保存"
                            }))),
                            Err(e) => Ok(ToolResult::error(format!("保存截图失败: {}", e)))
                        }
                    }
                    Err(e) => Ok(ToolResult::error(format!("创建新标签页失败: {}", e)))
                }
            }
            Err(e) => Ok(ToolResult::error(format!("启动浏览器失败: {}", e)))
        }
    }
}

/// 浏览器点击工具
pub struct BrowserClickTool;

#[async_trait]
impl Tool for BrowserClickTool {
    fn name(&self) -> &str {
        "browser_click"
    }

    fn description(&self) -> &str {
        "点击页面上的元素（通过CSS选择器或XPath）"
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "name": "browser_click",
            "description": self.description(),
            "parameters": {
                "type": "object",
                "properties": {
                    "url": {
                        "type": "string",
                        "description": "页面URL"
                    },
                    "selector": {
                        "type": "string",
                        "description": "CSS选择器"
                    }
                },
                "required": ["url", "selector"]
            }
        })
    }

    async fn execute(&self, args: serde_json::Value) -> Result<ToolResult> {
        use headless_chrome::Browser;
        
        let url = match args["url"].as_str() {
            Some(url) => url,
            None => return Ok(ToolResult::error("缺少url参数".to_string())),
        };

        let selector = match args["selector"].as_str() {
            Some(selector) => selector,
            None => return Ok(ToolResult::error("缺少selector参数".to_string())),
        };

        match Browser::default() {
            Ok(browser) => {
                match browser.new_tab() {
                    Ok(tab) => {
                        if let Err(e) = tab.navigate_to(url) {
                            return Ok(ToolResult::error(format!("导航失败: {}", e)));
                        }

                        if let Err(e) = tab.wait_until_navigated() {
                            return Ok(ToolResult::error(format!("等待页面加载失败: {}", e)));
                        }

                        match tab.wait_for_element(selector) {
                            Ok(element) => {
                                match element.click() {
                                    Ok(_) => Ok(ToolResult::success(serde_json::json!({
                                        "success": true,
                                        "selector": selector,
                                        "message": "元素点击成功"
                                    }))),
                                    Err(e) => Ok(ToolResult::error(format!("点击元素失败: {}", e)))
                                }
                            }
                            Err(e) => Ok(ToolResult::error(format!("查找元素失败: {}", e)))
                        }
                    }
                    Err(e) => Ok(ToolResult::error(format!("创建新标签页失败: {}", e)))
                }
            }
            Err(e) => Ok(ToolResult::error(format!("启动浏览器失败: {}", e)))
        }
    }
}

/// 浏览器填写表单工具
pub struct BrowserFillFormTool;

#[async_trait]
impl Tool for BrowserFillFormTool {
    fn name(&self) -> &str {
        "browser_fill_form"
    }

    fn description(&self) -> &str {
        "填写页面上的表单元素"
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "name": "browser_fill_form",
            "description": self.description(),
            "parameters": {
                "type": "object",
                "properties": {
                    "url": {
                        "type": "string",
                        "description": "页面URL"
                    },
                    "fields": {
                        "type": "object",
                        "description": "要填写的字段（选择器 -> 值）",
                        "additionalProperties": {
                            "type": "string"
                        }
                    }
                },
                "required": ["url", "fields"]
            }
        })
    }

    async fn execute(&self, args: serde_json::Value) -> Result<ToolResult> {
        use headless_chrome::Browser;
        
        let url = match args["url"].as_str() {
            Some(url) => url,
            None => return Ok(ToolResult::error("缺少url参数".to_string())),
        };

        let fields = match args["fields"].as_object() {
            Some(fields) => fields,
            None => return Ok(ToolResult::error("缺少fields参数".to_string())),
        };

        match Browser::default() {
            Ok(browser) => {
                match browser.new_tab() {
                    Ok(tab) => {
                        if let Err(e) = tab.navigate_to(url) {
                            return Ok(ToolResult::error(format!("导航失败: {}", e)));
                        }

                        if let Err(e) = tab.wait_until_navigated() {
                            return Ok(ToolResult::error(format!("等待页面加载失败: {}", e)));
                        }

                        let mut results = Vec::new();
                        for (selector, value) in fields {
                            match tab.wait_for_element(selector) {
                                Ok(element) => {
                                    if let Err(e) = element.click() {
                                        results.push(format!("点击{}失败: {}", selector, e));
                                        continue;
                                    }

                                    let value_str = value.as_str().unwrap_or("");
                                    let js = format!("document.querySelector('{}').value = '{}';", selector, value_str);
                                    if let Err(e) = tab.evaluate(&js, false) {
                                        results.push(format!("填写{}失败: {}", selector, e));
                                    } else {
                                        results.push(format!("{}已填写", selector));
                                    }
                                }
                                Err(e) => results.push(format!("查找{}失败: {}", selector, e))
                            }
                        }

                        Ok(ToolResult::success(serde_json::json!({
                            "success": true,
                            "results": results,
                            "message": "表单填写完成"
                        })))
                    }
                    Err(e) => Ok(ToolResult::error(format!("创建新标签页失败: {}", e)))
                }
            }
            Err(e) => Ok(ToolResult::error(format!("启动浏览器失败: {}", e)))
        }
    }
}

/// 浏览器获取页面内容工具
pub struct BrowserGetContentTool;

#[async_trait]
impl Tool for BrowserGetContentTool {
    fn name(&self) -> &str {
        "browser_get_content"
    }

    fn description(&self) -> &str {
        "获取当前页面的HTML内容"
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "name": "browser_get_content",
            "description": self.description(),
            "parameters": {
                "type": "object",
                "properties": {
                    "url": {
                        "type": "string",
                        "description": "页面URL"
                    }
                },
                "required": ["url"]
            }
        })
    }

    async fn execute(&self, args: serde_json::Value) -> Result<ToolResult> {
        use headless_chrome::Browser;
        
        let url = match args["url"].as_str() {
            Some(url) => url,
            None => return Ok(ToolResult::error("缺少url参数".to_string())),
        };

        match Browser::default() {
            Ok(browser) => {
                match browser.new_tab() {
                    Ok(tab) => {
                        if let Err(e) = tab.navigate_to(url) {
                            return Ok(ToolResult::error(format!("导航失败: {}", e)));
                        }

                        if let Err(e) = tab.wait_until_navigated() {
                            return Ok(ToolResult::error(format!("等待页面加载失败: {}", e)));
                        }

                        match tab.get_content() {
                            Ok(content) => {
                                let title = tab.get_title().unwrap_or_default();
                                Ok(ToolResult::success(serde_json::json!({
                                    "success": true,
                                    "title": title,
                                    "url": url,
                                    "content_length": content.len(),
                                    "content": content.chars().take(10000).collect::<String>()
                                })))
                            }
                            Err(e) => Ok(ToolResult::error(format!("获取内容失败: {}", e)))
                        }
                    }
                    Err(e) => Ok(ToolResult::error(format!("创建新标签页失败: {}", e)))
                }
            }
            Err(e) => Ok(ToolResult::error(format!("启动浏览器失败: {}", e)))
        }
    }
}

/// 浏览器执行JavaScript工具
pub struct BrowserEvaluateJsTool;

#[async_trait]
impl Tool for BrowserEvaluateJsTool {
    fn name(&self) -> &str {
        "browser_evaluate_js"
    }

    fn description(&self) -> &str {
        "在页面上执行JavaScript代码"
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "name": "browser_evaluate_js",
            "description": self.description(),
            "parameters": {
                "type": "object",
                "properties": {
                    "url": {
                        "type": "string",
                        "description": "页面URL"
                    },
                    "script": {
                        "type": "string",
                        "description": "要执行的JavaScript代码"
                    }
                },
                "required": ["url", "script"]
            }
        })
    }

    async fn execute(&self, args: serde_json::Value) -> Result<ToolResult> {
        use headless_chrome::Browser;
        
        let url = match args["url"].as_str() {
            Some(url) => url,
            None => return Ok(ToolResult::error("缺少url参数".to_string())),
        };

        let script = match args["script"].as_str() {
            Some(script) => script,
            None => return Ok(ToolResult::error("缺少script参数".to_string())),
        };

        match Browser::default() {
            Ok(browser) => {
                match browser.new_tab() {
                    Ok(tab) => {
                        if let Err(e) = tab.navigate_to(url) {
                            return Ok(ToolResult::error(format!("导航失败: {}", e)));
                        }

                        if let Err(e) = tab.wait_until_navigated() {
                            return Ok(ToolResult::error(format!("等待页面加载失败: {}", e)));
                        }

                        match tab.evaluate(script, false) {
                            Ok(result) => {
                                let result_str = format!("{:?}", result);
                                Ok(ToolResult::success(serde_json::json!({
                                    "success": true,
                                    "result": result_str.chars().take(5000).collect::<String>()
                                })))
                            }
                            Err(e) => Ok(ToolResult::error(format!("执行JS失败: {}", e)))
                        }
                    }
                    Err(e) => Ok(ToolResult::error(format!("创建新标签页失败: {}", e)))
                }
            }
            Err(e) => Ok(ToolResult::error(format!("启动浏览器失败: {}", e)))
        }
    }
}
