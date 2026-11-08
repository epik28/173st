#!/usr/bin/env python3
"""Emit the deterministic COLOR-001 native input corpus as ASCII TSV."""

import random
import struct
import sys


def f64hex(value):
    return struct.pack(">d", value).hex()


def climate_cases():
    seeds = [0, 1, -1, -(1 << 63), (1 << 63) - 1]
    points = [(0, 0), (1, 1), (-1, -1), (30_000_000, 30_000_000),
              (-30_000_000, -30_000_000), (30_000_000, -30_000_000),
              (-30_000_000, 30_000_000)]
    boundary = [-512, -256, -128, -64, -16, -1, 0, 1, 16, 64, 128, 256, 512]
    points.extend((x, z) for x in boundary for z in boundary)
    for index in range(256 - len(points)):
        x = ((index * 7919 + 12345) % 60_000_001) - 30_000_000
        z = ((index * 104729 + 67891) % 60_000_001) - 30_000_000
        points.append((x, z))
    for seed_index, seed in enumerate(seeds):
        for point_index, (x, z) in enumerate(points):
            yield f"c{seed_index:02d}-{point_index:03d}\tclimate\t{seed}\t{x}\t{z}"


def palette_cases():
    edge_pairs = [
        (0.0, 0.0), (1.0, 1.0), (0.0, 1.0), (1.0, 0.0),
        (float.fromhex("0x0.0000000000001p-1022"), 1.0),
        (1.0, float.fromhex("0x0.0000000000001p-1022")),
        (struct.unpack(">d", bytes.fromhex("3fefffffffffffff"))[0], 1.0),
        (1.0, struct.unpack(">d", bytes.fromhex("3fefffffffffffff"))[0]),
        (0.5, 0.5), (2.0 ** -52, 1.0 - 2.0 ** -52),
    ]
    pairs = list(edge_pairs)
    rng = random.Random(0xC010_001)
    while len(pairs) < 320:
        pairs.append((rng.random(), rng.random()))
    for index, (temperature, rainfall) in enumerate(pairs):
        yield f"p{index:03d}\tpalette\t{f64hex(temperature)}\t{f64hex(rainfall)}"


def main():
    rows = list(climate_cases()) + list(palette_cases())
    if len(rows) != 1600:
        raise RuntimeError(f"unexpected corpus size: {len(rows)}")
    sys.stdout.write("\n".join(rows) + "\n")


if __name__ == "__main__":
    main()
