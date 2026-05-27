mod wz;
mod wz_asset_loader;

use bevy::asset::RenderAssetUsages;
use bevy::color::{color_difference::EuclideanDistance, palettes::css};
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::sprite::Anchor;
use bevy::text::cosmic_text::SwashContent::Color;
use bevy::{
    color::palettes::css::GOLD,
    input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll},
    prelude::*,
};
use image::{DynamicImage, GenericImageView};
use indexmap::map::MutableKeys;
use std::ops::Neg;
use wz::Node as MyWzNode;
use wz_asset_loader::{WzMapTileAsset, WzMapTileLoader};
use wz_reader::{WzNode, WzNodeArc, WzNodeCast, version};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_linear()))
        .init_asset::<WzMapTileAsset>()
        .init_asset_loader::<WzMapTileLoader>()
        .add_systems(Startup, setup) // Add a system to run at startup
        .add_systems(Startup, draw_grid) // Add a system to run at startup
        .add_systems(Update, drag_camera)
        .add_systems(Update, write_coords)
        .add_systems(Startup, draw_map)
        .run();
}

fn draw_map(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let base: MyWzNode = wz::resolve_base().unwrap();
    let map = base.at_path("Map/Map/Map1/100000000.img").unwrap();
    for i in 0..8 {
        let layer = map.at_path(&i.to_string()).unwrap();
        if let Ok(tiles) = layer.at_path("tile") {
            if tiles.children().len() == 0 {
                continue;
            }
            let tile_set: String = layer.at_path("info/tS").unwrap().try_into().unwrap();

            let mut children = tiles.children();
            children.sort_by(|x1, x2, x3, x4| {
                x1.as_str()
                    .parse::<i32>()
                    .unwrap()
                    .cmp(&x3.as_str().parse::<i32>().unwrap())
            });
            for (_, tile_node) in children {
                let variant: String = tile_node.at_path("u").unwrap().try_into().unwrap();
                let index: i32 = tile_node.at_path("no").unwrap().try_into().unwrap();
                let x: f32 = tile_node.at_path("x").unwrap().try_into().unwrap();
                let y: f32 = tile_node.at_path("y").unwrap().try_into().unwrap();
                let tile_image_path = format!("Map/Tile/{}.img/{}/{}", tile_set, variant, index);
                let tile_asset = base.at_path(&tile_image_path).unwrap();
                let tile_origin: Vec2 = tile_asset.at_path("origin").unwrap().try_into().unwrap();

                let tile_image: DynamicImage =
                    base.at_path(&tile_image_path).unwrap().try_into().unwrap();

                let image = Image::new(
                    // 2D image of size 256x256
                    Extent3d {
                        width: tile_image.width(),
                        height: tile_image.height(),
                        depth_or_array_layers: 1,
                    },
                    TextureDimension::D2,
                    // Initialize it with a beige color
                    tile_image.into_bytes(),
                    // Use the same encoding as the color we set
                    TextureFormat::Rgba8UnormSrgb,
                    RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
                );
                let handle = images.add(image.clone());

                commands.spawn((
                    Sprite {
                        image: handle,
                        anchor: Anchor::TopLeft,
                        ..default()
                    },
                    Transform::from_xyz(x - tile_origin.x, (-y) + tile_origin.y, 0.0),
                ));
            }
        }

        if let Ok(objs) = layer.at_path("obj") {
            if objs.children().len() == 0 {
                continue;
            }


            for (_, obj_node) in objs.children() {
                let obj_set: String = obj_node.at_path("oS").unwrap().try_into().unwrap();
                let layer0: String = obj_node.at_path("l0").unwrap().try_into().unwrap();
                let layer1 :String = obj_node.at_path("l1").unwrap().try_into().unwrap();
                let layer2 :String = obj_node.at_path("l2").unwrap().try_into().unwrap();
                let x :f32 = obj_node.at_path("x").unwrap().try_into().unwrap();
                let y :f32 = obj_node.at_path("y").unwrap().try_into().unwrap();
                let obj_image_path = format!("Map/Obj/{}.img/{}/{}/{}/0", obj_set, layer0, layer1, layer2 );
                let obj_asset = base.at_path(&obj_image_path).unwrap();
                let obj_origin: Vec2 = obj_asset.at_path("origin").unwrap().try_into().unwrap();

                let obj_image: DynamicImage =
                    base.at_path(&obj_image_path).unwrap().try_into().unwrap();

                let image = Image::new(
                    // 2D image of size 256x256
                    Extent3d {
                        width: obj_image.width(),
                        height: obj_image.height(),
                        depth_or_array_layers: 1,
                    },
                    TextureDimension::D2,
                    // Initialize it with a beige color
                    obj_image.into_bytes(),
                    // Use the same encoding as the color we set
                    TextureFormat::Rgba8Unorm,
                    RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
                );
                let handle = images.add(image.clone());

                commands.spawn((
                    Sprite {
                        image: handle,
                        anchor: Anchor::TopLeft,
                        ..default()
                    },
                    Transform::from_xyz(x - obj_origin.x, (-y) + obj_origin.y, 0.0),
                ));
            }
        }
    }


}
fn draw_grid(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    window: Query<&Window>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let win = window.single().unwrap();
    let height = win.height();
    let width = win.width();

    let short_v = meshes.add(Rectangle::new(5.0, 100.0));
    let short_h = meshes.add(Rectangle::new(100.0, 5.0));
    let long_v = meshes.add(Rectangle::new(1.0, height));
    let long_h = meshes.add(Rectangle::new(width, 1.0));

    commands.spawn((
        Mesh2d(short_h),
        MeshMaterial2d(materials.add(ColorMaterial::from_color(Srgba::RED))),
        Transform::from_xyz(0.0, 0.0, 0.0),
    ));

    commands.spawn((
        Mesh2d(short_v),
        MeshMaterial2d(materials.add(ColorMaterial::from_color(Srgba::RED))),
        Transform::from_xyz(0.0, 0.0, 0.0),
    ));

    let h2 = height as i32 / 2 / 100;
    for i in -h2..h2 + 1 {
        let y = i as f32 * 100.0;
        commands.spawn((
            Mesh2d(long_h.clone()),
            MeshMaterial2d(materials.add(ColorMaterial::from_color(Srgba::WHITE))),
            Transform::from_xyz(0.0, y, 0.0),
        ));
    }

    let w2 = width as i32 / 2 / 100;
    for i in -w2..w2 + 1 {
        let x = i as f32 * 100.0;
        commands.spawn((
            Mesh2d(long_v.clone()),
            MeshMaterial2d(materials.add(ColorMaterial::from_color(Srgba::WHITE))),
            Transform::from_xyz(x, 0.0, 0.0),
        ));
    }
}

#[derive(Component)]
struct WorldCoordinate;

#[derive(Component)]
struct ScreenCoordinate;

fn setup(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let base: MyWzNode = wz::resolve_base().unwrap();

    // let body = node.read().unwrap().at_path_parsed("BasicEff.img/AranGetSkill/0");
    let body = base.at_path("Character/00002000.img/walk1/0/body").unwrap();
    let image: DynamicImage = body.try_into().unwrap();

    commands.spawn(Camera2d);

    commands.spawn((
        Text::new("world"),
        TextFont {
            // This font is loaded and will be used instead of the default font.
            font_size: 15.0,
            ..default()
        },
        TextShadow::default(),
        // Set the justification of the Text
        TextLayout::new_with_justify(JustifyText::Center),
        // Set the style of the Node itself.
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(5.0),
            left: Val::Px(5.0),
            ..default()
        },
        WorldCoordinate,
    ));

    commands.spawn((
        Text::new("screen"),
        TextFont {
            // This font is loaded and will be used instead of the default font.
            font_size: 15.0,
            ..default()
        },
        TextShadow::default(),
        // Set the justification of the Text
        TextLayout::new_with_justify(JustifyText::Center),
        // Set the style of the Node itself.
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(25.0),
            left: Val::Px(5.0),
            ..default()
        },
        ScreenCoordinate,
    ));

    // Create an image that we are going to draw into
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

    let handle = images.add(image);
    // Add it to Bevy's assets, so it can be used for rendering
    // this will give us a handle we can use
    // (to display it in a sprite, or as part of UI, etc.)

    // Create a sprite entity using our image
    commands.spawn(Sprite::from_image(handle));
}

fn drag_camera(
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    accumulated_mouse_motion: Res<AccumulatedMouseMotion>,
    mut camera: Query<&mut Transform, With<Camera>>,
) {
    if accumulated_mouse_motion.delta != Vec2::ZERO && mouse_button_input.pressed(MouseButton::Left)
    {
        let delta = accumulated_mouse_motion.delta;
        camera.single_mut().unwrap().translation += (delta * Vec2::new(-1.0, 1.0)).extend(0.0);
        info!("mouse moved ({}, {})", delta.x, delta.y);
    }
}

fn write_coords(
    mut world_coordinate: Query<&mut Text, With<WorldCoordinate>>,
    mut screen_coordinate: Query<&mut Text, (With<ScreenCoordinate>, Without<WorldCoordinate>)>,
    // query to get the window (so we can read the current cursor position)
    window: Query<&Window>,
    // query to get camera transform
    camera: Query<(&Camera, &GlobalTransform)>,
) {
    let (camera, camera_transform) = camera.single().unwrap();
    let window = window.single().unwrap();

    // check if the cursor is inside the window and get its position
    // then, ask bevy to convert into world coordinates, and truncate to discard Z
    if let Some(world_position) = window
        .cursor_position()
        .map(|cursor| camera.viewport_to_world(camera_transform, cursor))
        .and_then(|ray| ray.ok())
        .map(|ray| ray.origin.trunc())
    {
        world_coordinate.single_mut().unwrap().0 =
            format!("World coords: {}/{}", world_position.x, world_position.y);
    }

    if let Some(cursor_position) = window.cursor_position() {
        screen_coordinate.single_mut().unwrap().0 =
            format!("Screen coords: {}/{}", cursor_position.x, cursor_position.y);
    }
}
