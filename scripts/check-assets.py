"""Check release metadata, shared artwork and local website links without extra packages."""
from html.parser import HTMLParser
from pathlib import Path
from urllib.parse import urlsplit, unquote
import json
import re

ROOT = Path(__file__).resolve().parent.parent
version = re.search(r'^version = "([^"]+)"', (ROOT/'Cargo.toml').read_text(), re.M)[1]
assert json.loads((ROOT/'tauri.conf.json').read_text())['version'] == version
assert (ROOT/f'releases/v{version}.md').is_file()
for name in ['logo.ico', 'logo.png', 'logo.svg']:
    data = (ROOT/'resources'/name).read_bytes()
    for directory in ['docs/assets', 'ui/assets']:
        assert (ROOT/directory/name).read_bytes() == data, f'Artwork mismatch: {directory}/{name}'
for name in ['LICENSE', 'THIRD_PARTY_NOTICES.md', 'THIRD_PARTY_LICENSES.txt']:
    assert (ROOT/name).stat().st_size > 100


class Document(HTMLParser):
    def __init__(self):
        super().__init__()
        self.links = []
        self.ids = set()
        self.lang = None

    def handle_starttag(self, tag, attrs):
        attrs = dict(attrs)
        if tag == 'html':
            self.lang = attrs.get('lang')
        if 'id' in attrs:
            assert attrs['id'] not in self.ids, f'Duplicate id: {attrs["id"]}'
            self.ids.add(attrs['id'])
        for attribute in ['href', 'src']:
            if attribute in attrs:
                self.links.append(attrs[attribute])


for directory in ['docs', 'ui']:
    folder = ROOT/directory
    docs = {}
    for path in folder.glob('*.html'):
        doc = Document()
        doc.feed(path.read_text(encoding='utf-8'))
        assert doc.lang == 'zh-CN'
        docs[path] = doc
    for path, doc in docs.items():
        for link in doc.links:
            url = urlsplit(link)
            if url.scheme or url.netloc:
                continue
            target = (path.parent/unquote(url.path)).resolve() if url.path else path
            if target.is_dir():
                target /= 'index.html'
            assert target.is_relative_to(folder) and target.exists(), f'Broken link in {path}: {link}'
            if url.fragment and target in docs:
                assert url.fragment in docs[target].ids, f'Missing anchor in {path}: {link}'
print(f'Version {version}, artwork copies, licenses and local UI/Pages links verified.')
