use bevy::prelude::*;
use wz_reader::WzNodeCast;

use crate::wz::asset_loader::Foothold;

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

    PhysicsConstants {
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
    }
}

pub const PHYSICS_DT: f32 = 1.0 / 100.0;
pub const EPSILON: f32 = 0.00001;
pub const MAX_LAND_SPEED: f32 = 162.5;
pub const SHOE_WALK_SLANT: f32 = 0.9;
pub const SHOE_MASS: f32 = 100.0;

#[derive(Resource, Default)]
pub struct PhysicsAccumulator(pub f32);

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum PhysicsSet {
    Simulate,
}

#[derive(Component)]
pub struct PhysicsState {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub on_fh: bool,
    pub fh_id: i32,
    pub fh_group: i32,
    pub fh_layer: u8,
    pub left: bool,
    pub right: bool,
    pub up: bool,
    pub down: bool,
    pub jump_request: bool,
    pub enable_gravity: bool,
    pub enable_footholds: bool,
}

#[derive(Resource)]
pub struct FootholdGraph {
    pub footholds: Vec<Foothold>,
    next_idx: Vec<Option<usize>>,
    prev_idx: Vec<Option<usize>>,
}

impl FootholdGraph {
    pub fn from_footholds(mut footholds: Vec<Foothold>) -> Self {
        footholds.sort_by_key(|f| f.id);
        let n = footholds.len();
        let mut next_idx = vec![None; n];
        let mut prev_idx = vec![None; n];
        for i in 0..n {
            if let Some(nid) = footholds[i].next_id {
                if let Ok(j) = footholds.binary_search_by_key(&nid, |f| f.id) {
                    next_idx[i] = Some(j);
                }
            }
            if let Some(pid) = footholds[i].prev_id {
                if let Ok(j) = footholds.binary_search_by_key(&pid, |f| f.id) {
                    prev_idx[i] = Some(j);
                }
            }
        }
        Self { footholds, next_idx, prev_idx }
    }

    pub fn find_by_id(&self, id: i32) -> Option<usize> {
        self.footholds.binary_search_by_key(&id, |f| f.id).ok()
    }
}

pub struct PhysicsPlugin;

impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<PhysicsAccumulator>()
            .configure_sets(Update, PhysicsSet::Simulate)
            .add_systems(Update, (
                physics_update.in_set(PhysicsSet::Simulate),
                sync_physics_to_transform.after(PhysicsSet::Simulate),
                // TEMP: foothold gizmos disabled
                // draw_foothold_gizmos,
            ));
    }
}

pub fn draw_foothold_gizmos(
    graph: Option<Res<FootholdGraph>>,
    mut gizmos: Gizmos,
) {
    let Some(graph) = graph else { return };
    for fh in &graph.footholds {
        gizmos.line_2d(
            Vec2::new(fh.x1, fh.y1),
            Vec2::new(fh.x2, fh.y2),
            Color::srgb(1.0, 0.0, 1.0),
        );
    }
}

pub fn physics_update(
    time: Res<Time>,
    constants: Res<PhysicsConstants>,
    graph: Option<Res<FootholdGraph>>,
    mut accumulator: ResMut<PhysicsAccumulator>,
    mut query: Query<&mut PhysicsState>,
) {
    accumulator.0 += time.delta_secs();
    while accumulator.0 >= PHYSICS_DT {
        let graph = graph.as_deref();
        for mut ps in &mut query {
            step_physics(&mut ps, graph, &constants, PHYSICS_DT);
        }
        accumulator.0 -= PHYSICS_DT;
    }
}

fn step_physics(ps: &mut PhysicsState, graph: Option<&FootholdGraph>, constants: &PhysicsConstants, dt: f32) {
    if ps.jump_request {
        do_jump(ps, graph, constants);
        ps.jump_request = false;
    }

    if !ps.enable_footholds {
        apply_free_movement(ps, constants, dt);
        return;
    }

    if ps.on_fh {
        update_on_fh(ps, graph, constants, dt);
    } else {
        update_in_air(ps, graph, constants, dt);
    }
}

fn do_jump(ps: &mut PhysicsState, graph: Option<&FootholdGraph>, constants: &PhysicsConstants) {
    if !ps.on_fh {
        return;
    }

    if ps.down {
        let gap = graph.map_or(false, |g| {
            g.footholds.iter().any(|f| {
                f.id != ps.fh_id
                    && ((f.x1 < ps.x && ps.x < f.x2) || (f.x2 < ps.x && ps.x < f.x1))
                    && f.y_at(ps.x) < ps.y
            })
        });
        if gap {
            ps.vx = 0.0;
            ps.vy = -constants.jump_speed * 0.35355339;
            ps.on_fh = false;
            ps.fh_id = 0;
            return;
        }
    }

    ps.vy = constants.jump_speed;

    if let Some(graph) = graph {
        if let Some(idx) = graph.find_by_id(ps.fh_id) {
            let fh = &graph.footholds[idx];
            let fx = fh.x2 - fh.x1;
            let fy = fh.y2 - fh.y1;
            let uphill = (ps.left && fy < 0.0) || (ps.right && fy > 0.0);
            if uphill {
                let len2 = fx * fx + fy * fy;
                if len2 > EPSILON {
                    let fmax = constants.walk_speed * (1.0 + fy * fy / len2);
                    if ps.left {
                        ps.vx = ps.vx.max(-fmax);
                        if ps.vx > -fmax * 0.8 {
                            ps.vx = -fmax * 0.8;
                        }
                    } else {
                        ps.vx = ps.vx.min(fmax);
                        if ps.vx < fmax * 0.8 {
                            ps.vx = fmax * 0.8;
                        }
                    }
                }
            }
        }
    }

    ps.on_fh = false;
    ps.fh_id = 0;
}

fn update_on_fh(ps: &mut PhysicsState, graph: Option<&FootholdGraph>, constants: &PhysicsConstants, dt: f32) {
    let graph = match graph {
        Some(g) => g,
        None => {
            ps.on_fh = false;
            return;
        }
    };

    let idx = match graph.find_by_id(ps.fh_id) {
        Some(i) => i,
        None => {
            ps.on_fh = false;
            return;
        }
    };

    let fh = &graph.footholds[idx];
    let fx = fh.x2 - fh.x1;
    let fy = fh.y2 - fh.y1;
    let len2 = fx * fx + fy * fy;
    if len2 < EPSILON {
        return;
    }
    let len = len2.sqrt();

    let mut mvr = if fx.abs() > EPSILON {
        ps.vx * len / fx
    } else {
        0.0
    };

    mvr -= fh.force.unwrap_or(0) as f32;

    let fs = constants.walk_drag.max(constants.min_friction).min(constants.max_friction) / SHOE_MASS * dt;
    let maxf = constants.walk_speed;
    let slip = fy / len;

    if slip.abs() > SHOE_WALK_SLANT {
        let sf = constants.slip_force * slip;
        let ss = constants.slip_speed * slip;
        if ps.left { mvr -= fs; }
        if ps.right { mvr += fs; }
        mvr = if ss > 0.0 {
            ss.min(mvr + sf * dt)
        } else {
            ss.max(mvr + sf * dt)
        };
    } else {
        if ps.left {
            mvr = if mvr < -maxf {
                (-maxf).min(mvr + fs * dt)
            } else {
                (-maxf).max(mvr - constants.walk_force / SHOE_MASS * dt)
            };
        } else if ps.right {
            mvr = if mvr > maxf {
                maxf.max(mvr - fs * dt)
            } else {
                maxf.min(mvr + constants.walk_force / SHOE_MASS * dt)
            };
        } else {
            mvr = if mvr < 0.0 {
                0.0f32.max(mvr + fs * dt)
            } else {
                0.0f32.min(mvr - fs * dt)
            };
        }
    }

    mvr += fh.force.unwrap_or(0) as f32;

    ps.vx = mvr * fx / len;
    ps.vy = mvr * fy / len;

    let nx = ps.x + ps.vx * dt;

    if nx > fh.x2.max(fh.x1) {
        handle_fh_exit_right(ps, graph, idx, dt);
    } else if nx < fh.x1.min(fh.x2) {
        handle_fh_exit_left(ps, graph, idx, dt);
    } else {
        ps.x = nx;
        ps.y = fh.y_at(nx);
    }
}

fn handle_fh_exit_right(ps: &mut PhysicsState, graph: &FootholdGraph, idx: usize, _dt: f32) {
    let fh = &graph.footholds[idx];
    if let Some(next_idx) = graph.next_idx[idx] {
        let next = &graph.footholds[next_idx];
        if next.x1 < next.x2 {
            let fx = next.x2 - next.x1;
            let fy = next.y2 - next.y1;
            let len2 = fx * fx + fy * fy;
            if len2 > EPSILON {
                let dot = (ps.vx * fx + ps.vy * fy) / len2;
                ps.vx = dot * fx;
                ps.vy = dot * fy;
            }
            ps.x = next.x1;
            ps.y = next.y_at(next.x1);
            ps.fh_id = next.id;
            ps.fh_group = next.group;
            ps.fh_layer = next.layer;
        } else if next.y1 > next.y2 {
            ps.x = fh.x2.max(fh.x1) - EPSILON;
            ps.y = fh.y_at(ps.x);
            ps.vx = 0.0;
            ps.vy = 0.0;
        } else {
            ps.on_fh = false;
            ps.fh_id = 0;
        }
    } else {
        ps.on_fh = false;
        ps.fh_id = 0;
    }
}

fn handle_fh_exit_left(ps: &mut PhysicsState, graph: &FootholdGraph, idx: usize, _dt: f32) {
    let fh = &graph.footholds[idx];
    if let Some(prev_idx) = graph.prev_idx[idx] {
        let prev = &graph.footholds[prev_idx];
        if prev.x1 < prev.x2 {
            let fx = prev.x2 - prev.x1;
            let fy = prev.y2 - prev.y1;
            let len2 = fx * fx + fy * fy;
            if len2 > EPSILON {
                let dot = (ps.vx * fx + ps.vy * fy) / len2;
                ps.vx = dot * fx;
                ps.vy = dot * fy;
            }
            ps.x = prev.x2;
            ps.y = prev.y_at(prev.x2);
            ps.fh_id = prev.id;
            ps.fh_group = prev.group;
            ps.fh_layer = prev.layer;
        } else if prev.y1 < prev.y2 {
            ps.x = fh.x1.min(fh.x2) + EPSILON;
            ps.y = fh.y_at(ps.x);
            ps.vx = 0.0;
            ps.vy = 0.0;
        } else {
            ps.on_fh = false;
            ps.fh_id = 0;
        }
    } else {
        ps.on_fh = false;
        ps.fh_id = 0;
    }
}

fn update_in_air(ps: &mut PhysicsState, graph: Option<&FootholdGraph>, constants: &PhysicsConstants, dt: f32) {
    if ps.enable_gravity {
        ps.vy -= constants.gravity_acc * dt;
        ps.vy = ps.vy.max(-constants.fall_speed);
    }

    let drag_factor = constants.float_drag2 / SHOE_MASS * dt;
    if ps.left {
        ps.vx = ps.vx.max(-constants.float_drag2 * 0.00089285714);
        ps.vx -= 2.0 * drag_factor;
    } else if ps.right {
        ps.vx = ps.vx.min(constants.float_drag2 * 0.00089285714);
        ps.vx += 2.0 * drag_factor;
    } else {
        if ps.vy > -constants.fall_speed {
            let f = constants.float_coefficient * drag_factor;
            ps.vx = if ps.vx > 0.0 { (0.0f32).max(ps.vx - f) } else { (0.0f32).min(ps.vx + f) };
        } else {
            ps.vx = if ps.vx > 0.0 { (0.0f32).max(ps.vx - drag_factor) } else { (0.0f32).min(ps.vx + drag_factor) };
        }
    }

    let from_x = ps.x;
    let from_y = ps.y;
    let to_x = ps.x + ps.vx * dt;
    let to_y = ps.y + ps.vy * dt;

    let dx1 = to_x - from_x;
    let dy1 = to_y - from_y;

    if let Some(graph) = graph {
        let mut best = None;
        let mut best_t = 1.0;

        for (idx, fh) in graph.footholds.iter().enumerate() {
            let dx2 = fh.x2 - fh.x1;
            let dy2 = fh.y2 - fh.y1;
            let dx3 = from_x - fh.x1;
            let dy3 = from_y - fh.y1;

            let denom = dx1 * dy2 - dy1 * dx2;
            if denom.abs() < EPSILON {
                continue;
            }

            let t1 = (dx1 * dy3 - dy1 * dx3) / denom;
            let t2 = (dx2 * dy3 - dy2 * dx3) / denom;

            if t1 >= 0.0 && t1 <= 1.0 && t2 >= 0.0 && t2 < best_t && denom > 0.0 {
                best = Some(idx);
                best_t = t2;
            }
        }

        if let Some(idx) = best {
            let fh = &graph.footholds[idx];
            ps.x = from_x + best_t * dx1;
            ps.y = from_y + best_t * dy1;

            let fx = fh.x2 - fh.x1;
            let fy = fh.y2 - fh.y1;
            let len2 = fx * fx + fy * fy;
            if len2 > EPSILON {
                let dot = (ps.vx * fx + ps.vy * fy) / len2;
                ps.vx = dot * fx;
                ps.vy = dot * fy;
            }

            if ps.vy < -MAX_LAND_SPEED {
                ps.vy = -MAX_LAND_SPEED;
            }

            ps.on_fh = true;
            ps.fh_id = fh.id;
            ps.fh_group = fh.group;
            ps.fh_layer = fh.layer;
            return;
        }
    }

    ps.x = to_x;
    ps.y = to_y;
}

fn apply_free_movement(ps: &mut PhysicsState, constants: &PhysicsConstants, dt: f32) {
    if ps.left {
        ps.vx -= constants.fly_force / SHOE_MASS * dt;
        ps.vx = ps.vx.max(-constants.fly_speed);
    }
    if ps.right {
        ps.vx += constants.fly_force / SHOE_MASS * dt;
        ps.vx = ps.vx.min(constants.fly_speed);
    }
    if ps.up {
        ps.vy += constants.fly_force / SHOE_MASS * dt;
        ps.vy = ps.vy.min(constants.fly_speed);
    }
    if ps.down {
        ps.vy -= constants.fly_force / SHOE_MASS * dt;
        ps.vy = ps.vy.max(-constants.fly_speed);
    }
    ps.x += ps.vx * dt;
    ps.y += ps.vy * dt;
}

pub fn sync_physics_to_transform(mut query: Query<(&mut Transform, &PhysicsState)>) {
    for (mut transform, ps) in &mut query {
        transform.translation.x = ps.x;
        transform.translation.y = ps.y;
    }
}
