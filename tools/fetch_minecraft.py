#!/usr/bin/env python3
"""Download Minecraft (the version research-lock.json pins) from Mojang into a folder laid out like the
official launcher's game directory, plus the server jar, so horizon-prepare can compile Horizon's
exporters in CI. Nothing of it is published: only Horizon's own compiled classes leave the runner.
Usage: fetch_minecraft.py RESEARCH_LOCK OUT_DIR  (prints the server jar's path)."""
import hashlib, json, sys, urllib.request
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

def get(url):
    with urllib.request.urlopen(url, timeout=120) as r:
        return r.read()

def fetch(url, path, sha1=None):
    path = Path(path)
    if path.is_file() and (sha1 is None or hashlib.sha1(path.read_bytes()).hexdigest() == sha1):
        return
    data = get(url)
    if sha1 and hashlib.sha1(data).hexdigest() != sha1:
        sys.exit(f"{url}: SHA-1 differs")
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(data)

def main():
    lock = json.loads(Path(sys.argv[1]).read_text())["minecraft"]
    version, out = lock["version"], Path(sys.argv[2])
    # The version JSON Horizon pins, by its SHA-1: Mojang republishes them.
    sha1 = lock["manifest_sha1"]
    raw = get(f"https://piston-meta.mojang.com/v1/packages/{sha1}/{version}.json")
    if hashlib.sha1(raw).hexdigest() != sha1:
        sys.exit(f"{version}.json: SHA-1 differs")
    meta = json.loads(raw)
    vdir = out / "versions" / version
    vdir.mkdir(parents=True, exist_ok=True)
    (vdir / f"{version}.json").write_bytes(raw)
    jobs = [(meta["downloads"]["client"]["url"], vdir / f"{version}.jar", lock["client_sha1"])]
    server = out / "server" / f"server-{version}.jar"
    jobs.append((meta["downloads"]["server"]["url"], server, lock["server_sha1"]))
    for lib in meta["libraries"]:
        art = lib.get("downloads", {}).get("artifact")
        if art:
            jobs.append((art["url"], out / "libraries" / art["path"], art["sha1"]))
    index = meta["assetIndex"]
    jobs.append((index["url"], out / "assets" / "indexes" / f"{index['id']}.json", index["sha1"]))
    with ThreadPoolExecutor(16) as pool:
        list(pool.map(lambda j: fetch(*j), jobs))
    objects = json.loads((out / "assets" / "indexes" / f"{index['id']}.json").read_text())["objects"]
    def obj(o):
        h = o["hash"]
        fetch(f"https://resources.download.minecraft.net/{h[:2]}/{h}", out / "assets" / "objects" / h[:2] / h, h)
    with ThreadPoolExecutor(32) as pool:
        list(pool.map(obj, objects.values()))
    print(server)

if __name__ == "__main__":
    main()
