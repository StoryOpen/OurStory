use bevy::prelude::*;
use wz_reader::WzNodeCast;

#[derive(Resource)]
pub struct PhysicsConstants {
    pub gravity_acc: f32,
    pub jump_speed: f32,
    pub fall_speed: f32,
    pub walk_speed: f32,
    pub walk_force: f32,
    pub walk_drag: f32,
    pub slip_speed: f32,
    pub slip_force: f32,
    pub swim_speed: f32,
    pub swim_speed_dec: f32,
    pub swim_force: f32,
    pub fly_speed: f32,
    pub fly_force: f32,
    pub fly_jump_dec: f32,
    pub float_drag1: f32,
    pub float_drag2: f32,
    pub float_coefficient: f32,
    pub min_friction: f32,
    pub max_friction: f32,
}

fn get_f32(children: &[(&str, wz_reader::WzNodeArc)], name: &str) -> f32 {
    let (_, node) = children
        .iter()
        .find(|(n, _)| *n == name)
        .unwrap_or_else(|| panic!("Physics.img: missing field `{name}`"));

    let guard = node.read().expect("lock poisoned");
    if let Some(v) = guard.try_as_float() {
        return *v;
    }
    if let Some(v) = guard.try_as_double() {
        return *v as f32;
    }
    if let Some(v) = guard.try_as_int() {
        return *v as f32;
    }
    panic!("Physics.img: `{name}` is not numeric");
}

pub fn load_physics(base: &crate::wz::Node) -> PhysicsConstants {
    let physics_node = base
        .at_path("Map/Physics.img")
        .expect("Map/Physics.img not found");

    let guard = physics_node.wz_node.read().expect("lock poisoned");
    let image = guard
        .try_as_image()
        .expect("Map/Physics.img is not an image");

    let (children, _) = image
        .resolve_children(None)
        .expect("failed to resolve Map/Physics.img children");

    let refs: Vec<(&str, wz_reader::WzNodeArc)> = children
        .iter()
        .map(|(name, node)| (name.as_str(), node.clone()))
        .collect();

    PhysicsConstants {
        gravity_acc: get_f32(&refs, "gravityAcc"),
        jump_speed: get_f32(&refs, "jumpSpeed"),
        fall_speed: get_f32(&refs, "fallSpeed"),
        walk_speed: get_f32(&refs, "walkSpeed"),
        walk_force: get_f32(&refs, "walkForce"),
        walk_drag: get_f32(&refs, "walkDrag"),
        slip_speed: get_f32(&refs, "slipSpeed"),
        slip_force: get_f32(&refs, "slipForce"),
        swim_speed: get_f32(&refs, "swimSpeed"),
        swim_speed_dec: get_f32(&refs, "swimSpeedDec"),
        swim_force: get_f32(&refs, "swimForce"),
        fly_speed: get_f32(&refs, "flySpeed"),
        fly_force: get_f32(&refs, "flyForce"),
        fly_jump_dec: get_f32(&refs, "flyJumpDec"),
        float_drag1: get_f32(&refs, "floatDrag1"),
        float_drag2: get_f32(&refs, "floatDrag2"),
        float_coefficient: get_f32(&refs, "floatCoefficient"),
        min_friction: get_f32(&refs, "minFriction"),
        max_friction: get_f32(&refs, "maxFriction"),
    }
}
