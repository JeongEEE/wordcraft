# Word 포크 개발·배포

Word는 `JeongEEE/wordcraft`에서 관리하는 WordCraft 기반 개인 워드 프로세서다.
사용자에게 보이는 앱 이름은 **Word**, 유지관리자 표기는 `JeongEEE`, Cargo 패키지는
`wordcraft`와 `wordcraft-cli`다. 원본의 crate 이름, 명령 ID, `WORDCRAFT_*` 환경변수를
유지해 업스트림 변경을 계속 받아들인다.

> 초안: 자동 업데이트와 배포 스크립트는 아직 들어오지 않았다. 해당 작업이 끝나면 이 문서의
> 업데이트·배포 절을 채운다.

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

## 버전

정본은 `Cargo.toml`의 `[workspace.package].version`이다. major.minor는 업스트림을 따르고
patch는 포크 배포마다 올린다.
