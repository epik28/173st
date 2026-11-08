#!/usr/bin/env python3
"""Extract hash-pinned Beta palettes beside terrain.png; never overwrite resources."""
import argparse
import hashlib
import json
from pathlib import Path
import zipfile

from oracle import view_corpus

PALETTES = {
    'grasscolor.png': ('misc/grasscolor.png', 'd9b2d9073f06013b9f36b1c61a91678be478760abff31fae942ee52ad898ca1b'),
    'foliagecolor.png': ('misc/foliagecolor.png', '644a527dd7daea5a195bbed056a6a41e295b6043a05621f4c5c263a25fb171cc'),
}


def digest(data):
    return hashlib.sha256(data).hexdigest()


def prepare(root):
    jar, terrain = view_corpus.source(root)
    pending, paths = [], {}
    with zipfile.ZipFile(jar) as archive:
        for name, (member, expected) in PALETTES.items():
            contents = archive.read(member)
            if digest(contents) != expected:
                raise ValueError('original palette hash mismatch: ' + member)
            target = terrain.parent / name
            if target.is_symlink():
                raise ValueError('palette destination must not be a symlink: ' + str(target))
            if target.exists():
                if not target.is_file() or digest(target.read_bytes()) != expected:
                    raise ValueError('existing palette differs; refusing overwrite: ' + str(target))
            else:
                pending.append((target, contents))
            paths[name] = str(target)
    for target, contents in pending:
        with target.open('xb') as output:
            output.write(contents)
    return paths


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.parse_args()
    try:
        print(json.dumps(prepare(Path(__file__).resolve().parents[1]), indent=2))
    except (OSError, ValueError, KeyError, zipfile.BadZipFile) as error:
        parser.exit(1, str(error) + '\n')


if __name__ == '__main__':
    main()
