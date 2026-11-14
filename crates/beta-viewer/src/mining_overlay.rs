//! Cube overlay geometry. Native texture stages occupy terrain tiles 240..249.
pub fn vertices(origin: [i32; 3], progress: f32) -> Vec<[f32; 8]> {
    let [x, y, z] = origin.map(|n| n as f32);
    let tile = 240 + (progress * 10.0).floor().clamp(0.0, 9.0) as u16;
    let u0 = f32::from(tile % 16) / 16.0;
    let u1 = (f32::from(tile % 16) * 16.0 + 15.99) / 256.0;
    let v0 = f32::from(tile / 16) / 16.0;
    let v1 = (f32::from(tile / 16) * 16.0 + 15.99) / 256.0;
    let positions = [
        [
            [x, y, z + 1.0],
            [x, y, z],
            [x + 1.0, y, z],
            [x + 1.0, y, z + 1.0],
        ],
        [
            [x + 1.0, y + 1.0, z + 1.0],
            [x + 1.0, y + 1.0, z],
            [x, y + 1.0, z],
            [x, y + 1.0, z + 1.0],
        ],
        [
            [x, y + 1.0, z],
            [x + 1.0, y + 1.0, z],
            [x + 1.0, y, z],
            [x, y, z],
        ],
        [
            [x, y + 1.0, z + 1.0],
            [x, y, z + 1.0],
            [x + 1.0, y, z + 1.0],
            [x + 1.0, y + 1.0, z + 1.0],
        ],
        [
            [x, y + 1.0, z + 1.0],
            [x, y + 1.0, z],
            [x, y, z],
            [x, y, z + 1.0],
        ],
        [
            [x + 1.0, y, z + 1.0],
            [x + 1.0, y, z],
            [x + 1.0, y + 1.0, z],
            [x + 1.0, y + 1.0, z + 1.0],
        ],
    ];
    let uv = [
        [[u0, v1], [u0, v0], [u1, v0], [u1, v1]],
        [[u1, v1], [u1, v0], [u0, v0], [u0, v1]],
        [[u1, v0], [u0, v0], [u0, v1], [u1, v1]],
        [[u0, v0], [u0, v1], [u1, v1], [u1, v0]],
        [[u1, v0], [u0, v0], [u0, v1], [u1, v1]],
        [[u0, v1], [u1, v1], [u1, v0], [u0, v0]],
    ];
    let mut result = Vec::with_capacity(36);
    for face in 0..6 {
        for i in [0, 1, 2, 0, 2, 3] {
            let p = positions[face][i];
            let t = uv[face][i];
            result.push([p[0], p[1], p[2], t[0], t[1], 0.5, 0.5, 0.5]);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn texture_stages_and_cube_geometry() {
        let a = vertices([4, 5, 6], 0.05);
        let b = vertices([4, 5, 6], 0.95);
        assert_eq!(a.len(), 36);
        assert_eq!(a[0][0..3], [4.0, 5.0, 7.0]);
        assert_eq!(a[0][3], 0.0);
        assert_eq!(b[0][3], 9.0 / 16.0);
    }
}
