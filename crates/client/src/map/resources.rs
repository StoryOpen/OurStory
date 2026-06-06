use crate::wz::asset_loader::{Foothold, WzMapAsset};
use bevy::prelude::*;

#[derive(Debug, Clone, Copy, Resource)]
pub struct MapBounds {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
}

impl MapBounds {
    pub fn from_vr(vr_left: i32, vr_right: i32, vr_top: i32, vr_bottom: i32) -> Self {
        Self {
            left: vr_left as f32,
            right: vr_right as f32,
            top: vr_top as f32,
            bottom: vr_bottom as f32,
        }
    }

    pub fn from_footholds(footholds: &[Foothold]) -> Self {
        let mut left = f32::MAX;
        let mut right = f32::MIN;
        let mut top = f32::MIN;
        let mut bottom = f32::MAX;
        for f in footholds {
            left = left.min(f.x1).min(f.x2);
            right = right.max(f.x1).max(f.x2);
            top = top.max(f.y1).max(f.y2);
            bottom = bottom.min(f.y1).min(f.y2);
        }
        top += 75.0;
        bottom -= 300.0;
        Self {
            left,
            right,
            top,
            bottom,
        }
    }

    pub fn width(&self) -> f32 {
        self.right - self.left
    }

    pub fn height(&self) -> f32 {
        self.top - self.bottom
    }

    pub fn center(&self) -> Vec2 {
        Vec2::new(
            (self.left + self.right) * 0.5,
            (self.top + self.bottom) * 0.5,
        )
    }
}

pub enum MapState {
    None,
    Loading {
        path: String,
        handle: Handle<WzMapAsset>,
    },
    Loaded {
        path: String,
        sprites: Vec<Entity>,
        handle: Handle<WzMapAsset>,
    },
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
        Self {
            entries: Vec::with_capacity(capacity),
            capacity,
        }
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
