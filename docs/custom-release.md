# Word 포크 개발·배포

Word는 `JeongEEE/wordcraft`에서 관리하는 WordCraft 기반 개인 워드 프로세서다.
사용자에게 보이는 앱 이름은 **Word**, 유지관리자 표기는 `JeongEEE`, Cargo 패키지는
`wordcraft`와 `wordcraft-cli`다. 원본의 crate 이름, 명령 ID, `WORDCRAFT_*` 환경변수를
유지해 업스트림 변경을 계속 받아들인다.

## 브랜치와 원격

| 대상 | 역할 |
|---|---|
| `upstream` | `https://github.com/storytold/wordcraft.git` |
| `origin` | `https://github.com/JeongEEE/wordcraft.git` |
| `main` | 개인 커밋 없이 `upstream/main`만 동기화 |
| `my-custom` | 개인 변경 개발·검증·커밋 |

업스트림을 반영하는 순서는 다음과 같다. 충돌은 `my-custom`에서 해결한다.

```sh
git fetch --no-tags upstream
git switch main
git merge --ff-only upstream/main
git push origin main
git switch my-custom
git merge main
cargo xtask ci
```

커스텀을 `main`에 역병합하지 않는다. 공개한 커스텀 브랜치는 merge로 갱신하며
일반적인 유지보수에 force push나 rebase를 사용하지 않는다.

## 원본과 분리한 식별자

| 항목 | Word |
|---|---|
| updater app id / 배포 파일 stem | `word` |
| 업데이트 저장소 / 태그 | `JeongEEE/wordcraft` / `wordcraft-v<버전>` |
| macOS 앱 / bundle id | `Word.app` / `io.github.jeongeee.word` |
| macOS 설정 | `~/Library/Application Support/Word` |
| Linux·기타 설정 | `$XDG_CONFIG_HOME/word` (기본 `~/.config/word`) |
| Windows 설치 폴더 | `%LOCALAPPDATA%\Programs\JeongEEE\Word` (모든 사용자: `Program Files\JeongEEE\Word`) |
| Windows 설정 / registry | `%APPDATA%\Word` / `Software\JeongEEE\Word` |
| Windows ProgID | `JeongEEE.Word.Document` 등 `JeongEEE.Word.*` (MS Word의 `Word.Document.*`와 겹치지 않게) |
| Windows RegisteredApplications 이름 | `JeongEEE.Word` |
| Windows 시작 메뉴 바로가기 | `Word (JeongEEE)` (MS Word의 `Word.lnk`와 같은 폴더라 이름을 구별) |
| Windows UpgradeCode | `6BE9BB68-D1FF-4740-95DA-EB6D2B37870C` |
| Windows component GUID | 앱 `8EA63E2E-34D5-4E6E-987B-B34DEDF45D27`, 파일 형식 `F8BD16B6-8DF5-4988-A52B-0C7CECE66CE7` |
| 실행 파일 | `word.exe`, `word-cli.exe` / macOS `Word.app`, `word-cli` |
| 배포 파일 | `word-<버전>-macos-universal.dmg`, `word-cli-<버전>-macos-universal.zip`, `word-<버전>-windows-x64.msi`, `word-<버전>-windows-x64-portable.zip` |

UpgradeCode와 component GUID는 이후 릴리스에서 바꾸지 않는다. 바꾸면 기존 설치를
업그레이드하지 못하고 옆에 하나 더 설치된다. 파일 연결은 "연결 프로그램" 후보와
Windows 기본 앱 설정의 선택지로만 등록하고, 확장자의 기본 앱은 바꾸지 않는다.
ArtCraft 로고·워드마크는 배포물과 화면에서 뺐고, WordCraft 기반이라는 텍스트 출처와
원본 저작권·라이선스는 남긴다.

## 빌드와 검증

Rust 1.95 이상, rustfmt, clippy, `wasm32-unknown-unknown` target이 필요하다.
macOS universal 패키지에는 `x86_64-apple-darwin` target도 설치한다.

```sh
rustup component add rustfmt clippy
rustup target add wasm32-unknown-unknown x86_64-apple-darwin
cargo build --release --locked -p wordcraft -p wordcraft-cli
cargo test -p wordcraft-ui-egui --test fork_identity
python3 packaging/test_fork_contract.py
cargo xtask ci
```

macOS에는 Xcode 또는 Command Line Tools가 필요하다. 개발 도구 경로가 불완전한
Command Line Tools를 가리키지만 Xcode가 설치돼 있다면, 전역 설정을 바꾸지 않고
빌드 명령에 `DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer`를 지정할 수 있다.
Rust가 PATH에 없다면 먼저 `. "$HOME/.cargo/env"`를 실행한다.

화면 번역은 `craft-i18n` 저장소에서 빌드 때 넣는다. `craft-i18n.rev`의 커밋을 깨끗한
체크아웃으로 준비해 그 경로를 `CRAFT_I18N_DIR`에 지정하고, 배포 빌드는
`CRAFT_I18N_REQUIRED=1`로 누락을 막는다. 글꼴은 `storytold/craft-fonts`의 고정 커밋을
체크아웃해 `CRAFT_FONTS_DIR`로 지정한다.

macOS 패키징은 `packaging/macos/package.sh --arch universal`, Windows는 MSVC 도구와
WiX 5.0.2를 준비한 뒤 `pwsh packaging/windows/package.ps1 -Arch x64`로 실행한다.
Windows 설치 정책은 Windows에서 `pwsh packaging/windows/test-install-policy.ps1`로 검사한다.

## OS별 로컬 배포

빌드는 각자의 Mac/Windows에서 실행하고 GitHub에는 완성된 파일만 업로드한다.
Actions 실행·runner·배포 브랜치 푸시는 사용하지 않는다. `deploy.sh`는 Mac 명령의 별칭이다.
두 컴퓨터 모두 `my-custom`의 **같은 커밋과 버전**을 사용해야 한다. 한 릴리스를 마무리하기
전에는 소스 변경을 추가하지 않고 두 컴퓨터에서 배포 명령을 동시에 실행하지 않는다.

두 OS 스크립트는 같은 형식으로 단계·소요 시간·결과를 표시한다. 터미널에서는 색상과
진행 애니메이션을 사용하고, 파이프·파일 출력에서는 일반 텍스트를 사용한다. `NO_COLOR`로
색상을 끌 수 있다. 컴파일·패키징의 상세 출력은 git 공통 디렉터리의
`word-deploy-logs/*.log`에 실행별로 남기며, 실패하면 마지막 오류와 로그 경로,
같은 버전·옵션을 유지한 재시도 명령을 표시한다. `--dry-run`은 로그 파일도 만들지 않는다.
같은 저장소에서 배포가 겹치지 않도록 git 공통 디렉터리의 `word-deploy.lock`을 잡는다.

완료 화면은 버전·전체 소스 SHA·OS별 업로드 상태·업데이트 서명 상태·초안 링크를 보여준다.
Mac 업로드 후에는 Windows의 pull·커밋 확인·`-Version … -Draft` 명령을,
Windows 업로드 후 두 OS가 모두 있으면 Mac의 pull·커밋 확인·`--finalize` 명령을 안내한다.
Windows부터 실행했다면 먼저 Mac 빌드를 안내한다. 안내에 표시된 SHA를 확인하고
다음 단계로 넘어가며, 이미 공개한 결과에는 finalize를 다시 안내하지 않는다.

### 최초 환경 준비

공통: Python 3.11 이상, git, Rust 1.95.0 이상, GitHub CLI(`gh`)와 포크 쓰기 권한으로
`gh auth login`이 필요하다. 의존성 설치는 자동으로 시스템 설정을 변경하지 않도록 별도로 한다.

Mac: Xcode와 두 Rust target을 준비한다. Xcode가 있으면 스크립트가 `DEVELOPER_DIR`를 설정한다.
Cargo가 PATH에 없어도 `CARGO_HOME/bin`(미설정 시 `~/.cargo/bin`)에서 찾고 그 경로를
하위 Bash/PowerShell에도 전달한다. 전역 PATH나 셸 설정 파일은 변경하지 않는다.

```sh
rustup target add aarch64-apple-darwin x86_64-apple-darwin
```

Windows: PowerShell 7(`pwsh`), Visual Studio Build Tools의 **Desktop development with C++**
(MSVC x64, Windows SDK), .NET SDK와 WiX 5.0.2를 준비한다. **Developer PowerShell for VS**에서
실행하면 SDK의 `rc.exe` 등 도구 경로가 잡힌다. PowerShell 실행 정책이 차단하면 해당 터미널에서만
`Set-ExecutionPolicy -Scope Process Bypass`를 사용한다.
업데이트 개인키를 Windows로 복사할 필요는 없다. GitHub 인증은 Windows에서 별도로
`gh auth login`을 실행하고, 두 OS 업로드 후 키가 있는 Mac에서 서명·공개를 마무리한다.

```powershell
rustup target add x86_64-pc-windows-msvc
dotnet tool install --global wix --version 5.0.2
gh auth login
git clone --branch my-custom https://github.com/JeongEEE/wordcraft.git
cd wordcraft
```

빌드 입력 두 가지를 로컬 git 공통 디렉터리의 캐시에 받는다.

- 글꼴: `storytold/craft-fonts`의 `b076417b5880bd4154bd22f00695b1d28c6bea83`
  (업스트림 release workflow의 `craft-fonts-ref`와 같은 값)을 `word-release-fonts/`에 받아
  `CRAFT_FONTS_DIR`·`CRAFT_FONTS_REQUIRED=1`로 넘긴다.
- 화면 번역: 빌드할 커밋의 `craft-i18n.rev`가 가리키는 `storytold/craft-i18n` 커밋을
  `word-release-i18n/`에 받아 `CRAFT_I18N_DIR`·`CRAFT_I18N_REQUIRED=1`로 넘긴다. 이 값이 없으면
  화면이 영어로만 나오므로, rev 파일이 없거나 40자리 커밋이 아니면 배포를 멈춘다.

`CRAFT_FONTS_DIR`·`CRAFT_I18N_DIR`를 직접 지정했다면 그 경로가 있어야 하고 같은 고정 커밋의
깨끗한 체크아웃이어야 한다. 아니면 배포를 멈춘다. 첫 빌드는 오래 걸리고 이후에는 로컬
Cargo target 캐시를 재사용한다.

### 두 컴퓨터에서 실행

미공개 버전 `0.18.2`를 배포하는 예다. 어느 OS부터 실행해도 된다.

```sh
# Mac: 커밋된 소스 확인 후 로컬 빌드·검증·초안 업로드
./deploy-macos.sh 0.18.2 --draft
```

```powershell
# Windows: Mac에서 버전 커밋이 생겼다면 먼저 pull
git switch my-custom
git pull --ff-only origin my-custom
.\deploy-windows.ps1 -Version 0.18.2 -Draft
```

버전을 생략하면 **현재 소스가 미공개 버전일 때 그대로 사용**한다. 이미 공개된 버전이라면
첫 PC에서만 다음 패치를 준비하고 `Cargo.toml`·`Cargo.lock`을
`chore: Word <버전> 릴리스 버전 준비`로 커밋·푸시한다. 두 번째 PC는
그 커밋을 pull한 뒤 같은 버전을 사용한다. 명시한 공개 버전·낮은 버전은 거부한다.

각 PC는 자기 OS의 두 패키지와 빌드 증빙 JSON만 같은 초안에 올린다. 빌드는 미추적 파일을
제외한 확정 커밋의 임시 로컬 clone에서 수행한다. 긴 빌드 중 원래 체크아웃이 바뀌면
업로드를 중단한다. 빌드 실패 후 **자산이 0개이고 태그도 없는 미공개 초안**만 예외로,
같은 버전의 이전 커밋에서 현재 커밋으로 자동 갱신한다. 이전 커밋은 현재 소스의 조상이어야
하며 기존 Source commit 기록도 일치해야 한다. 따라서 첫 업로드 전에 upstream을 머지하거나
배포 도구를 고쳤어도 같은 명령으로 다시 시작할 수 있다. 이미 파일·태그가 있거나 공개된
릴리스는 재지정하지 않는다. 두 OS 중 하나라도 업로드했다면 같은 커밋으로 맞추거나 새 버전을
준비한다. `--finalize`는 빈 초안을 갱신하지 않는다.

Windows PC에는 업데이트 개인키가 없어도 된다. 두 OS가 모이면 키가 있는 Mac에서
빌드 없이 검증·서명·공개를 마무리한다.

```sh
./deploy-macos.sh 0.18.2 --finalize
```

개인키는 아래 "업데이트 서명 키" 절의 위치에 둔다. 다른 위치는
`WORD_UPDATE_KEY_FILE` 또는 `--key-file <경로>`로 지정하며 Windows는 `-KeyFile <경로>`와
`-Finalize`를 쓴다.

두 플랫폼을 내려받아 증빙의 버전·SHA·글꼴·번역 커밋·검증 기록·파일 해시를 확인하고,
`WordUpdate.json`·`WordUpdate.sig`와 체크섬을 만든다. 업로드한 최종 9개 자산의 바이트도
다시 확인하면 묻지 않고 바로 공개해 자동 업데이트를 배포한다. 공개를 미루려면
`--draft`/`-Draft`를 붙여 서명된 초안으로 남긴다. 키가 있는 PC에서 두 번째 OS 빌드를
`--draft` 없이 실행하면 `--finalize`를 따로 실행하지 않고 이 단계까지 이어져 바로 공개된다.

`--dry-run`/`-DryRun`은 네트워크·파일·커밋을 변경하지 않고 예정된 동작을 보여 준다.
`./deploy.sh`도 동일한 Mac 로컬 흐름이며 Actions로 돌아가는 옵션은 없다.
실패하면 같은 버전으로 재실행한다. 버전 준비 중 미커밋 파일이 남았다면 먼저 확인한다.
파일이 올라간 초안의 소스를 바꿔야 한다면 새 버전을 준비한다. 빈 초안의 갱신 중 다른
업로드·태그·메타데이터 변경이 감지되면 중단하므로 두 PC에서 동시에 배포하지 않는다.

### 로컬 검증 범위

양쪽 모두 `packaging/test_fork_contract.py`(fork identity), `cargo test --locked -p wordcraft-update`,
CLI 검사를 실행한다. CLI 검사는 `word-cli --version`에 배포 버전이 들어 있는지 보고, 내장 샘플을
`run --template sample --save sample.docx`로 저장한 뒤 `text sample.docx`로 다시 열어 샘플 제목이
읽히는지 확인한다. Mac은 `packaging/macos/package.sh --arch universal`로 만든 DMG의 서명과
임시 앱 교체·실패 시 복원(`native_dmg_roundtrip`)을 확인한다. Windows는
`packaging/windows/package.ps1 -Arch x64`의 MSI 정적 설치 정책과 임시 폴더의 포터블
업데이트·복원(`native_windows_portable_roundtrip`)을 검사한다.
**개인 PC의 기존 Word를 설치·제거하지 않는다.** 실제 MSI 설치·업그레이드 회귀는 깨끗한
일회용 Windows 환경에서만 `packaging/windows/test-auto-update.ps1`로 별도 실행한다.
정적 검증과 실제 설치 검증을 혼동하지 않는다.

배포 오케스트레이터 회귀 테스트는 `python3 packaging/test_deploy.py`다. 임시 git
원격과 통제된 GitHub/컴파일러 경계를 사용하며 실제 배포·공개를 수행하지 않는다.

## 업데이트 서명 키

자동 업데이트는 릴리스 목록(`WordUpdate.json`)의 서명을 앱에 고정된 공개키로 검증한다.

- 개인키 정본: `~/Dev/keystores/word/update-private-key.hex`. 저장소 밖이며 디렉터리 권한
  `700`, 파일 권한 `600`. 저장소를 지우거나 다시 클론해도 유지한다.
- 공개키: `crates/update/public-key.hex`에 고정해 git에 커밋한다. 환경변수로 덮어쓰는
  런타임 우회는 없고, `cargo test -p wordcraft-update`가 고정값을 확인한다.
- 개인키 값은 세션·명령 인자·출력·git에 넣지 않는다. 배포 스크립트가 파일에서 읽어
  `WORD_UPDATE_PRIVATE_KEY` 환경변수로 서명 하위 프로세스(`word-update-sign`)에만 전달한다.
  파일 위치는 `WORD_UPDATE_KEY_FILE` 또는 `--key-file`로 바꾼다.
- **키를 잃으면 이미 설치된 Word가 새 키를 믿지 못해 더는 업데이트할 수 없고, 사용자가
  직접 재설치해야 한다.** 개인키를 안전한 곳에 따로 백업한다. 키 교체는 자동으로
  전파되지 않는다.

키 생성은 소유자가 최초 한 번만 한다(이미 완료). 공개키는 앱 빌드 전에 커밋한다.

```sh
umask 077
mkdir -m 700 "$HOME/Dev/keystores/word"
cargo run --locked -p wordcraft-update --bin word-update-sign -- keygen \
  "$HOME/Dev/keystores/word/update-private-key.hex" \
  "$HOME/Dev/keystores/word/update-public-key.hex"
cp "$HOME/Dev/keystores/word/update-public-key.hex" crates/update/public-key.hex
```

## 버전과 릴리스

버전의 정본은 `Cargo.toml`의 `[workspace.package].version`이다. 업스트림 0.18.1 기준
첫 포크 버전은 `0.18.2`다. 다음 버전은 `cargo run -p xtask -- version set <버전>`으로 바꾸고
`Cargo.toml`과 `Cargo.lock`을 함께 커밋한다. 배포 스크립트는 이미 공개된 버전이면 이 작업을
대신한다.

버전의 major.minor는 업스트림 `main`을 따라가고 patch는 포크 배포마다 올라간다.
업스트림을 병합할 때 업스트림의 major.minor가 포크보다 높으면 포크 버전을 `X.Y.0`으로
바꾸고(예: 원본 `0.19.0`, 포크 `0.18.5` → `0.19.0`), 같으면 업스트림의 patch와 상관없이
포크 버전을 유지한다. 이후 배포 스크립트가 공개된 번호 다음 patch를 준비하므로
`X.Y.1`, `X.Y.2` 순으로 올라간다. 다음 배포 번호는 항상 이미 공개한 번호보다 높아야 한다.

태그는 **`wordcraft-v<버전>`**, 릴리스 제목은 `Word v<버전>`이다. 원본의 `v<버전>` 태그와
충돌하지 않으며, updater가 저장소 이름 접두사가 붙은 버전을 해석한다. 저장소 이름 변경은
이 규약과 업데이트 주소를 함께 바꾸는 별도 작업이다.

릴리스 초안은 위 OS별 스크립트가 `my-custom`의 확정 SHA로 생성한다. upstream workflow는
동기화용으로 유지하며 이 로컬 배포 경로는 해당 workflow를 호출하거나 그 트리거 브랜치에
푸시하지 않는다. 최종 자산은 9개다.

- `word-<버전>-macos-universal.dmg`: Apple Silicon·Intel용 Word.app.
- `word-cli-<버전>-macos-universal.zip`: macOS CLI(`word-cli`).
- `word-<버전>-windows-x64.msi`: Windows 설치 프로그램.
- `word-<버전>-windows-x64-portable.zip`: 설치 없이 실행하는 Windows 파일.
- `word-<버전>-macos-build.json`, `word-<버전>-windows-build.json`: 소스·글꼴·번역 커밋·검증·파일 해시 증빙.
- `WordUpdate.json`, `WordUpdate.sig`, `SHA256SUMS.txt`: 업데이트 키로 서명한 업데이트 목록과 체크섬.

draft는 저장소 협업자만 볼 수 있다. 두 OS가 다 갖춰지기 전에는 초안을 공개하지 않는다.
공개된 버전은 덮어쓰지 않는다.
