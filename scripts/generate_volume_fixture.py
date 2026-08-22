#!/usr/bin/env python3
"""Génère les fixtures synthétiques à volume réel d'OpenNever Forge.

Sorties déterministes (aucune horloge, aucun aléa) sous fixtures/synthetic/volume/ :
- dialogue_narrative.json : topologie DLG de 1 000 nœuds avec cycles, liens
  partagés et nœuds inaccessibles volontaires ;
- area_dense.json : zone 16x15 avec plus de 420 instances réparties ;
- manifest.json : comptages et empreintes SHA-256.

Usage :
    python scripts/generate_volume_fixture.py [--output DIR] [--check]
"""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
from pathlib import Path

SEED = 20260821
ENTRY_COUNT = 399
REPLY_COUNT = 602
GRID_WIDTH = 16
GRID_HEIGHT = 15
INSTANCE_TARGET = 444


def build_dialogue() -> dict:
    entries: list[dict] = []
    replies: list[dict] = [
        {"id": index, "entries": []} for index in range(REPLY_COUNT)
    ]

    for index in range(ENTRY_COUNT - 1):
        entries.append({"id": index, "replies": [index]})
    for index in range(ENTRY_COUNT - 2):
        replies[index]["entries"] = [index + 1]

    replies[0]["entries"] = [1, 0]
    replies[2]["entries"] = [3, 1]
    replies[4]["entries"] = [5, 3]

    entries[50] = {"id": 50, "replies": [50, REPLY_COUNT - 3]}
    entries[60] = {"id": 60, "replies": [60, REPLY_COUNT - 3]}
    replies[REPLY_COUNT - 3]["entries"] = [150]

    entries[70] = {"id": 70, "replies": [70, REPLY_COUNT - 2]}
    entries[80] = {"id": 80, "replies": [80, REPLY_COUNT - 2]}
    replies[REPLY_COUNT - 2]["entries"] = [250]

    entries.append({"id": ENTRY_COUNT - 1, "replies": [REPLY_COUNT - 1]})
    replies[REPLY_COUNT - 1]["entries"] = []

    return {
        "resref": "narrative_main",
        "starting": [0],
        "expectedCycles": 3,
        "expectedUnreachable": ["entry:398", "reply:601"],
        "entries": entries,
        "replies": replies,
    }


def build_area() -> dict:
    plan: list[tuple[str, int]] = [
        ("placeable", 140),
        ("creature", 120),
        ("sound", 40),
        ("trigger", 30),
        ("waypoint", 30),
        ("door", 24),
        ("encounter", 20),
        ("store", 20),
        ("item", 20),
    ]
    total = sum(count for _, count in plan)
    if total != INSTANCE_TARGET:
        raise SystemExit(f"répartition invalide : {total} != {INSTANCE_TARGET}")

    instances: list[dict] = []
    cursor = 0
    for category, count in plan:
        for offset in range(count):
            linear = cursor + offset
            x = round((linear * 7 % (GRID_WIDTH * 10)) / 4.0 + 0.5, 3)
            y = round((linear * 13 % (GRID_HEIGHT * 10)) / 4.0 + 0.5, 3)
            instance: dict = {
                "category": category,
                "tag": f"vol_{category}_{offset:03d}",
                "resref": f"vol_{category}",
                "x": x,
                "y": y,
                "z": 0.0,
                "bearing": (linear * SEED) % 4 * 90.0,
            }
            if category == "door" and offset < 4:
                instance["transitionDestination"] = f"vol_zone_{offset}"
                instance["transitionFlags"] = 0
            instances.append(instance)
        cursor += count

    return {
        "resref": "market_district",
        "width": GRID_WIDTH,
        "height": GRID_HEIGHT,
        "tileset": "tcn01",
        "tileCount": GRID_WIDTH * GRID_HEIGHT,
        "instances": instances,
    }


def canonical_bytes(payload: dict) -> bytes:
    text = json.dumps(payload, ensure_ascii=False, indent=1, sort_keys=True)
    return (text + "\n").encode("utf-8")


def generate(output: Path) -> None:
    dialogue = build_dialogue()
    area = build_area()
    files = {
        "dialogue_narrative.json": canonical_bytes(dialogue),
        "area_dense.json": canonical_bytes(area),
    }
    counts = {
        "dialogueEntries": len(dialogue["entries"]),
        "dialogueReplies": len(dialogue["replies"]),
        "dialogueNodes": len(dialogue["entries"]) + len(dialogue["replies"]),
        "areaInstances": len(area["instances"]),
        "areaTiles": area["tileCount"],
    }
    manifest = {
        "schema": "opennever-volume-fixture@1",
        "seed": SEED,
        "counts": counts,
        "files": {
            name: hashlib.sha256(content).hexdigest()
            for name, content in files.items()
        },
    }
    output.mkdir(parents=True, exist_ok=True)
    payload = dict(files)
    payload["manifest.json"] = canonical_bytes(manifest)
    for name, content in payload.items():
        (output / name).write_bytes(content)
    print(json.dumps(counts, sort_keys=True))


def check(output: Path) -> int:
    manifest_path = output / "manifest.json"
    if not manifest_path.is_file():
        print(f"manifeste absent : {manifest_path}")
        return 1
    manifest = json.loads(manifest_path.read_bytes())
    failures: list[str] = []
    for name, expected in sorted(manifest["files"].items()):
        path = output / name
        if not path.is_file():
            failures.append(f"{name} absent")
            continue
        actual = hashlib.sha256(path.read_bytes()).hexdigest()
        if actual != expected:
            failures.append(f"{name}: empreinte {actual} != {expected}")
    counts = manifest["counts"]
    if counts["dialogueNodes"] < 1000:
        failures.append("moins de 1000 noeuds de dialogue")
    if counts["areaInstances"] <= 420:
        failures.append("zone pas assez dense (<421 instances)")
    if failures:
        print(" ; ".join(failures))
        return 1
    print(json.dumps(counts, sort_keys=True))
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, default=Path("fixtures/synthetic/volume"))
    parser.add_argument("--check", action="store_true")
    arguments = parser.parse_args()
    if arguments.check:
        return check(arguments.output)
    generate(arguments.output)
    return 0


if __name__ == "__main__":
    sys.exit(main())
