//! Armor icon geometry from Beta 1.7.3's `uq.a(FZII)V` bytecode.

/// Return armor icon vertices in the icons atlas. Armor points use the native
/// 0..=20 scale, with full, half, and empty icons drawn right-to-left. No icons
/// are shown for zero armor. Each vertex is `[clip_x, clip_y, u, v]`.
#[must_use]
pub fn armor_vertices(width: u32, height: u32, gui_scale: u32, armor: i32) -> Vec<[f32; 4]> {
    if armor <= 0 {
        return Vec::new();
    }
    let scale = gui_scale.max(1);
    let logical_width = width.div_ceil(scale);
    let logical_height = height.div_ceil(scale);
    let s = scale as f32;
    let (w, h) = (width.max(1) as f32, height.max(1) as f32);
    let mut vertices = Vec::with_capacity(60);
    for i in 0..10 {
        let threshold = i * 2 + 1;
        let u = if threshold < armor {
            34.0
        } else if threshold == armor {
            25.0
        } else {
            16.0
        };
        let x = (logical_width as i32 / 2 + 91 - i * 8 - 9) as f32 * s;
        let y = (logical_height as i32 - 32) as f32 * s;
        let left = x / w * 2.0 - 1.0;
        let right = (x + 9.0 * s) / w * 2.0 - 1.0;
        let top = 1.0 - y / h * 2.0;
        let bottom = 1.0 - (y + 9.0 * s) / h * 2.0;
        let (u0, v0, u1, v1) = (u / 256.0, 9.0 / 256.0, (u + 9.0) / 256.0, 18.0 / 256.0);
        vertices.extend([
            [left, top, u0, v0],
            [right, top, u1, v0],
            [right, bottom, u1, v1],
            [left, top, u0, v0],
            [right, bottom, u1, v1],
            [left, bottom, u0, v1],
        ]);
    }
    vertices
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn armor_row_matches_native_icon_thresholds() {
        assert!(armor_vertices(800, 600, 2, 0).is_empty());
        for armor in 1..=20 {
            let quads = armor_vertices(800, 600, 2, armor);
            assert_eq!(quads.len(), 60);
            for (i, quad) in quads.chunks_exact(6).enumerate() {
                let threshold = i as i32 * 2 + 1;
                let expected = if threshold < armor {
                    34.0
                } else if threshold == armor {
                    25.0
                } else {
                    16.0
                };
                assert_eq!(quad[0][2], expected / 256.0, "armor {armor}, icon {i}");
            }
        }
    }

    #[test]
    fn armor_row_moves_from_right_to_left() {
        let q = armor_vertices(1281, 721, 3, 20);
        assert!(q[0][0] > q[6][0]);
        let width = 1281_u32;
        let expected_x = (width.div_ceil(3) as i32 / 2 + 91 - 9) as f32 * 3.0;
        assert_eq!(q[0][0], expected_x / width as f32 * 2.0 - 1.0);
    }
}
