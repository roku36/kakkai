use bevy::prelude::*;

use super::apply::FurnitureIndex;
use super::components::Furniture;
use super::messages::{MoveFurniture, PlaceFurniture, RemoveFurniture};

/// A furniture mutation, stored as the message that performs it. Undoing
/// means re-sending the inverse through the normal message funnel, so undo
/// exercises exactly the same code paths as direct edits.
#[derive(Clone, Debug)]
pub enum FurnitureAction {
    Place(PlaceFurniture),
    Move(MoveFurniture),
    Remove(RemoveFurniture),
}

const UNDO_LIMIT: usize = 100;

#[derive(Resource, Default)]
pub struct UndoStack {
    undo: Vec<FurnitureAction>,
    redo: Vec<FurnitureAction>,
    /// Messages the recorder should NOT record (they came from undo/redo).
    skip: usize,
    /// Frames left in which all messages are ignored (world load).
    pub suppress_frames: u32,
}

impl UndoStack {
    fn push_undo(&mut self, action: FurnitureAction) {
        self.undo.push(action);
        if self.undo.len() > UNDO_LIMIT {
            self.undo.remove(0);
        }
        // A fresh user action invalidates the redo branch.
        self.redo.clear();
    }
}

/// Cmd+Z / Cmd+Shift+Z. Pops an action, captures its inverse from current
/// state for the opposite stack, then replays it through the message funnel.
pub fn undo_redo_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut stack: ResMut<UndoStack>,
    index: Res<FurnitureIndex>,
    furniture: Query<(&Furniture, &Transform)>,
    mut place: MessageWriter<PlaceFurniture>,
    mut move_msg: MessageWriter<MoveFurniture>,
    mut remove: MessageWriter<RemoveFurniture>,
) {
    let command = keys.pressed(KeyCode::SuperLeft) || keys.pressed(KeyCode::SuperRight);
    if !command || !keys.just_pressed(KeyCode::KeyZ) {
        return;
    }
    let is_redo = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    let Some(action) = (if is_redo {
        stack.redo.pop()
    } else {
        stack.undo.pop()
    }) else {
        return;
    };

    // Inverse of the action we are about to replay, from pre-replay state.
    let lookup = |id| {
        index
            .0
            .get(&id)
            .and_then(|&entity| furniture.get(entity).ok())
    };
    let inverse = match &action {
        FurnitureAction::Place(msg) => {
            Some(FurnitureAction::Remove(RemoveFurniture { id: msg.id }))
        }
        FurnitureAction::Move(msg) => lookup(msg.id).map(|(_, transform)| {
            FurnitureAction::Move(MoveFurniture {
                id: msg.id,
                transform: *transform,
            })
        }),
        FurnitureAction::Remove(msg) => lookup(msg.id).map(|(item, transform)| {
            FurnitureAction::Place(PlaceFurniture {
                id: msg.id,
                model: item.model.clone(),
                transform: *transform,
            })
        }),
    };

    match action {
        FurnitureAction::Place(msg) => {
            place.write(msg);
        }
        FurnitureAction::Move(msg) => {
            move_msg.write(msg);
        }
        FurnitureAction::Remove(msg) => {
            remove.write(msg);
        }
    }
    stack.skip += 1;

    if let Some(inverse) = inverse {
        if is_redo {
            stack.undo.push(inverse);
        } else {
            stack.redo.push(inverse);
        }
    }
}

/// Records the inverse of every incoming user mutation BEFORE apply.rs
/// consumes it (so old transforms / removed furniture data are still alive).
pub fn record_actions(
    mut stack: ResMut<UndoStack>,
    mut places: MessageReader<PlaceFurniture>,
    mut moves: MessageReader<MoveFurniture>,
    mut removes: MessageReader<RemoveFurniture>,
    index: Res<FurnitureIndex>,
    furniture: Query<(&Furniture, &Transform)>,
) {
    if stack.suppress_frames > 0 {
        stack.suppress_frames -= 1;
        places.clear();
        moves.clear();
        removes.clear();
        return;
    }
    for msg in places.read() {
        if stack.skip > 0 {
            stack.skip -= 1;
            continue;
        }
        stack.push_undo(FurnitureAction::Remove(RemoveFurniture { id: msg.id }));
    }
    let moves: Vec<_> = moves.read().cloned().collect();
    for msg in moves {
        if stack.skip > 0 {
            stack.skip -= 1;
            continue;
        }
        if let Some(&entity) = index.0.get(&msg.id)
            && let Ok((_, transform)) = furniture.get(entity)
        {
            stack.push_undo(FurnitureAction::Move(MoveFurniture {
                id: msg.id,
                transform: *transform,
            }));
        }
    }
    let removes: Vec<_> = removes.read().cloned().collect();
    for msg in removes {
        if stack.skip > 0 {
            stack.skip -= 1;
            continue;
        }
        if let Some(&entity) = index.0.get(&msg.id)
            && let Ok((item, transform)) = furniture.get(entity)
        {
            stack.push_undo(FurnitureAction::Place(PlaceFurniture {
                id: msg.id,
                model: item.model.clone(),
                transform: *transform,
            }));
        }
    }
}
