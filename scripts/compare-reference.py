"""Compare the reproduced reference excerpt with the supplied PDF.

Requires pdfplumber. Run after building Carbon and compiling
examples/referencia/trecho.cb to tmp/pdfs/comparacao.pdf.
This checks typography/layout tolerances, not pixel identity.
"""
from pathlib import Path
import pdfplumber

root = Path(__file__).resolve().parents[1]
with pdfplumber.open(root / 'examples/referencia/original.pdf') as ref, pdfplumber.open(root / 'tmp/pdfs/comparacao.pdf') as actual:
    a, b = ref.pages[0], actual.pages[0]
    assert (a.width, a.height) == (b.width, b.height) == (612, 792)
    expected, rendered = a.extract_words()[:24], b.extract_words()[:24]
    assert len(rendered) == len(expected) == 24
    horizontal, vertical = [], []
    for source, target in zip(expected, rendered):
        assert source['text'] == target['text'], (source, target)
        horizontal.append(abs(source['x0'] - target['x0']))
        vertical.append(abs(source['top'] - target['top']))
    assert max(horizontal) < 0.75, max(horizontal)
    assert max(vertical) < 0.02, max(vertical)
    fonts = {c['fontname'].split('+')[-1] for c in b.chars}
    assert {'CMR17', 'CMR12', 'CMR10', 'CMBX12'} <= fonts, fonts
    print(f'24 reference words: max dx={max(horizontal):.4f} pt, max dy={max(vertical):.4f} pt; Computer Modern and Letter verified.')

    # Match the nine booktabs rules of the three tables on the reference page.
    assert len(a.lines) == len(b.lines) == 9
    for source, target in zip(a.lines, b.lines):
        for key in ['x0', 'x1', 'top']:
            assert abs(source[key] - target[key]) < 0.02, (key, source, target)
    print('All 9 booktabs rules match within 0.02 pt.')
