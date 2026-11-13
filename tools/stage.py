#!/usr/bin/env python3
"""Stage Horizon with MineHouse's mods applied: what the Horizon launcher's native build does
(docs/native-mods.md, "How a build runs"), for CI. Usage: stage.py SOURCE OUT [--mixins CLI]."""
import argparse, json, shutil, subprocess, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

def main():
    p = argparse.ArgumentParser()
    p.add_argument("source", type=Path, help="Horizon's source at horizon.json's revision")
    p.add_argument("out", type=Path, help="the staged (modded) workspace")
    p.add_argument("--mixins", default="horizon-mixins", help="the horizon-mixins CLI")
    a = p.parse_args()
    cfg = json.loads((ROOT / "horizon.json").read_text())
    # As the launcher's source archive: no links (horizon-mixins refuses them) and no .git.
    shutil.rmtree(a.source / ".git", ignore_errors=True)
    for path in list(a.source.rglob("*")):
        if path.is_symlink():
            path.unlink()
    (a.source / ".horizon-revision").write_text(cfg["horizon_revision"] + "\n")
    mods = a.out.parent / "mods-incoming"
    shutil.rmtree(mods, ignore_errors=True)
    mods.mkdir(parents=True)
    for m in cfg["mods"]:
        src = ROOT / m
        dst = mods / src.name
        shutil.copytree(src, dst, ignore=shutil.ignore_patterns("target", "Cargo.lock", ".gitignore"))
    shutil.rmtree(a.out, ignore_errors=True)
    r = subprocess.run([a.mixins, "apply", "--workspace", str(a.source), "--mods", str(mods), "--out", str(a.out)],
                       capture_output=True, text=True)
    sys.stdout.write(r.stdout)
    sys.stderr.write(r.stderr)
    sys.exit(r.returncode)

if __name__ == "__main__":
    main()
