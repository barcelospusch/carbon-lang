"""Compile every .cb example. Requires Tectonic + optional Mermaid runtime."""
import argparse
from pathlib import Path
import subprocess

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--binary', required=True, type=Path)
args = parser.parse_args()
root = Path(__file__).resolve().parents[1]
binary = args.binary.resolve()
output = root / 'output' / 'examples'
output.mkdir(parents=True, exist_ok=True)
for source in sorted((root / 'examples').rglob('*.cb')):
    relative = source.relative_to(root / 'examples')
    target = output / relative.with_suffix('.pdf')
    target.parent.mkdir(parents=True, exist_ok=True)
    subprocess.run([str(binary), str(source), '--output', str(target)], check=True)
    assert target.read_bytes().startswith(b'%PDF-'), target
    print(f'OK {relative}', flush=True)
