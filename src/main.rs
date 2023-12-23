use std::{collections::HashMap, sync::Arc};

use axum::{
    middleware,
    routing::{delete, get, patch, post, put},
    Router,
};
use lme_workspace::Workspace;
use routes::{
    server::{create_workspace, remove_workspace, server_select_workspace},
    workspace::{
        overlay_to_stacks, read_stack, remove_stack, set_class_name, set_name,
        set_stack_class_name, write_to_stacks, transform_group, remove_name, unset_class_name,
    },
};
use tokio::sync::RwLock;

mod errors;
mod routes;

pub type ServerStatus = Arc<RwLock<HashMap<String, Arc<RwLock<Workspace>>>>>;

#[tokio::main]
async fn main() {
    let status: ServerStatus =
        Arc::new(RwLock::new(HashMap::<String, Arc<RwLock<Workspace>>>::new()));

    let workspace = Router::new()
        .route("/stack/:stack_idx", get(read_stack))
        .route("/stack/:stack_idx", delete(remove_stack))
        .route("/stacks", patch(write_to_stacks))
        .route("/stacks", put(overlay_to_stacks))
        .route("/stacks/classes", patch(set_stack_class_name))
        .route("/stacks/transform_group", patch(transform_group))
        .route("/classes", patch(set_class_name))
        .route("/classes", delete(unset_class_name))
        .route("/id/:id_name/:atom_idx", put(set_name))
        .route("/id/:atom_idx", delete(remove_name));

    let app = Router::new()
        .nest("/:ws", workspace)
        .layer(middleware::from_fn_with_state(
            status.clone(),
            server_select_workspace,
        ))
        .route("/:ws", post(create_workspace))
        .route("/:ws", delete(remove_workspace))
        .route("/", get(|| async { "hello, world" }))
        .with_state(status);
    axum::serve(
        tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap(),
        app,
    )
    .await
    .unwrap();
}
