pub mod components;
pub mod loader;
pub mod loading;
pub mod screens;

use bevy::prelude::*;

use crate::wz::WzAssetLoader;
use crate::GameSet;
use loading::LoadingState;
use screens::login::{self, LoginSection, LoginAssets};

#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum UiState {
    Logo,
    #[default]
    Login,
    InGame,
}

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<UiState>()
            .init_state::<LoginSection>()
            .init_state::<login::EnteringGame>()
            .init_asset::<LoginAssets>()
            .init_asset_loader::<WzAssetLoader<LoginAssets>>()
            .insert_resource(LoadingState::default())
            .insert_resource(login::HoveredButton::default())
            // Logo
            .add_systems(OnEnter(UiState::Logo), screens::logo::start_logo_load)
            .add_systems(OnExit(UiState::Logo), exit_logo)
            .add_systems(
                Update,
                screens::logo::handle_logo_click
                    .run_if(in_state(UiState::Logo))
                    .in_set(GameSet::Ui),
            )
            // Login scene
            .add_systems(OnEnter(UiState::Login), login::enter_login)
            .add_systems(OnExit(UiState::Login), login::exit_login)
            .add_systems(
                OnEnter(login::EnteringGame::Idle),
                login::enter_enter_game.run_if(in_state(login::EnteringGame::EnteringGame)),
            )
            // Section-driven camera pan
            .add_systems(
                Update,
                login::on_login_section_changed
                    .run_if(in_state(UiState::Login))
                    .run_if(state_changed::<LoginSection>)
                    .in_set(GameSet::Ui),
            )
            .add_systems(
                Update,
                (
                    login::on_login_assets_loaded,
                    login::camera_pan_system,
                    login::update_button_hover,
                    login::handle_button_click,
                    login::fade_to_game.run_if(in_state(login::EnteringGame::EnteringGame)),
                    login::apply_login_fade,
                )
                    .run_if(in_state(UiState::Login))
                    .in_set(GameSet::Ui),
            )
            // InGame
            .add_systems(OnEnter(UiState::InGame), enter_ingame)
            .add_systems(OnExit(UiState::InGame), exit_ingame);
    }
}

fn exit_logo(
    mut commands: Commands,
    query: Query<
        Entity,
        Or<(
            With<screens::logo::NexonLogo>,
            With<screens::logo::WizetLogo>,
        )>,
    >,
) {
    commands.remove_resource::<screens::logo::PendingLogoLoad>();
    screens::logo::despawn_logo_screen(commands, query);
}

fn enter_ingame(_commands: Commands, _asset_server: Res<AssetServer>) {}

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
