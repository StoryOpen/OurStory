use bevy::asset::io::{
    AssetReaderError, AssetSourceBuilder, AssetSourceBuilders, ErasedAssetReader, PathStream,
    Reader, VecReader,
};
use bevy::prelude::*;
use bevy::tasks::BoxedFuture;
use std::path::Path;

pub struct WzAssetSourcePlugin;

impl Plugin for WzAssetSourcePlugin {
    fn build(&self, app: &mut App) {
        let mut sources = app
            .world_mut()
            .get_resource_or_init::<AssetSourceBuilders>();
        sources.insert("wz", AssetSourceBuilder::new(|| Box::new(WzAssetReader)));
    }
}

pub struct WzAssetReader;

impl ErasedAssetReader for WzAssetReader {
    fn read<'a>(
        &'a self,
        _path: &'a Path,
    ) -> BoxedFuture<'a, Result<Box<dyn Reader + 'a>, AssetReaderError>> {
        Box::pin(async { Ok(Box::new(VecReader::new(Vec::new())) as Box<dyn Reader>) })
    }

    fn read_meta<'a>(
        &'a self,
        path: &'a Path,
    ) -> BoxedFuture<'a, Result<Box<dyn Reader + 'a>, AssetReaderError>> {
        Box::pin(async move { Err(AssetReaderError::NotFound(path.to_path_buf())) })
    }

    fn read_directory<'a>(
        &'a self,
        path: &'a Path,
    ) -> BoxedFuture<'a, Result<Box<PathStream>, AssetReaderError>> {
        Box::pin(async move { Err(AssetReaderError::NotFound(path.to_path_buf())) })
    }

    fn is_directory<'a>(
        &'a self,
        _path: &'a Path,
    ) -> BoxedFuture<'a, Result<bool, AssetReaderError>> {
        Box::pin(async { Ok(false) })
    }

    fn read_meta_bytes<'a>(
        &'a self,
        path: &'a Path,
    ) -> BoxedFuture<'a, Result<Vec<u8>, AssetReaderError>> {
        Box::pin(async move { Err(AssetReaderError::NotFound(path.to_path_buf())) })
    }
}
