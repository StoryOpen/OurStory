pub mod components;
pub mod loader;
pub mod screens;
pub mod windows;

use bevy::prelude::*;

use components::{UiButton, UiLoginCheckbox, UiLoginScreen};
use loader::WzImageCache;
use screens::{login, logo};
use windows::{hud, stat};
use crate::GameSet;

#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum UiState {
    #[default]
    Logo,
    Login,
    WorldSelect,
    ChannelSelect,
    CharSelect,
    CharCreate,
    InGame,
}

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<UiState>()
            .init_resource::<WzImageCache>()
            .add_systems(OnEnter(UiState::Logo), enter_logo)
            .add_systems(OnExit(UiState::Logo), exit_logo)
            .add_systems(OnEnter(UiState::Login), enter_login)
            .add_systems(OnExit(UiState::Login), exit_login)
            .add_systems(OnEnter(UiState::InGame), enter_ingame)
            .add_systems(OnExit(UiState::InGame), exit_ingame)
            .add_systems(OnEnter(UiState::WorldSelect), enter_placeholder)
            .add_systems(OnEnter(UiState::ChannelSelect), enter_placeholder)
            .add_systems(OnEnter(UiState::CharSelect), enter_placeholder)
            .add_systems(OnEnter(UiState::CharCreate), enter_placeholder)
            .add_systems(
                Update,
                (
                    logo::update_logo_animation,
                    logo::handle_logo_click,
                    update_button_sprites,
                    handle_login_button_click,
                    handle_checkbox_toggle,
                )
                    .in_set(GameSet::Ui),
            );
    }
}

fn enter_logo(
    mut commands: Commands,
    mut cache: ResMut<WzImageCache>,
    mut images: ResMut<Assets<Image>>,
) {
    logo::preload_logo_frames(&mut commands, &mut cache, &mut images);
    logo::spawn_logo_screen(&mut commands);
}

fn exit_logo(
    commands: Commands,
    query: Query<Entity, With<logo::UiLogoScreen>>,
    animation: Option<ResMut<logo::LogoAnimation>>,
) {
    logo::despawn_logo_screen(commands, query, animation);
}

fn enter_login(
    mut commands: Commands,
    mut cache: ResMut<WzImageCache>,
    mut images: ResMut<Assets<Image>>,
) {
    login::spawn_login_screen(&mut commands, &mut cache, &mut images);
}

fn exit_login(mut commands: Commands, query: Query<Entity, With<UiLoginScreen>>) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}

fn enter_ingame(
    mut commands: Commands,
    mut cache: ResMut<WzImageCache>,
    mut images: ResMut<Assets<Image>>,
) {
    hud::spawn_hud(&mut commands, &mut cache, &mut images);
    stat::spawn_stat_window(&mut commands, &mut cache, &mut images);
}

fn exit_ingame(
    mut commands: Commands,
    hud_query: Query<Entity, With<components::UiHud>>,
    stat_query: Query<Entity, With<components::UiStatWindow>>,
) {
    for entity in &hud_query {
        commands.entity(entity).despawn();
    }
    for entity in &stat_query {
        commands.entity(entity).despawn();
    }
}

fn enter_placeholder(state: Res<State<UiState>>) {
    info!("Entering placeholder state: {:?}", state.get());
}

fn update_button_sprites(mut query: Query<(&Interaction, &mut UiButton, &mut ImageNode)>) {
    for (interaction, button, mut image_node) in query.iter_mut() {
        let new_image = match interaction {
            Interaction::None => button.normal.clone(),
            Interaction::Hovered => button.hover.clone(),
            Interaction::Pressed => button.pressed.clone(),
        };
        if image_node.image != new_image {
            image_node.image = new_image;
        }
    }
}

fn handle_login_button_click(
    interaction_query: Query<(&Interaction, &UiButton)>,
    mut next_state: ResMut<NextState<UiState>>,
    mut exit: MessageWriter<AppExit>,
) {
    for (interaction, button) in &interaction_query {
        if *interaction == Interaction::Pressed {
            match button.name.as_str() {
                "BtLogin" => {
                    info!("Login button pressed, transitioning to WorldSelect");
                    next_state.set(UiState::InGame);
                }
                "BtQuit" => {
                    info!("Quit button pressed");
                    exit.write(AppExit::Success);
                }
                "BtNew" => {
                    info!("Register button pressed (placeholder)");
                }
                "BtHomePage" => {
                    info!("Homepage button pressed (placeholder)");
                }
                "BtGuestLogin" => {
                    info!("Guest login pressed (placeholder)");
                }
                "BtEmailLost" => {
                    info!("Find email pressed (placeholder)");
                }
                "BtPasswdLost" => {
                    info!("Find password pressed (placeholder)");
                }
                "BtLoginIDLost" => {
                    info!("Find login ID pressed (placeholder)");
                }
                "BtLoginIDSave" => {
                    info!("Save login ID pressed (placeholder)");
                }
                _ => {}
            }
        }
    }
}

fn handle_checkbox_toggle(
    interaction_query: Query<(&Interaction, &UiButton)>,
    mut checkbox_query: Query<(&mut UiLoginCheckbox, &mut ImageNode)>,
    cache: Res<WzImageCache>,
) {
    for (interaction, button) in &interaction_query {
        if *interaction == Interaction::Pressed && button.name == "BtEmailSave" {
            for (mut checkbox, mut image_node) in &mut checkbox_query {
                checkbox.0 = !checkbox.0;
                let path = if checkbox.0 {
                    "UI/Login.img/Title/check/1"
                } else {
                    "UI/Login.img/Title/check/0"
                };
                if let Some(handle) = cache.get(path) {
                    image_node.image = handle;
                }
                info!("Email save checkbox toggled: {}", checkbox.0);
            }
        }
    }
}
