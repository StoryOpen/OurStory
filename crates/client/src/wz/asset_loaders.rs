use bevy::{
    asset::{AssetLoader, LoadContext, io::Reader, RenderAssetUsages},
    prelude::*,
    reflect::TypePath,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;

// ═══════════════════════════════════════════════════════════════════════
//  Image conversion helpers
// ═══════════════════════════════════════════════════════════════════════

/// Convert a DynamicImage to a Bevy Image.
fn dyn_to_bevy_image(img: &image::DynamicImage) -> Image {
    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();
    Image::new(
        Extent3d { width, height, depth_or_array_layers: 1 },
        TextureDimension::D2,
        rgba.into_raw(),
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    )
}

/// Decode PNG bytes into a Bevy Image.
fn png_to_bevy_image(png_bytes: &[u8]) -> Image {
    let dyn_img = image::load_from_memory(png_bytes)
        .expect("failed to decode PNG");
    dyn_to_bevy_image(&dyn_img)
}

/// Load a single image by its WZ path. Cross-platform.
/// Embed already-decoded Bevy Images into the load context as labeled assets.
/// Embed already-decoded Bevy Images into the load context as labeled assets.
fn embed_images(
    images: &HashMap<String, Image>,
    load_context: &mut LoadContext<'_>,
) -> HashMap<String, Handle<Image>> {
    let mut handles = HashMap::new();
    for (path, image) in images {
        let handle = load_context.add_labeled_asset(path.clone(), image.clone());
        handles.insert(path.clone(), handle);
    }
    handles
}

/// Convert PNG bytes from a bundle into labelled Bevy Image assets.
fn png_images_to_bevy(
    pngs: &std::collections::HashMap<String, Vec<u8>>,
    load_context: &mut LoadContext<'_>,
) -> std::collections::HashMap<String, Handle<Image>> {
    let mut handles = std::collections::HashMap::new();
    for (path, png_bytes) in pngs {
        let image = png_to_bevy_image(png_bytes);
        let handle = load_context.add_labeled_asset(path.clone(), image);
        handles.insert(path.clone(), handle);
    }
    handles
}

/// Strip the `wz://` prefix and the given extension from an asset path.
/// Returns WZ path without extension.
fn parse_asset_path<'a>(asset_path: &'a str, ext: &str) -> &'a str {
    let stripped = asset_path.strip_prefix("wz://").unwrap_or(asset_path);
    stripped.strip_suffix(ext).unwrap_or(stripped).trim_end_matches('.')
}

// ═══════════════════════════════════════════════════════════════════════
//  WzImage — loads a single image via wz://...*.wzimg
// ═══════════════════════════════════════════════════════════════════════

#[derive(Default, TypePath)]
pub struct WzImageLoader;

impl AssetLoader for WzImageLoader {
    type Asset = Image;
    type Settings = ();
    type Error = ImageLoaderError;

    async fn load(
        &self,
        _reader: &mut dyn Reader,
        _settings: &(),
        load_context: &mut LoadContext<'_>,
    ) -> Result<Image, Self::Error> {
        let asset_path = load_context.path().path().to_string_lossy().to_string();
        let wz_path = parse_asset_path(&asset_path, ".wzimg");
        let source = wz::source::default_source();
        let dyn_img = source.image_dynamic(wz_path).await?;
        Ok(dyn_to_bevy_image(&dyn_img))
    }

    fn extensions(&self) -> &[&str] {
        &["wzimg"]
    }
}

#[derive(Debug, Error)]
pub enum ImageLoaderError {
    #[error("WzImageLoader failed")]
    LoadFailed,
    #[error("WZ source error: {0}")]
    WzSource(#[from] wz::source::WzSourceError),
}

// ═══════════════════════════════════════════════════════════════════════
//  Singleton data assets (loaded once at startup)
// ═══════════════════════════════════════════════════════════════════════

// ── Physics ──

#[derive(Asset, TypePath, Debug)]
pub struct WzPhysicsAsset(pub Arc<wz::PhysicsConstants>);

#[derive(Debug, Error)]
pub enum PhysicsLoaderError {
    #[error("WZ error: {0}")]
    WzError(#[from] wz::WzError),
    #[error("WZ source error: {0}")]
    WzSource(#[from] wz::source::WzSourceError),
}

#[derive(Default, TypePath)]
pub struct WzPhysicsLoader;

impl AssetLoader for WzPhysicsLoader {
    type Asset = WzPhysicsAsset;
    type Settings = ();
    type Error = PhysicsLoaderError;

    async fn load(
        &self,
        _reader: &mut dyn Reader,
        _settings: &(),
        load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let _ = load_context;
        let constants = wz::source::load_physics().await?;
        Ok(WzPhysicsAsset(constants))
    }

    fn extensions(&self) -> &[&str] {
        &["physics"]
    }
}

// ── ZMap ──

#[derive(Asset, TypePath, Debug)]
pub struct WzZMapAsset(pub Vec<(String, usize)>);

#[derive(Debug, Error)]
pub enum ZMapLoaderError {
    #[error("WZ error: {0}")]
    WzError(#[from] wz::WzError),
    #[error("WZ source error: {0}")]
    WzSource(#[from] wz::source::WzSourceError),
}

#[derive(Default, TypePath)]
pub struct WzZMapLoader;

impl AssetLoader for WzZMapLoader {
    type Asset = WzZMapAsset;
    type Settings = ();
    type Error = ZMapLoaderError;

    async fn load(
        &self,
        _reader: &mut dyn Reader,
        _settings: &(),
        load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let _ = load_context;
        let entries = wz::source::load_zmap().await?;
        Ok(WzZMapAsset(entries))
    }

    fn extensions(&self) -> &[&str] {
        &["zmap"]
    }
}

// ── Skill Database ──

#[derive(Asset, TypePath, Debug)]
pub struct WzSkillDatabaseAsset(pub Arc<wz::SkillDatabase>);

#[derive(Debug, Error)]
pub enum SkillDatabaseLoaderError {
    #[error("WZ error: {0}")]
    WzError(#[from] wz::WzError),
    #[error("WZ source error: {0}")]
    WzSource(#[from] wz::source::WzSourceError),
}

#[derive(Default, TypePath)]
pub struct WzSkillDatabaseLoader;

impl AssetLoader for WzSkillDatabaseLoader {
    type Asset = WzSkillDatabaseAsset;
    type Settings = ();
    type Error = SkillDatabaseLoaderError;

    async fn load(
        &self,
        _reader: &mut dyn Reader,
        _settings: &(),
        load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let _ = load_context;
        let db = wz::source::load_skill_database().await?;
        Ok(WzSkillDatabaseAsset(db))
    }

    fn extensions(&self) -> &[&str] {
        &["skilldb"]
    }
}

// ── Job Catalog — job_id→label pairs loaded from WZ ──

#[derive(Asset, TypePath, Debug)]
pub struct WzJobCatalogAsset(pub Vec<(u32, String)>);

#[derive(Debug, Error)]
pub enum JobCatalogLoaderError {
    #[error("WZ error: {0}")]
    WzError(#[from] wz::WzError),
    #[error("WZ source error: {0}")]
    WzSource(#[from] wz::source::WzSourceError),
}

#[derive(Default, TypePath)]
pub struct WzJobCatalogLoader;

impl AssetLoader for WzJobCatalogLoader {
    type Asset = WzJobCatalogAsset;
    type Settings = ();
    type Error = JobCatalogLoaderError;

    async fn load(
        &self,
        _reader: &mut dyn Reader,
        _settings: &(),
        load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let _ = load_context;
        let entries = wz::source::load_job_catalog().await?;
        Ok(WzJobCatalogAsset(entries))
    }

    fn extensions(&self) -> &[&str] {
        &["jobcat"]
    }
}

// ── Action Lists — basic and composite action names ──

#[derive(Asset, TypePath, Debug)]
pub struct WzActionListsAsset {
    pub basic: Vec<String>,
    pub composite: Vec<String>,
}

#[derive(Debug, Error)]
pub enum ActionListsLoaderError {
    #[error("WZ error: {0}")]
    WzError(#[from] wz::WzError),
    #[error("WZ source error: {0}")]
    WzSource(#[from] wz::source::WzSourceError),
}

#[derive(Default, TypePath)]
pub struct WzActionListsLoader;

impl AssetLoader for WzActionListsLoader {
    type Asset = WzActionListsAsset;
    type Settings = ();
    type Error = ActionListsLoaderError;

    async fn load(
        &self,
        _reader: &mut dyn Reader,
        _settings: &(),
        load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let _ = load_context;
        let (basic, composite) = wz::source::load_action_lists().await?;
        Ok(WzActionListsAsset { basic, composite })
    }

    fn extensions(&self) -> &[&str] {
        &["actlist"]
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Character data assets (loaded per action)
// ═══════════════════════════════════════════════════════════════════════

// ── Character Body ──
// Path format: char-body/{skin}/{action}

#[derive(Asset, TypePath, Debug)]
pub struct WzCharBodyAsset {
    pub frames: Vec<wz::BodyFrame>,
}

#[derive(Debug, Error)]
pub enum CharBodyLoaderError {
    #[error("WZ error: {0}")]
    WzError(#[from] wz::WzError),
    #[error("WZ source error: {0}")]
    WzSource(#[from] wz::source::WzSourceError),
}

#[derive(Default, TypePath)]
pub struct WzCharBodyLoader;

impl AssetLoader for WzCharBodyLoader {
    type Asset = WzCharBodyAsset;
    type Settings = ();
    type Error = CharBodyLoaderError;

    async fn load(
        &self,
        _reader: &mut dyn Reader,
        _settings: &(),
        load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let asset_path = load_context.path().path().to_string_lossy().to_string();
        let path = parse_asset_path(&asset_path, ".charbody")
            .strip_prefix("char-body/")
            .unwrap_or("2000/stand1");
        let (skin_str, action) = path.split_once('/').unwrap_or(("2000", "stand1"));
        let skin: u32 = skin_str.parse().unwrap_or(2000);
        let character = wz::source::load_character_body(skin, action).await?;
        Ok(WzCharBodyAsset { frames: character.frames.clone() })
    }

    fn extensions(&self) -> &[&str] {
        &["charbody"]
    }
}

// ── Hair Body ──
// Path format: char-hair/{hair_id}/{action}

#[derive(Asset, TypePath, Debug)]
pub struct WzHairBodyAsset {
    pub frames: Vec<wz::BodyFrame>,
}

#[derive(Debug, Error)]
pub enum HairBodyLoaderError {
    #[error("WZ error: {0}")]
    WzError(#[from] wz::WzError),
    #[error("WZ source error: {0}")]
    WzSource(#[from] wz::source::WzSourceError),
}

#[derive(Default, TypePath)]
pub struct WzHairBodyLoader;

impl AssetLoader for WzHairBodyLoader {
    type Asset = WzHairBodyAsset;
    type Settings = ();
    type Error = HairBodyLoaderError;

    async fn load(
        &self,
        _reader: &mut dyn Reader,
        _settings: &(),
        load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let asset_path = load_context.path().path().to_string_lossy().to_string();
        let path = parse_asset_path(&asset_path, ".charhair")
            .strip_prefix("char-hair/")
            .unwrap_or("30000/stand1");
        let (id_str, action) = path.split_once('/').unwrap_or(("30000", "stand1"));
        let hair_id: u32 = id_str.parse().unwrap_or(30000);
        let hair = wz::source::load_hair_body(hair_id, action).await
            .map(|h| h.frames.clone())
            .unwrap_or_default();
        Ok(WzHairBodyAsset { frames: hair })
    }

    fn extensions(&self) -> &[&str] {
        &["charhair"]
    }
}

// ── Equip Action ──
// Path format: char-equip/{item_id}/{action}

#[derive(Asset, TypePath, Debug)]
pub struct WzEquipActionAsset {
    pub frames: Vec<wz::BodyFrame>,
}

#[derive(Debug, Error)]
pub enum EquipActionLoaderError {
    #[error("WZ error: {0}")]
    WzError(#[from] wz::WzError),
    #[error("WZ source error: {0}")]
    WzSource(#[from] wz::source::WzSourceError),
}

#[derive(Default, TypePath)]
pub struct WzEquipActionLoader;

impl AssetLoader for WzEquipActionLoader {
    type Asset = WzEquipActionAsset;
    type Settings = ();
    type Error = EquipActionLoaderError;

    async fn load(
        &self,
        _reader: &mut dyn Reader,
        _settings: &(),
        load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let asset_path = load_context.path().path().to_string_lossy().to_string();
        let path = parse_asset_path(&asset_path, ".charequip")
            .strip_prefix("char-equip/")
            .unwrap_or("0/stand1");
        let (id_str, action) = path.split_once('/').unwrap_or(("0", "stand1"));
        let item_id: i32 = id_str.parse().unwrap_or(0);
        let frames = wz::source::load_equip_action(item_id, action).await
            .map(|eq| eq.frames.clone())
            .unwrap_or_default();
        Ok(WzEquipActionAsset { frames })
    }

    fn extensions(&self) -> &[&str] {
        &["charequip"]
    }
}

// ── Face Expression ──
// Path format: char-face/{face_id}/{expression}

#[derive(Asset, TypePath, Debug)]
pub struct WzFaceExpressionAsset {
    pub frames: Vec<wz::FaceFrame>,
}

#[derive(Debug, Error)]
pub enum FaceExpressionLoaderError {
    #[error("WZ error: {0}")]
    WzError(#[from] wz::WzError),
    #[error("WZ source error: {0}")]
    WzSource(#[from] wz::source::WzSourceError),
}

#[derive(Default, TypePath)]
pub struct WzFaceExpressionLoader;

impl AssetLoader for WzFaceExpressionLoader {
    type Asset = WzFaceExpressionAsset;
    type Settings = ();
    type Error = FaceExpressionLoaderError;

    async fn load(
        &self,
        _reader: &mut dyn Reader,
        _settings: &(),
        load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let asset_path = load_context.path().path().to_string_lossy().to_string();
        let path = parse_asset_path(&asset_path, ".charface")
            .strip_prefix("char-face/")
            .unwrap_or("20000/blink");
        let (id_str, expression) = path.split_once('/').unwrap_or(("20000", "blink"));
        let face_id: u32 = id_str.parse().unwrap_or(20000);
        let expr = wz::source::load_face_expression(face_id, expression).await?;
        Ok(WzFaceExpressionAsset { frames: expr.frames.clone() })
    }

    fn extensions(&self) -> &[&str] {
        &["charface"]
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  WzUiSpriteAsset — bundles a UI sprite image with its origin
// ═══════════════════════════════════════════════════════════════════════

/// A UI sprite asset that bundles the image with its origin (pivot point).
/// Loaded via `asset_server.load::<WzUiSpriteAsset>("wz://path.wzuisprite")`.
/// The image is embedded as a labeled sub-asset.
#[derive(Asset, TypePath, Debug, wz_derive::WzAsset)]
#[wz(ext = "wzuisprite", path = ".")]
pub struct WzUiSpriteAsset {
    #[wz(image)]
    pub image: Handle<Image>,
    #[wz(default)]
    #[wz(origin)]
    pub origin: Vec2,
}

// ═══════════════════════════════════════════════════════════════════════
//  Map / Mob / NPC assets
// ═══════════════════════════════════════════════════════════════════════

// ── WzMapAsset (manual — complex cross-referenced path construction) ──

#[derive(Asset, TypePath, Debug)]
pub struct WzMapAsset {
    pub data: Arc<wz::MapData>,
    pub images: HashMap<String, Handle<Image>>,
}

#[derive(Debug, Error)]
pub enum MapLoaderError {
    #[error("WZ error: {0}")]
    WzError(#[from] wz::WzError),
    #[error("WZ source error: {0}")]
    WzSource(#[from] wz::source::WzSourceError),
}

#[derive(Default, TypePath)]
pub struct WzMapLoader;

impl AssetLoader for WzMapLoader {
    type Asset = WzMapAsset;
    type Settings = ();
    type Error = MapLoaderError;

    async fn load(
        &self,
        _reader: &mut dyn Reader,
        _settings: &(),
        load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let asset_path = load_context.path().path().to_string_lossy().to_string();
        let wz_path = parse_asset_path(&asset_path, ".map");

        let map_id = wz_path
            .trim_end_matches(".img")
            .rsplit('/')
            .next()
            .and_then(|s| s.parse().ok())
            .unwrap_or_else(|| {
                warn!("WzMapLoader: failed to parse map ID from '{}', using 0", wz_path);
                0
            });

        let bundle = wz::source::load_map_bundle(map_id).await?;
        let images = png_images_to_bevy(&bundle.images, load_context);
        Ok(WzMapAsset { data: Arc::new(bundle.data), images })
    }

    fn extensions(&self) -> &[&str] {
        &["map"]
    }
}

// ── MobAsset (derive-based) ──

#[derive(Asset, TypePath, Debug, wz_derive::WzAsset)]
#[wz(ext = "mob", path = "Mob/{id:07}.img")]
pub struct MobAsset {
    pub id: i32,
    pub info: MobInfo,
    #[wz(children(skip = ["info"], require_child = "0"))]
    pub actions: HashMap<String, MobAction>,
}

#[derive(Debug, Clone, wz_derive::WzAsset)]
pub struct MobInfo {
    pub level: i32,
    #[wz(rename = "maxHP")]     pub max_hp: i32,
    #[wz(rename = "maxMP")]     pub max_mp: i32,
    pub exp: i32,
    #[wz(rename = "PADamage")]   pub pad: i32,
    #[wz(rename = "PDDamage")]   pub pdd: i32,
    #[wz(rename = "MADamage")]   pub mad: i32,
    #[wz(rename = "MDDamage")]   pub mdd: i32,
    pub acc: i32,
    pub eva: i32,
    pub speed: i32,
    #[wz(rename = "bodyAttack")] pub body_attack: i32,
    pub undead: bool,
    pub pushed: i32,
    #[wz(rename = "mobType")]    pub mob_type: i32,
    #[wz(rename = "summonType")] pub summon_type: i32,
    #[wz(rename = "elemAttr")]   pub elem_attr: Option<String>,
    pub fs: Option<f32>,
}

#[derive(Debug, Clone, wz_derive::WzAsset)]
pub struct MobAction {
    pub frames: Vec<MobFrame>,
}

#[derive(Debug, Clone, wz_derive::WzAsset)]
pub struct MobFrame {
    #[wz(default)]
    pub delay: u32,
    #[wz(children(skip = ["delay", "face", "z"], require_child = "origin"))]
    pub parts: Vec<MobPart>,
}

#[derive(Debug, Clone, wz_derive::WzAsset)]
pub struct MobPart {
    pub name: String,
    #[wz(image)]
    pub image: Handle<Image>,
    #[wz(origin)]
    pub origin: Vec2,
}

// ── NpcAsset (derive-based) ──

#[derive(Asset, TypePath, Debug, wz_derive::WzAsset)]
#[wz(ext = "npc", path = "Npc/{id:07}.img")]
pub struct NpcAsset {
    pub id: i32,
    #[wz(children(skip = ["info"], require_child = "0"))]
    pub actions: HashMap<String, NpcAction>,
}

#[derive(Debug, Clone, wz_derive::WzAsset)]
pub struct NpcAction {
    pub frames: Vec<NpcFrame>,
}

#[derive(Debug, Clone, wz_derive::WzAsset)]
pub struct NpcFrame {
    #[wz(default)]
    pub delay: u32,
    #[wz(image)]
    pub image: Handle<Image>,
    #[wz(origin)]
    pub origin: Vec2,
}

// ═══════════════════════════════════════════════════════════════════════
//  WzPortalFramesAsset — portal animation frames (global, loaded once)
// ═══════════════════════════════════════════════════════════════════════

#[derive(Asset, TypePath, Debug, wz_derive::WzAsset)]
#[wz(ext = "portal-frames", path = "Map/MapHelper.img/portal/game/pv")]
pub struct WzPortalFramesAsset {
    #[wz(children(skip = []))]
    pub frames: Vec<PortalFrame>,
}

#[derive(Debug, Clone, wz_derive::WzAsset)]
pub struct PortalFrame {
    #[wz(image)]
    pub image: Handle<Image>,
    #[wz(origin)]
    pub origin: Vec2,
    #[wz(default)]
    pub delay: u32,
}

// ═══════════════════════════════════════════════════════════════════════
//  WzLogoFramesAsset — loads all logo animation frames from the Nexon
//  and Wizet subdirectories, discovered at load time.
// ═══════════════════════════════════════════════════════════════════════

/// Contains handles for all logo animation frame images.
/// Loaded via `asset_server.load::<WzImageFramesAsset>("wz://UI/Logo/Wizet.frames")`.
/// Each frame image is embedded as a labeled sub-asset.
#[derive(Asset, TypePath, Debug, wz_derive::WzAsset)]
#[wz(ext = "frames", path = "{dir}.img/{leaf}")]
pub struct WzImageFramesAsset {
    #[wz(children_images)]
    pub frames: Vec<Handle<Image>>,
}

// ═══════════════════════════════════════════════════════════════════════
//  WzUiBundleAsset — a set of UI images requested by caller-specified
//  paths. Loads as one GET /wz/bundle/paths/... (cacheable).
// ═══════════════════════════════════════════════════════════════════════

/// A bundle of UI sprite images keyed by their WZ paths.
/// The asset path encodes comma-separated WZ paths:
///   wz://bundle-paths/UI/Login.img/Common/frame,UI/Login.img/Title/MSTitle.wzbundle
#[derive(Asset, TypePath, Debug)]
pub struct WzUiBundleAsset {
    pub images: HashMap<String, Handle<Image>>,
    pub origins: HashMap<String, Vec2>,
}

#[derive(Debug, Error)]
pub enum UiBundleLoaderError {
    #[error("WZ error: {0}")]
    WzError(#[from] wz::WzError),
    #[error("WZ source error: {0}")]
    WzSource(#[from] wz::source::WzSourceError),
}

#[derive(Default, TypePath)]
pub struct WzUiBundleLoader;

impl AssetLoader for WzUiBundleLoader {
    type Asset = WzUiBundleAsset;
    type Settings = ();
    type Error = UiBundleLoaderError;

    async fn load(
        &self,
        _reader: &mut dyn Reader,
        _settings: &(),
        load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let asset_path = load_context.path().path().to_string_lossy().to_string();
        // Format: wz://bundle-paths/path1,path2,...wzbundle
        let inner = asset_path
            .strip_prefix("wz://bundle-paths/")
            .unwrap_or("");
        let inner = inner.strip_suffix(".wzbundle").unwrap_or(inner);
        let paths: Vec<String> = inner
            .split(',')
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect();

        let bundle = wz::source::load_image_bundle(&paths).await?;
        let mut images = HashMap::new();
        for (path, png) in &bundle.images {
            let bevy_img = png_to_bevy_image(png);
            let handle = load_context.add_labeled_asset(path.clone(), bevy_img);
            images.insert(path.clone(), handle);
        }
        let mut origins = HashMap::new();
        for (path, (x, y)) in &bundle.origins {
            origins.insert(path.clone(), Vec2::new(*x, *y));
        }
        Ok(WzUiBundleAsset { images, origins })
    }

    fn extensions(&self) -> &[&str] {
        &["wzbundle"]
    }
}

