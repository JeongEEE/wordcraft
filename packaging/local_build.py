"""Build and test on the current desktop OS. Never install/remove the user's Word."""
import os
from contextlib import contextmanager
from pathlib import Path
import re
import tempfile

from release_assets import DeployError, FONTS_SHA, package_names

I18N_URL = 'https://github.com/storytold/craft-i18n.git'


def pinned_checkout(d, variable, url, sha, cache, label):
    configured = d.env.get(variable)
    folder = Path(configured).expanduser().resolve() if configured else d.common/cache/sha
    if not folder.exists():
        if configured:
            raise DeployError(f'{variable}가 없습니다: {folder}')
        folder.parent.mkdir(parents=True, exist_ok=True)
        d.command('git', 'clone', '--no-checkout', url, folder, capture=False, label=f'릴리스 {label} 다운로드')
        d.command('git', '-C', folder, 'checkout', '--detach', sha, capture=False, label=f'고정 {label} 버전 준비')
    actual = d.command('git', '-C', folder, 'rev-parse', 'HEAD').stdout.strip()
    dirty = d.command('git', '-C', folder, 'status', '--porcelain').stdout.strip()
    if actual != sha or dirty:
        raise DeployError(f'릴리스 {label}은 고정 커밋 {sha[:8]}의 깨끗한 체크아웃이어야 합니다: {folder}')
    return folder


def prepare_fonts(d):
    fonts = pinned_checkout(d, 'CRAFT_FONTS_DIR', 'https://github.com/storytold/craft-fonts.git', FONTS_SHA,
                            'word-release-fonts', '글꼴')
    d.env.update(CRAFT_FONTS_DIR=str(fonts), CRAFT_FONTS_REQUIRED='1')


def i18n_rev(d):
    # The pin comes from the commit being built, not the working tree.
    result = d.command('git', 'show', f'{d.sha}:craft-i18n.rev', optional=True)
    rev = result.stdout.strip()
    if result.returncode or not re.fullmatch(r'[0-9a-f]{40}', rev):
        raise DeployError('빌드할 소스의 craft-i18n.rev가 40자리 커밋이 아닙니다. 번역 없이 배포하지 않습니다.')
    return rev


def prepare_i18n(d):
    # Without craft-i18n the interface is English only; CRAFT_I18N_REQUIRED makes the build fail instead.
    i18n = pinned_checkout(d, 'CRAFT_I18N_DIR', I18N_URL, i18n_rev(d), 'word-release-i18n', '번역')
    d.env.update(CRAFT_I18N_DIR=str(i18n), CRAFT_I18N_REQUIRED='1')


@contextmanager
def committed_source(d):
    # Build only committed files. Untracked work and edits in the shared checkout
    # cannot become release input while another session uses it.
    with tempfile.TemporaryDirectory(prefix='word-build-source-') as scratch:
        source = Path(scratch)/'source'
        d.command('git', 'clone', '--quiet', '--shared', '--no-checkout', d.root, source)
        d.command('git', '-C', source, 'checkout', '--quiet', '--detach', d.sha)
        yield source


def build(d, output):
    prepare_fonts(d)
    prepare_i18n(d)
    d.assert_source()
    with committed_source(d) as source:
        return build_source(d, output, source)


def build_source(d, output, source):
    d.env.update(DIST=str(output), WORDCRAFT_VERSION=d.version, WORDCRAFT_BUILD_SHA=d.sha,
                 CARGO_TARGET_DIR=str(d.root/'target'))
    d.env.pop('WORD_UPDATE_SMOKE_PACKAGE', None)
    d.env.pop('WORD_UPDATE_SMOKE_INSTALL', None)
    def run(*args, **kwargs):
        return d.command(*args, cwd=source, **kwargs)

    run(os.sys.executable, 'packaging/test_fork_contract.py', capture=False, label='포크 설치 식별자 검증')
    run(d.cargo, 'test', '--locked', '-p', 'wordcraft-update', capture=False, label='자동 업데이트 회귀 검사')
    if d.args.platform == 'macos':
        run('bash', 'packaging/macos/package.sh', '--arch', 'universal', capture=False, label='macOS universal 빌드·DMG 패키징')
        # The signed binary that goes into the CLI ZIP.
        cli = d.root/f'target/macos-package/word-cli-{d.version}-macos-universal/word-cli'
        run(d.cargo, 'test', '--locked', '-p', 'wordcraft-update', '--lib', 'native_dmg_roundtrip', '--', '--ignored',
                  capture=False, label='DMG 업데이트·실패 시 복원 검증',
                  env=dict(d.env, WORD_UPDATE_SMOKE_PACKAGE=str(output/package_names(d.version, 'macos')[0])))
    else:
        run('pwsh', '-NoProfile', '-File', 'packaging/windows/package.ps1', '-Arch', 'x64',
            capture=False, label='Windows x64 빌드·MSI/ZIP 패키징')
        cli = d.root/'target/windows-package/x64/word-cli.exe'
        # The MSI's static policy check is part of package.ps1. Installation tests are
        # deliberately limited to a disposable machine; never touch a personal install.
        run(d.cargo, 'test', '--locked', '-p', 'wordcraft-update', '--lib', 'native_windows_portable_roundtrip', '--', '--ignored',
                  capture=False, label='포터블 업데이트·실패 시 복원 검증',
                  env=dict(d.env, WORD_UPDATE_SMOKE_PACKAGE=str(output/package_names(d.version, 'windows')[1])))
    if d.version not in d.command(cli, '--version').stdout:
        raise DeployError('빌드한 CLI의 버전이 다릅니다.')
    with tempfile.TemporaryDirectory(prefix='word-cli-smoke-') as scratch:
        document = Path(scratch)/'sample.docx'
        d.command(cli, 'run', '--template', 'sample', '--save', document, capture=False, label='DOCX 저장 검증')
        # Title of the built-in sample (crates/engine/src/sample.rs).
        if 'The Open Studio Handbook' not in d.command(cli, 'text', document).stdout:
            raise DeployError('CLI로 저장한 DOCX를 다시 열어 읽지 못했습니다.')
    d.assert_source()
    return ['fork-identity', 'updater-tests', 'package-policy', 'cli-smoke']
