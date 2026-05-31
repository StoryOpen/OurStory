use bevy::prelude::*;
use crate::wz::asset_loader::WzMapAsset;

pub enum MapState {
    None,
    Loading { path: String, handle: Handle<WzMapAsset> },
    Loaded { path: String, sprites: Vec<Entity> },
}

#[derive(Resource)]
pub struct CurrentMap(pub MapState);

#[derive(Resource)]
pub struct MapCache {
    entries: Vec<(String, Handle<WzMapAsset>)>,
    capacity: usize,
}

impl MapCache {
    pub fn new(capacity: usize) -> Self {
        Self { entries: Vec::with_capacity(capacity), capacity }
    }

    pub fn get(&mut self, path: &str) -> Option<Handle<WzMapAsset>> {
        let pos = self.entries.iter().position(|(p, _)| p == path)?;
        let (_, handle) = self.entries.remove(pos);
        self.entries.push((path.to_string(), handle.clone()));
        Some(handle)
    }

    pub fn insert(&mut self, path: String, handle: Handle<WzMapAsset>) {
        if self.entries.iter().any(|(p, _)| p == &path) {
            return;
        }
        if self.entries.len() >= self.capacity {
            self.entries.remove(0);
        }
        self.entries.push((path, handle));
    }
}
