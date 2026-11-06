#!/usr/bin/env python3
"""Build the static documentation demo from public Markdown. Requires pandoc."""
import json, re, subprocess
from pathlib import Path

root = Path(__file__).resolve().parent.parent
out = root / 'site/docs'
excluded = {'authored/http-api', 'authored/self-hosted-lore-auth', 'authored/production-presign-requirements', 'generated/environment'}
sources = sorted(path for folder in ('authored', 'generated', 'examples') for path in (root / 'docs' / folder).rglob('*.md'))
for key in excluded:
    for suffix in ('.html', '.md'):
        (out / (key.replace('/', '--') + suffix)).unlink(missing_ok=True)
pages = {}
template = (out / 'index.html').read_text()
for source in sources:
    key = source.relative_to(root / 'docs').as_posix()[:-3]
    if key in excluded:
        continue
    text = source.read_text()
    html = subprocess.run(['pandoc', '--from=gfm', '--to=html5', '--wrap=none'], input=text, text=True, capture_output=True, check=True).stdout
    title_match = re.search(r'<h[12][^>]*>(.*?)</h[12]>', html, re.S)
    title = re.sub(r'<[^>]+>', '', title_match.group(1)) if title_match else source.stem.replace('-', ' ').title()
    pages[key] = {'title': title, 'html': html, 'source': 'docs/' + key + '.md'}
    filename = 'index' if key == 'authored/introduction' else key.replace('/', '--')
    (out / (filename + '.md')).write_text(text)
    if filename != 'index':
        (out / (filename + '.html')).write_text(template)
(out / 'content.js').write_text('window.PX_DOCS = ' + json.dumps(pages, ensure_ascii=False) + ';\n')
print(f'Built {len(pages)} documentation pages and Markdown counterparts')
