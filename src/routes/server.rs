use std::sync::Arc;

use axum::{
    extract::{Path, Query, Request, State},
    middleware::Next,
    response::IntoResponse,
    response::Response,
};
use lme_workspace::Workspace;
use serde::Deserialize;
use tokio::sync::RwLock;

use crate::{errors::LMEAPIErrors, ServerStatus};

#[derive(Deserialize)]
pub struct WorkspaceNameParam {
    pub ws: String,
}

#[derive(Deserialize)]
pub struct WorkspaceCreateParam {
    core_size: usize,
}

pub async fn create_workspace(
    State(status): State<ServerStatus>,
    Path(WorkspaceNameParam { ws }): Path<WorkspaceNameParam>,
    Query(WorkspaceCreateParam { core_size }): Query<WorkspaceCreateParam>,
) -> Result<(), LMEAPIErrors> {
    if status.read().await.contains_key(&ws) {
        Err(LMEAPIErrors::WorkspaceNameConfilict)
    } else {
        status
            .write()
            .await
            .insert(ws, Arc::new(RwLock::new(Workspace::new(core_size))));
        Ok(())
    }
}

pub async fn remove_workspace(
    State(status): State<ServerStatus>,
    Path(WorkspaceNameParam { ws }): Path<WorkspaceNameParam>,
) -> Result<(), LMEAPIErrors> {
    if status.write().await.remove(&ws).is_some() {
        Ok(())
    } else {
        Err(LMEAPIErrors::WorkspaceNotFound)
    }
}

pub async fn server_select_workspace(
    State(status): State<ServerStatus>,
    Path(WorkspaceNameParam { ws }): Path<WorkspaceNameParam>,
    mut req: Request,
    next: Next,
) -> Response {
    if let Some(workspace) = status.read().await.get(&ws) {
        req.extensions_mut().insert(workspace.clone());
        next.run(req).await
    } else {
        LMEAPIErrors::WorkspaceNotFound.into_response()
    }
}
