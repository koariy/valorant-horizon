#!/usr/bin/env python3
"""Test in a temporary copy: native bundle crates must not ship Cargo.lock."""
from pathlib import Path
import shutil
import subprocess
import tempfile

root = Path(__file__).resolve().parents[1]
with tempfile.TemporaryDirectory(prefix="valorant-horizon-") as temporary:
    shutil.copytree(root / "spatial", Path(temporary) / "spatial", ignore=shutil.ignore_patterns("target", "Cargo.lock"))
    shutil.copytree(root / "native-state", Path(temporary) / "native-state", ignore=shutil.ignore_patterns("target", "Cargo.lock"))
    shutil.copytree(root / "helpers", Path(temporary) / "helpers")
    shutil.copytree(root / "bridge", Path(temporary) / "bridge")
    engine = Path(temporary) / "engine"
    shutil.copytree(root / "engine", engine, ignore=shutil.ignore_patterns("target", "Cargo.lock"))
    for arguments in [
        ["fmt", "--check"],
        ["clippy", "--all-targets", "--", "-D", "warnings"],
        ["test"],
        ["build", "--release"],
    ]:
        command = ["cargo", arguments[0], "--manifest-path", str(engine / "Cargo.toml"), *arguments[1:]]
        print("+ " + " ".join(command), flush=True)
        subprocess.run(command, check=True)
    subprocess.run(["rustfmt", "--edition", "2024", "--check", *map(str, sorted([*(root / "native-state/src").glob("*.rs"), *(root / "helpers").glob("*.rs"), *(root / "bridge").glob("*.rs")]))], check=True)
