//! # Agent Execution Chain (AEC)
//!
//! 严格遵循 VERIDACTUS v0.2.1 §1.6.1 Agent Execution Chain。
//! 实现多代理执行链的追踪和验证。

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentExecutionChain {
    pub chain_id: String,
    pub root_trace_id: String,
    pub entries: Vec<ChainEntry>,
    pub chain_state: ChainState,
    pub created_at: String,
    pub completed_at: Option<String>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainEntry {
    pub entry_id: String,
    pub trace_id: String,
    pub parent_entry_id: Option<String>,
    pub agent_id: String,
    pub agent_role: AgentRole,
    pub step_number: u32,
    pub execution_type: ExecutionType,
    pub tool_calls: Vec<ToolCallRecord>,
    pub timestamp: String,
    pub status: ExecutionStatus,
    pub output_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AgentRole {
    Initiator,
    Planner,
    Executor,
    Reviewer,
    Summarizer,
    Tool,
    Orchestrator,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionType {
    Planning,
    ToolExecution,
    Reasoning,
    Summarization,
    Review,
    Delegation,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Retried,
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallRecord {
    pub tool_name: String,
    pub args_hash: String,
    pub result_hash: String,
    pub success: bool,
    pub latency_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ChainState {
    Running,
    Completed,
    Failed,
    Suspended,
}

pub struct AgentExecutionChainManager {
    chains: std::sync::RwLock<HashMap<String, AgentExecutionChain>>,
}

impl AgentExecutionChainManager {
    pub fn new() -> Self {
        Self {
            chains: std::sync::RwLock::new(HashMap::new()),
        }
    }

    pub fn create_chain(&self, root_trace_id: &str) -> AgentExecutionChain {
        let chain = AgentExecutionChain {
            chain_id: format!("chain_{}", uuid::Uuid::new_v4()),
            root_trace_id: root_trace_id.to_string(),
            entries: Vec::new(),
            chain_state: ChainState::Running,
            created_at: chrono::Utc::now().to_rfc3339(),
            completed_at: None,
            metadata: HashMap::new(),
        };

        let mut chains = self.chains.write().unwrap();
        chains.insert(chain.chain_id.clone(), chain.clone());
        chain
    }

    pub fn add_entry(&self, chain_id: &str, entry: ChainEntry) -> Result<(), ChainError> {
        let mut chains = self.chains.write().unwrap();
        let chain = chains.get_mut(chain_id).ok_or(ChainError::NotFound)?;
        
        chain.entries.push(entry);
        Ok(())
    }

    pub fn get_chain(&self, chain_id: &str) -> Option<AgentExecutionChain> {
        let chains = self.chains.read().unwrap();
        chains.get(chain_id).cloned()
    }

    pub fn complete_chain(&self, chain_id: &str) -> Result<(), ChainError> {
        let mut chains = self.chains.write().unwrap();
        let chain = chains.get_mut(chain_id).ok_or(ChainError::NotFound)?;
        
        chain.chain_state = ChainState::Completed;
        chain.completed_at = Some(chrono::Utc::now().to_rfc3339());
        Ok(())
    }

    pub fn validate_chain(&self, chain_id: &str) -> ChainValidationResult {
        let chains = self.chains.read().unwrap();
        let chain = match chains.get(chain_id) {
            Some(c) => c,
            None => return ChainValidationResult {
                valid: false,
                errors: vec!["Chain not found".to_string()],
                warnings: Vec::new(),
            },
        };

        let mut errors = Vec::new();
        let warnings = Vec::new();

        if chain.entries.is_empty() {
            errors.push("Chain has no entries".to_string());
        }

        let mut parent_ids = std::collections::HashSet::new();
        for entry in &chain.entries {
            if let Some(parent_id) = &entry.parent_entry_id {
                parent_ids.insert(parent_id.clone());
            }
        }

        for entry in &chain.entries {
            if !parent_ids.contains(&entry.entry_id) && entry.parent_entry_id.is_some() {
                errors.push(format!("Entry {} has invalid parent reference", entry.entry_id));
            }
        }

        let root_entries: Vec<_> = chain.entries.iter().filter(|e| e.parent_entry_id.is_none()).collect();
        if root_entries.len() != 1 {
            errors.push(format!("Expected exactly 1 root entry, found {}", root_entries.len()));
        }

        ChainValidationResult {
            valid: errors.is_empty(),
            errors,
            warnings,
        }
    }

    pub fn get_execution_path(&self, chain_id: &str) -> Option<Vec<ChainEntry>> {
        let chains = self.chains.read().unwrap();
        let chain = chains.get(chain_id)?;

        let mut path = Vec::new();
        let mut queue = VecDeque::new();
        
        let root_entries: Vec<_> = chain.entries.iter().filter(|e| e.parent_entry_id.is_none()).cloned().collect();
        if let Some(root) = root_entries.first() {
            queue.push_back(root.clone());
        }

        while let Some(entry) = queue.pop_front() {
            path.push(entry.clone());
            let children: Vec<_> = chain.entries
                .iter()
                .filter(|e| e.parent_entry_id == Some(entry.entry_id.clone()))
                .cloned()
                .collect();
            for child in children {
                queue.push_back(child);
            }
        }

        Some(path)
    }
}

#[derive(Debug)]
pub enum ChainError {
    NotFound,
    InvalidEntry,
    ChainAlreadyCompleted,
}

impl std::fmt::Display for ChainError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChainError::NotFound => write!(f, "Chain not found"),
            ChainError::InvalidEntry => write!(f, "Invalid chain entry"),
            ChainError::ChainAlreadyCompleted => write!(f, "Chain already completed"),
        }
    }
}

impl std::error::Error for ChainError {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainValidationResult {
    pub valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_and_validate_chain() {
        let manager = AgentExecutionChainManager::new();
        
        let chain = manager.create_chain("trace-123");
        
        let entry1 = ChainEntry {
            entry_id: "entry-1".to_string(),
            trace_id: "trace-123".to_string(),
            parent_entry_id: None,
            agent_id: "agent-1".to_string(),
            agent_role: AgentRole::Initiator,
            step_number: 1,
            execution_type: ExecutionType::Planning,
            tool_calls: Vec::new(),
            timestamp: "2026-05-17T10:00:00Z".to_string(),
            status: ExecutionStatus::Completed,
            output_hash: "sha256:abc".to_string(),
        };
        
        manager.add_entry(&chain.chain_id, entry1).unwrap();
        
        let result = manager.validate_chain(&chain.chain_id);
        assert!(result.valid);
    }

    #[test]
    fn test_chain_validation_failure() {
        let manager = AgentExecutionChainManager::new();
        let chain = manager.create_chain("trace-123");
        
        let result = manager.validate_chain(&chain.chain_id);
        assert!(!result.valid);
        assert!(result.errors.contains(&"Chain has no entries".to_string()));
    }
}