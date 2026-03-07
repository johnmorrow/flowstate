use chrono::{DateTime, Utc};
use nanoid::nanoid;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

const ID_ALPHABET: &[char] = &[
    'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's',
    't', 'u', 'v', 'w', 'x', 'y', 'z', '0', '1', '2', '3', '4', '5', '6', '7', '8', '9',
];

pub fn generate_task_id() -> String {
    format!("tk_{}", nanoid!(8, ID_ALPHABET))
}

pub fn generate_attachment_id() -> String {
    format!("at_{}", nanoid!(8, ID_ALPHABET))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Pending,
    InProgress,
    Done,
    Cancelled,
    Blocked,
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Status::Pending => write!(f, "pending"),
            Status::InProgress => write!(f, "in_progress"),
            Status::Done => write!(f, "done"),
            Status::Cancelled => write!(f, "cancelled"),
            Status::Blocked => write!(f, "blocked"),
        }
    }
}

impl FromStr for Status {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "pending" => Ok(Status::Pending),
            "in_progress" => Ok(Status::InProgress),
            "done" => Ok(Status::Done),
            "cancelled" => Ok(Status::Cancelled),
            "blocked" => Ok(Status::Blocked),
            _ => Err(format!("invalid status: {s}")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScheduleType {
    Once,
    Daily,
    Weekly,
    Recurring,
    Deadline,
}

impl fmt::Display for ScheduleType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ScheduleType::Once => write!(f, "once"),
            ScheduleType::Daily => write!(f, "daily"),
            ScheduleType::Weekly => write!(f, "weekly"),
            ScheduleType::Recurring => write!(f, "recurring"),
            ScheduleType::Deadline => write!(f, "deadline"),
        }
    }
}

impl FromStr for ScheduleType {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "once" => Ok(ScheduleType::Once),
            "daily" => Ok(ScheduleType::Daily),
            "weekly" => Ok(ScheduleType::Weekly),
            "recurring" => Ok(ScheduleType::Recurring),
            "deadline" => Ok(ScheduleType::Deadline),
            _ => Err(format!("invalid schedule type: {s}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub status: Status,
    pub schedule_type: ScheduleType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub due_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recur_rule: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default = "default_metadata", skip_serializing_if = "is_empty_object")]
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

fn default_metadata() -> serde_json::Value {
    serde_json::json!({})
}

fn is_empty_object(v: &serde_json::Value) -> bool {
    v.as_object().is_some_and(|m| m.is_empty())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment {
    pub id: String,
    pub task_id: String,
    pub name: String,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size_bytes: Option<i64>,
    pub created_at: DateTime<Utc>,
}

impl Task {
    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags.iter().any(|t| t == tag)
    }
}
