use bevy::prelude::*;
use bevy::sprite::Anchor;

use crate::wz::frames::WzFrameAnimationAsset;

/// Drives a [`Sprite`] through the frames of a [`WzFrameAnimationAsset`].
///
/// A single global system advances every `Animatable` based on each frame's
/// own duration (`WzFrameAsset::delay`). Positioning is left to other systems;
/// this only sets the sprite's image and anchor (the frame's `origin`).
///
/// To "pause" a specific animation, remove this component — the system stops
/// touching the sprite and it holds the last shown frame. To pause everything,
/// freeze `Time` (or the schedule).
#[derive(Component)]
#[require(Sprite, Anchor)]
pub struct Animatable {
    pub animation: Handle<WzFrameAnimationAsset>,
    pub frame: usize,
    pub elapsed: f32,
    pub mode: Playback,
    pub direction: isize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Playback {
    /// Wrap to the first frame after the last.
    Loop,
    /// Ping-pong: reverse direction at both ends.
    Zigzag,
    /// Play once, then despawn the entity.
    Once,
}

impl Animatable {
    pub fn new(animation: Handle<WzFrameAnimationAsset>, mode: Playback) -> Self {
        Self {
            animation,
            frame: 0,
            elapsed: 0.0,
            mode,
            direction: 1,
        }
    }
}

/// Emitted (targeted at the entity) when an [`Animatable`] with [`Playback::Once`]
/// reaches its final frame and is about to be despawned.
///
/// Opt-in: only entities carrying a matching observer react. Spawners add the
/// observer via `commands.entity(e).observe(handler)` if they care about completion.
#[derive(EntityEvent)]
pub struct AnimationFinished {
    pub entity: Entity,
}

pub fn tick_animations(
    time: Res<Time>,
    animations: Res<Assets<WzFrameAnimationAsset>>,
    images: Res<Assets<Image>>,
    mut commands: Commands,
    mut query: Query<(Entity, &mut Animatable, &mut Sprite, &mut Anchor)>,
) {
    let dt = time.delta_secs();

    for (entity, mut anim, mut sprite, mut anchor) in &mut query {
        // Asset not loaded yet -> nothing to do.
        let Some(asset) = animations.get(&anim.animation) else {
            continue;
        };
        let frames = &asset.frames;
        let n = frames.len();
        if n == 0 {
            continue;
        }
        // Clamp instead of panicking on an out-of-range index.
        if anim.frame >= n {
            anim.frame = n - 1;
        }

        // Advance based on the CURRENT frame's own duration.
        let delay = frames[anim.frame].delay as f32 / 1000.0;
        anim.elapsed += dt;
        let due = if delay <= 0.0 {
            true // bad data: flip fast but bounded (1 frame/tick)
        } else {
            anim.elapsed >= delay
        };

        if due {
            anim.elapsed = 0.0;
            match anim.mode {
                Playback::Loop => anim.frame = (anim.frame + 1) % n,
                Playback::Zigzag if n > 1 => {
                    let next = anim.frame as isize + anim.direction;
                    if next >= n as isize {
                        anim.direction = -1;
                        anim.frame = n - 2;
                    } else if next < 0 {
                        anim.direction = 1;
                        anim.frame = 1;
                    } else {
                        anim.frame = next as usize;
                    }
                }
                Playback::Once if anim.frame >= n - 1 => {
                    commands.trigger(AnimationFinished { entity });
                    commands.entity(entity).despawn();
                    continue;
                }
                _ => anim.frame = (anim.frame + 1).min(n - 1),
            }
        }

        // Apply the current frame every tick: image + origin-as-anchor.
        let frame = &frames[anim.frame];
        let Some(image) = images.get(&frame.image) else {
            continue; // image not loaded yet -> try next tick
        };
        let size = image.size_f32();
        sprite.image = frame.image.clone();
        anchor.0 = Vec2::new(
            frame.origin.x / size.x - 0.5,
            0.5 - frame.origin.y / size.y, // y-flip: WZ origin is y-down
        );
    }
}
