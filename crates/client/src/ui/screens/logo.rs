use bevy::asset::AssetEvent;
use bevy::ecs::message::MessageReader;
use bevy::prelude::*;

use crate::ui::UiState;
use crate::ui::screens::logo::LogoPhase::{Nexon, Wizet};
use crate::wz::asset_loaders::WzImageFramesAsset;

const FRAME_DURATION: f32 = 0.1;

#[derive(Component)]
pub struct UiLogoScreen;

#[derive(Component)]
pub struct LogoSprite;

#[derive(Component)]
struct WizetLogo;

#[derive(Component)]
struct NexonLogo;

#[derive(Component)]
pub(crate) struct LogoAnim {
    frames: Vec<Handle<Image>>,
    current_frame: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LogoPhase {
    Wizet,
    Nexon,
}

#[derive(Resource)]
pub(crate) struct LogoPhaseState {
    phase: LogoPhase,
}

#[derive(Resource)]
pub struct PendingLogoLoad {
    wizet: Handle<WzImageFramesAsset>,
    nexon: Handle<WzImageFramesAsset>,
}

#[derive(Resource)]
pub struct LogoTimer {
    timer: Timer,
}

pub fn start_logo_load(mut commands: Commands, asset_server: Res<AssetServer>) {
    spawn_ui(&mut commands);
    let wizet = asset_server.load::<WzImageFramesAsset>("wz://UI/Logo/Wizet.frames");
    let nexon = asset_server.load::<WzImageFramesAsset>("wz://UI/Logo/Nexon.frames");
    commands.insert_resource(PendingLogoLoad { nexon, wizet });
    commands.insert_resource(LogoTimer {
        timer: Timer::from_seconds(FRAME_DURATION, TimerMode::Repeating),
    });
    commands.insert_resource(LogoEntities {
        wizet: None,
        nexon: None,
    });
}

fn spawn_ui(commands: &mut Commands) {
    commands
        .spawn((
            UiLogoScreen,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                display: Display::Flex,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            GlobalZIndex(-1),
        ))
        .with_children(|parent| {
            parent.spawn((
                LogoSprite,
                ImageNode::default(),
                Node {
                    width: Val::Px(800.0),
                    height: Val::Px(600.0),
                    ..default()
                },
            ));
        });
}

pub fn despawn_logo_screen(mut commands: Commands, query: Query<Entity, With<UiLogoScreen>>) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}

#[derive(Resource, Default)]
pub(crate) struct LogoEntities {
    wizet: Option<Entity>,
    nexon: Option<Entity>,
}

pub fn on_logo_loaded(
    mut events: MessageReader<AssetEvent<WzImageFramesAsset>>,
    logo_assets: Res<Assets<WzImageFramesAsset>>,
    mut logo_entities: ResMut<LogoEntities>,
    pending: Option<Res<PendingLogoLoad>>,
    mut commands: Commands,
) {
    let Some(pending) = pending else { return };
    for event in events.read() {
        if let AssetEvent::LoadedWithDependencies { id } = event {
            if let Some(asset) = logo_assets.get(*id) {
                if pending.wizet.id() == *id {
                    let entity = commands.spawn((
                        LogoAnim {
                            current_frame: 0,
                            frames: asset.frames.clone(),
                        },
                        WizetLogo,
                    ));
                    logo_entities.wizet = Some(entity.id());
                }
                if pending.nexon.id() == *id {
                    commands.insert_resource(LogoPhaseState {
                        phase: LogoPhase::Nexon,
                    });
                    let entity = commands.spawn((
                        LogoAnim {
                            current_frame: 0,
                            frames: asset.frames.clone(),
                        },
                        NexonLogo,
                    ));
                    logo_entities.nexon = Some(entity.id());
                }
            }
        }
    }
}

pub fn update_logo_animation(
    time: Res<Time>,
    phase_state: Option<ResMut<LogoPhaseState>>,
    mut logo_entities: ResMut<LogoEntities>,
    mut timer: ResMut<LogoTimer>,
    mut logo_query: Query<(Entity, &mut LogoAnim)>,
    mut sprite_query: Query<&mut ImageNode, With<LogoSprite>>,
    mut next_state: ResMut<NextState<UiState>>,
    mut commands: Commands,
) {
    let Some(mut phase_state) = phase_state else {
        return;
    };

    match phase_state.phase {
        Nexon => {
            if let Some(nexon_logo) = logo_entities.nexon {
                if let Ok(mut logo) = logo_query.get_mut(nexon_logo) {
                    if timer.timer.tick(time.delta()).just_finished() {
                        if let Ok(mut sprite) = sprite_query.single_mut() {
                            let frame = logo.1.frames.get(logo.1.current_frame).unwrap();
                            sprite.image = frame.clone();
                            logo.1.current_frame += 1;
                            if logo.1.current_frame >= logo.1.frames.len() {
                                timer.timer.reset();
                                phase_state.phase = LogoPhase::Wizet;
                                commands.entity(nexon_logo).despawn();
                            }
                        }
                    }
                }
            }
        }
        Wizet => {
            if let Some(wizet_logo) = logo_entities.wizet {
                if let Ok(mut logo) = logo_query.get_mut(wizet_logo) {
                    if timer.timer.tick(time.delta()).just_finished() {
                        if let Ok(mut sprite) = sprite_query.single_mut() {
                            let frame = logo.1.frames.get(logo.1.current_frame).unwrap();
                            sprite.image = frame.clone();
                            logo.1.current_frame += 1;
                            if logo.1.current_frame >= logo.1.frames.len() {
                                commands.entity(wizet_logo).despawn();
                                next_state.set(UiState::Login);
                            }
                        }
                    }
                }
            }
        }
    }
}

pub fn handle_logo_click(
    mut next_state: ResMut<NextState<UiState>>,
    buttons: Res<ButtonInput<MouseButton>>,
    phase_state: Option<ResMut<LogoPhaseState>>,
    mut logo_entities: ResMut<LogoEntities>,
    mut commands: Commands,
) {
    if !buttons.just_pressed(MouseButton::Left) {
        return;
    }
    let Some(mut phase_state) = phase_state else {
        return;
    };
    match phase_state.phase {
        LogoPhase::Wizet => {
            phase_state.phase = LogoPhase::Nexon;
        }
        LogoPhase::Nexon => {
            if let Some(e) = logo_entities.nexon {
                commands.entity(e).despawn();
            }
            commands.remove_resource::<LogoPhaseState>();
            next_state.set(UiState::Login);
        }
    }
}
