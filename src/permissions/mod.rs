use regex::Regex;
use std::collections::HashMap;

pub struct PermissionChecker {
    allow_rules: Vec<String>,
    deny_rules: Vec<String>,
}

impl PermissionChecker {
    pub fn new(allow_rules: Vec<String>, deny_rules: Vec<String>) -> Self {
        Self {
            allow_rules,
            deny_rules,
        }
    }

    pub fn check(&self, action: &str, resource: &str) -> bool {
        for pattern in &self.deny_rules {
            if Self::matches(pattern, action, resource) {
                return false;
            }
        }

        for pattern in &self.allow_rules {
            if Self::matches(pattern, action, resource) {
                return true;
            }
        }

        false
    }

    fn matches(pattern: &str, action: &str, resource: &str) -> bool {
        let regex_pattern = pattern
            .replace("**", "\x00DOUBLE_STAR\x00")
            .replace(".", "\\.")
            .replace("(", "\\(")
            .replace(")", "\\)")
            .replace("+", "\\+")
            .replace("[", "\\[")
            .replace("]", "\\]")
            .replace("{", "\\{")
            .replace("}", "\\}")
            .replace("^", "\\^")
            .replace("$", "\\$")
            .replace("?", "\\.")
            .replace("\x00DOUBLE_STAR\x00", ".*")
            .replace("*", "[^/]*");
        
        if let Ok(re) = Regex::new(&regex_pattern) {
            return re.is_match(action) || re.is_match(resource);
        }
        pattern.contains(action) || pattern.contains(resource)
    }
}
