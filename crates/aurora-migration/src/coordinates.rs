use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CanonicalTransform {
    pub position: [f32; 3],
    pub rotation: [f32; 4],
}

/// Converts NWN Z-up coordinates to the bundle's right-handed Y-up basis.
///
/// `[x, y, z] -> [x, z, -y]`
pub fn canonical_position(source: [f32; 3]) -> Option<[f32; 3]> {
    source
        .iter()
        .all(|value| value.is_finite())
        .then_some([source[0], source[2], -source[1]])
}

pub fn source_position(canonical: [f32; 3]) -> Option<[f32; 3]> {
    canonical.iter().all(|value| value.is_finite()).then_some([
        canonical[0],
        -canonical[2],
        canonical[1],
    ])
}

/// Changes a source-space quaternion into the canonical basis.
///
/// The basis has determinant +1, so the vector part is transformed exactly like a position and
/// the scalar component is retained. Triangle winding therefore remains unchanged.
pub fn canonical_quaternion(source: [f32; 4]) -> Option<[f32; 4]> {
    source
        .iter()
        .all(|value| value.is_finite())
        .then_some([source[0], source[2], -source[1], source[3]])
}

pub fn source_quaternion(canonical: [f32; 4]) -> Option<[f32; 4]> {
    canonical.iter().all(|value| value.is_finite()).then_some([
        canonical[0],
        -canonical[2],
        canonical[1],
        canonical[3],
    ])
}

pub fn canonical_yaw(yaw_radians: f32) -> Option<[f32; 4]> {
    yaw_radians.is_finite().then(|| {
        let half = yaw_radians * 0.5;
        [0.0, half.sin(), 0.0, half.cos()]
    })
}

pub fn canonical_quarter_turn(orientation: u32) -> [f32; 4] {
    let yaw = (orientation % 4) as f32 * std::f32::consts::FRAC_PI_2;
    canonical_yaw(yaw).expect("a finite quarter-turn always has a quaternion")
}

pub fn canonical_transform(position: [f32; 3], yaw_radians: f32) -> Option<CanonicalTransform> {
    Some(CanonicalTransform {
        position: canonical_position(position)?,
        rotation: canonical_yaw(yaw_radians)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coordinate_basis_round_trips() {
        let source = [3.0, -7.0, 2.5];
        let canonical = canonical_position(source).expect("finite");
        assert_eq!(canonical, [3.0, 2.5, 7.0]);
        assert_eq!(source_position(canonical), Some(source));
    }

    #[test]
    fn orthogonal_rotations_are_normalized_and_deterministic() {
        let quarter = canonical_quarter_turn(1);
        let repeated = canonical_quarter_turn(5);
        assert_eq!(quarter, repeated);
        assert!((quarter[1] - std::f32::consts::FRAC_1_SQRT_2).abs() < 1e-6);
        assert!((quarter[3] - std::f32::consts::FRAC_1_SQRT_2).abs() < 1e-6);
    }

    #[test]
    fn maps_grid_axes_and_all_quarter_turns() {
        assert_eq!(
            canonical_position([10.0, 0.0, 0.0]),
            Some([10.0, 0.0, -0.0])
        );
        assert_eq!(
            canonical_position([0.0, 10.0, 0.0]),
            Some([0.0, 0.0, -10.0])
        );
        assert_eq!(
            canonical_position([0.0, 0.0, 10.0]),
            Some([0.0, 10.0, -0.0])
        );
        let turns = (0..4).map(canonical_quarter_turn).collect::<Vec<_>>();
        assert_eq!(turns[0], [0.0, 0.0, 0.0, 1.0]);
        assert!((turns[1][1] - std::f32::consts::FRAC_1_SQRT_2).abs() < 1e-6);
        assert!((turns[2][1] - 1.0).abs() < 1e-6);
        assert!((turns[3][1] - std::f32::consts::FRAC_1_SQRT_2).abs() < 1e-6);
        assert!((turns[3][3] + std::f32::consts::FRAC_1_SQRT_2).abs() < 1e-6);
    }

    #[test]
    fn converts_door_bearing_and_general_quaternion() {
        let door = canonical_transform([4.0, 7.0, 1.5], std::f32::consts::FRAC_PI_2)
            .expect("finite door transform");
        assert_eq!(door.position, [4.0, 1.5, -7.0]);
        assert!((door.rotation[1] - std::f32::consts::FRAC_1_SQRT_2).abs() < 1e-6);

        let source = [0.1, 0.2, 0.3, 0.9];
        let canonical = canonical_quaternion(source).expect("finite quaternion");
        assert_eq!(canonical, [0.1, 0.3, -0.2, 0.9]);
        assert_eq!(source_quaternion(canonical), Some(source));
    }

    #[test]
    fn basis_preserves_triangle_winding() {
        let a = canonical_position([0.0, 0.0, 0.0]).expect("a");
        let b = canonical_position([1.0, 0.0, 0.0]).expect("b");
        let c = canonical_position([0.0, 1.0, 0.0]).expect("c");
        let ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
        let ac = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
        let normal = [
            ab[1] * ac[2] - ab[2] * ac[1],
            ab[2] * ac[0] - ab[0] * ac[2],
            ab[0] * ac[1] - ab[1] * ac[0],
        ];
        assert_eq!(normal, [0.0, 1.0, 0.0]);
    }

    #[test]
    fn rejects_non_finite_coordinates_and_rotations() {
        assert!(canonical_position([0.0, f32::NAN, 0.0]).is_none());
        assert!(canonical_yaw(f32::INFINITY).is_none());
        assert!(canonical_quaternion([0.0, 0.0, f32::NAN, 1.0]).is_none());
    }
}
