use bevy::prelude::*;

use crate::wz::asset_loaders::WzUiSpriteAsset;

/// Handles for a single UI sprite asset (image + origin).
/// Created by kicking off an asset load; use `is_ready()` to check
/// when the asset has resolved, then read `.image` and `.origin`.
pub struct UiSpriteHandle(pub Handle<WzUiSpriteAsset>);

impl UiSpriteHandle {
    pub fn load(path: &str, asset_server: &AssetServer) -> Self {
        Self(asset_server.load::<WzUiSpriteAsset>(format!("wz://{path}.wzuisprite")))
    }

    pub fn is_ready(&self, assets: &Assets<WzUiSpriteAsset>) -> bool {
        assets.contains(&self.0)
    }

    pub fn image(&self, assets: &Assets<WzUiSpriteAsset>) -> Handle<Image> {
        assets
            .get(&self.0)
            .map(|a| a.image.clone())
            .unwrap_or_default()
    }

    pub fn origin(&self, assets: &Assets<WzUiSpriteAsset>) -> Vec2 {
        assets
            .get(&self.0)
            .map(|a| a.origin)
            .unwrap_or(Vec2::ZERO)
    }
}

/// Handles for a UI button's four sprites (normal, hover, pressed, disabled).
pub struct ButtonSpriteHandles {
    pub normal: UiSpriteHandle,
    pub hover: UiSpriteHandle,
    pub pressed: UiSpriteHandle,
    pub disabled: UiSpriteHandle,
}

impl ButtonSpriteHandles {
    pub fn load(button_path: &str, asset_server: &AssetServer) -> Self {
        Self {
            normal: UiSpriteHandle::load(
                &format!("{button_path}/normal/0"),
                asset_server,
            ),
            hover: UiSpriteHandle::load(
                &format!("{button_path}/mouseOver/0"),
                asset_server,
            ),
            pressed: UiSpriteHandle::load(
                &format!("{button_path}/pressed/0"),
                asset_server,
            ),
            disabled: UiSpriteHandle::load(
                &format!("{button_path}/disabled/0"),
                asset_server,
            ),
        }
    }

    pub fn is_ready(&self, assets: &Assets<WzUiSpriteAsset>) -> bool {
        self.normal.is_ready(assets)
            && self.hover.is_ready(assets)
            && self.pressed.is_ready(assets)
            && self.disabled.is_ready(assets)
    }

    pub fn to_button(
        &self,
        assets: &Assets<WzUiSpriteAsset>,
        name: &str,
    ) -> crate::ui::components::UiButton {
        crate::ui::components::UiButton {
            name: name.into(),
            normal: self.normal.image(assets),
            hover: self.hover.image(assets),
            pressed: self.pressed.image(assets),
            disabled: self.disabled.image(assets),
        }
    }
}
