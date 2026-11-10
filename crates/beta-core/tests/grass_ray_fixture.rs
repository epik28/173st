use beta_core::block::{
    BlockHitKind, BlockScene, BlockSceneBounds, BlockSceneChunk, RayResult, RayShape,
};
use beta_core::chunk::{BlockData, ChunkData};
use beta_core::geometry::Vec3;

const INPUT: &str = include_str!("../../../tests/fixtures/block-placement/grass-rays-input.tsv");
const NATIVE: &str = include_str!("../../../tests/fixtures/block-placement/grass-rays-native.tsv");

#[test]
fn tallgrass_ray_shape_matches_every_native_fixture_hit_and_miss() {
    let input_lines: Vec<_> = INPUT.lines().collect();
    let native_lines: Vec<_> = NATIVE.lines().collect();
    assert_eq!(
        input_lines[0],
        "#BLOCK_GRASS_RAY_INPUT\t1\taf1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c"
    );
    assert_eq!(
        native_lines[0],
        "#BLOCK_GRASS_RAY_NATIVE\t1\taf1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c"
    );
    assert_eq!(input_lines[1], "#id\tsx\tsy\tsz\tex\tey\tez");
    assert_eq!(
        native_lines[1],
        "#id\tkind\tx\ty\tz\tside\tpx\tpy\tpz\tpx_bits\tpy_bits\tpz_bits"
    );
    assert_eq!(input_lines.len(), 16);
    assert_eq!(native_lines.len(), 16);

    let mut chunk = ChunkData::new();
    chunk
        .set(
            0,
            10,
            0,
            BlockData {
                id: 31,
                metadata: 0,
                ..BlockData::default()
            },
        )
        .unwrap();
    let scene = BlockScene::new(
        BlockSceneBounds::new(0, 0, 1, 1).unwrap(),
        vec![BlockSceneChunk {
            x: 0,
            z: 0,
            data: chunk,
        }],
    )
    .unwrap();
    let shapes = |id, metadata| match (id, metadata) {
        (31, 0) => RayShape::TallGrass,
        _ => RayShape::Unsupported,
    };

    for (input, native) in input_lines[2..].iter().zip(&native_lines[2..]) {
        let input: Vec<_> = input.split('\t').collect();
        let expected: Vec<_> = native.split('\t').collect();
        assert_eq!(input.len(), 7);
        assert_eq!(expected.len(), 12);
        assert_eq!(input[0], expected[0]);
        let coordinate = |index: usize| input[index].parse::<f64>().unwrap();
        let origin = Vec3::new(coordinate(1), coordinate(2), coordinate(3));
        let end = Vec3::new(coordinate(4), coordinate(5), coordinate(6));
        let actual = scene
            .raycast_segment_with_shapes(origin, end, false, &shapes)
            .unwrap();
        if expected[1] == "miss" {
            assert_eq!(actual, RayResult::Miss, "{}", input[0]);
            continue;
        }
        assert_eq!(expected[1], "block", "{}", input[0]);

        let RayResult::Hit(hit) = actual else {
            panic!(
                "{}: expected native tallgrass hit, got {actual:?}",
                input[0]
            );
        };
        assert_eq!(hit.kind, BlockHitKind::TallGrass, "{}", input[0]);
        assert_eq!((hit.x, hit.y, hit.z), (0, 10, 0), "{}", input[0]);
        assert_eq!(hit.face.to_string(), expected[5], "{}", input[0]);
        assert_eq!(
            hit.position.x.to_bits(),
            u64::from_str_radix(expected[9], 16).unwrap(),
            "{}",
            input[0]
        );
        assert_eq!(
            hit.position.y.to_bits(),
            u64::from_str_radix(expected[10], 16).unwrap(),
            "{}",
            input[0]
        );
        assert_eq!(
            hit.position.z.to_bits(),
            u64::from_str_radix(expected[11], 16).unwrap(),
            "{}",
            input[0]
        );
    }
}
