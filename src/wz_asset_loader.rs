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
pub(crate) struct WzMapTileAsset {
    origin: Vec2,
    z: i32,
    image: Image,
}

#[derive(Default)]
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
        let path = load_context.path();
        let base: crate::wz::Node = wz::resolve_base().unwrap();
        let tile = base.at_path(path.to_str().unwrap()).unwrap();
        let origin : Vec2 = tile.at_path("origin").unwrap().try_into().unwrap();
        let z : i32 = tile.at_path("z").unwrap().try_into().unwrap();
        let image: DynamicImage = tile.try_into().unwrap();
        let image = Image::new(
            // 2D image of size 256x256
            Extent3d {
                width: image.width(),
                height: image.height(),
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            // Initialize it with a beige color
            image.into_bytes(),
            // Use the same encoding as the color we set
            TextureFormat::Rgba8Unorm,
            RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
        );

        Ok(WzMapTileAsset{
            origin,
            z,
            image,
        })
    }

    fn extensions(&self) -> &[&str] {
        &["map_tile"]
    }
}