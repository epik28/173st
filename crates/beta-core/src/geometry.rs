//! Beta 1.7.3-compatible vectors and axis-aligned bounding boxes.
//!
//! Floating-point operation order follows the pinned client's `bt`, `eq`, and
//! `vf` methods. Vector lengths use the client's `MathHelper.sqrt_double`
//! behavior, which narrows the square root to `f32` before returning it.

use crate::math::sqrt_f64_to_f32;

const INTERMEDIATE_DELTA_SQUARED_MIN: f64 = 1.0000000116860974E-7;
const VECTOR_LENGTH_EPSILON: f64 = 1.0E-4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    X,
    Y,
    Z,
}

impl Axis {
    const fn index(self) -> usize {
        match self {
            Self::X => 0,
            Self::Y => 1,
            Self::Z => 2,
        }
    }
}

/// A three-dimensional vector. `new` matches the original public factory and
/// canonicalizes negative zero; results from arithmetic preserve their raw bits.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vec3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Vec3 {
    #[must_use]
    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Self {
            x: canonical_zero(x),
            y: canonical_zero(y),
            z: canonical_zero(z),
        }
    }

    fn raw(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    #[must_use]
    pub fn vector_to(self, other: Self) -> Self {
        Self::raw(other.x - self.x, other.y - self.y, other.z - self.z)
    }

    #[must_use]
    pub fn cross(self, other: Self) -> Self {
        Self::raw(
            self.y * other.z - self.z * other.y,
            self.z * other.x - self.x * other.z,
            self.x * other.y - self.y * other.x,
        )
    }

    #[must_use]
    pub fn add_xyz(self, x: f64, y: f64, z: f64) -> Self {
        Self::raw(self.x + x, self.y + y, self.z + z)
    }

    #[must_use]
    pub fn normalize(self) -> Self {
        let length = f64::from(sqrt_f64_to_f32(
            self.x * self.x + self.y * self.y + self.z * self.z,
        ));
        if length < VECTOR_LENGTH_EPSILON {
            Self::raw(0.0, 0.0, 0.0)
        } else {
            Self::raw(self.x / length, self.y / length, self.z / length)
        }
    }

    #[must_use]
    pub fn length(self) -> f64 {
        f64::from(sqrt_f64_to_f32(
            self.x * self.x + self.y * self.y + self.z * self.z,
        ))
    }

    #[must_use]
    pub fn distance(self, other: Self) -> f64 {
        let x = other.x - self.x;
        let y = other.y - self.y;
        let z = other.z - self.z;
        f64::from(sqrt_f64_to_f32(x * x + y * y + z * z))
    }

    #[must_use]
    pub fn distance_squared(self, other: Self) -> f64 {
        let x = other.x - self.x;
        let y = other.y - self.y;
        let z = other.z - self.z;
        x * x + y * y + z * z
    }

    #[must_use]
    pub fn intermediate(self, end: Self, axis: Axis, plane: f64) -> Option<Self> {
        let start_component = self.component(axis);
        let delta = end.component(axis) - start_component;
        if delta * delta < INTERMEDIATE_DELTA_SQUARED_MIN {
            return None;
        }
        let fraction = (plane - start_component) / delta;
        if !(0.0..=1.0).contains(&fraction) {
            return None;
        }
        Some(Self::raw(
            self.x + (end.x - self.x) * fraction,
            self.y + (end.y - self.y) * fraction,
            self.z + (end.z - self.z) * fraction,
        ))
    }

    fn component(self, axis: Axis) -> f64 {
        match axis {
            Axis::X => self.x,
            Axis::Y => self.y,
            Axis::Z => self.z,
        }
    }
}

fn canonical_zero(value: f64) -> f64 {
    if value == 0.0 { 0.0 } else { value }
}

/// Axis-aligned box in native min-x/min-y/min-z/max-x/max-y/max-z order.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Aabb {
    pub min: [f64; 3],
    pub max: [f64; 3],
}

impl Aabb {
    /// Constructs a box without normalizing or changing signed-zero coordinates.
    #[must_use]
    pub const fn new(min: [f64; 3], max: [f64; 3]) -> Self {
        Self { min, max }
    }

    /// Expands the box only in the direction of each signed displacement.
    #[must_use]
    pub fn swept(self, x: f64, y: f64, z: f64) -> Self {
        let mut min = self.min;
        let mut max = self.max;
        for (axis, delta) in [x, y, z].into_iter().enumerate() {
            if delta < 0.0 {
                min[axis] += delta;
            }
            if delta > 0.0 {
                max[axis] += delta;
            }
        }
        Self::new(min, max)
    }

    /// Expands equally on both sides, preserving the native add/subtract order.
    #[must_use]
    pub fn expanded(self, x: f64, y: f64, z: f64) -> Self {
        Self::new(
            [self.min[0] - x, self.min[1] - y, self.min[2] - z],
            [self.max[0] + x, self.max[1] + y, self.max[2] + z],
        )
    }

    #[must_use]
    pub fn offset(self, x: f64, y: f64, z: f64) -> Self {
        Self::new(
            [self.min[0] + x, self.min[1] + y, self.min[2] + z],
            [self.max[0] + x, self.max[1] + y, self.max[2] + z],
        )
    }

    /// Clips a movement delta against this obstacle, using `moving` as the box
    /// being translated. Touching faces count as eligible for clipping.
    #[must_use]
    pub fn clip(self, moving: Self, axis: Axis, mut delta: f64) -> f64 {
        let i = axis.index();
        let j = (i + 1) % 3;
        let k = (i + 2) % 3;
        if self.max[j] <= moving.min[j]
            || self.min[j] >= moving.max[j]
            || self.max[k] <= moving.min[k]
            || self.min[k] >= moving.max[k]
        {
            return delta;
        }
        if delta > 0.0 && moving.max[i] <= self.min[i] {
            let gap = self.min[i] - moving.max[i];
            if gap < delta {
                delta = gap;
            }
        } else if delta < 0.0 && moving.min[i] >= self.max[i] {
            let gap = self.max[i] - moving.min[i];
            if gap > delta {
                delta = gap;
            }
        }
        delta
    }

    #[must_use]
    pub fn intersects(self, other: Self) -> bool {
        other.max[0] > self.min[0]
            && other.min[0] < self.max[0]
            && other.max[1] > self.min[1]
            && other.min[1] < self.max[1]
            && other.max[2] > self.min[2]
            && other.min[2] < self.max[2]
    }

    #[must_use]
    pub fn contains(self, point: Vec3) -> bool {
        point.x > self.min[0]
            && point.x < self.max[0]
            && point.y > self.min[1]
            && point.y < self.max[1]
            && point.z > self.min[2]
            && point.z < self.max[2]
    }

    #[must_use]
    pub fn average_edge(self) -> f64 {
        let x = self.max[0] - self.min[0];
        let y = self.max[1] - self.min[1];
        let z = self.max[2] - self.min[2];
        (x + y + z) / 3.0
    }

    /// Intersects the segment against the six box planes. Equal-distance ties
    /// retain native face iteration order: 4, 5, 0, 1, 2, 3.
    #[must_use]
    pub fn ray_intersect(self, start: Vec3, end: Vec3) -> Option<RayHit> {
        let candidates = [
            (Axis::X, self.min[0], 4),
            (Axis::X, self.max[0], 5),
            (Axis::Y, self.min[1], 0),
            (Axis::Y, self.max[1], 1),
            (Axis::Z, self.min[2], 2),
            (Axis::Z, self.max[2], 3),
        ];
        let mut closest: Option<RayHit> = None;
        let mut closest_distance = 0.0;
        for (axis, plane, face) in candidates {
            let Some(position) = start.intermediate(end, axis, plane) else {
                continue;
            };
            let idx = axis.index();
            let other_a = (idx + 1) % 3;
            let other_b = (idx + 2) % 3;
            if position.component(Axis::from_index(other_a)) < self.min[other_a]
                || position.component(Axis::from_index(other_a)) > self.max[other_a]
                || position.component(Axis::from_index(other_b)) < self.min[other_b]
                || position.component(Axis::from_index(other_b)) > self.max[other_b]
            {
                continue;
            }
            // `eq.a(bt,bt)` compares with `bt.d(bt)` (squared distance); using
            // the narrowed f32 square root can collapse distinct near ties.
            let distance_squared = start.distance_squared(position);
            if closest.is_none() || distance_squared < closest_distance {
                closest = Some(RayHit { face, position });
                closest_distance = distance_squared;
            }
        }
        closest
    }
}

impl Axis {
    fn from_index(index: usize) -> Self {
        match index {
            0 => Self::X,
            1 => Self::Y,
            _ => Self::Z,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RayHit {
    pub face: u8,
    pub position: Vec3,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn box_unit() -> Aabb {
        Aabb::new([0.0; 3], [1.0; 3])
    }

    #[test]
    fn constructors_preserve_the_native_signed_zero_rules() {
        let vector = Vec3::new(-0.0, 1.0, -0.0);
        assert_eq!(vector.x.to_bits(), 0.0_f64.to_bits());
        assert_eq!(vector.z.to_bits(), 0.0_f64.to_bits());
        let bounds = Aabb::new([-0.0, 0.0, 0.0], [1.0, 1.0, 1.0]);
        assert_eq!(bounds.min[0].to_bits(), (-0.0_f64).to_bits());
        assert_eq!(
            Vec3::new(1.0, 0.0, 0.0)
                .vector_to(Vec3::new(0.0, 0.0, 0.0))
                .x
                .to_bits(),
            (-1.0_f64).to_bits()
        );
    }

    #[test]
    fn vector_length_normalization_and_intermediate_thresholds_match_native() {
        let v = Vec3::new(1.0, 2.0, 2.0);
        assert_eq!(v.length().to_bits(), 3.0_f64.to_bits());
        assert_eq!(v.normalize(), Vec3::new(1.0 / 3.0, 2.0 / 3.0, 2.0 / 3.0));
        assert_eq!(
            Vec3::new(0.000_099_9, 0.0, 0.0).normalize(),
            Vec3::new(0.0, 0.0, 0.0)
        );
        let epsilon_above = f64::from(f32::from_bits(0.000_1_f32.to_bits() + 1));
        assert_eq!(
            Vec3::new(epsilon_above, 0.0, 0.0).normalize(),
            Vec3::new(1.0, 0.0, 0.0)
        );

        let start = Vec3::new(0.0, 0.0, 0.0);
        let just_below = f64::from_bits(INTERMEDIATE_DELTA_SQUARED_MIN.to_bits() - 1).sqrt();
        let just_above = f64::from_bits(INTERMEDIATE_DELTA_SQUARED_MIN.to_bits() + 1).sqrt();
        assert_eq!(
            start.intermediate(Vec3::new(just_below, 0.0, 0.0), Axis::X, 0.0),
            None
        );
        assert!(
            start
                .intermediate(Vec3::new(just_above, 0.0, 0.0), Axis::X, 0.0)
                .is_some()
        );
        assert_eq!(
            start.intermediate(Vec3::new(1.0, 0.0, 0.0), Axis::X, 1.000_001),
            None
        );
    }

    #[test]
    fn box_sweep_clip_contact_intersection_and_contains_use_strict_edges() {
        let obstacle = Aabb::new([2.0, 0.0, 0.0], [3.0, 1.0, 1.0]);
        let moving = Aabb::new([0.0, 0.0, 0.0], [1.0, 1.0, 1.0]);
        assert_eq!(
            moving.swept(2.0, -1.0, 0.0),
            Aabb::new([0.0, -1.0, 0.0], [3.0, 1.0, 1.0])
        );
        assert_eq!(obstacle.clip(moving, Axis::X, 2.0), 1.0);
        assert_eq!(obstacle.clip(moving, Axis::X, -2.0), -2.0);
        assert!(!moving.intersects(obstacle));
        assert!(!moving.contains(Vec3::new(0.0, 0.5, 0.5)));
        assert!(moving.contains(Vec3::new(0.5, 0.5, 0.5)));
        assert_eq!(moving.average_edge(), 1.0);
        assert_eq!(
            moving.expanded(0.5, 1.0, 1.5),
            Aabb::new([-0.5, -1.0, -1.5], [1.5, 2.0, 2.5])
        );
        assert_eq!(
            moving.offset(1.0, 2.0, 3.0),
            Aabb::new([1.0, 2.0, 3.0], [2.0, 3.0, 4.0])
        );
        assert!(moving.intersects(Aabb::new([0.5, 0.5, 0.5], [1.5, 1.5, 1.5])));
    }

    #[test]
    fn rays_hit_all_faces_and_keep_native_tie_order() {
        let b = box_unit();
        for (start, end, face, point) in [
            (
                Vec3::new(0.5, -1.0, 0.5),
                Vec3::new(0.5, 2.0, 0.5),
                0,
                Vec3::new(0.5, 0.0, 0.5),
            ),
            (
                Vec3::new(0.5, 2.0, 0.5),
                Vec3::new(0.5, -1.0, 0.5),
                1,
                Vec3::new(0.5, 1.0, 0.5),
            ),
            (
                Vec3::new(0.5, 0.5, -1.0),
                Vec3::new(0.5, 0.5, 2.0),
                2,
                Vec3::new(0.5, 0.5, 0.0),
            ),
            (
                Vec3::new(0.5, 0.5, 2.0),
                Vec3::new(0.5, 0.5, -1.0),
                3,
                Vec3::new(0.5, 0.5, 1.0),
            ),
            (
                Vec3::new(-1.0, 0.5, 0.5),
                Vec3::new(2.0, 0.5, 0.5),
                4,
                Vec3::new(0.0, 0.5, 0.5),
            ),
            (
                Vec3::new(2.0, 0.5, 0.5),
                Vec3::new(-1.0, 0.5, 0.5),
                5,
                Vec3::new(1.0, 0.5, 0.5),
            ),
        ] {
            let hit = b.ray_intersect(start, end).unwrap();
            assert_eq!(hit.face, face);
            assert_eq!(hit.position, point);
        }
        let tie = b
            .ray_intersect(Vec3::new(-1.0, -1.0, 0.5), Vec3::new(2.0, 2.0, 0.5))
            .unwrap();
        assert_eq!(tie.face, 4);
        let from_inside = b
            .ray_intersect(Vec3::new(0.5, 0.5, 0.5), Vec3::new(2.0, 0.5, 0.5))
            .unwrap();
        assert_eq!(from_inside.face, 5);
        assert_eq!(from_inside.position, Vec3::new(1.0, 0.5, 0.5));
        assert_eq!(
            b.ray_intersect(Vec3::new(-1.0, 2.0, 0.5), Vec3::new(2.0, 2.0, 0.5)),
            None
        );
    }

    #[test]
    fn ray_near_ties_compare_squared_distance_for_thin_boxes_on_every_axis() {
        let cases = [
            (
                Aabb::new([0.0, 0.0, 0.0], [1.0e-6, 1.0, 1.0]),
                Vec3::new(1.0e6, 0.5, 0.5),
                Vec3::new(-1.0e6, 0.5, 0.5),
                5,
            ),
            (
                Aabb::new([0.0, 0.0, 0.0], [1.0, 1.0e-6, 1.0]),
                Vec3::new(0.5, 1.0e6, 0.5),
                Vec3::new(0.5, -1.0e6, 0.5),
                1,
            ),
            (
                Aabb::new([0.0, 0.0, 0.0], [1.0, 1.0, 1.0e-6]),
                Vec3::new(0.5, 0.5, 1.0e6),
                Vec3::new(0.5, 0.5, -1.0e6),
                3,
            ),
        ];
        for (bounds, start, end, expected_face) in cases {
            assert_eq!(
                bounds.ray_intersect(start, end).unwrap().face,
                expected_face
            );
        }
    }
}
