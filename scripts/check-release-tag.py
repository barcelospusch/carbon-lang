"""Fail if a release tag differs from the package version or is not checked out."""
import os
import subprocess
import tomllib
from pathlib import Path

root = Path(__file__).resolve().parents[1]
version = tomllib.loads((root / 'Cargo.toml').read_text(encoding='utf-8-sig'))['package']['version']
tag = os.environ.get('RELEASE_TAG', '')
if tag != 'v' + version:
    raise SystemExit(f'Expected tag v{version}; got {tag!r}')
head = subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=root, text=True).strip()
commit = subprocess.check_output(['git', 'rev-list', '-n', '1', tag], cwd=root, text=True).strip()
if head != commit:
    raise SystemExit('The checked-out commit does not match the release tag.')
print(f'Release {tag} at {commit}')
