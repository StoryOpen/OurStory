use bevy::prelude::*;
use crate::wz::asset_loader::WzMapAsset;

#[derive(Event)]
pub struct RequestMap(pub String);

#[derive(Event)]
pub struct MapReady {
    pub path: String,
    pub handle: Handle<WzMapAsset>,
}
