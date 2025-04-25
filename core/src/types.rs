use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct NodeStatus {
    pub online: bool,
    pub last_sync: Option<DateTime<Utc>>,
} 