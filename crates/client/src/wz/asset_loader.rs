use bevy::{
    asset::{io::Reader, AssetLoader, LoadContext},
    prelude::*,
    reflect::TypePath,
};
use bevy::asset::RenderAssetUsages;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use image::DynamicImage;

use thiserror::Error;

#[derive(Asset, TypePath, Debug)]
pub struct WzMapAsset {
    pub sprites: Vec<WzSpriteData>,
}

#[derive(Debug)]
pub struct WzSpriteData {
    pub image: Handle<Image>,
    pub x: f32,
    pub y: f32,
    pub z: i32,
    pub origin: Vec2,
}

#[non_exhaustive]
#[derive(Debug, Error)]
pub enum MapLoaderError {}

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
        let asset_path = load_context.path().path().to_string_lossy();
        let wz_path = asset_path.strip_suffix(".map").unwrap_or(&asset_path);

        let base = crate::wz::get_cached_base();
        let map = base.at_path(wz_path).expect("map node not found");

        let mut sprites = Vec::new();

        for i in 0..8 {
            let layer = match map.at_path(&i.to_string()) {
                Ok(l) => l,
                Err(_) => continue,
            };

            let tile_set: String = match layer.at_path("info/tS") {
                Ok(n) => n.try_into().unwrap(),
                Err(_) => continue,
            };

            if let Ok(tiles) = layer.at_path("tile") {
                let mut children = tiles.children();
                children.sort_by(|x1, _x2, x3, _x4| {
                    x1.as_str().parse::<i32>().unwrap()
                        .cmp(&x3.as_str().parse::<i32>().unwrap())
                });
                for (_, tile_node) in children {
                    let variant: String = tile_node.at_path("u").unwrap().try_into().unwrap();
                    let index: i32 = tile_node.at_path("no").unwrap().try_into().unwrap();
                    let x: f32 = tile_node.at_path("x").unwrap().try_into().unwrap();
                    let y: f32 = tile_node.at_path("y").unwrap().try_into().unwrap();
                    let z: i32 = tile_node.at_path("zM")
                        .ok()
                        .and_then(|n| -> Option<i32> { n.try_into().ok() })
                        .unwrap_or_else(|| {
                            let n = base.at_path(&format!("Map/Tile/{}.img/{}/{}", tile_set, variant, index)).unwrap();
                            n.at_path("z").ok()
                                .and_then(|n| -> Option<i32> { n.try_into().ok() })
                                .unwrap_or(0)
                        });

                    let img_path = format!("Map/Tile/{}.img/{}/{}", tile_set, variant, index);
                    let img_node = base.at_path(&img_path).unwrap();
                    let origin: Vec2 = img_node.at_path("origin").unwrap().try_into().unwrap();

                    let handle = load_or_decode_image(&img_node, load_context, img_path);
                    sprites.push(WzSpriteData { image: handle, x, y, z, origin });
                }
            }

            if let Ok(objs) = layer.at_path("obj") {
                for (_, obj_node) in objs.children() {
                    let obj_set: String = obj_node.at_path("oS").unwrap().try_into().unwrap();
                    let layer0: String = obj_node.at_path("l0").unwrap().try_into().unwrap();
                    let layer1: String = obj_node.at_path("l1").unwrap().try_into().unwrap();
                    let layer2: String = obj_node.at_path("l2").unwrap().try_into().unwrap();
                    let x: f32 = obj_node.at_path("x").unwrap().try_into().unwrap();
                    let y: f32 = obj_node.at_path("y").unwrap().try_into().unwrap();
                    let z: i32 = obj_node.at_path("z").unwrap().try_into().unwrap();

                    let img_path = format!("Map/Obj/{}.img/{}/{}/{}/0", obj_set, layer0, layer1, layer2);
                    let img_node = base.at_path(&img_path).unwrap();
                    let origin: Vec2 = img_node.at_path("origin").unwrap().try_into().unwrap();

                    let handle = load_or_decode_image(&img_node, load_context, img_path);
                    sprites.push(WzSpriteData { image: handle, x, y, z, origin });
                }
            }
        }

        Ok(WzMapAsset { sprites })
    }

    fn extensions(&self) -> &[&str] {
        &["map"]
    }
}

fn load_or_decode_image(
    node: &crate::wz::Node,
    load_context: &mut LoadContext<'_>,
    wz_path: String,
) -> Handle<Image> {
    if load_context.has_labeled_asset(&wz_path) {
        return load_context.get_label_handle::<Image>(&wz_path);
    }
    let dynamic_image: DynamicImage = node.clone().try_into().unwrap();
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
    load_context.add_labeled_asset(wz_path, image)
}
