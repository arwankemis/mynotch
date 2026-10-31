"""Check repository refs and distribution contents before public publication."""
import hashlib
import argparse
import io
import re
import subprocess
import sys
import zipfile
from pathlib import Path, PurePosixPath

ROOT = Path(__file__).resolve().parents[1]
DOCS = {'README.md', 'PANDUAN.md', 'PANDUAN-MOD.md', 'PORTABILITAS.md', 'PENGEMBANGAN.md'}
IMAGES = {'README.md', 'calendar.jpg', 'clipboard.jpg', 'music.jpg', 'notes.jpg',
          'reminders.jpg', 'settings.jpg', 'stage-0.jpg', 'stage-1.jpg', 'weather.jpg'}
ROOT_FILES = {'.gitignore', 'README.md', 'CHANGELOG.md', 'LICENSE', 'SHA256SUMS.txt',
              'THIRD_PARTY_NOTICES.md', 'mynotch.exe', 'libunwind.dll', 'RELEASE_NOTES.md'}
INTERNAL = re.compile(r'(?i)(?:^|/)(?:_archive|__pycache__|data|preview-data|scratch|mockups|superpowers|qa-[^/]*)(?:/|$)|(?:^|/)(?:AUDIT[-.]|LAPORAN[-.]|HANDOFF[-.]|PROMPT[-.]|WORK-|READY-TEST|RENCANA-|FIX-|BENCHMARK-|PENGUJIAN\.)')
LOCAL_PATH = re.compile(r'(?i)(?:[a-z]:[\\/](?:Users|home)[\\/][A-Za-z0-9_.-]+|/home/[A-Za-z0-9_.-]+|rea[_]poni[_]nusa)')
REPORT = re.compile(r'(?im)^\s*(?:#{1,6}\s*)?(?:Permintaan pengguna:|User requests:|Handoff Gravity|Laporan pekerjaan|Work ledger|Dokumen Handoff)')
SECRET = re.compile(r'-----BEGIN (?:RSA |OPENSSH |EC )?PRIVATE KEY-----|\bgh[pousr]_[A-Za-z0-9]{30,}\b|\bgithub_pat_[A-Za-z0-9_]{30,}\b')
TEXT = {'.md', '.txt', '.json', '.toml', '.lock', '.rs', '.lua', '.ps1', '.py', '.cjs', '.cs', '.csproj', '.config', '.svg', '.html', '.yml', '.yaml'}


# Fingerprints identify local-only integration tokens without publishing their names.
PRIVATE_TOKEN_DIGESTS = frozenset(['353b4dc738308030243445e365f5582cbdb6a521c64b243065c842e1c696ce8e', '451a1a74c4c42bb87c095bbd1c4c8e5dfb0c033435076cb29a39a12507d6183f', '4ff01ac6e72dd460b1ac0a3f0ad388a01b8a9f891e30a52d6f83c56508e4aeac', '889f998ebf5fdd1608ad764894a18dddb1218784c84707c4f9627f0dd4e07fd3', '94823f6cf97162661db0f99129851053496291c1edc4ebd6cc36830b1c3332c5', 'cdac4c4c59cc83120cd29efc7f4fad105454986ad4530df29157a25a5928fecb', 'd3c5adb62e809ec57aaf109b0fc0d4dc13c05c888302bec5401ae1a087af3f73', 'd9030165f86fa69df4dca9f4e5b2e2bfe3c450dce202253b24d758e64e1c2619', 'e31dc8a4a77744f610c7307438e5aca4dca5af222d700337e0133c8e6c9f853f', 'f4ee00fbeeeb7fa97e4c06fbfdbd4dc2c9bc98d8db0ab41d69ad1d22004aa7e8', 'f705c4668e998e9755a87fd241ef74f35031691a3fea37f803c8d5be0982dea3'])

def check_file(name, data, package=False, depth=0):
    errors = []
    tokens = re.findall(rb'[A-Za-z][A-Za-z0-9_]*', name.encode().lower() + b' ' + data.replace(b'\x00', b'').lower())
    if any(hashlib.sha256(token).hexdigest() in PRIVATE_TOKEN_DIGESTS for token in set(tokens)):
        errors.append(f'{name}: local-only integration identifier')
    name = name.replace('\\', '/')
    parts = PurePosixPath(name).parts
    if name.startswith('/') or '..' in parts or re.match(r'^[A-Za-z]:', name):
        return [f'{name}: unsafe path']
    if parts and parts[0].lower() == 'mynotch':
        name = '/'.join(parts[1:])
    if not name or name.endswith('/'):
        return errors
    # source/data contains shipped holiday fixtures, not user runtime data.
    if INTERNAL.search(name) and not re.fullmatch(r'source/data/holidays-\d{4}\.json', name):
        errors.append(f'{name}: internal artifact path')
    if re.search(r'(?i)(?:^|/)(?:AGENTS\.md|RULES\.md|\.env(?:\.[^/]*)?)$|\.(?:log|tmp|pdb|dmp|pyc|pyo)$', name):
        errors.append(f'{name}: local instructions, configuration, or diagnostics')
    if name.startswith('docs/'):
        relative = name[5:]
        if relative not in DOCS and not (relative.startswith('images/') and relative[7:] in IMAGES):
            errors.append(f'{name}: documentation not on public allowlist')
    if '/' not in name and name not in ROOT_FILES:
        errors.append(f'{name}: unexpected root file')
    allowed_dirs = {'docs', 'examples', 'licenses'} if package else {'docs', 'examples', 'licenses', 'scripts', 'source', 'release', 'vendor', '.github'}
    if '/' in name and name.split('/')[0] not in allowed_dirs:
        errors.append(f'{name}: unexpected directory')
    if package and re.search(r'(?i)(power-provider|AGENTS\.md|\.env(?:\.|$))', name):
        errors.append(f'{name}: private runtime or configuration')
    if PurePosixPath(name).suffix.lower() in TEXT or name.endswith('.gitignore'):
        text = data.decode('utf-8', errors='replace')
        if LOCAL_PATH.search(text):
            errors.append(f'{name}: machine-specific path or workspace reference')
        if REPORT.search(text):
            errors.append(f'{name}: internal report content')
        if SECRET.search(text):
            errors.append(f'{name}: credential material')
        if name.startswith('docs/') and re.search(r'(?i)handoff', text):
            errors.append(f'{name}: internal runtime documentation')
    if name.lower().endswith('.zip'):
        if depth >= 3:
            return errors + [f'{name}: nested archive depth exceeded']
        try:
            with zipfile.ZipFile(io.BytesIO(data)) as archive:
                if sum(i.file_size for i in archive.infolist()) > 256 * 1024 * 1024:
                    return errors + [f'{name}: archive too large to audit']
                for item in archive.infolist():
                    if item.is_dir():
                        continue
                    # Example mod ZIPs contain mod.json/main.lua at their root.
                    inner = item.filename
                    if name.startswith('examples/'):
                        inner = 'examples/mods/package/' + inner
                    errors.extend(f'{name}!{e}' for e in check_file(inner, archive.read(item), package, depth + 1))
        except (zipfile.BadZipFile, RuntimeError) as error:
            errors.append(f'{name}: unreadable archive ({type(error).__name__})')
    return errors


def git(*args):
    return subprocess.check_output(['git', '-C', str(ROOT), *args])


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    group = parser.add_mutually_exclusive_group()
    group.add_argument('--git-ref', default=None)
    group.add_argument('--index', action='store_true')
    group.add_argument('--directory', type=Path)
    group.add_argument('--archive', type=Path)
    args = parser.parse_args()
    errors, count = [], 0
    if args.archive:
        with zipfile.ZipFile(args.archive) as archive:
            for item in archive.infolist():
                if not item.is_dir():
                    errors.extend(check_file(item.filename, archive.read(item), True))
                    count += 1
    elif args.directory:
        for path in args.directory.rglob('*'):
            if path.is_file():
                errors.extend(check_file(path.relative_to(args.directory).as_posix(), path.read_bytes(), True))
                count += 1
    else:
        ref = args.git_ref or 'HEAD'
        paths = git('ls-files', '-z') if args.index else git('ls-tree', '-rz', '--name-only', ref)
        for raw in paths.split(b'\0'):
            if not raw:
                continue
            name = raw.decode('utf-8')
            blob = git('show', (':' if args.index else ref + ':') + name)
            errors.extend(check_file(name, blob))
            count += 1
    for error in errors:
        print('BLOCKED: ' + error, file=sys.stderr)
    if errors:
        print(f'Publication blocked: {len(errors)} finding(s).', file=sys.stderr)
        return 1
    print(f'Publication check passed: {count} files. Review images and release text manually before publishing.')
    return 0


if __name__ == '__main__':
    sys.exit(main())
