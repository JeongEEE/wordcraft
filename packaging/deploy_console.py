"""Shared terminal presentation for native releases; no deployment side effects."""
from contextlib import contextmanager
from datetime import datetime
import os
import shlex
import subprocess
import sys
import tempfile
import time

from release_assets import package_names, receipt_name


def duration(seconds):
    seconds = int(seconds)
    return f'{seconds // 60}분 {seconds % 60:02d}초' if seconds >= 60 else f'{seconds}초'


def retry_command(args, version):
    windows = args.platform == 'windows'
    parts = ['.\\deploy-windows.ps1' if windows else './deploy-macos.sh']
    if version:
        parts += ['-Version', version] if windows else [version]
    for enabled, mac, win in [(args.draft, '--draft', '-Draft'), (args.finalize, '--finalize', '-Finalize'),
                               (args.dry_run, '--dry-run', '-DryRun')]:
        if enabled:
            parts.append(win if windows else mac)
    if args.key_file:
        quoted = "'" + args.key_file.replace("'", "''") + "'" if windows else shlex.quote(args.key_file)
        parts += ['-KeyFile' if windows else '--key-file', quoted]
    return ' '.join(parts)


class Console:
    def __init__(self):
        tty = sys.stdout.isatty() and os.environ.get('TERM') != 'dumb'
        self.animated = tty and (os.name != 'nt' or bool(os.environ.get('WT_SESSION') or os.environ.get('ANSICON')))
        self.color = self.animated and 'NO_COLOR' not in os.environ
        self.started = time.monotonic()
        self.log_path = None

    def text(self, text='', tone=None, file=None):
        colors = {'accent': '36', 'good': '32', 'warn': '33', 'bad': '31'}
        target = file or sys.stdout
        if self.color and target.isatty() and tone in colors:
            text = f'\x1b[{colors[tone]}m{text}\x1b[0m'
        print(text, file=target, flush=True)

    def banner(self, platform, finalize=False, dry_run=False):
        name = 'macOS' if platform == 'macos' else 'Windows'
        mode = '릴리스 마무리' if finalize else f'{name} 로컬 배포'
        self.text('\n' + '─' * 52, 'accent')
        self.text(f'  Word  /  {mode}' + ('  [dry-run]' if dry_run else ''), 'accent')
        self.text('─' * 52, 'accent')

    def start_log(self, common):
        folder = common/'word-deploy-logs'
        folder.mkdir(exist_ok=True)
        # Unique names keep retries and concurrent historical logs separate.
        with tempfile.NamedTemporaryFile(dir=folder, prefix=datetime.now().strftime('%Y%m%d-%H%M%S-'),
                                         suffix='.log', delete=False) as handle:
            self.log_path = folder/os.path.basename(handle.name)
            handle.write(b'Word local deployment: detailed subprocess output\n')
        self.text(f'  상세 로그: {self.log_path}')

    @contextmanager
    def step(self, number, total, label):
        self.text(f'\n[{number}/{total}] {label}', 'accent')
        started = time.monotonic()
        try:
            yield
        except BaseException:
            self.text(f'  중단 · {duration(time.monotonic() - started)}', 'bad')
            raise
        else:
            self.text(f'  완료 · {duration(time.monotonic() - started)}', 'good')

    def run_command(self, args, cwd, env, label):
        if self.log_path is None:
            # Standalone use retains the normal inherited subprocess streams.
            return subprocess.run(args, cwd=cwd, env=env)
        self.text(f'  > {label}')
        started = time.monotonic()
        with self.log_path.open('ab') as output:
            output.write(('\n$ ' + shlex.join(args) + '\n').encode('utf-8'))
            output.flush()
            with subprocess.Popen(args, cwd=cwd, env=env, stdout=output, stderr=subprocess.STDOUT) as process:
                frame, frames = 0, '|/-\\'
                try:
                    while True:
                        try:
                            process.wait(timeout=0.2)
                            break
                        except subprocess.TimeoutExpired:
                            if self.animated:
                                sys.stdout.write(f'\r\x1b[2K  {frames[frame % 4]} {label} · {duration(time.monotonic() - started)}')
                                sys.stdout.flush()
                                frame += 1
                except KeyboardInterrupt:
                    if process.poll() is None:
                        process.terminate()
                        try:
                            process.wait(timeout=5)
                        except subprocess.TimeoutExpired:
                            process.kill(); process.wait()
                    raise
                finally:
                    if self.animated:
                        sys.stdout.write('\r\x1b[2K'); sys.stdout.flush()
            code = process.returncode
        self.text(f'  {"성공" if code == 0 else "실패"} · {label} · {duration(time.monotonic() - started)}',
                  'good' if code == 0 else 'bad')
        return subprocess.CompletedProcess(args, code)

    def tail(self):
        if self.log_path is None:
            return ''
        with self.log_path.open('rb') as handle:
            handle.seek(0, os.SEEK_END)
            handle.seek(max(0, handle.tell() - 8192))
            return '\n'.join(handle.read().decode('utf-8', errors='replace').splitlines()[-16:])

    def result(self, deployment, status, release):
        d = deployment
        self.text('\n' + '─' * 52, 'good')
        title = '공개 완료' if status == 'published' else '초안 유지 · 공개 전'
        self.text(f'  Word {d.version}  /  {title}', 'good')
        self.text(f'  소요 시간: {duration(time.monotonic() - self.started)}')
        self.text(f'  소스: {d.sha}')
        names = {row['name'] for row in release['assets']}
        complete = {}
        for key, name in [('macos', 'macOS'), ('windows', 'Windows')]:
            expected = package_names(d.version, key) + [receipt_name(d.version, key)]
            complete[key] = set(expected) <= names
            self.text(f'  {name}: ' + ('업로드 완료' if complete[key] else '대기'))
        self.text('  업데이트 서명: ' + ('검증 완료' if status in {'ready', 'published'} else '마무리 필요'))
        self.text(f'  릴리스: {release["url"]}')
        self.text(f'  상세 로그: {self.log_path}')
        self.text('─' * 52, 'good')
        if status == 'published':
            return
        if not complete['windows']:
            self.text('\n다음 · Windows에서 빌드·업로드 (Developer PowerShell for VS)', 'accent')
            commands = [f'.\\deploy-windows.ps1 -Version {d.version} -Draft']
        elif not complete['macos']:
            self.text('\n다음 · MacBook에서 macOS 빌드·업로드', 'accent')
            commands = [f'./deploy-macos.sh {d.version} --draft']
        else:
            self.text('\n다음 · MacBook에서 서명·공개 마무리 (재빌드 없음)', 'accent')
            commands = [f'./deploy-macos.sh {d.version} --finalize']
        self.text('  레포지토리 폴더에서 순서대로 실행하세요. 미클론 PC는 먼저 my-custom을 clone하세요.')
        for command in ['git switch my-custom', 'git pull --ff-only origin my-custom', 'git rev-parse HEAD']:
            self.text(f'    {command}')
        self.text(f'  HEAD가 위 소스 {d.sha[:8]}와 같은지 확인한 뒤:')
        for command in commands:
            self.text(f'    {command}')
        self.text('  두 OS 업로드 후 MacBook에서 finalize하면 확인 질문 없이 바로 공개·배포됩니다.')

    def failure(self, error, retry):
        if self.log_path:
            try:
                with self.log_path.open('a', encoding='utf-8') as log:
                    log.write(f'\n배포 중단: {error}\n재시도: {retry}\n')
            except OSError:
                # Preserve the original diagnostic even when the disk is full.
                pass
        self.text('\n배포 중단', 'bad', file=sys.stderr)
        self.text(str(error), file=sys.stderr)
        if self.log_path:
            self.text(f'상세 로그: {self.log_path}', file=sys.stderr)
        self.text(f'같은 버전으로 재시도: {retry}', file=sys.stderr)
