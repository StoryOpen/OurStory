use crate::error::WzError;
use crate::node::Node;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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

impl PhysicsConstants {
    pub(crate) fn load(base: &Node) -> Result<Self, WzError> {
        let physics_node = base.at_path("Map/Physics.img")?;

        Ok(PhysicsConstants {
            gravity_acc: physics_node.required("gravityAcc"),
            jump_speed: physics_node.required("jumpSpeed"),
            fall_speed: physics_node.required("fallSpeed"),
            walk_speed: physics_node.required("walkSpeed"),
            walk_force: physics_node.required("walkForce"),
            walk_drag: physics_node.required("walkDrag"),
            slip_speed: physics_node.required("slipSpeed"),
            slip_force: physics_node.required("slipForce"),
            swim_speed: physics_node.required("swimSpeed"),
            swim_speed_dec: physics_node.required("swimSpeedDec"),
            swim_force: physics_node.required("swimForce"),
            fly_speed: physics_node.required("flySpeed"),
            fly_force: physics_node.required("flyForce"),
            fly_jump_dec: physics_node.required("flyJumpDec"),
            float_drag1: physics_node.required("floatDrag1"),
            float_drag2: physics_node.required("floatDrag2"),
            float_coefficient: physics_node.required("floatCoefficient"),
            min_friction: physics_node.required("minFriction"),
            max_friction: physics_node.required("maxFriction"),
        })
    }
}
