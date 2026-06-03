use bevy::prelude::*;
use crate::wz::asset_loader::WzMapAsset;
use super::resources::MapBounds;

#[derive(Event)]
pub struct RequestMap(pub String);

#[derive(Event)]
pub struct MapReady {
    pub path: String,
    pub handle: Handle<WzMapAsset>,
}

#[derive(Event)]
pub struct MapLoaded {
    pub path: String,
    pub bounds: MapBounds,
}
