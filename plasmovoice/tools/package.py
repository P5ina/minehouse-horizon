#!/usr/bin/env python3
"""Package the mod as a source-only `.hnmod`, the file Horizon's launcher installs and builds.

The archive holds `horizon-native.json` and `mixins/voicechat/` (its `.rs` and `.toml` files, the
engine crate's included), with fixed dates and modes, so the same sources always give the same bytes.
`--install` also copies it into the launcher's native mods folder.
"""
import hashlib
import json
from pathlib import Path
import shutil
import sys
import tomllib
import zipfile

ROOT = Path(__file__).resolve().parents[1]
INSTALLED = Path.home() / '.local/share/horizon/horizon-native-mods'


def package():
    compat = json.loads((ROOT / 'compatibility.json').read_text())
    bundle = ROOT / 'mixins' / 'voicechat'
    mixin = tomllib.loads((bundle / 'horizon-mixin.toml').read_text())
    mod = mixin['mod']
    if mixin.get('compatibility', {}).get('horizon_revision') != compat['horizon_revision']:
        raise SystemExit('horizon-mixin.toml and compatibility.json pin different Horizon revisions')
    if (bundle / 'engine' / 'Cargo.lock').exists():
        raise SystemExit('remove mixins/voicechat/engine/Cargo.lock: a mod crate builds on Horizon\'s lock')
    # No `hooks`: the mod reads game fields beyond the native hook points, so it
    # is pinned to the Horizon commit it was built and tested on.
    manifest = dict(schema=1, id=mod['id'], version=mod['version'], game=compat['game'],
                    horizon=compat['horizon_revision'], mixins=compat['horizon_mixins_revision'])
    files = {'horizon-native.json': (json.dumps(manifest, indent=2, sort_keys=True) + '\n').encode()}
    for path in sorted(bundle.rglob('*')):
        if path.is_file() and path.suffix in ('.rs', '.toml'):
            files[path.relative_to(ROOT).as_posix()] = path.read_bytes()
    out = ROOT / 'target' / f"{mod['id']}-{mod['version']}.hnmod"
    out.parent.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(out, 'w', compression=zipfile.ZIP_DEFLATED) as archive:
        for name, content in sorted(files.items()):
            info = zipfile.ZipInfo(name, (2026, 1, 1, 0, 0, 0))
            info.compress_type = zipfile.ZIP_DEFLATED
            info.external_attr = 0o100644 << 16
            archive.writestr(info, content)
    result = dict(file=str(out), files=len(files), size=out.stat().st_size,
                  sha256=hashlib.sha256(out.read_bytes()).hexdigest())
    if '--install' in sys.argv[1:]:
        INSTALLED.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(out, INSTALLED / f"{mod['id']}.hnmod")
        result['installed'] = str(INSTALLED / f"{mod['id']}.hnmod")
    print(json.dumps(result, indent=2))


if __name__ == '__main__':
    package()
