use bevy::prelude::*;

pub fn plugin(app: &mut App) {
    app.init_state::<ControlMode>();
    app.add_systems(Update, toggle_mode);
}

/// Walk: cursor grabbed, first-person movement.
/// Build: cursor free, library panel + furniture placement/editing.
#[derive(States, Copy, Clone, Eq, PartialEq, Hash, Debug, Default)]
pub enum ControlMode {
    #[default]
    Walk,
    Build,
}

fn toggle_mode(
    keys: Res<ButtonInput<KeyCode>>,
    state: Res<State<ControlMode>>,
    mut next: ResMut<NextState<ControlMode>>,
) {
    if keys.just_pressed(KeyCode::Tab) {
        next.set(match state.get() {
            ControlMode::Walk => ControlMode::Build,
            ControlMode::Build => ControlMode::Walk,
        });
    }
}
