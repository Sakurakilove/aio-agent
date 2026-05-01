mod permissions;

pub use permissions::{SkillPermission, SkillPermissionManager};

use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tracing::{info, warn};

const SKILLS_DIR: &str = "~/.aio-agent/skills";
const HERMES_SKILL_FILE: &str = "SKILL.md";
const SKILL_NAME_PATTERN: &str = r"---\nname: (.+?)\n---";
const MAX_SKILL_NAME_LEN: usize = 64;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillMetadata {
    pub name: String,
    pub description: String,
    pub version: String,
    pub author: String,
    #[serde(default)]
    pub license: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub dependencies: Vec<String>,
    #[serde(default)]
    pub metadata: Option<MetadataSection>,
    #[serde(default)]
    pub hermes: Option<HermesMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataSection {
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HermesMetadata {
    #[serde(rename = "requires")]
    #[serde(default)]
    pub requires_tools: Vec<String>,
    #[serde(rename = "triggers")]
    #[serde(default)]
    pub trigger_patterns: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillEntry {
    pub path: PathBuf,
    pub metadata: SkillMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillStatus {
    pub total_skills: usize,
    pub categories: Vec<String>,
}

pub struct SkillManager {
    skills_cache: Vec<SkillEntry>,
    pub permission_manager: SkillPermissionManager,
}

impl SkillManager {
    pub fn new() -> Result<Self> {
        let skills_dir = Self::get_skills_dir()?;
        if !skills_dir.exists() {
            fs::create_dir_all(&skills_dir)?;
        }

        let mut manager = Self {
            skills_cache: Vec::new(),
            permission_manager: SkillPermissionManager::new(),
        };
        manager.load_skills()?;
        Ok(manager)
    }

    pub fn list_skills(&self) -> &[SkillEntry] {
        &self.skills_cache
    }

    pub fn search_skills(&self, query: &str) -> Vec<&SkillEntry> {
        let query_lower = query.to_lowercase();
        self.skills_cache
            .iter()
            .filter(|skill| {
                skill.metadata.name.to_lowercase().contains(&query_lower)
                    || skill.metadata.description.to_lowercase().contains(&query_lower)
                    || skill.metadata.tags.iter().any(|tag| tag.to_lowercase().contains(&query_lower))
            })
            .collect()
    }

    pub fn auto_generate_skill(
        &self,
        skill_name: &str,
        required_tools: &[String],
        category: &str,
    ) -> Result<PathBuf> {
        let skill_path = Self::get_skills_dir()?;
        let skill_dir = skill_path.join(Self::sanitize_skill_name(skill_name));

        if skill_dir.exists() {
            return Err(anyhow::anyhow!("Skill already exists: {}", skill_name));
        }
        fs::create_dir_all(&skill_dir)?;

        let skill_content = Self::generate_skill_md(skill_name, required_tools);

        let skill_file = skill_dir.join(HERMES_SKILL_FILE);
        fs::write(&skill_file, skill_content)?;

        info!("Auto-generated skill: {} at {}", skill_name, skill_file.display());
        Ok(skill_file)
    }

    pub fn create_skill(
        &self,
        name: &str,
        description: &str,
        content: &str,
        category: &str,
        tags: Option<Vec<String>>,
    ) -> Result<PathBuf> {
        let skill_path = Self::get_skills_dir()?;
        let skill_dir = skill_path.join(Self::sanitize_skill_name(name));

        if skill_dir.exists() {
            return Err(anyhow::anyhow!("Skill already exists: {}", name));
        }
        fs::create_dir_all(&skill_dir)?;

        let tags_list = tags
            .map(|t| t
                .iter()
                .map(|tag| format!("  - {}", tag))
                .collect::<Vec<_>>()
                .join("\n"))
            .unwrap_or_default();

        let metadata = format!(
            "---
name: {}
description: {}
version: 1.0.0
author: AIO Agent
license: MIT
metadata:
  tags:
{}
dependencies: []
---
",
            name,
            description,
            if tags_list.is_empty() { "".to_string() } else { format!("\n{}", tags_list) },
        );

        let full_content = format!("{}{}", metadata, content);
        let skill_file = skill_dir.join(HERMES_SKILL_FILE);
        fs::write(&skill_file, full_content)?;

        info!("Created skill: {} at {}", name, skill_file.display());
        Ok(skill_file)
    }

    pub fn get_status(&self) -> SkillStatus {
        let categories: Vec<String> = self.skills_cache
            .iter()
            .map(|s| s.metadata.tags.first().cloned().unwrap_or_else(|| "uncategorized".to_string()))
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        SkillStatus {
            total_skills: self.skills_cache.len(),
            categories,
        }
    }

    pub fn load_skills(&mut self) -> Result<()> {
        let skills_dir = Self::get_skills_dir()?;
        self.skills_cache.clear();

        if !skills_dir.exists() {
            return Ok(());
        }

        for entry in walkdir::WalkDir::new(&skills_dir).max_depth(3).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();

            if path.file_name().map(|n| n == HERMES_SKILL_FILE).unwrap_or(false) {
                info!("Found skill file: {}", path.display());
                match Self::parse_skill_metadata(path) {
                    Ok(metadata) => {
                        let skill_name = metadata.name.clone();
                        let skill_entry = SkillEntry {
                            path: path.to_path_buf(),
                            metadata,
                        };
                        self.skills_cache.push(skill_entry);
                        info!("Loaded skill: {}", skill_name);
                    }
                    Err(e) => {
                        warn!("Failed to parse skill at {}: {}", path.display(), e);
                    }
                }
            }
        }

        info!("Loaded {} skills total", self.skills_cache.len());
        Ok(())
    }

    fn get_skills_dir() -> Result<PathBuf> {
        let home = dirs_next::home_dir()
            .context("Could not determine home directory")?;
        Ok(home.join(".aio-agent").join("skills"))
    }

    fn sanitize_skill_name(name: &str) -> String {
        let sanitized: String = name
            .chars()
            .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '-' })
            .collect();

        let sanitized = sanitized.trim_matches('-');
        if sanitized.len() > MAX_SKILL_NAME_LEN {
            sanitized[..MAX_SKILL_NAME_LEN].to_string()
        } else {
            sanitized.to_string()
        }
    }

    fn parse_skill_metadata(skill_file: &Path) -> Result<SkillMetadata> {
        let content = fs::read_to_string(skill_file)?;

        if !content.starts_with("---") {
            return Err(anyhow::anyhow!("Invalid skill file: must start with ---"));
        }

        let yaml_start = 3;
        let rest = &content[yaml_start..];
        let yaml_end = rest.find("\n---").ok_or_else(|| {
            anyhow::anyhow!("Invalid skill file: missing closing ---")
        })?;

        let yaml_content = &rest[..yaml_end].trim();

        let metadata: SkillMetadata = serde_yaml::from_str(yaml_content)
            .with_context(|| format!("Failed to parse skill metadata from {}", skill_file.display()))?;

        Ok(metadata)
    }

    fn generate_skill_md(skill_name: &str, required_tools: &[String]) -> String {
        let tools_list = required_tools
            .iter()
            .map(|tool| format!("  - {}", tool))
            .collect::<Vec<_>>()
            .join("\n");

        format!(
            "---
name: {}
description: Auto-generated skill for {}
version: 1.0.0
author: AIO Agent
license: MIT
metadata:
  tags:
  - auto-generated
dependencies: []

hermes:
  requires:
{}
  triggers:
  - \"{}\"
---

# {}

## Description
Auto-generated skill for {} using tools: {}

## Instructions
This skill was auto-generated. Please review and customize as needed.

## Examples
Example 1:
  Input: [example input]
  Output: [expected output]

## Notes
- This skill was auto-generated and may need manual adjustment
",
            skill_name,
            skill_name,
            tools_list,
            skill_name,
            skill_name,
            skill_name,
            required_tools.join(", "),
        )
    }
}
