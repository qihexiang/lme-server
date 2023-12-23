use std::{
    collections::HashMap,
    sync::Arc,
};

use axum::{extract::Path, Extension, Json};
use lme2s::{
    entity::{Atoms, Layer, Molecule},
    nalgebra::Transform3,
    Workspace,
};
use serde::Deserialize;
use tokio::sync::RwLock;

use crate::errors::LMEAPIErrors;

type WorkspaceRef = Arc<RwLock<Workspace>>;

#[derive(Deserialize)]
pub struct StackIndexParam {
    stack_idx: usize,
}

pub async fn read_stack(
    Extension(workspace): Extension<WorkspaceRef>,
    Path(StackIndexParam { stack_idx }): Path<StackIndexParam>,
) -> Result<Json<Molecule>, LMEAPIErrors> {
    let stack = workspace
        .read()
        .await
        .read_stack(stack_idx)
        .map_err(|e| LMEAPIErrors::Workspace(e))?;
    Ok(Json(stack.read().clone()))
}

pub async fn remove_stack(
    Extension(workspace): Extension<WorkspaceRef>,
    Path(StackIndexParam { stack_idx }): Path<StackIndexParam>,
) -> Result<(), LMEAPIErrors> {
    workspace
        .write()
        .await
        .remove_stack(stack_idx)
        .map_err(|e| LMEAPIErrors::Workspace(e))?;
    Ok(())
}

#[derive(Deserialize)]
pub struct WriteToStacks {
    stack_idxs: Vec<usize>,
    patch: Molecule,
}

pub async fn write_to_stacks(
    Extension(workspace): Extension<WorkspaceRef>,
    Json(WriteToStacks { stack_idxs, patch }): Json<WriteToStacks>,
) -> Result<(), LMEAPIErrors> {
    workspace
        .write()
        .await
        .write_to_stacks(&stack_idxs, &patch)
        .map_err(|e| LMEAPIErrors::Workspace(e))
}

#[derive(Deserialize)]
pub struct OverlayToStacks {
    stack_idxs: Vec<usize>,
    layer: Layer,
}

pub async fn overlay_to_stacks(
    Extension(workspace): Extension<WorkspaceRef>,
    Json(OverlayToStacks { stack_idxs, layer }): Json<OverlayToStacks>,
) -> Result<(), LMEAPIErrors> {
    workspace
        .write()
        .await
        .overlay_to_stacks(&stack_idxs, &layer)
        .map_err(|e| LMEAPIErrors::Workspace(e))
}

#[derive(Deserialize)]
pub struct IdxNameParam {
    atom_idx: usize,
    id_name: String,
}

pub async fn set_name(
    Extension(workspace): Extension<WorkspaceRef>,
    Path(IdxNameParam { atom_idx, id_name }): Path<IdxNameParam>,
) -> Result<(), LMEAPIErrors> {
    workspace
        .write()
        .await
        .set_name(atom_idx, id_name)
        .map_err(|e| LMEAPIErrors::Workspace(e))
}

#[derive(Deserialize)]
pub struct RemoveNameParam {
    atom_idx: usize
}

pub async fn remove_name(
    Extension(workspace): Extension<WorkspaceRef>,
    Path(RemoveNameParam { atom_idx }): Path<RemoveNameParam>
) -> Result<(), LMEAPIErrors> {
    workspace
        .write()
        .await
        .remove_name(atom_idx)
        .map_err(|e| LMEAPIErrors::Workspace(e))
}

pub async fn set_class_name(
    Extension(workspace): Extension<WorkspaceRef>,
    Json(iter): Json<Vec<(String, usize)>>,
) -> Result<(), LMEAPIErrors> {
    Ok(workspace.write().await.set_class_name(iter))
}

pub async fn unset_class_name(
    Extension(workspace): Extension<WorkspaceRef>,
    Json(iter): Json<Vec<(String, usize)>>,
) -> Result<(), LMEAPIErrors> {
    Ok(workspace.write().await.unset_class_name(iter))
}

#[derive(Deserialize)]
pub struct StackClassNamePatch {
    stack_idxs: Vec<usize>,
    class_names: Vec<(String, usize)>,
}

pub async fn set_stack_class_name(
    Extension(workspace): Extension<WorkspaceRef>,
    Json(StackClassNamePatch {
        stack_idxs,
        class_names,
    }): Json<StackClassNamePatch>,
) -> Result<(), LMEAPIErrors> {
    let mut patch = Molecule::default();
    patch.classes.extend(class_names);
    write_to_stacks(
        Extension(workspace),
        Json(WriteToStacks { stack_idxs, patch }),
    )
    .await
}

#[derive(Deserialize)]
pub struct TransformGroup {
    stack_idxs: Vec<usize>,
    class_name: String,
    transform: Transform3<f64>,
    overlay: bool,
}

pub async fn transform_group(
    Extension(workspace): Extension<WorkspaceRef>,
    Json(TransformGroup {
        stack_idxs,
        class_name,
        transform,
        overlay,
    }): Json<TransformGroup>,
) -> Result<(), LMEAPIErrors> {
    let patch = {
        let workspace = workspace.read().await;
        let stacks = stack_idxs
            .iter()
            .copied()
            .map(|stack_idx| workspace.read_stack(stack_idx))
            .collect::<Vec<_>>();
        if stacks.iter().all(|stack| stack.is_ok()) {
            let patch = stacks
                .into_iter()
                .map(|stack| stack.expect("Checked not None here"))
                .map(|stack| {
                    let classes = stack.get_classes(workspace.get_classes());
                    let atom_idxs = classes.get_left(&class_name);
                    let atoms = atom_idxs
                        .into_iter()
                        .filter_map(|idx| stack.read().atoms.get(idx).map(|atom| (idx, atom)))
                        .map(|(idx, atom)| (idx, Some(atom.transform_position(&transform))))
                        .collect::<HashMap<_, _>>();
                    let mut molecule = Molecule::default();
                    molecule.atoms = Atoms::from(atoms);
                    molecule
                })
                .zip(stack_idxs.iter().copied())
                .collect::<Vec<_>>();
            Ok(patch)
        } else {
            Err(LMEAPIErrors::Workspace(lme2s::WorkspaceError::StackNotFound))
        }
    }?;

    if overlay {
        for (patch, stack_idx) in patch {
            overlay_to_stacks(
                Extension(workspace.clone()),
                Json(OverlayToStacks {
                    stack_idxs: vec![stack_idx],
                    layer: Layer::Fill(patch),
                }),
            )
            .await?;
        }
        Ok(())
    } else {
        for (patch, stack_idx) in patch {
            write_to_stacks(
                Extension(workspace.clone()),
                Json(WriteToStacks {
                    stack_idxs: vec![stack_idx],
                    patch,
                }),
            )
            .await?;
        }
        Ok(())
    }
}
