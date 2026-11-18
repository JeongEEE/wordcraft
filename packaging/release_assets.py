"""Bounded release receipts and payload verification, independent of the build host."""
import hashlib
import json
from pathlib import Path

FONTS_SHA = 'b076417b5880bd4154bd22f00695b1d28c6bea83'
MAX_PACKAGE = 2 << 30
MAX_RECEIPT = 32 * 1024


class DeployError(Exception):
    pass


def package_names(version, platform=None):
    names = {'macos': [f'word-{version}-macos-universal.dmg', f'word-cli-{version}-macos-universal.zip'],
             'windows': [f'word-{version}-windows-x64.msi', f'word-{version}-windows-x64-portable.zip']}
    return names[platform] if platform else names['macos'] + names['windows']


def receipt_name(version, platform):
    return f'word-{version}-{platform}-build.json'


def payload_names(version):
    return package_names(version) + [receipt_name(version, p) for p in ['macos', 'windows']]


def file_hash(path):
    if path.is_symlink() or not path.is_file() or not 0 < path.stat().st_size <= MAX_PACKAGE:
        raise DeployError(f'유효한 릴리스 파일이 아닙니다: {path.name}')
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def write_receipt(folder, version, source, platform, checks, i18n):
    data = dict(schema=1, version=version, source=source, platform=platform, fonts=FONTS_SHA, i18n=i18n, checks=checks,
                packages=[dict(name=n, size=(folder/n).stat().st_size, sha256=file_hash(folder/n))
                          for n in package_names(version, platform)])
    path = folder/receipt_name(version, platform)
    path.write_text(json.dumps(data, ensure_ascii=False, indent=2)+'\n', encoding='utf-8')
    return path


def validate_payloads(folder, version, source, i18n):
    for platform in ['macos', 'windows']:
        path = folder/receipt_name(version, platform)
        if path.is_symlink() or not path.is_file() or path.stat().st_size > MAX_RECEIPT:
            raise DeployError(f'빌드 증빙이 없거나 너무 큽니다: {path.name}')
        receipt = json.loads(path.read_text(encoding='utf-8'))
        if (receipt.get('schema') != 1 or receipt.get('version') != version or receipt.get('source') != source
                or receipt.get('platform') != platform or receipt.get('fonts') != FONTS_SHA or receipt.get('i18n') != i18n
                or not {'fork-identity', 'updater-tests', 'package-policy', 'cli-smoke'} <= set(receipt.get('checks', []))):
            raise DeployError(f'소스·버전·검증이 일치하지 않는 빌드 증빙입니다: {path.name}')
        rows = receipt.get('packages', [])
        if len(rows) != 2 or {row['name'] for row in rows} != set(package_names(version, platform)):
            raise DeployError(f'패키지 목록이 일치하지 않습니다: {path.name}')
        for row in rows:
            package = folder/row['name']
            if file_hash(package) != row['sha256'] or package.stat().st_size != row['size']:
                raise DeployError(f'빌드 증빙과 파일 해시가 다릅니다: {package.name}')


def write_checksums(folder, version):
    names = payload_names(version) + ['WordUpdate.json', 'WordUpdate.sig']
    (folder/'SHA256SUMS.txt').write_text(''.join(f'{file_hash(folder/n)}  {n}\n' for n in names), encoding='utf-8')


def validate_signed_files(folder, version):
    path = folder/'WordUpdate.json'
    if path.stat().st_size > MAX_RECEIPT:
        raise DeployError('업데이트 목록이 너무 큽니다.')
    data = json.loads(path.read_text(encoding='utf-8'))
    expected = set(package_names(version)) - {f'word-cli-{version}-macos-universal.zip'}
    rows = data.get('assets', [])
    if (data.get('schema') != 1 or data.get('repository') != 'JeongEEE/wordcraft'
            or data.get('version') != version or data.get('tag') != f'wordcraft-v{version}'
            or len(rows) != 3 or {r['name'] for r in rows} != expected):
        raise DeployError('서명된 업데이트 목록의 식별자 또는 파일 목록이 다릅니다.')
    for row in rows:
        p = folder/row['name']
        if file_hash(p) != row['sha256'] or p.stat().st_size != row['size']:
            raise DeployError('서명된 업데이트 목록의 해시가 다릅니다.')
    signature = (folder/'WordUpdate.sig').read_text(encoding='utf-8').strip()
    if len(signature) != 128 or any(c not in '0123456789abcdef' for c in signature):
        raise DeployError('업데이트 서명의 형식이 올바르지 않습니다.')
