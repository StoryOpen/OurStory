use bevy::{
    asset::{AssetLoader, LoadContext, io::Reader},
    prelude::*,
    reflect::TypePath,
};
use std::sync::Arc;
use thiserror::Error;

#[derive(Asset, TypePath, Debug)]
pub struct WzMapAsset(pub Arc<wz::MapData>);

#[derive(Debug, Error)]
pub enum MapLoaderError {
    #[error("WZ error: {0}")]
    WzError(#[from] wz::WzError),
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
        let wz_path = asset_path.strip_suffix(".map").unwrap_or_else(|| {
            warn!("WzMapLoader: path '{}' doesn't end with .map, using as-is", asset_path);
            &asset_path
        });

        let map_id = wz_path
            .trim_end_matches(".img")
            .rsplit('/')
            .next()
            .and_then(|s| s.parse().ok())
            .unwrap_or_else(|| {
                warn!("WzMapLoader: failed to parse map ID from '{}', using 0", wz_path);
                0
            });

        let wz = wz::WzData::global();
        let data = wz.load_map(map_id)?;
        Ok(WzMapAsset(data))
    }

    fn extensions(&self) -> &[&str] {
        &["map"]
    }
}

#[derive(Asset, TypePath, Debug)]
pub struct WzMobAsset(pub Arc<wz::MobData>);

#[derive(Debug, Error)]
pub enum MobLoaderError {
    #[error("WZ error: {0}")]
    WzError(#[from] wz::WzError),
}

#[derive(Default, TypePath)]
pub struct WzMobLoader;

impl AssetLoader for WzMobLoader {
    type Asset = WzMobAsset;
    type Settings = ();
    type Error = MobLoaderError;

    async fn load(
        &self,
        _reader: &mut dyn Reader,
        _settings: &(),
        load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let asset_path = load_context.path().path().to_string_lossy().to_string();
        let wz_path = asset_path.strip_suffix(".mob").unwrap_or_else(|| {
            warn!("WzMobLoader: path '{}' doesn't end with .mob, using as-is", asset_path);
            &asset_path
        });

        let mob_id = wz_path
            .trim_end_matches(".img")
            .rsplit('/')
            .next()
            .and_then(|s| s.parse().ok())
            .unwrap_or_else(|| {
                warn!("WzMobLoader: failed to parse mob ID from '{}', using 0", wz_path);
                0
            });

        let wz = wz::WzData::global();
        let data = wz.load_mob(mob_id)?;
        Ok(WzMobAsset(data))
    }

    fn extensions(&self) -> &[&str] {
        &["mob"]
    }
}

#[derive(Asset, TypePath, Debug)]
pub struct WzNpcAsset(pub Arc<wz::NpcData>);

#[derive(Debug, Error)]
pub enum NpcLoaderError {
    #[error("WZ error: {0}")]
    WzError(#[from] wz::WzError),
}

#[derive(Default, TypePath)]
pub struct WzNpcLoader;

impl AssetLoader for WzNpcLoader {
    type Asset = WzNpcAsset;
    type Settings = ();
    type Error = NpcLoaderError;

    async fn load(
        &self,
        _reader: &mut dyn Reader,
        _settings: &(),
        load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let asset_path = load_context.path().path().to_string_lossy().to_string();
        let wz_path = asset_path.strip_suffix(".npc").unwrap_or_else(|| {
            warn!("WzNpcLoader: path '{}' doesn't end with .npc, using as-is", asset_path);
            &asset_path
        });

        let npc_id = wz_path
            .trim_end_matches(".img")
            .rsplit('/')
            .next()
            .and_then(|s| s.parse().ok())
            .unwrap_or_else(|| {
                warn!("WzNpcLoader: failed to parse npc ID from '{}', using 0", wz_path);
                0
            });

        let wz = wz::WzData::global();
        let data = wz.load_npc(npc_id)?;
        Ok(WzNpcAsset(data))
    }

    fn extensions(&self) -> &[&str] {
        &["npc"]
    }
}
