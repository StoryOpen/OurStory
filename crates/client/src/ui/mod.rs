pub mod components;
pub mod loader;
pub mod loading;
pub mod screens;
pub mod windows;

use bevy::prelude::*;

use crate::GameSet;
use components::{UiButton, UiLoginCheckbox, UiLoginScreen};
use loading::LoadingState;
use screens::login::LoginCheckImages;
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
            .insert_resource(LoadingState::default())
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
                    screens::logo::on_logo_loaded,
                    screens::logo::update_logo_animation,
                    screens::logo::handle_logo_click,
                )
                    .run_if(in_state(UiState::Logo))
                    .in_set(GameSet::Ui),
            )
            .add_systems(
                Update,
                (update_button_sprites, handle_login_button_click).in_set(GameSet::Ui),
            )
            .add_systems(
                Update,
                screens::login::check_login_ready
                    .run_if(in_state(UiState::Login))
                    .in_set(GameSet::Ui),
            )
            .add_systems(
                Update,
                (
                    handle_checkbox_toggle,
                    windows::hud::check_hud_ready,
                    windows::stat::check_stat_ready,
                )
                    .in_set(GameSet::Ui),
            );
    }
}

fn enter_logo(commands: Commands, asset_server: Res<AssetServer>) {
    screens::logo::start_logo_load(commands, asset_server);
}

fn exit_logo(mut commands: Commands, query: Query<Entity, With<screens::logo::UiLogoScreen>>) {
    commands.remove_resource::<screens::logo::PendingLogoLoad>();
    screens::logo::despawn_logo_screen(commands, query);
}

fn enter_login(mut commands: Commands, asset_server: Res<AssetServer>) {
    screens::login::start_login_load(&mut commands, &asset_server);
}

fn exit_login(mut commands: Commands, query: Query<Entity, With<UiLoginScreen>>) {
    commands.remove_resource::<screens::login::PendingLoginScreen>();
    commands.remove_resource::<screens::login::LoginCheckImages>();
    for entity in &query {
        commands.entity(entity).despawn();
    }
}

fn enter_ingame(mut commands: Commands, asset_server: Res<AssetServer>) {
    windows::hud::start_hud_load(&mut commands, &asset_server);
    windows::stat::start_stat_load(&mut commands, &asset_server);
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
    check_images: Option<Res<LoginCheckImages>>,
) {
    let Some(check_images) = check_images else { return };
    for (interaction, button) in &interaction_query {
        if *interaction == Interaction::Pressed && button.name == "BtEmailSave" {
            for (mut checkbox, mut image_node) in &mut checkbox_query {
                checkbox.0 = !checkbox.0;
                image_node.image = if checkbox.0 {
                    check_images.checked.clone()
                } else {
                    check_images.unchecked.clone()
                };
                info!("Email save checkbox toggled: {}", checkbox.0);
            }
        }
    }
}
