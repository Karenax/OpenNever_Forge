#!/usr/bin/env python3
"""Generate the redistributable Lot 1 MOD/HAK/TLK fixture."""

from __future__ import annotations

import argparse
import hashlib
import json
import struct
from pathlib import Path


FIELD_CEXOSTRING = 10
FIELD_RESREF = 11
FIELD_CEXOLOCSTRING = 12
FIELD_LIST = 15
ERF_HEADER_SIZE = 160


def put_u32(buffer: bytearray, offset: int, value: int) -> None:
    buffer[offset : offset + 4] = struct.pack("<I", value)


def append_string(data: bytearray, value: str) -> int:
    encoded = value.encode("utf-8")
    offset = len(data)
    data.extend(struct.pack("<I", len(encoded)))
    data.extend(encoded)
    return offset


def append_resref(data: bytearray, value: str) -> int:
    encoded = value.encode("ascii")
    if len(encoded) > 255:
        raise ValueError("ResRef exceeds one-byte fixture limit")
    offset = len(data)
    data.append(len(encoded))
    data.extend(encoded)
    return offset


def append_locstring(data: bytearray, value: str) -> int:
    encoded = value.encode("utf-8")
    offset = len(data)
    data.extend(struct.pack("<IIIII", 16 + len(encoded), 0xFFFFFFFF, 1, 0, len(encoded)))
    data.extend(encoded)
    return offset


def build_module_ifo() -> bytes:
    labels = (
        "Mod_MinGameVer",
        "Mod_Name",
        "Mod_Description",
        "Mod_Tag",
        "Mod_CustomTlk",
        "Mod_Entry_Area",
        "Mod_HakList",
        "Mod_Hak",
    )
    field_data = bytearray()
    fields = (
        (FIELD_CEXOSTRING, 0, append_string(field_data, "1.69")),
        (FIELD_CEXOLOCSTRING, 1, append_locstring(field_data, "OpenNever Forge Lot 1")),
        (
            FIELD_CEXOLOCSTRING,
            2,
            append_locstring(field_data, "Redistributable synthetic dependency fixture"),
        ),
        (FIELD_CEXOSTRING, 3, append_string(field_data, "OPENNEVER_LOT1")),
        (FIELD_CEXOSTRING, 4, append_string(field_data, "forge_dialog")),
        (FIELD_RESREF, 5, append_resref(field_data, "startarea")),
        (FIELD_LIST, 6, 0),
        (FIELD_CEXOSTRING, 7, append_string(field_data, "forge_assets")),
    )

    struct_offset = 56
    field_offset = struct_offset + 2 * 12
    label_offset = field_offset + len(fields) * 12
    field_data_offset = label_offset + len(labels) * 16
    field_indices_offset = field_data_offset + len(field_data)
    field_indices_size = 7 * 4
    list_indices_offset = field_indices_offset + field_indices_size
    list_indices_size = 8
    result = bytearray(list_indices_offset + list_indices_size)
    result[0:4] = b"IFO "
    result[4:8] = b"V3.2"
    for offset, value in (
        (8, struct_offset),
        (12, 2),
        (16, field_offset),
        (20, len(fields)),
        (24, label_offset),
        (28, len(labels)),
        (32, field_data_offset),
        (36, len(field_data)),
        (40, field_indices_offset),
        (44, field_indices_size),
        (48, list_indices_offset),
        (52, list_indices_size),
    ):
        put_u32(result, offset, value)

    result[struct_offset : struct_offset + 12] = struct.pack("<III", 0xFFFFFFFF, 0, 7)
    result[struct_offset + 12 : struct_offset + 24] = struct.pack("<III", 0, 7, 1)
    for index, (field_type, label_index, data) in enumerate(fields):
        start = field_offset + index * 12
        result[start : start + 12] = struct.pack("<III", field_type, label_index, data)
    for index, label in enumerate(labels):
        encoded = label.encode("ascii")
        start = label_offset + index * 16
        result[start : start + len(encoded)] = encoded
    result[field_data_offset:field_indices_offset] = field_data
    for index in range(7):
        put_u32(result, field_indices_offset + index * 4, index)
    put_u32(result, list_indices_offset, 1)
    put_u32(result, list_indices_offset + 4, 1)
    return bytes(result)


def build_erf(file_type: bytes, resources: tuple[tuple[str, int, bytes], ...]) -> bytes:
    entry_count = len(resources)
    key_offset = ERF_HEADER_SIZE
    resource_offset = key_offset + entry_count * 24
    data_offset = resource_offset + entry_count * 8
    result = bytearray(data_offset)
    result[0:4] = file_type
    result[4:8] = b"V1.0"
    put_u32(result, 16, entry_count)
    put_u32(result, 20, ERF_HEADER_SIZE)
    put_u32(result, 24, key_offset)
    put_u32(result, 28, resource_offset)
    put_u32(result, 32, 126)
    put_u32(result, 36, 215)

    cursor = data_offset
    for index, (resref, resource_type, payload) in enumerate(resources):
        encoded = resref.encode("ascii")
        if len(encoded) > 16:
            raise ValueError("ResRef exceeds 16-byte ERF limit")
        key_start = key_offset + index * 24
        result[key_start : key_start + len(encoded)] = encoded
        put_u32(result, key_start + 16, index)
        result[key_start + 20 : key_start + 22] = struct.pack("<H", resource_type)
        resource_start = resource_offset + index * 8
        put_u32(result, resource_start, cursor)
        put_u32(result, resource_start + 4, len(payload))
        result.extend(payload)
        cursor += len(payload)
    return bytes(result)


def build_tlk() -> bytes:
    text = b"OpenNever Forge synthetic custom TLK"
    header_size = 20
    entry_size = 40
    result = bytearray(header_size + entry_size)
    result[0:4] = b"TLK "
    result[4:8] = b"V3.0"
    result[8:20] = struct.pack("<III", 0, 1, header_size + entry_size)
    result[20:60] = struct.pack("<I16sIIIIf", 1, b"", 0, 0, 0, len(text), 0.0)
    result.extend(text)
    return bytes(result)


def file_record(root: Path, path: Path) -> dict[str, object]:
    payload = path.read_bytes()
    return {
        "path": path.relative_to(root).as_posix(),
        "sha256": hashlib.sha256(payload).hexdigest().upper(),
        "sizeBytes": len(payload),
    }


def write_fixture(root: Path, force: bool = False) -> dict[str, object]:
    module_path = root / "module" / "forge_lot1.mod"
    hak_path = root / "user" / "hak" / "forge_assets.hak"
    tlk_path = root / "user" / "tlk" / "forge_dialog.tlk"
    manifest_path = root / "manifest.json"
    outputs = (module_path, hak_path, tlk_path, manifest_path)
    existing = [path for path in outputs if path.exists()]
    if existing and not force:
        joined = ", ".join(str(path) for path in existing)
        raise FileExistsError(f"fixture outputs already exist: {joined}")

    for path in outputs:
        path.parent.mkdir(parents=True, exist_ok=True)
    module_path.write_bytes(build_erf(b"MOD ", (
        ("module", 2014, build_module_ifo()),
        ("forge_start", 2009, b'#include "forge_shared"\nvoid main() { ForgeHello(); }\n'),
        ("forge_start", 2010, b"NCS V1.0\x00\x01\x02\x03"),
        ("forge_shared", 2009, b"void ForgeHello() { SpeakString(\"Forge\"); }\n"),
    )))
    hak_path.write_bytes(build_erf(b"HAK ", ()))
    tlk_path.write_bytes(build_tlk())
    manifest = {
        "schemaVersion": 1,
        "fixture": "lot1_custom_tlk",
        "license": "CC0-1.0",
        "generator": "scripts/generate_lot1_fixture.py",
        "expected": {
            "moduleName": "OpenNever Forge Lot 1",
            "moduleTag": "OPENNEVER_LOT1",
            "entryArea": "startarea",
            "hakFiles": ["forge_assets"],
            "customTlk": "forge_dialog",
            "customTlkText": "OpenNever Forge synthetic custom TLK",
            "resolvedDependencies": 2,
            "scripts": 2,
            "nss": 2,
            "ncs": 1,
        },
        "files": [file_record(root, path) for path in (module_path, hak_path, tlk_path)],
    }
    manifest_path.write_text(
        json.dumps(manifest, ensure_ascii=False, indent=2) + "\n", encoding="utf-8", newline="\n"
    )
    return manifest


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output", type=Path)
    parser.add_argument("--force", action="store_true", help="replace only the known fixture files")
    arguments = parser.parse_args()
    manifest = write_fixture(arguments.output.resolve(), arguments.force)
    print(json.dumps(manifest, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
