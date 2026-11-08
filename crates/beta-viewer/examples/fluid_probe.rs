use std::{
    io::{self, Read},
    process::ExitCode,
};

use beta_view::fluid::{self, FluidCell, FluidNeighborhood};

const INPUT: &str = include_str!("../../../tests/fixtures/fluid/input.tsv");
const LIMIT: usize = 1024;

fn samples() -> Vec<(String, FluidNeighborhood)> {
    let mut out = Vec::with_capacity(44);
    for id in [8, 9] {
        for metadata in 0..16 {
            out.push((format!("level-{id}-{metadata}"), base(id, metadata)));
        }
    }
    for name in [
        "east-source",
        "west-flow",
        "north-source",
        "south-flow",
        "diagonal-water",
        "four-sources",
        "water-above",
        "flow-column",
        "solid-east",
        "solid-west",
        "solid-north",
        "solid-south",
    ] {
        let mut n = base(9, 0);
        let water = |id, metadata| Some(FluidCell { id, metadata });
        match name {
            "east-source" => n.east = water(9, 0),
            "west-flow" => n.west = water(8, 7),
            "north-source" => n.north = water(9, 0),
            "south-flow" => n.south = water(8, 12),
            "diagonal-water" => n.south_east = water(9, 0),
            "four-sources" => {
                n.east = water(9, 0);
                n.west = water(9, 0);
                n.north = water(9, 0);
                n.south = water(9, 0);
            }
            "water-above" => n.above = water(9, 0),
            "flow-column" => {
                n.center = FluidCell { id: 8, metadata: 8 };
                n.below = water(8, 8);
            }
            "solid-east" => n.east = water(1, 0),
            "solid-west" => n.west = water(1, 0),
            "solid-north" => n.north = water(1, 0),
            "solid-south" => n.south = water(1, 0),
            _ => unreachable!(),
        }
        out.push((name.to_owned(), n));
    }
    out
}

fn base(id: u8, metadata: u8) -> FluidNeighborhood {
    FluidNeighborhood {
        center: FluidCell { id, metadata },
        below: None,
        above: None,
        east: None,
        west: None,
        north: None,
        south: None,
        north_east: None,
        north_west: None,
        south_east: None,
        south_west: None,
    }
}

fn run(input: &[u8]) -> Result<String, String> {
    if input.len() > LIMIT {
        return Err("input limit exceeded".into());
    }
    if !input.is_ascii() || input != INPUT.as_bytes() {
        return Err("input differs from frozen FLUID-001-C01 suite".into());
    }
    let mut output = String::new();
    for (name, neighborhood) in samples() {
        let mesh = fluid::mesh(0, 64, 0, neighborhood).map_err(|e| e.to_string())?;
        if mesh.vertices.is_empty() {
            return Err(format!("empty native case {name}"));
        }
        output.push_str(&format!(
            "case\t{name}\t{}\t{}\t1\t{}\n",
            neighborhood.center.id,
            neighborhood.center.metadata,
            mesh.vertices.len()
        ));
        for v in mesh.vertices {
            let words = [
                v.position[0].to_bits(),
                v.position[1].to_bits(),
                v.position[2].to_bits(),
                v.uv[0].to_bits(),
                v.uv[1].to_bits(),
                v.color,
                0,
                0,
            ];
            output.push_str(&format!("vertex\t{name}"));
            for word in words {
                output.push_str(&format!("\t{word:08x}"));
            }
            output.push('\n');
        }
    }
    Ok(output)
}

fn main() -> ExitCode {
    if std::env::args_os().len() != 1 {
        eprintln!("fluid_probe accepts stdin only");
        return ExitCode::FAILURE;
    }
    let mut input = Vec::new();
    if io::stdin()
        .take((LIMIT + 1) as u64)
        .read_to_end(&mut input)
        .is_err()
    {
        eprintln!("failed to read input");
        return ExitCode::FAILURE;
    }
    match run(&input) {
        Ok(output) => {
            print!("{output}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("fluid_probe: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn frozen_suite_matches_all_native_rows() {
        let actual = run(INPUT.as_bytes()).unwrap();
        let expected: String = include_str!("../../../tests/fixtures/fluid/native.tsv")
            .lines()
            .filter(|line| line.starts_with("case\t") || line.starts_with("vertex\t"))
            .map(|line| format!("{line}\n"))
            .collect();
        assert_eq!(actual, expected);
    }
    #[test]
    fn rejects_unfrozen_or_oversized_input_without_output() {
        assert!(run(b"bad\n").is_err());
        assert!(run(&vec![b'x'; LIMIT + 1]).is_err());
    }
}
