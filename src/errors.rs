use axum::{http::StatusCode, response::IntoResponse, Json};
use lme2s::WorkspaceError;
use serde::Serialize;

#[derive(Serialize, Clone, Copy)]
pub enum LMEAPIErrors {
    WorkspaceNameConfilict,
    WorkspaceNotFound,
    Workspace(WorkspaceError),
}

impl IntoResponse for LMEAPIErrors {
    fn into_response(self) -> axum::response::Response {
        let data = Json(self);
        (
            match self {
                Self::WorkspaceNameConfilict => StatusCode::CONFLICT,
                Self::WorkspaceNotFound => StatusCode::NOT_FOUND,
                Self::Workspace(ws_error) => match ws_error {
                    WorkspaceError::IndexOutOfCoreSize => StatusCode::BAD_REQUEST,
                    WorkspaceError::NoSuchIdName | WorkspaceError::StackNotFound => {
                        StatusCode::NOT_FOUND
                    }
                },
            },
            data,
        )
            .into_response()
    }
}
