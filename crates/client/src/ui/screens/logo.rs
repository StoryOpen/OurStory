use bevy::prelude::*;

use crate::ui::loader::WzImageCache;

const FRAME_DURATION_MS: f32 = 100.0;

#[derive(Component)]
pub struct UiLogoScreen;

#[derive(Component)]
pub struct LogoSprite;

#[derive(Resource)]
pub struct LogoAnimation {
    nexon_frames: Vec<Handle<Image>>,
    wizet_frames: Vec<Handle<Image>>,
    current_phase: LogoPhase,
    current_frame: usize,
    timer: Timer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LogoPhase {
    Nexon,
    Wizet,
    Done,
}

pub fn preload_logo_frames(
    commands: &mut Commands,
    cache: &mut ResMut<WzImageCache>,
    images: &mut ResMut<Assets<Image>>,
) {
    let mut nexon_frames = Vec::new();
    for i in 0..=136 {
        let path = format!("UI/Logo.img/Nexon/{i}");
        let handle = cache.get_or_load(&path, images);
        nexon_frames.push(handle);
    }

    let mut wizet_frames = Vec::new();
    for i in 0..=60 {
        let path = format!("UI/Logo.img/Wizet/{i}");
        let handle = cache.get_or_load(&path, images);
        wizet_frames.push(handle);
    }

    commands.insert_resource(LogoAnimation {
        nexon_frames,
        wizet_frames,
        current_phase: LogoPhase::Nexon,
        current_frame: 0,
        timer: Timer::from_seconds(FRAME_DURATION_MS / 1000.0, TimerMode::Once),
    });
}

pub fn spawn_logo_screen(commands: &mut Commands) {
    commands
        .spawn((
            Name::new("LogoScreen"),
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                ..default()
            },
            BackgroundColor(Color::WHITE),
            UiLogoScreen,
        ))
        .with_children(|parent| {
            parent.spawn((
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    ..default()
                },
                ImageNode::default(),
                Interaction::default(),
                Visibility::Visible,
                LogoSprite,
            ));
        });
}

pub fn update_logo_animation(
    time: Res<Time>,
    mut animation: ResMut<LogoAnimation>,
    mut sprite_query: Query<&mut ImageNode, With<LogoSprite>>,
    mut next_state: ResMut<NextState<crate::ui::UiState>>,
) {
    animation.timer.tick(time.delta());

    if !animation.timer.just_finished() {
        return;
    }

    let frame_count = match animation.current_phase {
        LogoPhase::Nexon => animation.nexon_frames.len(),
        LogoPhase::Wizet => animation.wizet_frames.len(),
        LogoPhase::Done => return,
    };

    if animation.current_frame >= frame_count {
        match animation.current_phase {
            LogoPhase::Nexon => {
                animation.current_phase = LogoPhase::Wizet;
                animation.current_frame = 0;
                info!("Nexon logo done, starting Wizet");
            }
            LogoPhase::Wizet => {
                animation.current_phase = LogoPhase::Done;
                info!("Wizet logo done, transitioning to Login");
                next_state.set(crate::ui::UiState::Login);
                return;
            }
            LogoPhase::Done => return,
        }
    }

    let frame_idx = animation.current_frame;
    let handle = match animation.current_phase {
        LogoPhase::Nexon => animation.nexon_frames.get(frame_idx).cloned(),
        LogoPhase::Wizet => animation.wizet_frames.get(frame_idx).cloned(),
        LogoPhase::Done => None,
    };

    if let Some(handle) = handle {
        for mut sprite in &mut sprite_query {
            sprite.image = handle.clone();
        }
    }

    animation.current_frame += 1;
    animation.timer.reset();
}

pub fn handle_logo_click(
    interaction_query: Query<&Interaction, (With<LogoSprite>, Changed<Interaction>)>,
    mut animation: ResMut<LogoAnimation>,
    mut next_state: ResMut<NextState<crate::ui::UiState>>,
) {
    for interaction in &interaction_query {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match animation.current_phase {
            LogoPhase::Nexon => {
                animation.current_phase = LogoPhase::Wizet;
                animation.current_frame = 0;
                animation.timer.reset();
                info!("Logo skipped: Nexon -> Wizet");
            }
            LogoPhase::Wizet => {
                animation.current_phase = LogoPhase::Done;
                info!("Logo skipped: Wizet -> Login");
                next_state.set(crate::ui::UiState::Login);
            }
            LogoPhase::Done => {}
        }
    }
}

pub fn despawn_logo_screen(
    mut commands: Commands,
    query: Query<Entity, With<UiLogoScreen>>,
    animation: Option<ResMut<LogoAnimation>>,
) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
    if let Some(mut anim) = animation {
        anim.current_phase = LogoPhase::Done;
        anim.current_frame = 0;
    }
}
