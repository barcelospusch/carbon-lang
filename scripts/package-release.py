"""Build a portable Carbon release archive, excluding caches and runtimes."""
import argparse
import hashlib
import os
import json
import subprocess
import tomllib
from pathlib import Path
import shutil
import tarfile
import tempfile
import zipfile

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--target', required=True)
parser.add_argument('--version', default=os.environ.get('RELEASE_TAG'))
parser.add_argument('--binary', required=True, type=Path)
parser.add_argument('--out', type=Path, default=Path('dist'))
args = parser.parse_args()
# Only platform/version identifiers are accepted as archive names.
for value in (args.target, args.version):
    if not value or not all(c.isascii() and (c.isalnum() or c in '.-_') for c in value):
        parser.error('invalid archive identifier')
root = Path(__file__).resolve().parents[1]
version = tomllib.loads((root / 'Cargo.toml').read_text(encoding='utf-8-sig'))['package']['version']
if args.version != 'v' + version:
    parser.error(f'expected version v{version}')
args.out.mkdir(parents=True, exist_ok=True)
name = f'carbon-{args.version}-{args.target}'
with tempfile.TemporaryDirectory() as temporary:
    stage = Path(temporary) / name
    stage.mkdir()
    shutil.copy2(args.binary, stage / ('carbon.exe' if 'windows' in args.target else 'carbon'))
    for file in ['README.md', 'LICENSE', 'CHANGELOG.md', 'THIRD_PARTY_NOTICES.md', 'CONTRIBUTING.md', 'SECURITY.md']:
        shutil.copy2(root / file, stage / file)
    for file in ['Cargo.toml', 'Cargo.lock']:
        shutil.copy2(root / file, stage / file)
    for folder in ['docs', 'examples', 'scripts', 'src', '.github']:
        shutil.copytree(root / folder, stage / folder, ignore=shutil.ignore_patterns('__pycache__', '*.pyc'))
    (stage / 'tools' / 'mermaid').mkdir(parents=True)
    for file in ['package.json', 'package-lock.json']:
        shutil.copy2(root / 'tools' / 'mermaid' / file, stage / 'tools' / 'mermaid' / file)
    metadata = json.loads(subprocess.check_output(['cargo', 'metadata', '--locked', '--format-version', '1', '--filter-platform', args.target], cwd=root, text=True))
    summaries = []
    for package in metadata['packages']:
        if package['name'] == 'carbon':
            continue
        location = Path(package['manifest_path']).parent
        files = {p for pattern in ('LICENSE*', 'license*', 'LICENCE*', 'licence*', 'COPYING*', 'NOTICE*') for p in location.glob(pattern) if p.is_file()}
        if package.get('license_file'):
            declared = location / package['license_file']
            if declared.is_file():
                files.add(declared)
        folder = stage / 'licenses' / (package['name'] + '-' + package['version'])
        folder.mkdir(parents=True, exist_ok=True)
        for file in files:
            shutil.copy2(file, folder / file.name)
        summaries.append({'name': package['name'], 'version': package['version'], 'license': package.get('license'), 'notices': sorted(p.name for p in files)})
    (stage / 'licenses' / 'dependencies.json').write_text(json.dumps(summaries, indent=2) + '\n', encoding='utf-8')
    if 'windows' in args.target:
        archive = args.out / f'{name}.zip'
        with zipfile.ZipFile(archive, 'w', compression=zipfile.ZIP_DEFLATED, strict_timestamps=False) as writer:
            for file in sorted(stage.rglob('*')):
                if file.is_file():
                    writer.write(file, file.relative_to(stage.parent))
    else:
        archive = args.out / f'{name}.tar.gz'
        with tarfile.open(archive, 'w:gz') as writer:
            writer.add(stage, arcname=name)
checksum = hashlib.sha256(archive.read_bytes()).hexdigest()
archive.with_name(archive.name + '.sha256').write_text(f'{checksum}  {archive.name}\n', encoding='ascii')
print(archive)
