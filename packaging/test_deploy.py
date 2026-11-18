"""Local-release integration tests: real git/remotes/files; external GitHub and build boundaries only."""
import argparse
import contextlib
import io
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

SOURCE = Path(__file__).resolve().parent
sys.path.insert(0, str(SOURCE))
import deploy

GH = r'''#!/usr/bin/env python3
import hashlib, json, os, pathlib, shutil, sys
args = sys.argv[1:]
base = pathlib.Path(os.environ['TEST_GH'])
settings = json.loads((base/'settings.json').read_text())
with (base/'calls').open('a') as f: f.write(json.dumps(args)+'\n')
if settings.get('api_failure') and args[0] == 'api':
    print('network failure', file=sys.stderr); sys.exit(1)
p = base/'release.json'
r = json.loads(p.read_text()) if p.exists() else None
if args[:2] == ['auth', 'status']: sys.exit(0)
if args[0] == 'api':
    rows = settings.get('releases', [])
    if r: rows = rows + [dict(tag_name=r['tagName'], draft=r['isDraft'], prerelease=False)]
    print(json.dumps([rows])); sys.exit(0)
if args[:2] == ['release', 'view']:
    if not r: sys.exit(1)
    print(json.dumps(r)); sys.exit(0)
if args[:2] == ['release', 'create']:
    if r: sys.exit(1)
    r = dict(tagName=args[2], isDraft=True, isPrerelease=False, targetCommitish=args[args.index('--target')+1],
             body=pathlib.Path(args[args.index('--notes-file')+1]).read_text(), assets=[], url='https://github.com/JeongEEE/wordcraft/releases/draft')
elif args[:2] == ['release', 'upload']:
    files = args[3:args.index('--clobber')]
    for name in files:
        f = pathlib.Path(name); data = f.read_bytes(); shutil.copyfile(f, base/'assets'/f.name)
        r['assets'] = [a for a in r['assets'] if a['name'] != f.name]
        counter = int((base/'counter').read_text()) if (base/'counter').exists() else 0
        (base/'counter').write_text(str(counter+1))
        r['assets'].append(dict(name=f.name, size=len(data), state='uploaded', id=str(counter), updatedAt=str(counter), digest='sha256:'+hashlib.sha256(data).hexdigest()))
elif args[:2] == ['release', 'download']:
    folder = pathlib.Path(args[args.index('--dir')+1]); folder.mkdir(parents=True, exist_ok=True)
    patterns = [args[i+1] for i,a in enumerate(args) if a == '--pattern']
    for a in r['assets']:
        if not patterns or a['name'] in patterns: shutil.copyfile(base/'assets'/a['name'], folder/a['name'])
    sys.exit(0)
elif args[:2] == ['release', 'edit']:
    if '--target' in args: r['targetCommitish'] = args[args.index('--target')+1]
    if '--notes-file' in args: r['body'] = pathlib.Path(args[args.index('--notes-file')+1]).read_text()
    if '--draft=true' in args: r['isDraft'] = True
    if '--draft=false' in args:
        r['isDraft'] = False; (base/'published').touch()
else:
    print('Unexpected gh command: '+str(args), file=sys.stderr); sys.exit(2)
p.write_text(json.dumps(r))
'''

CARGO = r'''#!/usr/bin/env python3
import hashlib, json, os, pathlib, sys, tomllib
args=sys.argv[1:]
base=pathlib.Path(os.environ['TEST_GH'])
if 'version' in args:
    new=args[-1]
    old=tomllib.loads(pathlib.Path('Cargo.toml').read_text())['workspace']['package']['version']
    for name in ['Cargo.toml','Cargo.lock']:
        p=pathlib.Path(name); p.write_text(p.read_text().replace('version = "'+old+'"','version = "'+new+'"'))
    sys.exit(0)
if 'sign' in args:
    if json.loads((base/'settings.json').read_text()).get('sign_failure'): sys.exit(1)
    (base/'sign-source').write_text(pathlib.Path('source.rs').read_text())
    i=args.index('sign'); folder=pathlib.Path(args[i+1]); version=args[i+2]
    # Like word-update-sign: a missing or unprovisioned pinned key never matches the signing secret.
    if len(pathlib.Path(args[i+3]).read_text().strip()) != 64:
        print("The signing secret does not match Word's pinned public key.",file=sys.stderr); sys.exit(1)
    rows=[]
    for suffix in ['macos-universal.dmg','windows-x64.msi','windows-x64-portable.zip']:
        name='word-'+version+'-'+suffix; data=(folder/name).read_bytes()
        rows.append(dict(name=name,size=len(data),sha256=hashlib.sha256(data).hexdigest()))
    (folder/'WordUpdate.json').write_text(json.dumps(dict(schema=1,repository='JeongEEE/wordcraft',version=version,tag='wordcraft-v'+version,assets=rows)))
    (folder/'WordUpdate.sig').write_text('aa'*64)
    sys.exit(0)
print('Unexpected cargo command',args,file=sys.stderr); sys.exit(2)
'''


class LocalDeployTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(prefix='word-local-release-')
        self.addCleanup(self.tmp.cleanup)
        self.base = Path(self.tmp.name)
        self.repo = self.base/'repo'; self.repo.mkdir()
        self.remote = self.base/'remote.git'
        self.gh = self.base/'github'; self.gh.mkdir(); (self.gh/'assets').mkdir()
        self.settings()
        self.bin = self.base/'bin'; self.bin.mkdir()
        for name, source in [('gh', GH), ('cargo', CARGO)]:
            p = self.bin/name
            p.write_text(source.replace('#!/usr/bin/env python3', '#!'+sys.executable, 1)); p.chmod(0o755)
        self.env = dict(os.environ, GIT_CONFIG_GLOBAL=os.devnull, GIT_CONFIG_NOSYSTEM='1',
                        GIT_TERMINAL_PROMPT='0', TEST_GH=str(self.gh), WORD_UPDATE_KEY_FILE=str(self.base/'absent.hex'), PATH=str(self.bin)+os.pathsep+os.environ['PATH'])
        self.env_patch = patch.dict(os.environ, self.env); self.env_patch.start(); self.addCleanup(self.env_patch.stop)
        self.git('init','-b','my-custom'); self.git('config','user.name','Release Test'); self.git('config','user.email','test@example.invalid')
        self.git('init','--bare',str(self.remote))
        self.git('remote','add','origin','https://github.com/JeongEEE/wordcraft.git')
        self.git('config',f'url.{self.remote}.insteadOf','https://github.com/JeongEEE/wordcraft.git')
        (self.repo/'Cargo.toml').write_text('[workspace.package]\nversion = "0.18.2"\n')
        (self.repo/'Cargo.lock').write_text('version = "0.18.2"\n')
        (self.repo/'source.rs').write_text('// committed source\n')
        (self.repo/'crates/update').mkdir(parents=True)
        (self.repo/'crates/update/public-key.hex').write_text('00'*32)
        self.i18n_rev = 'c'*40
        (self.repo/'craft-i18n.rev').write_text(self.i18n_rev+'\n')
        self.git('add','Cargo.toml','Cargo.lock','source.rs','crates/update/public-key.hex','craft-i18n.rev'); self.git('commit','-m','initial')
        self.sha = self.git('rev-parse','HEAD')
        self.git('push','origin','HEAD:refs/heads/my-custom')
        self.key = self.base/'private.hex'; self.key.write_text('2a'*32)
        self.builds = []
        self.during_build = None

    def git(self, *args):
        return subprocess.check_output(['git',*args],cwd=self.repo,env=self.env,text=True,stderr=subprocess.DEVNULL).strip()

    def settings(self, **values):
        (self.gh/'settings.json').write_text(json.dumps(values))

    def build(self, deployment, output):
        from release_assets import package_names
        self.builds.append(deployment.args.platform)
        for name in package_names(deployment.version, deployment.args.platform):
            (output/name).write_bytes(('native package '+name).encode())
        if self.during_build: self.during_build()
        return ['fork-identity','updater-tests','package-policy','cli-smoke']

    def run_deploy(self, os_name='macos', version=None, *, finalize=False, key=False, draft=False, dry_run=False):
        args = argparse.Namespace(platform=os_name, version=version, finalize=finalize, draft=draft, dry_run=dry_run,
                                  key_file=str(self.key) if key else None)
        out = io.StringIO()
        try:
            with patch.object(deploy,'ROOT',self.repo), patch('platform.system',return_value='Darwin' if os_name=='macos' else 'Windows'), \
                    patch('local_build.build',side_effect=self.build), \
                    contextlib.redirect_stdout(out):
                deploy.Deployment(args).run()
            return True, out.getvalue()
        except (deploy.DeployError, OSError, ValueError, KeyError, TypeError) as error:
            return False, out.getvalue()+str(error)

    def ok(self, *args, **kwargs):
        success, message = self.run_deploy(*args, **kwargs)
        self.assertTrue(success, message)
        return message

    def release(self):
        return json.loads((self.gh/'release.json').read_text())

    def change_release(self, **fields):
        r=self.release(); r.update(fields); (self.gh/'release.json').write_text(json.dumps(r))

    def calls(self):
        return [json.loads(line) for line in (self.gh/'calls').read_text().splitlines()]

    def test_fallback_cargo_directory_is_available_to_child_shells(self):
        home = self.base/'home'
        for custom in [False, True]:
            with self.subTest(custom_cargo_home=custom):
                cargo_home = self.base/'Custom Rust Home' if custom else home/'.cargo'
                tools = cargo_home/'bin'
                tools.mkdir(parents=True)
                for name in ['cargo', 'rustc']:
                    tool = tools/name
                    tool.write_text('#!'+sys.executable+'\nprint("'+name+' from selected toolchain")\n')
                    tool.chmod(0o755)
                with patch.dict(os.environ, PATH='/usr/bin:/bin', CARGO_HOME=str(cargo_home) if custom else ''), \
                        patch.object(Path, 'home', return_value=home), patch.object(deploy, 'ROOT', self.repo):
                    deployment = deploy.Deployment(argparse.Namespace())
                    result = deployment.command('/bin/bash', '-c', 'cargo --version && rustc --version', optional=True)
                    self.assertEqual(result.returncode, 0, result.stderr)
                    self.assertIn('cargo from selected toolchain', result.stdout)
                    self.assertIn('rustc from selected toolchain', result.stdout)
                    self.assertEqual(os.environ['PATH'], '/usr/bin:/bin')

    def test_two_os_upload_same_version_then_finalize_without_actions(self):
        self.ok('macos',draft=True)
        self.assertEqual(len(self.release()['assets']),3)
        self.assertFalse((self.gh/'published').exists())
        self.ok('windows',draft=True)
        self.assertEqual(len(self.release()['assets']),6)
        self.assertEqual(self.git('rev-parse','HEAD'),self.sha)
        self.ok('macos',finalize=True,key=True)
        self.assertEqual(len(self.release()['assets']),9)
        self.assertTrue((self.gh/'published').exists())
        self.assertEqual(self.builds,['macos','windows'])
        self.assertFalse(any(c[0] in ['workflow','run'] for c in self.calls()))
        self.assertEqual(self.git('ls-remote','origin','refs/heads/my-custom-release'),'')

    def test_windows_can_upload_first_and_mac_can_finish(self):
        self.ok('windows',draft=True)
        self.ok('macos',key=True)
        self.assertTrue((self.gh/'published').exists())

    def test_complete_upload_without_key_stays_draft(self):
        self.ok('macos',draft=True); self.ok('windows')
        self.assertEqual(len(self.release()['assets']),6)
        self.assertFalse((self.gh/'published').exists())

    def test_mac_upload_guides_windows_with_frozen_version_and_source(self):
        message = self.ok('macos', draft=True)
        self.assertIn('.\\deploy-windows.ps1 -Version 0.18.2 -Draft', message)
        self.assertIn('git pull --ff-only origin my-custom', message)
        self.assertIn(self.sha, message)
        self.assertIn('macOS: 업로드 완료', message)
        self.assertIn('Windows: 대기', message)
        self.assertNotIn('\x1b[', message)
        self.assertTrue(list((self.repo/'.git/word-deploy-logs').glob('*.log')))

    def test_windows_upload_guides_mac_finalize_without_another_build(self):
        self.ok('macos', draft=True)
        message = self.ok('windows', draft=True)
        self.assertIn('./deploy-macos.sh 0.18.2 --finalize', message)
        self.assertIn('git pull --ff-only origin my-custom', message)
        self.assertIn('Windows: 업로드 완료', message)
        self.assertNotIn('.\\deploy-windows.ps1 -Version', message)

    def test_windows_first_guides_mac_build_before_finalize(self):
        message = self.ok('windows', draft=True)
        self.assertIn('./deploy-macos.sh 0.18.2 --draft', message)
        self.assertIn('macOS: 대기', message)
        self.assertNotIn('./deploy-macos.sh 0.18.2 --finalize', message)

    def test_publication_summary_does_not_offer_finalize_again(self):
        self.ok('macos', draft=True); self.ok('windows', draft=True)
        message = self.ok(finalize=True, key=True)
        self.assertIn('공개 완료', message)
        self.assertNotIn('./deploy-macos.sh 0.18.2 --finalize', message)

    def test_build_failure_records_diagnostic_and_preserves_retry_flags(self):
        def fail():
            raise deploy.DeployError('simulated interrupted build')
        self.during_build = fail
        out = io.StringIO()
        with patch.object(deploy, 'ROOT', self.repo), patch('platform.system', return_value='Darwin'), \
                patch('local_build.build', side_effect=self.build), \
                patch.object(sys, 'argv', ['deploy.py', '--platform', 'macos', '--draft']), \
                contextlib.redirect_stdout(out), contextlib.redirect_stderr(out):
            result = deploy.main()
        self.assertEqual(result, 1)
        self.assertIn('./deploy-macos.sh 0.18.2 --draft', out.getvalue())
        self.assertIn('simulated interrupted build', out.getvalue())
        self.assertIn('.log', out.getvalue())
        logs = list((self.repo/'.git/word-deploy-logs').glob('*.log'))
        self.assertEqual(len(logs), 1)
        self.assertIn('simulated interrupted build', logs[0].read_text())

    def test_windows_failure_preserves_draft_and_quoted_key_path(self):
        self.settings(api_failure=True)
        out = io.StringIO()
        with patch.object(deploy, 'ROOT', self.repo), patch('platform.system', return_value='Windows'), \
                patch.object(sys, 'argv', ['deploy.py', '--platform', 'windows', '--draft', '--key-file', "C:\\keys\\Jeong's word.hex"]), \
                contextlib.redirect_stdout(out), contextlib.redirect_stderr(out):
            result = deploy.main()
        self.assertEqual(result, 1)
        self.assertIn(".\\deploy-windows.ps1 -Version 0.18.2 -Draft -KeyFile 'C:\\keys\\Jeong''s word.hex'", out.getvalue())
        self.assertFalse((self.gh/'published').exists())

    def test_long_child_output_is_logged_and_failure_tail_is_visible(self):
        out = io.StringIO()
        with patch.object(deploy, 'ROOT', self.repo), contextlib.redirect_stdout(out):
            deployment = deploy.Deployment(argparse.Namespace())
            deployment.console.start_log(self.repo/'.git')
            with self.assertRaises(deploy.DeployError) as raised:
                deployment.command(sys.executable, '-c',
                                   'print("first detail"); print("last error"); raise SystemExit(7)', capture=False)
        self.assertIn('last error', str(raised.exception))
        self.assertNotIn('first detail', out.getvalue())
        self.assertIn('first detail', deployment.console.log_path.read_text())
        self.assertIn('last error', deployment.console.log_path.read_text())

    def test_current_published_version_bumps_once_and_other_os_reuses_it(self):
        self.settings(releases=[dict(tag_name='wordcraft-v0.18.2',draft=False,prerelease=False)])
        self.ok('macos',draft=True); self.ok('windows',draft=True)
        self.assertIn('0.18.3',(self.repo/'Cargo.toml').read_text())
        self.assertEqual(self.git('rev-list','--count','HEAD'),'2')
        self.assertEqual(self.release()['tagName'],'wordcraft-v0.18.3')

    def test_explicit_published_version_is_rejected_before_build(self):
        self.settings(releases=[dict(tag_name='wordcraft-v0.18.2',draft=False,prerelease=False)])
        self.assertFalse(self.run_deploy(version='0.18.2')[0]); self.assertEqual(self.builds,[])

    def test_dry_run_has_no_writes_network_or_build(self):
        before=self.git('status','--porcelain')
        self.ok(dry_run=True)
        self.assertFalse((self.gh/'calls').exists()); self.assertEqual(self.builds,[])
        self.assertEqual(self.git('status','--porcelain'),before)
        self.assertEqual(self.git('rev-parse','HEAD'),self.sha)

    def test_dirty_source_and_wrong_remote_stop_before_network(self):
        (self.repo/'source.rs').write_text('// unfinished')
        self.assertFalse(self.run_deploy()[0]); self.assertFalse((self.gh/'calls').exists())
        self.git('restore','source.rs'); self.git('remote','set-url','origin','https://github.com/storytold/wordcraft.git')
        self.assertFalse(self.run_deploy()[0]); self.assertFalse((self.gh/'calls').exists())

    def test_api_failure_does_not_build_or_mutate_version(self):
        self.settings(api_failure=True)
        self.assertFalse(self.run_deploy()[0]); self.assertEqual(self.builds,[])
        self.assertEqual(self.git('rev-parse','HEAD'),self.sha)

    def test_other_source_draft_cannot_receive_upload(self):
        self.ok(draft=True); self.change_release(targetCommitish='a'*40)
        self.assertFalse(self.run_deploy('windows')[0]); self.assertEqual(len(self.release()['assets']),3)

    def create_empty_draft(self):
        def fail():
            raise deploy.DeployError('simulated interrupted build')
        self.during_build = fail
        self.assertFalse(self.run_deploy(draft=True)[0])
        self.during_build = None
        self.assertEqual(self.release()['assets'], [])

    def advance_source(self):
        (self.repo/'source.rs').write_text('// changed after upstream merge\n')
        self.git('add', 'source.rs'); self.git('commit', '-m', 'upstream merged')
        return self.git('rev-parse', 'HEAD')

    def test_empty_untagged_draft_follows_updated_source_before_retry(self):
        self.create_empty_draft()
        current = self.advance_source()
        self.ok(draft=True)
        release = self.release()
        self.assertEqual(release['targetCommitish'], current)
        self.assertIn(f'Source commit: `{current}`', release['body'])
        self.assertNotIn(f'Source commit: `{self.sha}`', release['body'])
        self.assertEqual(release['tagName'], 'wordcraft-v0.18.2')
        self.assertEqual(len(release['assets']), 3)
        self.assertTrue(release['isDraft'])
        self.assertEqual(self.git('rev-list', '--count', 'HEAD'), '2')
        self.assertFalse((self.gh/'published').exists())

    def test_uploaded_draft_does_not_follow_updated_source(self):
        self.ok(draft=True)
        original = self.release()
        self.advance_source()
        self.assertFalse(self.run_deploy('windows')[0])
        self.assertEqual(self.release(), original)
        self.assertEqual(self.builds, ['macos'])

    def test_tagged_empty_draft_is_not_retargeted(self):
        self.create_empty_draft()
        self.advance_source()
        self.git('tag', 'wordcraft-v0.18.2')
        self.git('push', 'origin', 'refs/tags/wordcraft-v0.18.2')
        original = self.release()
        self.assertFalse(self.run_deploy()[0])
        self.assertEqual(self.release(), original)

    def test_empty_draft_with_wrong_source_marker_is_not_retargeted(self):
        self.create_empty_draft()
        self.advance_source()
        self.change_release(body='Source commit: `'+'a'*40+'`')
        original = self.release()
        self.assertFalse(self.run_deploy()[0])
        self.assertEqual(self.release(), original)

    def test_finalize_does_not_retarget_an_empty_draft(self):
        self.create_empty_draft()
        self.advance_source()
        original = self.release()
        self.assertFalse(self.run_deploy(finalize=True, key=True)[0])
        self.assertEqual(self.release(), original)

    def test_empty_draft_from_another_version_is_not_retargeted(self):
        for name in ['Cargo.toml', 'Cargo.lock']:
            p = self.repo/name
            p.write_text(p.read_text().replace('0.18.2', '0.18.1'))
        self.git('add', 'Cargo.toml', 'Cargo.lock'); self.git('commit', '-m', 'older version')
        self.create_empty_draft()
        for name in ['Cargo.toml', 'Cargo.lock']:
            p = self.repo/name
            p.write_text(p.read_text().replace('0.18.1', '0.18.2'))
        self.git('add', 'Cargo.toml', 'Cargo.lock'); self.git('commit', '-m', 'new version')
        self.change_release(tagName='wordcraft-v0.18.2')
        original = self.release()
        self.assertFalse(self.run_deploy()[0])
        self.assertEqual(self.release(), original)

    def test_empty_draft_from_unrelated_history_is_not_retargeted(self):
        self.create_empty_draft()
        orphan = self.git('commit-tree', self.git('rev-parse', 'HEAD^{tree}'), '-m', 'unrelated history')
        self.change_release(targetCommitish=orphan, body=f'Source commit: `{orphan}`')
        original = self.release()
        self.assertFalse(self.run_deploy()[0])
        self.assertEqual(self.release(), original)

    def test_new_asset_during_empty_draft_check_prevents_retarget(self):
        self.create_empty_draft()
        self.advance_source()
        read = deploy.Deployment.read_release
        calls = []
        def changed(d):
            calls.append(None)
            if len(calls) == 2:
                self.change_release(assets=[dict(name='word-0.18.2-windows-x64.msi', size=100, state='uploaded')])
            return read(d)
        with patch.object(deploy.Deployment, 'read_release', changed):
            self.assertFalse(self.run_deploy()[0])
        self.assertEqual(self.release()['targetCommitish'], self.sha)
        self.assertFalse(any(c[:2] == ['release', 'edit'] for c in self.calls()))

    def test_source_edit_during_build_prevents_upload(self):
        self.during_build=lambda: (self.repo/'source.rs').write_text('// concurrent edit')
        self.assertFalse(self.run_deploy()[0])
        self.assertFalse(any(c[:2]==['release','upload'] for c in self.calls()))

    def test_build_uses_committed_source_without_untracked_work(self):
        import local_build
        (self.repo/'scratch.rs').write_text('// untracked work')
        args=argparse.Namespace(platform='macos',version=None,finalize=False,draft=True,dry_run=False,key_file=None)
        with patch.object(deploy,'ROOT',self.repo):
            deployment=deploy.Deployment(args)
        deployment.sha=self.sha; deployment.version='0.18.2'
        def inspect_source(d, output, source):
            self.assertFalse((source/'scratch.rs').exists())
            self.assertEqual((source/'source.rs').read_text(),'// committed source\n')
            return ['verified']
        with tempfile.TemporaryDirectory() as folder, patch('local_build.prepare_fonts'), patch('local_build.prepare_i18n') as i18n, patch('local_build.build_source',side_effect=inspect_source):
            self.assertEqual(local_build.build(deployment,Path(folder)),['verified'])
        i18n.assert_called_once_with(deployment)

    def test_release_asset_change_during_confirmation_blocks_publication(self):
        self.ok('macos',draft=True); self.ok('windows',draft=True)
        original_hash=deploy.assets.file_hash
        def replace_asset(path):
            # The last step before publication re-hashes uploaded files; change the release meanwhile.
            r=self.release(); r['assets'][0]['id']='new upload'; self.change_release(assets=r['assets'])
            return original_hash(path)
        args=argparse.Namespace(platform='macos',version=None,finalize=True,draft=False,dry_run=False,key_file=str(self.key))
        with patch.object(deploy,'ROOT',self.repo), patch.object(deploy.assets,'file_hash',side_effect=replace_asset):
            with self.assertRaises(deploy.DeployError): deploy.Deployment(args).run()
        self.assertFalse((self.gh/'published').exists())

    def test_missing_platform_prevents_finalize(self):
        self.ok(draft=True)
        self.assertFalse(self.run_deploy(finalize=True,key=True)[0])
        self.assertFalse((self.gh/'published').exists())

    def test_tampered_payload_or_receipt_cannot_be_signed_or_published(self):
        self.ok('macos',draft=True); self.ok('windows',draft=True)
        f=self.gh/'assets/word-0.18.2-windows-x64.msi'; original=f.read_bytes(); f.write_bytes(b'tampered')
        self.assertFalse(self.run_deploy(finalize=True,key=True)[0])
        f.write_bytes(original)
        receipt=self.gh/'assets/word-0.18.2-windows-build.json'
        original=receipt.read_text()
        self.assertEqual(json.loads(original)['i18n'], self.i18n_rev)
        for field in ['source','i18n']:
            data=json.loads(original); data[field]='b'*40; receipt.write_text(json.dumps(data))
            self.assertFalse(self.run_deploy(finalize=True,key=True)[0])
        self.assertFalse((self.gh/'published').exists())

    def test_existing_tag_with_wrong_commit_cannot_be_published(self):
        self.ok('macos',draft=True); self.ok('windows',draft=True)
        self.git('commit','--allow-empty','-m','other'); self.git('tag','wordcraft-v0.18.2'); self.git('push','origin','refs/tags/wordcraft-v0.18.2')
        self.git('reset','--soft',self.sha)
        self.assertFalse(self.run_deploy(finalize=True,key=True)[0])
        self.assertFalse((self.gh/'published').exists())

    def test_signer_uses_frozen_source_when_checkout_changes_after_key_read(self):
        self.ok('macos',draft=True); self.ok('windows',draft=True)
        read=Path.read_text
        def changed_after_key(path, *args, **kwargs):
            value=read(path,*args,**kwargs)
            if path == self.key: (self.repo/'source.rs').write_text('// changed after private key read')
            return value
        with patch.object(Path,'read_text',changed_after_key):
            self.assertFalse(self.run_deploy(finalize=True,key=True)[0])
        self.assertEqual((self.gh/'sign-source').read_text(),'// committed source\n')
        self.assertFalse((self.gh/'published').exists())

    def test_sign_failure_keeps_draft_unpublished(self):
        self.ok('macos',draft=True); self.ok('windows',draft=True); self.settings(sign_failure=True)
        self.assertFalse(self.run_deploy(finalize=True,key=True)[0])
        self.assertFalse((self.gh/'published').exists())

    def test_finalize_publishes_without_prompt(self):
        self.ok('macos',draft=True); self.ok('windows',draft=True)
        with patch('builtins.input',side_effect=AssertionError('no prompt expected')):
            self.ok(finalize=True,key=True)
        self.assertTrue((self.gh/'published').exists())

    def test_draft_flag_keeps_signed_draft(self):
        self.ok('macos',draft=True); self.ok('windows',draft=True)
        self.ok(finalize=True,key=True,draft=True)
        self.assertEqual(len(self.release()['assets']),9)
        self.assertFalse((self.gh/'published').exists())

    def test_unprovisioned_public_key_fails_signing_and_stays_draft(self):
        # Until the release key is provisioned the pinned public key is empty; the signer refuses it.
        (self.repo/'crates/update/public-key.hex').write_text('')
        self.git('add','crates/update/public-key.hex'); self.git('commit','-m','no release key yet')
        self.ok('macos',draft=True); self.ok('windows',draft=True)
        success, message = self.run_deploy(finalize=True,key=True)
        self.assertFalse(success)
        self.assertIn("does not match Word's pinned public key", message)
        self.assertFalse((self.gh/'published').exists())
        self.assertTrue(self.release()['isDraft'])
        self.assertFalse({'WordUpdate.json','WordUpdate.sig','SHA256SUMS.txt'} & {a['name'] for a in self.release()['assets']})

    def i18n_deployment(self, configured=None):
        import local_build
        upstream = self.base/'craft-i18n'
        if not upstream.exists():
            upstream.mkdir()
            run = lambda *a: subprocess.check_output(['git',*a],cwd=upstream,env=self.env,text=True,stderr=subprocess.DEVNULL).strip()
            run('init','-b','main'); run('config','user.name','Release Test'); run('config','user.email','test@example.invalid')
            (upstream/'catalog.ftl').write_text('pinned\n'); run('add','catalog.ftl'); run('commit','-m','pinned')
            self.i18n_rev = run('rev-parse','HEAD')
            (upstream/'catalog.ftl').write_text('newer\n'); run('commit','-am','newer')
            (self.repo/'craft-i18n.rev').write_text(self.i18n_rev+'\n')
            self.git('add','craft-i18n.rev'); self.git('commit','-m','pin craft-i18n')
        env = {'GIT_CONFIG_COUNT':'1', 'GIT_CONFIG_KEY_0':f'url.{upstream}.insteadOf', 'GIT_CONFIG_VALUE_0':local_build.I18N_URL}
        if configured: env['CRAFT_I18N_DIR'] = str(configured)
        with patch.dict(os.environ, env), patch.object(deploy,'ROOT',self.repo):
            d = deploy.Deployment(argparse.Namespace(platform='macos'))
        d.sha = self.git('rev-parse','HEAD'); d.common = self.repo/'.git'
        return d

    def test_i18n_is_a_clean_checkout_of_the_committed_pin(self):
        import local_build
        d = self.i18n_deployment()
        # Only the committed pin counts; an edit in the shared checkout is not release input.
        (self.repo/'craft-i18n.rev').write_text('d'*40+'\n')
        local_build.prepare_i18n(d)
        folder = self.repo/'.git/word-release-i18n'/self.i18n_rev
        self.assertEqual(d.env['CRAFT_I18N_DIR'], str(folder))
        self.assertEqual(d.env['CRAFT_I18N_REQUIRED'], '1')
        self.assertEqual((folder/'catalog.ftl').read_text(), 'pinned\n')
        again = self.i18n_deployment()
        local_build.prepare_i18n(again)  # the next build reuses the cached checkout
        self.assertEqual(again.env['CRAFT_I18N_DIR'], str(folder))

    def test_i18n_checkout_must_exist_match_the_pin_and_be_clean(self):
        import local_build
        with self.assertRaisesRegex(deploy.DeployError, 'CRAFT_I18N_DIR가 없습니다'):
            local_build.prepare_i18n(self.i18n_deployment(self.base/'missing-i18n'))
        other = self.base/'other-i18n'
        subprocess.check_call(['git','clone','--quiet',str(self.base/'craft-i18n'),str(other)],env=self.env)
        with self.assertRaisesRegex(deploy.DeployError, '고정 커밋'):
            local_build.prepare_i18n(self.i18n_deployment(other))
        subprocess.check_call(['git','-C',str(other),'checkout','--quiet','--detach',self.i18n_rev],env=self.env)
        local_build.prepare_i18n(self.i18n_deployment(other))
        (other/'catalog.ftl').write_text('edited\n')
        with self.assertRaisesRegex(deploy.DeployError, '깨끗한 체크아웃'):
            local_build.prepare_i18n(self.i18n_deployment(other))

    def test_missing_or_invalid_i18n_pin_blocks_the_build(self):
        import local_build
        d = self.i18n_deployment()
        for content in [None, 'main\n']:
            with self.subTest(content=content):
                if content is None:
                    self.git('rm','--quiet','craft-i18n.rev')
                else:
                    (self.repo/'craft-i18n.rev').write_text(content); self.git('add','craft-i18n.rev')
                self.git('commit','-m','break pin'); d.sha = self.git('rev-parse','HEAD')
                with self.assertRaisesRegex(deploy.DeployError, '번역 없이 배포하지 않습니다'):
                    local_build.prepare_i18n(d)
        self.assertNotIn('CRAFT_I18N_DIR', d.env)

if __name__=='__main__': unittest.main()
