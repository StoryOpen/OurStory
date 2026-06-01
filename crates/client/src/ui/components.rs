use bevy::prelude::*;

#[derive(Component)]
#[allow(dead_code)]
pub struct UiWindow {
    pub name: String,
}

#[derive(Component)]
#[allow(dead_code)]
pub struct UiButton {
    pub name: String,
    pub normal: Handle<Image>,
    pub hover: Handle<Image>,
    pub pressed: Handle<Image>,
    pub disabled: Handle<Image>,
}

#[derive(Component)]
pub struct UiStatWindow;

#[derive(Component)]
pub struct UiHud;
