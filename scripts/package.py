#!/usr/bin/env python3
"""Create a deterministic source-only hnmod (no game or downloaded assets)."""
from pathlib import Path
import json
import zipfile
import tomllib

root = Path(__file__).resolve().parents[1]
version=tomllib.loads((root / "horizon-mixin.toml").read_text())["mod"]["version"]
manifest = dict(schema=1, id="valorant_horizon", version=version, game="26.3",
                horizon="41f822b659789780f58d32c63ddfb6887ea1b8bf",
                mixins="5a64af938ec86b00d53d16a408aabc0038fb3e57")
files = [root / "horizon-mixin.toml", root / "engine/Cargo.toml"]
files += sorted((root / "engine/src").rglob("*.rs"))
files += sorted((root / "helpers").glob("*.rs"))
files += sorted((root / "bridge").glob("*.rs"))
files += [root / "spatial/Cargo.toml", *sorted((root / "spatial/src").rglob("*.rs"))]
files += [root / "native-state/Cargo.toml", *sorted((root / "native-state/src").rglob("*.rs"))]
out = root / f"dist/valorant_horizon-{version}.hnmod"
out.parent.mkdir(exist_ok=True)
with zipfile.ZipFile(out, "w", compression=zipfile.ZIP_DEFLATED) as archive:
    def put(name, data):
        info = zipfile.ZipInfo(name, (2026, 11, 6, 0, 0, 0))
        info.compress_type = zipfile.ZIP_DEFLATED
        info.external_attr = 0o100644 << 16
        archive.writestr(info, data)
    put("horizon-native.json", json.dumps(manifest, indent=2).encode())
    for file in files:
        put("mixins/valorant_horizon/" + file.relative_to(root).as_posix(), file.read_bytes())
print(out)
