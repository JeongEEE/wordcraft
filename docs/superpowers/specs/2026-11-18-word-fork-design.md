# Word 개인 포크 설계

`JeongEEE/wordcraft`를 **Word**라는 개인 앱으로 배포한다. 모델은 같은 소유자의
`JeongEEE/gridcraft` 포크(앱 이름 Grid)다. 그 포크의 `my-custom` 브랜치에서 아래 묶음을
가져오되, 코드를 그대로 cherry-pick하지 않고 wordcraft의 현재 구조에 맞춰 옮긴다.

## 승인된 요구사항

- 앱 표시 이름 **Word**, 유지관리자 표기 `JeongEEE`.
- `main`은 `upstream/main`(storytold/wordcraft) 동기화 전용, 개인 변경은 `my-custom`.
- Mac/Windows 각자의 PC에서 빌드해 GitHub 릴리스 초안에 올리고, Mac에서 서명 후
  **확인 질문 없이 바로 공개**한다. GitHub Actions는 배포에 쓰지 않는다.
- 설치본은 이 포크의 공개 릴리스만 보고 서명 검증 후 스스로 업데이트한다.

## 원본과 분리하는 식별자

| 항목 | Word |
|---|---|
| updater app id / 배포 파일 stem | `word` |
| 업데이트 저장소 / 태그 | `JeongEEE/wordcraft` / `wordcraft-v<버전>` |
| macOS 앱 / bundle id | `Word.app` / `io.github.jeongeee.word` |
| macOS 설정 | `~/Library/Application Support/Word` |
| Windows 설치 폴더 | `%LOCALAPPDATA%\Programs\JeongEEE\Word` |
| Windows 설정 / registry | `%APPDATA%\Word` / `Software\JeongEEE\Word` |
| Windows ProgID | `JeongEEE.Word.Document` 등 — MS Word의 `Word.Document.*`와 겹치지 않게 접두사 |
| Windows UpgradeCode·component GUID | 새로 생성해 고정, 이후 릴리스에서 유지 |
| 실행 파일 | `word.exe`, `word-cli.exe`, macOS `Word.app` / `word-cli` |

Cargo 패키지·crate·명령 ID·`WORDCRAFT_*` 환경변수 이름은 유지한다(업스트림 병합 충돌 최소화).
ArtCraft 로고·워드마크는 배포물과 화면에서 빼고, WordCraft 기반이라는 텍스트 출처와
원본 저작권·라이선스는 남긴다. 파일 연결은 Open With 후보만 등록하고 기본 앱은 바꾸지 않는다.

## 자동 업데이트

gridcraft 포크의 `crates/update`(포크 전용 업데이트 crate)와 `updates_ui.rs`(+`updates_ui/view.rs`)
최종 상태를 옮긴다. 포함하는 동작:

- 시작 시·Help·6시간 주기로 포크의 공개 릴리스만 확인, 백그라운드 다운로드·검증.
- `WordUpdate.json`(저장소·태그·버전·파일명·크기·SHA256) + `WordUpdate.sig`(Ed25519).
  공개키는 `crates/update/public-key.hex`에 고정, 런타임 덮어쓰기 없음.
- 오른쪽 아래 진행 카드 → 준비 완료 모달(**재시작하여 업데이트 / 나중에**) → 저장 확인을 거친
  재시작 → 성공 토스트. 재시작 시 열려 있던 파일을 다시 연다.
- Mac: DMG readonly 마운트·bundle id·버전·codesign 확인 후 교체, 실패 시 복원.
  Windows: MSI는 원래 설치 문맥 유지(1619 오류 수정 포함), 포터블은 정해진 파일만 교체·복원.
- 업스트림 craft-update의 시작 시 확인은 이 경로로 대체한다(원본 저장소 릴리스를 보지 않게).

개인키는 저장소 밖 `~/Dev/keystores/word/update-private-key.hex`(디렉터리 700, 파일 600).
**키 생성은 소유자가 직접 터미널에서 한 번** 실행하고, 세션은 공개키 파일만 읽는다.
개인키 값은 명령 인자·출력·git에 넣지 않는다.

## 배포 스크립트

gridcraft의 `deploy-macos.sh`, `deploy-windows.ps1`, `deploy.sh`, `packaging/deploy.py`,
`deploy_console.py`, `local_build.py`, `release_assets.py`, `test_deploy.py`를 옮긴다.
동작은 gridcraft 최신과 같다: 같은 SHA·버전으로 두 OS 업로드 → 키가 있는 Mac에서 finalize
→ 9개 자산 재확인 → 묻지 않고 공개. `--draft`는 공개를 미룬다.

wordcraft는 화면 번역을 `craft-i18n` 저장소에서 빌드 때 넣으므로, 로컬 빌드는 글꼴
(`craft-fonts` 고정 커밋)과 함께 `craft-i18n.rev`의 커밋도 깨끗한 체크아웃으로 준비해
`CRAFT_I18N_DIR`에 넣는다. 없으면 화면이 영어로만 나오므로 배포를 막는다.
CLI 검증은 wordcraft CLI의 문서 저장·재열기 명령으로 바꾼다.

## 버전

정본은 `Cargo.toml`의 `[workspace.package].version`. major.minor는 업스트림을 따르고
patch는 포크 배포마다 올린다. 업스트림 0.18.1 기준 첫 포크 버전은 `0.18.2`.

## 포크 전용 문구 번역

업데이트 카드·모달처럼 포크에서 새로 생긴 문구는 craft-i18n에 없다. 포크 안에 작은
한국어 보조 표(포크 전용 키만)를 두고 craft-i18n 조회보다 먼저 찾는다. 업스트림 문구는 건드리지 않는다.

## 범위 밖

gridcraft 포크의 앱 기능 수정(한글 IME, 창 위치 기억, 리본 접기, 아랍어 테스트 정리,
한국어 번역 보강), Linux/웹 배포, 유료 코드 서명, 업스트림 PR, 저장소 이름 변경.
원하면 이후 별도 작업으로 옮긴다.

## 검증

- `cargo test -p wordcraft-update`, fork identity 테스트, `packaging/test_fork_contract.py`,
  `packaging/test_deploy.py`, `cargo xtask ci`(실패는 실제 결과대로 기록).
- Mac: `packaging/macos/package.sh --arch universal`로 `Word.app` DMG 생성, codesign·plist·버전,
  DMG 업데이트·복원 smoke, CLI 저장·재열기.
- Windows는 이 Mac에서 빌드할 수 없다. 정적 MSI 정책 검사와 스크립트 테스트까지만 하고,
  실제 Windows 빌드는 소유자의 Windows PC에서 첫 배포 때 확인한다.
- 개인 PC의 기존 WordCraft 설치·설정은 건드리지 않는다.
