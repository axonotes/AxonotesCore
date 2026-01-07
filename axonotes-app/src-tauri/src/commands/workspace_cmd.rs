//! # Workspace Management Commands
//!
//! Tauri commands for managing UI workspace configurations.
//!
//! ## Workspace Storage
//!
//! Workspaces store JSON-stringified Dockview layout configurations.
//! The frontend passes the complete layout state as a string, and we
//! store it as-is with a workspace ID.
//!
//! ## Commands
//!
//! - `create_workspace`: Create a new workspace with a config string
//! - `get_workspace`: Get a workspace's config by ID
//! - `list_workspaces`: List all workspaces
//! - `update_workspace`: Update an existing workspace's config
//! - `delete_workspace`: Delete a workspace by ID

use crate::database;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrontendWorkspace {
    pub id: String,
    pub config: String,
    pub created_at: u64,
    pub updated_at: u64,
}

impl From<database::Workspace> for FrontendWorkspace {
    fn from(w: database::Workspace) -> Self {
        FrontendWorkspace {
            id: w.id,
            config: w.config,
            created_at: w.created_at,
            updated_at: w.updated_at,
        }
    }
}

#[tauri::command]
pub async fn create_workspace(id: String, config: String) -> Result<FrontendWorkspace, String> {
    database::create_workspace(id, config).await.map(Into::into)
}

#[tauri::command]
pub async fn get_workspace(id: String) -> Result<Option<FrontendWorkspace>, String> {
    database::get_workspace(id)
        .await
        .map(|opt| opt.map(Into::into))
}

#[tauri::command]
pub async fn list_workspaces() -> Result<Vec<FrontendWorkspace>, String> {
    database::list_workspaces()
        .await
        .map(|workspaces| workspaces.into_iter().map(Into::into).collect())
}

#[tauri::command]
pub async fn update_workspace(id: String, config: String) -> Result<FrontendWorkspace, String> {
    database::update_workspace(id, config).await.map(Into::into)
}

#[tauri::command]
pub async fn delete_workspace(id: String) -> Result<bool, String> {
    database::delete_workspace(id).await
}
