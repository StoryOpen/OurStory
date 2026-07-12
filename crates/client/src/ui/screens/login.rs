//! A single tall canvas (1024 × 3072) with a panning camera that moves one
//! 768px section at a time — login → world-select → character-select →
//! character-create. The camera eases between sections (cubic-out ease, no
//! overshoot) and input is locked during the pan.
//!
//! Flow: Logo → Login → InGame, with sub-states LoginSection and EnteringGame.
//! All assets are loaded once via `LoginAssets` (rooted at `UI/Login.img`).

use bevy::camera::ScalingMode;
use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use bevy_asset::RenderAssetUsages;
use std::collections::HashMap;
use wz_derive::WzAsset;

use crate::camera::resources::MainCamera;
use crate::ui::UiState;
use crate::ui::components::{UiButton, UiLoginScreen};
use crate::wz::button_asset::WzButtonAsset;
use crate::wz::frames::{WzFrameAnimationAsset, WzFrameAsset};

// ── asset structs ────────────────────────────────────────────────────────

#[derive(Clone, Debug, WzAsset)]
pub struct LoginTitle {
    #[wz(image, child = "MSTitle")]
    pub ms_title: Handle<Image>,
    #[wz(path = "BtLogin")]
    pub bt_login: WzButtonAsset,
    #[wz(path = "BtNew")]
    pub bt_new: WzButtonAsset,
    #[wz(path = "BtQuit")]
    pub bt_quit: WzButtonAsset,
}

#[derive(Clone, Debug, WzAsset)]
pub struct LoginWorldSelect {
    #[wz(image, child = "chBackgrn")]
    pub background: Handle<Image>,
    #[wz(path = "BtGoworld")]
    pub bt_goworld: WzButtonAsset,
}

#[derive(Clone, Debug, WzAsset)]
pub struct LoginCharSelect {
    #[wz(image, child = "charInfo1")]
    pub char_info1: Handle<Image>,
    #[wz(image, child = "charInfo2")]
    pub char_info2: Handle<Image>,
    #[wz(image, child = "charInfo3")]
    pub char_info3: Handle<Image>,
    #[wz(path = "BtSelect")]
    pub bt_select: WzButtonAsset,
    #[wz(path = "BtNew")]
    pub bt_new: WzButtonAsset,
}

#[derive(Clone, Debug, WzAsset)]
pub struct LoginNewChar {
    #[wz(image, child = "statTb")]
    pub stat_table: Handle<Image>,
    #[wz(image, child = "charName")]
    pub char_name: Handle<Image>,
    #[wz(image, child = "charAlert")]
    pub char_alert: Handle<Image>,
    #[wz(image, child = "charSet")]
    pub char_set: Handle<Image>,
    #[wz(path = "BtYes")]
    pub bt_yes: WzButtonAsset,
    #[wz(path = "BtNo")]
    pub bt_no: WzButtonAsset,
    #[wz(path = "dice")]
    pub dice: WzFrameAnimationAsset,
}

#[derive(Asset, TypePath, Clone, Debug, WzAsset)]
#[wz(ext = "login", path = "UI/Login.img")]
pub struct LoginAssets {
    #[wz(child = "Common/frame")]
    pub frame: WzFrameAsset,
    #[wz(child = "Title")]
    pub title: LoginTitle,
    #[wz(child = "WorldSelect")]
    pub world_select: LoginWorldSelect,
    #[wz(child = "CharSelect")]
    pub char_select: LoginCharSelect,
    #[wz(child = "NewChar")]
    pub new_char: LoginNewChar,
    #[wz(path = "Map/Obj/login.img/Title")]
    pub login_obj: HashMap<String, HashMap<String, WzFrameAnimationAsset>>,
    #[wz(path = "Map/Obj/login.img/CharSelect")]
    pub char_select_obj: HashMap<String, HashMap<String, WzFrameAnimationAsset>>,
    #[wz(path = "Map/Obj/login.img/WorldSelect")]
    pub world_select_obj: HashMap<String, HashMap<String, WzFrameAnimationAsset>>,
    #[wz(path = "Map/Obj/login.img/NewChar")]
    pub new_char_obj: HashMap<String, HashMap<String, WzFrameAnimationAsset>>,
}

pub const SECTION_H: f32 = 768.0;
pub const CANVAS_W: f32 = 1024.0;
pub const CANVAS_H: f32 = SECTION_H * 4.0;

const PAN_DURATION: f32 = 0.4;
const FADE_DURATION: f32 = 0.5;

/// Render layer for screen-space entities (separate from the panning main camera).
const SCREEN_LAYER: RenderLayers = RenderLayers::layer(1);

#[derive(Component)]
pub(crate) struct ScreenCamera;

/// Marker for entities that render in screen space (fixed, not affected by camera pan).
#[derive(Component)]
pub(crate) struct ScreenEntity;

#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum LoginSection {
    #[default]
    Login,
    WorldSelect,
    CharSelect,
    CharCreate,
}

impl LoginSection {
    fn index(&self) -> usize {
        match self {
            Self::Login => 0,
            Self::WorldSelect => 1,
            Self::CharSelect => 2,
            Self::CharCreate => 3,
        }
    }
}

#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum EnteringGame {
    #[default]
    Idle,
    EnteringGame,
}

fn cubic_ease_out(t: f32) -> f32 {
    let t1 = 1.0 - t;
    1.0 - t1 * t1 * t1
}

#[derive(Resource)]
pub(crate) struct LoginAssetsHandle(pub Handle<LoginAssets>);

#[derive(Resource)]
pub(crate) struct LoginFade(pub f32);

#[derive(Resource, Default)]
pub(crate) struct LoginInputLock(pub bool);

#[derive(Resource)]
pub(crate) struct PanState {
    start_y: f32,
    target_y: f32,
    t: f32,
}

#[derive(Component)]
pub(crate) struct ButtonRect(pub Vec2);

#[derive(Resource, Default)]
pub(crate) struct HoveredButton(pub Option<Entity>);

#[derive(Component)]
pub(crate) struct LoginEntity;

pub fn enter_login(mut commands: Commands, asset_server: Res<AssetServer>) {
    let handle: Handle<LoginAssets> = asset_server.load("wz://UI/Login.img.login");
    commands.insert_resource(LoginAssetsHandle(handle));
    commands.insert_resource(LoginFade(1.0));
    commands.insert_resource(LoginInputLock(false));
    commands.spawn(UiLoginScreen);
}

pub fn exit_login(
    mut commands: Commands,
    query: Query<Entity, With<LoginEntity>>,
    screen: Query<Entity, With<UiLoginScreen>>,
    screen_cam: Query<Entity, With<ScreenCamera>>,
    screen_entities: Query<Entity, With<ScreenEntity>>,
) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
    for entity in &screen {
        commands.entity(entity).despawn();
    }
    for entity in &screen_cam {
        commands.entity(entity).despawn();
    }
    for entity in &screen_entities {
        commands.entity(entity).despawn();
    }
    commands.remove_resource::<LoginAssetsHandle>();
    commands.remove_resource::<LoginFade>();
    commands.remove_resource::<LoginInputLock>();
    commands.remove_resource::<PanState>();
}

pub fn enter_enter_game(mut fade: ResMut<LoginFade>, mut lock: ResMut<LoginInputLock>) {
    fade.0 = 1.0;
    lock.0 = true;
}

/// Max seconds to wait for login assets before exiting on failure.
const ASSET_LOAD_TIMEOUT: f32 = 15.0;

pub fn on_login_assets_loaded(
    mut commands: Commands,
    handle: Res<LoginAssetsHandle>,
    assets: Res<Assets<LoginAssets>>,
    images: Res<Assets<Image>>,
    login_section: Res<State<LoginSection>>,
    time: Res<Time>,
    mut elapsed: Local<f32>,
    mut exit: MessageWriter<AppExit>,
) {
    let Some(root) = assets.get(&handle.0) else {
        *elapsed += time.delta_secs();
        if *elapsed > ASSET_LOAD_TIMEOUT {
            error!("Login assets failed to load after {ASSET_LOAD_TIMEOUT}s, exiting");
            exit.write(AppExit::Success);
        }
        return;
    };
    let section = login_section.get();

    spawn_title_section(&mut commands, &images, &root.title);
    spawn_world_select_section(&mut commands, &images, &root.world_select);
    spawn_char_select_section(&mut commands, &images, &root.char_select);
    spawn_new_char_section(&mut commands, &images, &root.new_char);

    let y = section_y(section.index());
    commands.insert_resource(PanState {
        start_y: y,
        target_y: y,
        t: 1.0,
    });

    spawn_screen_space_camera(&mut commands);
}

fn section_y(index: usize) -> f32 {
    index as f32 * SECTION_H + SECTION_H * 0.5
}

impl PanState {
    fn start_move(start: f32, target: f32) -> Self {
        Self {
            start_y: start,
            target_y: target,
            t: 0.0,
        }
    }
}

pub fn camera_pan_system(
    time: Res<Time>,
    mut pan: Option<ResMut<PanState>>,
    mut camera: Query<&mut Transform, With<MainCamera>>,
    mut lock: ResMut<LoginInputLock>,
) {
    let Some(mut pan) = pan else { return };
    if pan.t >= 1.0 {
        return;
    }
    let Ok(mut transform) = camera.single_mut() else {
        return;
    };
    pan.t = (pan.t + time.delta_secs() / PAN_DURATION).min(1.0);
    let eased = cubic_ease_out(pan.t);
    transform.translation.y = pan.start_y + (pan.target_y - pan.start_y) * eased;
    if pan.t >= 1.0 {
        lock.0 = false;
    }
}

pub fn on_login_section_changed(
    mut commands: Commands,
    section: Res<State<LoginSection>>,
    camera: Query<&Transform, With<MainCamera>>,
    mut lock: ResMut<LoginInputLock>,
) {
    let y = section_y(section.get().index());
    let start_y = camera.single().map(|t| t.translation.y).unwrap_or(y);
    commands.insert_resource(PanState::start_move(start_y, y));
    lock.0 = true;
}

fn cursor_to_world(window: &Window, camera_xf: &GlobalTransform, area: &Rect) -> Option<Vec2> {
    let cursor = window.cursor_position()?;
    let window_size = Vec2::new(window.width(), window.height());
    let area_size = area.size();
    let scale = area_size / window_size;
    let center = camera_xf.translation().truncate();
    Some(center + (cursor - window_size * 0.5) * scale * Vec2::new(1.0, -1.0))
}

pub fn update_button_hover(
    windows: Query<&Window>,
    camera: Query<(&Projection, &GlobalTransform), With<MainCamera>>,
    mut buttons: Query<(Entity, &Transform, &ButtonRect, &UiButton, &mut Sprite)>,
    mut hovered: ResMut<HoveredButton>,
    lock: Res<LoginInputLock>,
) {
    if lock.0 {
        return;
    }
    let Ok(window) = windows.single() else { return };
    let Ok((projection, cam_xf)) = camera.single() else {
        return;
    };
    let Projection::Orthographic(ortho) = projection else {
        return;
    };
    let Some(world) = cursor_to_world(window, cam_xf, &ortho.area) else {
        return;
    };

    if let Some(prev) = hovered.0.take() {
        if let Ok((_, _, _, btn, mut sprite)) = buttons.get_mut(prev) {
            sprite.image = btn.normal.clone();
        }
    }

    for (entity, xf, rect, btn, mut sprite) in buttons.iter_mut() {
        let center = xf.translation.truncate();
        if (world.x - center.x).abs() <= rect.0.x && (world.y - center.y).abs() <= rect.0.y {
            sprite.image = btn.hover.clone();
            hovered.0 = Some(entity);
            break;
        }
    }
}

pub fn handle_button_click(
    windows: Query<&Window>,
    camera: Query<(&Projection, &GlobalTransform), With<MainCamera>>,
    buttons: Query<(&Transform, &ButtonRect, &UiButton)>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut next_section: ResMut<NextState<LoginSection>>,
    mut next_phase: ResMut<NextState<EnteringGame>>,
    lock: Res<LoginInputLock>,
    phase: Res<State<EnteringGame>>,
    mut exit: MessageWriter<AppExit>,
) {
    if lock.0 || *phase.get() == EnteringGame::EnteringGame {
        return;
    }
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }
    let Ok(window) = windows.single() else { return };
    let Ok((projection, cam_xf)) = camera.single() else {
        return;
    };
    let Projection::Orthographic(ortho) = projection else {
        return;
    };
    let Some(world) = cursor_to_world(window, cam_xf, &ortho.area) else {
        return;
    };

    for (xf, rect, btn) in buttons.iter() {
        let center = xf.translation.truncate();
        if (world.x - center.x).abs() <= rect.0.x && (world.y - center.y).abs() <= rect.0.y {
            match btn.name.as_str() {
                "BtLogin" => next_section.set(LoginSection::WorldSelect),
                "BtGoworld" => next_section.set(LoginSection::CharSelect),
                "BtSelect" | "BtNew" => next_section.set(LoginSection::CharCreate),
                "BtYes" => next_phase.set(EnteringGame::EnteringGame),
                "BtNo" => next_section.set(LoginSection::CharSelect),
                "BtQuit" => {
                    exit.write(AppExit::Success);
                }
                _ => {}
            }
            break;
        }
    }
}

pub fn fade_to_game(
    time: Res<Time>,
    mut fade: ResMut<LoginFade>,
    mut phase: ResMut<NextState<EnteringGame>>,
    mut next_ui: ResMut<NextState<UiState>>,
) {
    fade.0 -= time.delta_secs() / FADE_DURATION;
    if fade.0 <= 0.0 {
        fade.0 = 0.0;
        phase.set(EnteringGame::Idle);
        next_ui.set(UiState::InGame);
    }
}

pub fn apply_login_fade(fade: Res<LoginFade>, mut sprites: Query<&mut Sprite, With<LoginEntity>>) {
    let a = fade.0;
    for mut sprite in sprites.iter_mut() {
        sprite.color.set_alpha(a);
    }
}

fn section_top(index: usize) -> f32 {
    index as f32 * SECTION_H
}

fn first_frame(anim: &WzFrameAnimationAsset) -> Handle<Image> {
    anim.frames
        .first()
        .map(|f| f.image.clone())
        .unwrap_or_default()
}

fn half_size_of(handle: &Handle<Image>, images: &Assets<Image>) -> Vec2 {
    images
        .get(handle)
        .map(|img| Vec2::new(img.width() as f32, img.height() as f32) * 0.5)
        .unwrap_or(Vec2::new(50.0, 25.0))
}

fn spawn_button(
    commands: &mut Commands,
    images: &Assets<Image>,
    btn: &WzButtonAsset,
    name: &str,
    section: usize,
    x: f32,
    y: f32,
) {
    let normal = first_frame(&btn.normal);
    let hs = half_size_of(&normal, images);
    commands.spawn((
        Name::new(format!("Btn_{name}")),
        Sprite {
            image: normal.clone(),
            ..default()
        },
        Transform::from_xyz(x, section_top(section) + y, 1.0),
        Visibility::default(),
        ButtonRect(hs),
        UiButton {
            name: name.into(),
            normal,
            hover: first_frame(&btn.mouse_over),
            pressed: first_frame(&btn.pressed),
            disabled: first_frame(&btn.disabled),
        },
        LoginEntity,
    ));
}

fn spawn_image(
    commands: &mut Commands,
    handle: &Handle<Image>,
    name: &str,
    section: usize,
    x: f32,
    y: f32,
) {
    commands.spawn((
        Name::new(name.to_string()),
        Sprite {
            image: handle.clone(),
            ..default()
        },
        Transform::from_xyz(x, section_top(section) + y, 0.0),
        Visibility::default(),
        LoginEntity,
    ));
}

fn spawn_title_section(commands: &mut Commands, images: &Assets<Image>, title: &LoginTitle) {
    let s = LoginSection::Login.index();
    spawn_image(commands, &title.ms_title, "MSTitle", s, 112.0, 50.0);
    spawn_button(
        commands,
        images,
        &title.bt_login,
        "BtLogin",
        s,
        400.0,
        400.0,
    );
    spawn_button(commands, images, &title.bt_quit, "BtQuit", s, 400.0, 450.0);
    spawn_button(commands, images, &title.bt_new, "BtNew", s, 450.0, 400.0);
}

fn spawn_world_select_section(
    commands: &mut Commands,
    images: &Assets<Image>,
    ws: &LoginWorldSelect,
) {
    let s = LoginSection::WorldSelect.index();
    spawn_image(commands, &ws.background, "chBackgrn", s, 0.0, 0.0);
    spawn_button(
        commands,
        images,
        &ws.bt_goworld,
        "BtGoworld",
        s,
        400.0,
        500.0,
    );
}

fn spawn_char_select_section(
    commands: &mut Commands,
    images: &Assets<Image>,
    cs: &LoginCharSelect,
) {
    let s = LoginSection::CharSelect.index();
    spawn_image(commands, &cs.char_info1, "charInfo1", s, 0.0, 0.0);
    spawn_image(commands, &cs.char_info2, "charInfo2", s, 100.0, 0.0);
    spawn_image(commands, &cs.char_info3, "charInfo3", s, 200.0, 0.0);
    spawn_button(commands, images, &cs.bt_select, "BtSelect", s, 400.0, 500.0);
    spawn_button(commands, images, &cs.bt_new, "BtNew", s, 400.0, 600.0);
}

fn spawn_new_char_section(commands: &mut Commands, images: &Assets<Image>, nc: &LoginNewChar) {
    let s = LoginSection::CharCreate.index();
    spawn_image(commands, &nc.stat_table, "statTb", s, 0.0, 0.0);
    spawn_image(commands, &nc.char_name, "charName", s, 100.0, 0.0);
    spawn_image(commands, &nc.char_alert, "charAlert", s, 200.0, 0.0);
    spawn_image(commands, &nc.char_set, "charSet", s, 300.0, 0.0);
    spawn_button(commands, images, &nc.bt_yes, "BtYes", s, 400.0, 500.0);
    spawn_button(commands, images, &nc.bt_no, "BtNo", s, 400.0, 550.0);
    if let Some(first) = nc.dice.frames.first() {
        commands.spawn((
            Name::new("dice".to_string()),
            Sprite {
                image: first.image.clone(),
                ..default()
            },
            Transform::from_xyz(200.0, section_top(s) + 600.0, 1.0),
            Visibility::default(),
            LoginEntity,
        ));
    }
}

// ── screen-space helpers ──────────────────────────────────────────────────

fn spawn_screen_space_camera(commands: &mut Commands) {
    commands.spawn((
        Name::new("ScreenSpaceCamera"),
        Camera2d,
        ScreenCamera,
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::FixedVertical {
                viewport_height: SECTION_H,
            },
            ..OrthographicProjection::default_2d()
        }),
        SCREEN_LAYER,
        Transform::from_xyz(0.0, SECTION_H * 0.5, 100.0),
    ));
}

/// Spawn a sprite that stays fixed on screen regardless of camera pan.
pub(crate) fn spawn_screen_sprite(
    commands: &mut Commands,
    image: Handle<Image>,
    x: f32,
    y: f32,
    z: f32,
) {
    commands.spawn((
        Name::new("ScreenEntity"),
        Sprite { image, ..default() },
        Transform::from_xyz(x, y, z),
        Visibility::default(),
        SCREEN_LAYER,
        ScreenEntity,
    ));
}
