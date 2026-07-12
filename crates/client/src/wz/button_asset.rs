use bevy::prelude::*;
use wz_derive::WzAsset;

use crate::wz::frames::WzFrameAnimationAsset;

#[derive(Clone, Debug, WzAsset)]
pub struct WzButtonAsset {
    #[wz(path = "normal")]
    pub normal: WzFrameAnimationAsset,
    #[wz(path = "mouseOver")]
    pub mouse_over: WzFrameAnimationAsset,
    #[wz(path = "pressed")]
    pub pressed: WzFrameAnimationAsset,
    #[wz(path = "disabled")]
    pub disabled: WzFrameAnimationAsset,
}
