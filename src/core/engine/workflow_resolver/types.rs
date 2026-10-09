use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct ResolvedWorkflow {
    pub path: PathBuf,
    pub data: serde_json::Value,
}

#[derive(Debug)]
pub enum WorkflowResolveError {
    BadRequest(String),
    NotFound(String),
    Internal(String),
}

impl std::fmt::Display for WorkflowResolveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BadRequest(msg) => write!(f, "Bad Request: {}", msg),
            Self::NotFound(msg) => write!(f, "Not Found: {}", msg),
            Self::Internal(msg) => write!(f, "Internal Error: {}", msg),
        }
    }
}

impl std::error::Error for WorkflowResolveError {}
