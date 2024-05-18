use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Represents a directory in the tree
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Directory {
    pub id: Uuid,
    pub name: String,
    pub parent: Option<Uuid>,
    pub children: Vec<Uuid>,
    pub files: Vec<Uuid>,
}

/// Represents a file in the tree
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct File {
    pub id: Uuid,
    pub name: String,
    pub dir: Option<Uuid>,
}
