mod wz;
mod wz_asset_loader;
mod wz_asset_source;

use bevy::asset::RenderAssetUsages;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::sprite::Anchor;
use bevy::{
    input::mouse::AccumulatedMouseMotion,
    prelude::*,
};
use image::DynamicImage;
use wz::Node as MyWzNode;
use wz_asset_loader::{WzMapTileAsset, WzMapTileLoader};
use wz_asset_source::WzAssetSourcePlugin;

fn main() {
    App::new()
        .add_plugins(WzAssetSourcePlugin)
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_linear()))
        .init_asset::<WzMapTileAsset>()
        .init_asset_loader::<WzMapTileLoader>()
        .add_systems(Startup, setup)
        .add_systems(Startup, draw_grid)
        .add_systems(Startup, discover_tiles)
        .add_systems(Update, spawn_loaded)
        .add_systems(Update, drag_camera)
        .add_systems(Update, write_coords)
        .run();
}

#[derive(Component)]
struct PendingSpawn {
    x: f32,
    y: f32,
    handle: Handle<WzMapTileAsset>,
}

fn discover_tiles(mut commands: Commands, asset_server: Res<AssetServer>) {
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
            children.sort_by(|x1, _x2, x3, _x4| {
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
                let asset_path = format!("wz://Map/Tile/{}.img/{}/{}.map_tile", tile_set, variant, index);
                let handle = asset_server.load::<WzMapTileAsset>(&asset_path);
                commands.spawn(PendingSpawn { x, y, handle });
            }
        }

        if let Ok(objs) = layer.at_path("obj") {
            if objs.children().len() == 0 {
                continue;
            }
            for (_, obj_node) in objs.children() {
                let obj_set: String = obj_node.at_path("oS").unwrap().try_into().unwrap();
                let layer0: String = obj_node.at_path("l0").unwrap().try_into().unwrap();
                let layer1: String = obj_node.at_path("l1").unwrap().try_into().unwrap();
                let layer2: String = obj_node.at_path("l2").unwrap().try_into().unwrap();
                let x: f32 = obj_node.at_path("x").unwrap().try_into().unwrap();
                let y: f32 = obj_node.at_path("y").unwrap().try_into().unwrap();
                let asset_path = format!("wz://Map/Obj/{}.img/{}/{}/{}/0.map_tile", obj_set, layer0, layer1, layer2);
                let handle = asset_server.load::<WzMapTileAsset>(&asset_path);
                commands.spawn(PendingSpawn { x, y, handle });
            }
        }
    }
}

fn spawn_loaded(
    mut commands: Commands,
    pending: Query<(Entity, &PendingSpawn)>,
    assets: Res<Assets<WzMapTileAsset>>,
) {
    for (entity, spawn) in &pending {
        if let Some(asset) = assets.get(&spawn.handle) {
            commands.entity(entity).insert((
                Sprite::from_image(asset.image.clone()),
                Anchor::TOP_LEFT,
                Transform::from_xyz(
                    spawn.x - asset.origin.x,
                    (-spawn.y) + asset.origin.y,
                    asset.z as f32,
                ),
            ));
            commands.entity(entity).remove::<PendingSpawn>();
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

    let body = base.at_path("Character/00002000.img/walk1/0/body").unwrap();
    let image: DynamicImage = body.try_into().unwrap();

    commands.spawn(Camera2d);

    commands.spawn((
        Text::new("world"),
        TextFont {
            font_size: FontSize::Px(15.0),
            ..default()
        },
        TextShadow::default(),
        TextLayout::justify(Justify::Center),
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
            font_size: FontSize::Px(15.0),
            ..default()
        },
        TextShadow::default(),
        TextLayout::justify(Justify::Center),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(25.0),
            left: Val::Px(5.0),
            ..default()
        },
        ScreenCoordinate,
    ));

    let image = Image::new(
        Extent3d {
            width: image.width(),
            height: image.height(),
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        image.into_bytes(),
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );

    let handle = images.add(image);
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
    window: Query<&Window>,
    camera: Query<(&Camera, &GlobalTransform)>,
) {
    let (camera, camera_transform) = camera.single().unwrap();
    let window = window.single().unwrap();

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