use bevy::prelude::*;

use crate::animation::{Animatable, AnimationFinished, Playback};
use crate::ui::UiState;
use crate::wz::frames::WzFrameAnimationAsset;

#[derive(Component)]
pub struct NexonLogo;

#[derive(Component)]
pub struct WizetLogo;

#[derive(Resource)]
pub(crate) struct PendingLogoLoad {
    wizet: Handle<WzFrameAnimationAsset>,
}

pub fn start_logo_load(mut commands: Commands, asset_server: Res<AssetServer>) {
    let wizet = asset_server.load::<WzFrameAnimationAsset>("wz://UI/Logo.img/Wizet.frame-anim");
    let nexon = asset_server.load::<WzFrameAnimationAsset>("wz://UI/Logo.img/Nexon.frame-anim");
    commands.insert_resource(PendingLogoLoad {
        wizet: wizet.clone(),
    });
    spawn_logo(&mut commands, nexon, NexonLogo);
}

fn spawn_logo(commands: &mut Commands, animation: Handle<WzFrameAnimationAsset>, marker: impl Bundle) {
    commands
        .spawn((Animatable::new(animation, Playback::Once), marker))
        .observe(on_logo_finished);
}

pub fn despawn_logo_screen(
    mut commands: Commands,
    query: Query<Entity, Or<(With<NexonLogo>, With<WizetLogo>)>>,
) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}

fn on_logo_finished(
    trigger: On<AnimationFinished>,
    nexon: Query<(), With<NexonLogo>>,
    wizet: Query<(), With<WizetLogo>>,
    pending: Res<PendingLogoLoad>,
    mut next_state: ResMut<NextState<UiState>>,
    mut commands: Commands,
) {
    let finished = trigger.event_target();
    if nexon.contains(finished) {
        // Only start Wizet if it isn't already running (guards double-trigger).
        if wizet.is_empty() {
            spawn_logo(&mut commands, pending.wizet.clone(), WizetLogo);
        }
    } else if wizet.contains(finished) {
        next_state.set(UiState::Login);
    }
}

pub fn handle_logo_click(
    mut commands: Commands,
    buttons: Res<ButtonInput<MouseButton>>,
    logo_query: Query<Entity, (With<Animatable>, Or<(With<NexonLogo>, With<WizetLogo>)>)>,
) {
    if !buttons.just_pressed(MouseButton::Left) {
        return;
    }
    if let Ok(entity) = logo_query.single() {
        // Skip: signal completion so the sequence advances (Nexon -> Wizet -> Login),
        // then drop the current sprite.
        commands.trigger(AnimationFinished { entity });
        commands.entity(entity).despawn();
    }
}
