#!/usr/bin/env python3
"""Generate the bounded native LIGHT-001 input corpus."""

import sys


def samples():
    index = 0
    # Full nibble domain in both supported dimensions at three minimum values.
    for dimension in (0, -1):
        for block in range(16):
            for sky in range(16):
                for subtraction in range(16):
                    for minimum in (0, 7, 15):
                        yield (f"s{index:05d}\tsample\t{dimension}\t{block}\t{sky}\t"
                               f"{subtraction}\t{minimum}")
                        index += 1
        # Isolate all provider brightness-table entries through original fd.a(IIII).
        for level in range(16):
            yield f"table-{dimension}-{level}\tsample\t{dimension}\t0\t0\t0\t{level}"


def times():
    index = 0
    for tick in range(24_000):
        for partial, phex in ((0, "00000000"), (1, "3f800000")):
            yield f"ow-cycle-{index:05d}\ttime\t0\t{tick}\t{phex}"
            index += 1
    edge_times = [-(1 << 63), (1 << 63) - 1, -24_001, -1, 0, 1, 23_999, 24_000]
    partials = ("00000000", "80000000", "3f000000", "3f800000")
    for dimension in (0, -1):
        for tick in edge_times:
            for partial in partials:
                yield f"edge-time-{dimension}-{index:04d}\ttime\t{dimension}\t{tick}\t{partial}"
                index += 1


def emissions():
    for block_id in range(256):
        yield f"emit-{block_id:03d}\temission\t{block_id}"


def main():
    rows = list(samples()) + list(times()) + list(emissions())
    if len(rows) > 100_000 or len(set(row.split("\t", 1)[0] for row in rows)) != len(rows):
        raise RuntimeError("unexpected LIGHT-001 corpus shape/duplicate IDs")
    sys.stdout.write("\n".join(rows) + "\n")


if __name__ == "__main__":
    main()
