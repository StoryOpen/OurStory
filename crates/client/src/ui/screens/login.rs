//! A single tall canvas (1024 × 3072) with a panning camera that moves one
//! 768px section at a time — login → world-select → character-select →
//! character-create. The camera eases between sections (cubic-out ease, no
//! overshoot) and input is locked during the pan.
//!
//! Flow: Logo → Login → InGame, with sub-states LoginSection and EnteringGame.
//! All assets are loaded once via `LoginAssets` (rooted at `UI/Login.img`).

use bevy::camera::ClearColorConfig;
use bevy::camera::ScalingMode;
use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use bevy_asset::RenderAssetUsages;
use std::collections::HashMap;
use wz_derive::WzAsset;

use crate::camera::resources::{BaseResolution, MainCamera};
use crate::picking::Pickable;
use crate::ui::UiState;
use crate::ui::components::{UiButton, UiLoginScreen};
use crate::wz::button_asset::WzButtonAsset;
use crate::wz::frames::{WzFrameAnimationAsset, WzFrameAsset};


#[derive(Clone, Debug, WzAsset)]
pub struct LoginTitle {
    #[wz(path = "Map/Back/login.img/back/11")]
    pub background: WzFrameAsset,
    #[wz(path = "BtLogin")]
    pub bt_login: WzButtonAsset,
    #[wz(path = "BtLoginIDSave")]
    pub bt_login_save: WzButtonAsset,
    #[wz(path = "BtLoginIDLost")]
    pub bt_login_lost: WzButtonAsset,
    #[wz(path = "BtPasswdLost")]
    pub bt_password_lost: WzButtonAsset,
    #[wz(path = "BtHomePage")]
    pub bt_home_page: WzButtonAsset,
    #[wz(path = "BtNew")]
    pub bt_new: WzButtonAsset,
    #[wz(path = "BtQuit")]
    pub bt_quit: WzButtonAsset,
    #[wz(path = "Map/Obj/login.img/Title")]
    pub objs: HashMap<String, HashMap<String, WzFrameAnimationAsset>>,
}

#[derive(Clone, Debug, WzAsset)]
pub struct LoginWorldSelect {
    #[wz(path = "BtGoworld")]
    pub bt_goworld: WzButtonAsset,
}

#[derive(Clone, Debug, WzAsset)]
pub struct LoginCharSelect {
    #[wz(path = "BtSelect")]
    pub bt_select: WzButtonAsset,
    #[wz(path = "BtNew")]
    pub bt_new: WzButtonAsset,
}

#[derive(Clone, Debug, WzAsset)]
pub struct LoginNewChar {
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
    #[wz(path = "Map/Obj/login.img/CharSelect")]
    pub char_select_obj: HashMap<String, HashMap<String, WzFrameAnimationAsset>>,
    #[wz(path = "Map/Obj/login.img/WorldSelect")]
    pub world_select_obj: HashMap<String, HashMap<String, WzFrameAnimationAsset>>,
    #[wz(path = "Map/Obj/login.img/NewChar")]
    pub new_char_obj: HashMap<String, HashMap<String, WzFrameAnimationAsset>>,
}

const PAN_DURATION: f32 = 0.4;
const FADE_DURATION: f32 = 0.5;

const SCREEN_LAYER: RenderLayers = RenderLayers::layer(1);

#[derive(Component)]
pub(crate) struct ScreenCamera;

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

/// Tracks whether the login screen's assets have finished loading.
#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum LoginAssetsState {
    #[default]
    Loading,
    Loaded,
}

/// Waits for the login assets to finish loading (via `AssetEvent`), then advances
/// [`LoginAssetsState`] to `Loaded` (which spawns the screen exactly once via `OnEnter`).
pub fn watch_login_assets(
    handle: Res<LoginAssetsHandle>,
    mut events: MessageReader<AssetEvent<LoginAssets>>,
    mut next: ResMut<NextState<LoginAssetsState>>,
) {
    for event in events.read() {
        if event.is_loaded_with_dependencies(handle.0.id()) {
            info!(
                "[login] LoginAssets loaded with dependencies (id={:?}); advancing LoginAssetsState -> Loaded",
                handle.0.id()
            );
            next.set(LoginAssetsState::Loaded);
        } else if matches!(event, AssetEvent::Added { .. } | AssetEvent::Modified { .. }) {
            debug!("[login] asset event for other handle: {event:?}");
        }
    }
}

/// Spawns the full login screen. Runs once, on entering [`LoginAssetsState::Loaded`].
pub fn spawn_login_screen(
    mut commands: Commands,
    handle: Res<LoginAssetsHandle>,
    assets: Res<Assets<LoginAssets>>,
    login_section: Res<State<LoginSection>>,
    base: Res<BaseResolution>,
) {
    let Some(root) = assets.get(&handle.0) else {
        error!(
            "[login] spawn_login_screen: LoginAssets handle {:?} not present in Assets<LoginAssets> — early return, nothing spawned",
            handle.0.id()
        );
        return;
    };
    info!("[login] spawn_login_screen: LoginAssets present, spawning sections");
    let section = login_section.get();
    let section_h = base.height;

    spawn_title_section(&mut commands, &root.title, section_h);
    spawn_world_select_section(&mut commands, &root.world_select, section_h);
    spawn_char_select_section(&mut commands, &root.char_select, section_h);
    spawn_new_char_section(&mut commands, &root.new_char, section_h);

    let y = section_y(section.index(), section_h);
    commands.insert_resource(PanState {
        start_y: y,
        target_y: y,
        t: 0.0,
    });

    spawn_screen_space_camera(&mut commands, &*base);

    // Fixed full-screen login frame; drawn on top by the screen-space camera and
    // does not pan with the main camera.
    spawn_screen_sprite(
        &mut commands,
        root.frame.image.clone(),
        0.0,
        base.height * 0.5,
        0.0,
    );

    info!(
        "[login] spawn_login_screen complete: spawned screen-space camera; world-space entities queued via commands (deferred)"
    );
}

/// One-shot diagnostic: a few frames after the login assets load, count the
/// entities actually present in the world so we can confirm spawning worked.
/// Commands from `spawn_login_screen` are deferred, so we wait a few frames.
pub fn diagnose_login_spawn(
    mut frames: Local<u32>,
    state: Res<State<LoginAssetsState>>,
    main_cam: Query<Entity, With<MainCamera>>,
    screen_cam: Query<Entity, With<ScreenCamera>>,
    login_entities: Query<Entity, With<LoginEntity>>,
    screen_entities: Query<Entity, With<ScreenEntity>>,
) {
    if *state.get() != LoginAssetsState::Loaded {
        return;
    }
    *frames += 1;
    if *frames != 5 {
        return;
    }
    info!(
        "[login] entity counts after load: MainCamera={}, ScreenCamera={}, LoginEntity(world)={}, ScreenEntity={}",
        main_cam.iter().count(),
        screen_cam.iter().count(),
        login_entities.iter().count(),
        screen_entities.iter().count(),
    );
}

/// Resets asset-load state so re-entering the login screen spawns it again.
pub fn reset_login_assets_state(mut next: ResMut<NextState<LoginAssetsState>>) {
    next.set(LoginAssetsState::Loading);
}

fn section_y(index: usize, section_h: f32) -> f32 {
    index as f32 * section_h
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
    base: Res<BaseResolution>,
    camera: Query<&Transform, With<MainCamera>>,
    mut lock: ResMut<LoginInputLock>,
) {
    let y = section_y(section.get().index(), base.height);
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
            // TEMP: no-op button actions.
            match btn.name.as_str() {
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

fn section_top(index: usize, section_h: f32) -> f32 {
    index as f32 * section_h
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

impl WzFrameAsset {
    /// Spawn this frame's image as a static login entity.
    pub(crate) fn spawn(self, world: &mut World, transform: Transform) {
        world.spawn((
            Name::new("Frame"),
            Sprite { image: self.image, ..default() },
            transform,
            Visibility::default(),
            LoginEntity,
        ));
    }
}

impl WzFrameAnimationAsset {
    /// Spawn the first frame of this animation as a static login entity.
    /// `name` is the logical id matched by inspection tools (BRP, etc.).
    /// When `pickable` is false the entity is excluded from pointer picking.
    pub(crate) fn spawn(self, world: &mut World, transform: Transform, name: &str, pickable: bool) {
        let Some(first) = self.frames.into_iter().next() else { return; };
        let bundle = (
            Name::new(name.to_string()),
            Sprite { image: first.image, ..default() },
            transform,
            Visibility::default(),
            LoginEntity,
        );
        if pickable {
            world.spawn((bundle, Pickable));
        } else {
            world.spawn(bundle);
        }
    }
}

impl WzButtonAsset {
    /// Spawn this button (all four state frames) as an interactive login entity.
    /// `name` is the logical id matched by click handling.
    pub(crate) fn spawn(self, world: &mut World, transform: Transform, name: &str) {
        let normal = first_frame(&self.normal);
        let hs = {
            let images = world.resource::<Assets<Image>>();
            half_size_of(&normal, images)
        };
        world.spawn((
            Name::new(format!("Btn_{name}")),
            Sprite { image: normal.clone(), ..default() },
            transform,
            Visibility::default(),
            ButtonRect(hs),
            UiButton {
                name: name.to_string(),
                normal,
                hover: first_frame(&self.mouse_over),
                pressed: first_frame(&self.pressed),
                disabled: first_frame(&self.disabled),
            },
            LoginEntity,
            Pickable,
        ));
    }
}

fn spawn_title_section(
    commands: &mut Commands,
    title: &LoginTitle,
    section_h: f32,
) {
    let s = LoginSection::Login.index();
    let at = move |x: f32, y: f32, z: f32| Transform::from_xyz(x, section_top(s, section_h) + y, z);

    let background = title.background.clone();
    commands.queue(move |world: &mut World| background.spawn(world, at(-28.0, -9.0, 0.0)));

    // Every button field in LoginTitle. Positions taken from the running game
    // via BRP (real WZ layout); spawned at z=10 so they render in front.
    for (btn, name, x, y) in [
        (&title.bt_login, "BtLogin", 240.0, 54.333),
        (&title.bt_login_save, "BtLoginIDSave", 57.333, -14.0),
        (&title.bt_login_lost, "BtLoginIDLost", 167.333, -13.333),
        (&title.bt_password_lost, "BtPasswordLost", 241.0, -14.0),
        (&title.bt_home_page, "BtHomePage", 143.333, -67.0),
        (&title.bt_new, "BtNew", 42.667, -69.333),
        (&title.bt_quit, "BtQuit", 239.0, -68.333),
    ] {
        let btn = btn.clone();
        let name = name.to_string();
        commands.queue(move |world: &mut World| btn.spawn(world, at(x, y, 10.0), &name));
    }

    // objs groups: logo, effect, signboard — each maps sub-keys to a frame
    // animation. logo/signboard use one group-level position; effect frames
    // each have their own position (read from the running game via BRP).
    for group in ["logo", "effect", "signboard"] {
        let Some(anims) = title.objs.get(group) else { continue };
        let pickable = false;
        for (key, anim) in anims {
            let anim = anim.clone();
            let name = format!("{group}/{key}");
            let (x, y) = match (group, key.as_str()) {
                ("logo", _) => (14.333, 165.667),
                ("signboard", _) => (109.0, -29.0),
                ("effect", "0") => (212.333, 151.0),
                ("effect", "1") => (219.333, 90.999),
                ("effect", "2") => (166.667, 142.333),
                ("effect", "3") => (166.333, 104.667),
                ("effect", "4") => (211.0, 109.667),
                ("effect", "5") => (248.333, 76.667),
                _ => (0.0, 0.0),
            };
            commands.queue(move |world: &mut World| anim.spawn(world, at(x, y, 2.0), &name, pickable));
        }
    }
}

fn spawn_world_select_section(
    commands: &mut Commands,
    ws: &LoginWorldSelect,
    section_h: f32,
) {
    let s = LoginSection::WorldSelect.index();
    let at = move |x: f32, y: f32, z: f32| Transform::from_xyz(x, section_top(s, section_h) + y, z);
    let bt_goworld = ws.bt_goworld.clone();
    commands.queue(move |world: &mut World| {
        bt_goworld.spawn(world, at(400.0, 500.0, 1.0), "BtGoworld")
    });
}

fn spawn_char_select_section(
    commands: &mut Commands,
    cs: &LoginCharSelect,
    section_h: f32,
) {
    let s = LoginSection::CharSelect.index();
    let at = move |x: f32, y: f32, z: f32| Transform::from_xyz(x, section_top(s, section_h) + y, z);
    let bt_select = cs.bt_select.clone();
    commands.queue(move |world: &mut World| {
        bt_select.spawn(world, at(400.0, 500.0, 1.0), "BtSelect")
    });
    let bt_new = cs.bt_new.clone();
    commands.queue(move |world: &mut World| bt_new.spawn(world, at(400.0, 600.0, 1.0), "BtNew"));
}

fn spawn_new_char_section(
    commands: &mut Commands,
    nc: &LoginNewChar,
    section_h: f32,
) {
    let s = LoginSection::CharCreate.index();
    let at = move |x: f32, y: f32, z: f32| Transform::from_xyz(x, section_top(s, section_h) + y, z);
    let bt_yes = nc.bt_yes.clone();
    commands.queue(move |world: &mut World| {
        bt_yes.spawn(world, at(400.0, 500.0, 1.0), "BtYes")
    });
    let bt_no = nc.bt_no.clone();
    commands.queue(move |world: &mut World| bt_no.spawn(world, at(400.0, 550.0, 1.0), "BtNo"));
    let dice = nc.dice.clone();
    commands.queue(move |world: &mut World| dice.spawn(world, at(200.0, 600.0, 1.0), "dice", true));
}

// ── screen-space helpers ──────────────────────────────────────────────────

fn spawn_screen_space_camera(commands: &mut Commands, base: &BaseResolution) {
    commands.spawn((
        Name::new("ScreenSpaceCamera"),
        Camera2d,
        ScreenCamera,
        Camera {
            // Draw on top of the main camera without clearing its output.
            clear_color: ClearColorConfig::None,
            order: 1,
            ..default()
        },
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::FixedVertical {
                viewport_height: base.height,
            },
            ..OrthographicProjection::default_2d()
        }),
        SCREEN_LAYER,
        Transform::from_xyz(0.0, base.height * 0.5, 100.0),
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
