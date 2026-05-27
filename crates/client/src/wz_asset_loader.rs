use bevy::{
    asset::{io::Reader, AssetLoader, LoadContext},
    prelude::*,
    reflect::TypePath,
};
use bevy::asset::RenderAssetUsages;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use image::DynamicImage;
use thiserror::Error;
use crate::wz;

#[derive(Asset, TypePath, Debug)]
#[allow(dead_code)]
pub(crate) struct WzMapTileAsset {
    pub origin: Vec2,
    pub z: i32,
    pub image: Handle<Image>,
}

#[derive(Default, TypePath)]
pub(crate) struct WzMapTileLoader;


/// Possible errors that can be produced by [`CustomAssetLoader`]
#[non_exhaustive]
#[derive(Debug, Error)]
pub enum CustomAssetLoaderError {
}

impl AssetLoader for WzMapTileLoader {
    type Asset = WzMapTileAsset;
    type Settings = ();
    type Error = CustomAssetLoaderError;
    async fn load(
        &self,
        _reader: &mut dyn Reader,
        _settings: &(),
        load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let path = load_context.path().path().to_string_lossy();
        let wz_path = path.strip_suffix(".map_tile").unwrap_or(&path);
        let base = wz::resolve_base().unwrap();
        let tile = base.at_path(wz_path).unwrap();
        let origin : Vec2 = tile.at_path("origin").unwrap().try_into().unwrap();
        let z : i32 = tile.at_path("z").unwrap().try_into().unwrap();
        let dynamic_image: DynamicImage = tile.try_into().unwrap();
        let image = Image::new(
            Extent3d {
                width: dynamic_image.width(),
                height: dynamic_image.height(),
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            dynamic_image.into_bytes(),
            TextureFormat::Rgba8Unorm,
            RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
        );

        let image_handle = load_context.add_labeled_asset("image", image);

        Ok(WzMapTileAsset{
            origin,
            z,
            image: image_handle,
        })
    }

    fn extensions(&self) -> &[&str] {
        &["map_tile"]
    }
}