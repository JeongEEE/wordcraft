//! Korean the craft-i18n catalogs don't have, in three tables. [`super::tr`] reads the first two when
//! the interface is Korean:
//!
//! - [`get`]: strings only this personal fork (Word) has: the update card, prompt and window, and
//!   the About window's own sentences. Looked up before the catalog.
//! - [`gap`]: upstream strings the Korean catalog hasn't translated yet. Looked up only when the
//!   catalog has no translation, so a later craft-i18n bump that adds one wins over this table.
//! - [`with_framework_rows`]: the framework's own words (the unsaved-changes question on quitting,
//!   the file prompts), which the framework looks up in the catalogs, never through [`super::tr`].
//!   They join the Korean catalog as rows it doesn't have yet.

use std::collections::HashMap;
use std::sync::LazyLock;

type Table = &'static [(&'static str, &'static str)];

/// Fork-only strings.
const FORK: Table = &[
    // About
    ("About Word", "Word 정보"),
    ("A personal word processor based on WordCraft by the ArtCraft team.", "ArtCraft 팀의 WordCraft를 바탕으로 만든 개인용 워드 프로세서입니다."),
    ("Word releases and source: github.com/JeongEEE/wordcraft", "Word 릴리스와 소스: github.com/JeongEEE/wordcraft"),
    // Ribbon
    ("Collapse the ribbon", "리본 접기"),
    ("Check for\nUpdates…", "업데이트\n확인…"),
    ("Check for Updates", "업데이트 확인"),
    // Updates: the corner card
    ("Checking for updates", "업데이트 확인 중"),
    ("Looking for a new version", "새 버전을 확인하고 있습니다"),
    ("Downloading the update", "업데이트 다운로드 중"),
    ("Trying to connect again", "연결을 다시 시도하고 있습니다"),
    ("Preparing the update download", "업데이트 다운로드 준비 중"),
    ("You can keep working", "작업을 계속하셔도 됩니다"),
    ("Verifying the update", "업데이트 검증 중"),
    ("Checking the downloaded file", "다운로드한 파일을 확인하고 있습니다"),
    ("Update ready", "업데이트 준비됨"),
    ("Open the restart options", "재시작 옵션 열기"),
    // Updates: the restart prompt
    ("Update ready to install", "업데이트 준비 완료"),
    ("The new version is ready.\nRestart Word now?", "새 버전을 사용할 준비가 되었습니다.\n지금 Word를 재시작할까요?"),
    (
        "Documents with unsaved changes are asked about first,\nand the open files open again after the restart.",
        "저장하지 않은 문서는 먼저 확인하고,\n열려 있던 파일은 재시작 후 다시 엽니다.",
    ),
    ("Restart to Update", "재시작하여 업데이트"),
    ("Later", "나중에"),
    // Updates: the window and the notices
    ("Word Update", "Word 업데이트"),
    ("You can check for updates.", "업데이트를 확인할 수 있습니다."),
    ("Word {version} is up to date.", "Word {version} — 최신 버전입니다."),
    ("Word {version} is available.", "Word {version} 새 버전이 있습니다."),
    ("Get the installer from the releases page", "릴리스에서 설치 파일 받기"),
    ("Couldn't prepare the update.", "업데이트를 준비하지 못했습니다."),
    ("Try Again", "다시 시도"),
    ("Download from the releases page", "릴리스에서 직접 받기"),
    ("Word was updated to {version}.", "Word {version} 업데이트가 완료되었습니다."),
    ("Couldn't install the update. {reason}", "업데이트를 설치하지 못했습니다. {reason}"),
    (
        "Updates are off in this window: WordCraft was started with another program's AppImage settings. Start WordCraft from its own menu entry to update it.",
        "이 창에서는 업데이트를 사용할 수 없습니다. WordCraft가 다른 프로그램의 AppImage 설정으로 시작되었습니다. 업데이트하려면 WordCraft 자체 메뉴 항목에서 시작하세요.",
    ),
    ("Update check is not supported in the web browser.", "웹 브라우저에서는 업데이트 확인을 지원하지 않습니다."),
];

/// Upstream strings without a catalog translation.
const GAP: Table = &[
    // Home › Paragraph Tools
    ("Paragraph\nTools", "단락\n도구"),
    ("Paragraph Tools", "단락 도구"),
    ("Add Blank Paragraphs Between", "단락 사이에 빈 단락 추가"),
    ("Insert Blank Paragraphs Between", "단락 사이에 빈 단락 삽입"),
    ("Delete Blank Paragraphs", "빈 단락 삭제"),
    ("Delete Blank Paragraphs in the Whole Document", "문서 전체의 빈 단락 삭제"),
    ("Remove Extra Spaces", "불필요한 공백 제거"),
    ("Remove Extra Spaces in the Whole Document", "문서 전체의 불필요한 공백 제거"),
    ("Remove Leading Spaces", "앞쪽 공백 제거"),
    ("Remove Leading Spaces in the Whole Document", "문서 전체의 앞쪽 공백 제거"),
    ("Remove Line Breaks", "줄 바꿈 제거"),
    ("Join Lines", "줄 합치기"),
    ("Separate Paragraphs", "단락 나누기"),
    ("Split Lines into Paragraphs", "줄을 단락으로 나누기"),
    ("Convert Line Breaks to Paragraphs", "줄 바꿈을 단락으로 변환"),
    ("Convert Indents to Spaces", "들여쓰기를 공백으로 변환"),
    ("Convert Spaces to Indentation", "공백을 들여쓰기로 변환"),
    ("Indent Leading Spaces", "앞쪽 공백을 들여쓰기로 변환"),
    ("Indents to Leading Spaces", "들여쓰기를 앞쪽 공백으로 변환"),
    ("Scale Font Sizes", "글꼴 크기 조절"),
    ("Fit\nOne Page", "한 페이지에\n맞추기"),
    ("Fit One Page", "한 페이지에 맞추기"),
    ("Define New Bullet…", "새 글머리 기호 정의…"),
    ("Bullet character", "글머리 기호 문자"),
    ("Symbol…", "기호…"),
    ("Emphasis Mark", "강조점"),
    ("Special:", "첫 줄:"),
    ("By:", "값:"),
    // Review
    ("Accept All Changes and Stop Tracking", "모든 변경 내용 수락 후 추적 중지"),
    ("Accept and Move to Next", "수락 후 다음으로 이동"),
    ("Reject All Changes and Stop Tracking", "모든 변경 내용 거부 후 추적 중지"),
    ("Reject and Move to Next", "거부 후 다음으로 이동"),
    ("Not in Dictionary", "사전에 없음"),
    ("Suggestions", "제안"),
    ("Ignore Once", "한 번 무시"),
    ("More options", "옵션 더 보기"),
    ("Moved from:", "이동 전:"),
    ("Moved to:", "이동 후:"),
    ("Delete Previous Sentence", "이전 문장 삭제"),
    // Highlight and page colors
    ("Bright Green", "밝은 초록"),
    ("Dark Blue", "어두운 파랑"),
    ("Dark Red", "어두운 빨강"),
    ("Dark Yellow", "어두운 노랑"),
    ("Teal", "청록"),
    ("Turquoise", "옥색"),
    ("Gray-25%", "회색 25%"),
    ("Gray-50%", "회색 50%"),
    ("Sepia", "세피아"),
    ("Inverse", "반전"),
    ("Page color", "페이지 색"),
    ("Reading Page Color", "읽기 페이지 색"),
    ("Document background", "문서 배경"),
    ("Screen Paper Color", "화면 용지 색"),
    ("Screen paper color:", "화면 용지 색:"),
    (
        "Screen only. Document backgrounds, printing and exports keep their colours. Dark page and explicit Read Mode colours take precedence.",
        "화면에서만 적용됩니다. 문서 배경, 인쇄, 내보내기에는 원래 색이 유지됩니다. 어두운 페이지와 읽기 모드에서 직접 지정한 색이 우선합니다.",
    ),
    ("Item Color", "항목 색"),
    ("Shape color", "도형 색"),
    ("Style Color", "스타일 색"),
    ("Set the colors in the Text Pane to use custom colors.", "사용자 지정 색을 쓰려면 텍스트 창에서 색을 설정하세요."),
    // Window
    ("Arrange All", "모두 정렬"),
    ("Switch\nWindows", "창\n전환"),
    ("Switch Windows", "창 전환"),
    ("View Side by Side", "나란히 보기"),
    ("Synchronous Scrolling", "동시 스크롤"),
    ("No other WordCraft windows are open.", "열려 있는 다른 WordCraft 창이 없습니다."),
    ("Close Document", "문서 닫기"),
    ("Select Canvas Member", "캔버스 항목 선택"),
    ("Strict Export Report", "엄격 모드 내보내기 보고서"),
    // Panes
    ("Pane Options", "창 옵션"),
    ("Dock Left", "왼쪽에 도킹"),
    ("Dock Right", "오른쪽에 도킹"),
    ("Float", "띄우기"),
    ("Move Pane: {title}", "창 이동: {title}"),
    ("Resize Pane", "창 크기 조절"),
    ("Styles Pane Position", "스타일 창 위치"),
    ("{name} options", "{name} 옵션"),
    // Why a command is unavailable
    ("a shape inside a drawing canvas is placed by the canvas", "그리기 캔버스 안의 도형은 캔버스가 위치를 정함"),
    ("not in Read Mode", "읽기 모드에서는 사용할 수 없음"),
    ("not in a protected document", "보호된 문서에서는 사용할 수 없음"),
    ("not with tracked changes", "변경 내용 추적 중에는 사용할 수 없음"),
    ("only floating connectors in the body text can be glued", "본문의 떠 있는 연결선만 붙일 수 있음"),
    ("only for Markdown (.md) files", "Markdown(.md) 파일에서만 사용할 수 있음"),
    ("only in Markdown Source", "Markdown 소스에서만 사용할 수 있음"),
    ("only the desktop app has several windows", "창을 여러 개 쓰는 것은 데스크톱 앱뿐임"),
    ("turn on View Side by Side first", "먼저 나란히 보기를 켜세요"),
    // Markdown source
    ("Markdown Source", "Markdown 소스"),
    ("Markdown Source Text", "Markdown 소스 텍스트"),
    (
        "This document has content Markdown can't hold. Changing the text here removes it; one Undo afterwards brings it back.",
        "이 문서에는 Markdown으로 표현할 수 없는 내용이 있습니다. 여기서 텍스트를 바꾸면 그 내용이 사라지며, 이후 실행 취소를 한 번 하면 되돌아옵니다.",
    ),
    ("Markdown Source takes up to 2 MB of text", "Markdown 소스는 텍스트 2 MB까지 다룰 수 있습니다"),
    ("Nothing to undo", "실행 취소할 작업이 없습니다"),
    ("Nothing to redo", "다시 실행할 작업이 없습니다"),
    (
        "Saved. Edits in the formatted view meant writing the Markdown out again, so its own layout wasn't kept.",
        "저장했습니다. 서식 보기에서 편집해 Markdown을 다시 써야 했으므로 원래 Markdown의 배치는 유지되지 않았습니다.",
    ),
    // Fields and references
    ("Error! Bookmark not defined.", "오류! 책갈피가 정의되지 않았습니다."),
    ("Error! Reference source not found.", "오류! 참조 원본을 찾을 수 없습니다."),
    ("Heading", "제목"),
    ("Heading text", "제목 텍스트"),
    ("Reference type:", "참조 유형:"),
    ("Insert reference to:", "삽입할 참조 대상:"),
    ("For which item:", "대상 항목:"),
    ("Nothing of this type in the document.", "문서에 이 유형의 항목이 없습니다."),
    ("Corporate Author", "단체 작성자"),
    ("No new sources in that file", "해당 파일에 새 원본이 없습니다"),
    ("Imported 1 source", "원본 1개를 가져왔습니다"),
    ("Imported {n} sources", "원본 {n}개를 가져왔습니다"),
    // Macros
    ("Macro name:", "매크로 이름:"),
    ("There are no macros yet. Record one to add it here.", "아직 매크로가 없습니다. 기록하면 여기에 추가됩니다."),
    ("Run", "실행"),
    ("Stop Recording", "기록 중지"),
    ("Record…", "기록…"),
    // Language
    ("Separate text with", "텍스트 구분 기호"),
    ("Mark the whole document as:", "문서 전체를 다음 언어로 표시:"),
    ("Apply to the whole document", "문서 전체에 적용"),
    ("Spanish (Latin America)", "스페인어(라틴아메리카)"),
    ("Spanish (Uruguay)", "스페인어(우루과이)"),
    ("The number must be a divisor of {max}.", "{max}의 약수여야 합니다."),
    ("Update automatically", "자동으로 업데이트"),
    // Proofing options
    ("*Bold* and _italic_ as real formatting", "*굵게*와 _기울임꼴_을 실제 서식으로"),
    ("Add a period with double-space", "공백 두 번으로 마침표 추가"),
    ("Built-in Heading styles", "기본 제공 제목 스타일"),
    ("Set left- and first-indent with tabs and backspaces", "탭과 백스페이스로 왼쪽 및 첫 줄 들여쓰기 설정"),
    ("Automatically add words to list (First Letter)", "목록에 단어 자동 추가(첫 글자)"),
    ("Automatically add words to list (INitial CAps)", "목록에 단어 자동 추가(첫 두 글자 대문자)"),
    // Go To, Find, command search
    ("Select a place in this document:", "이 문서에서 위치 선택:"),
    ("Top of the Document", "문서 맨 위"),
    ("Find “{text}” in the document", "문서에서 “{text}” 찾기"),
    ("Enter bookmark name:", "책갈피 이름 입력:"),
    ("This document has no bookmarks.", "이 문서에는 책갈피가 없습니다."),
    ("Type a page number, or +2 / -1.", "페이지 번호를 입력하세요. +2 또는 -1도 입력할 수 있습니다."),
    ("Hidden text is not shown unless you display it.", "숨김 텍스트는 표시하도록 설정해야 보입니다."),
    ("Type a size in any unit, such as 3 cm or 300 px.", "단위를 붙여 크기를 입력하세요. 예: 3 cm, 300 px"),
    ("Column Edge", "열 가장자리"),
    ("Current Document", "현재 문서"),
    (
        "Crop on the page works on unturned pictures: set Rotation to 0° first, or use the crop boxes",
        "페이지에서 자르기는 회전하지 않은 그림에서만 됩니다. 먼저 회전을 0°로 설정하거나 자르기 상자를 사용하세요",
    ),
    // Document properties
    ("Advanced Properties…", "고급 속성…"),
    ("View and edit the document's custom properties.", "문서의 사용자 지정 속성을 보고 편집합니다."),
    ("Property names are 1–{max} characters.", "속성 이름은 1~{max}자입니다."),
    ("Property values are at most {max} bytes.", "속성 값은 최대 {max}바이트입니다."),
    ("Only text properties can be changed.", "텍스트 속성만 변경할 수 있습니다."),
    ("Select a property to change.", "변경할 속성을 선택하세요."),
    ("This document has no custom properties.", "이 문서에는 사용자 지정 속성이 없습니다."),
    (
        "Text properties only: Word's number, date and yes-no types aren't offered.",
        "텍스트 속성만 지원합니다. Word의 숫자, 날짜, 예/아니요 형식은 제공하지 않습니다.",
    ),
    // Recovery
    ("Restore opens an unsaved copy. Your original file stays unchanged.", "복원하면 저장되지 않은 사본이 열립니다. 원본 파일은 그대로 유지됩니다."),
    ("Recovery is disabled while this document has a password.", "이 문서에 암호가 걸려 있는 동안에는 복구를 사용할 수 없습니다."),
    ("Local recovery is unavailable or still loading.", "로컬 복구를 사용할 수 없거나 아직 불러오는 중입니다."),
    ("No recovery copies.", "복구본이 없습니다."),
    ("Recovery copy", "복구본"),
    ("Discard…", "삭제…"),
    ("Continue without restoring", "복원하지 않고 계속"),
    ("Discard the recovery copy of {name}?", "{name}의 복구본을 삭제할까요?"),
    ("This cannot be undone. Your original file stays unchanged.", "이 작업은 되돌릴 수 없습니다. 원본 파일은 그대로 유지됩니다."),
    // Messages
    ("Removed {name} from the recent list", "최근 목록에서 {name}을(를) 삭제했습니다"),
    ("Removed {count} documents from the recent list", "최근 목록에서 문서 {count}개를 삭제했습니다"),
    (
        "Couldn't save {name}, so it is still open and nothing was lost: {error}",
        "{name}을(를) 저장하지 못했습니다. 문서는 열려 있으며 잃은 내용은 없습니다: {error}",
    ),
    ("Wait for the print job to finish submitting before closing.", "닫기 전에 인쇄 작업 제출이 끝날 때까지 기다리세요."),
    ("Could not save the preferences: {error}", "환경 설정을 저장하지 못했습니다: {error}"),
    // Status bar
    (
        "The spelling dictionary for {language} couldn't be read, so it isn't checked. File: {file}. Reason: {reason}",
        "{language} 맞춤법 사전을 읽을 수 없어 검사하지 않습니다. 파일: {file}. 원인: {reason}",
    ),
    ("{language} (dictionary unreadable)", "{language} (사전을 읽을 수 없음)"),
    ("{language}: {file} (unreadable: {reason})", "{language}: {file} (읽을 수 없음: {reason})"),
    ("Text Size", "텍스트 크기"),
    ("{chars} characters", "문자 {chars}개"),
    ("{selected} of {chars} characters", "문자 전체 {chars}개 중 {selected}개 선택"),
    ("{chars} characters (no spaces)", "문자 {chars}개(공백 제외)"),
    ("{selected} of {chars} characters (no spaces)", "문자 전체 {chars}개(공백 제외) 중 {selected}개 선택"),
    ("Status bar", "상태 표시줄"),
    ("Status bar count:", "상태 표시줄 집계:"),
    (
        "You can also right-click the count in the status bar to change it.",
        "상태 표시줄의 집계를 마우스 오른쪽 버튼으로 클릭해서 바꿀 수도 있습니다.",
    ),
    // Options
    ("Scrolling", "스크롤"),
    ("Mouse wheel scroll speed:", "마우스 휠 스크롤 속도:"),
    ("Touchpad scroll speed:", "터치패드 스크롤 속도:"),
    ("On X11, the touchpad follows the mouse wheel speed.", "X11에서는 터치패드가 마우스 휠 속도를 따릅니다."),
    ("Display server:", "디스플레이 서버:"),
    ("Tell me when a document's missing fonts can be downloaded", "문서에 없는 글꼴을 다운로드할 수 있을 때 알림"),
    // Missing fonts
    ("Missing Fonts", "없는 글꼴"),
    ("Get the Missing Fonts", "없는 글꼴 받기"),
    ("Get the Missing Fonts…", "없는 글꼴 받기…"),
    ("Shown in substitute fonts. Click for details.", "대체 글꼴로 표시됩니다. 클릭하면 자세히 볼 수 있습니다."),
    ("Not installed — shown as", "설치되지 않음 — 표시 글꼴"),
    // Counts (`tn`): the `other` form, Korean having one.
    ("{n} fonts not installed", "설치되지 않은 글꼴 {n}개"),
    ("…and {n} more", "…외 {n}개"),
    ("{n} missing fonts can be downloaded", "없는 글꼴 {n}개를 다운로드할 수 있습니다"),
    (
        "{font} isn't installed on this computer, so its text is shown in {shown}. Line breaks and page count can differ from where the document was made. If you have the font file, add it with Add Font… at the end of this list.",
        "{font} 글꼴이 이 컴퓨터에 설치되어 있지 않아 텍스트가 {shown}(으)로 표시됩니다. 줄 바꿈과 페이지 수가 문서를 만든 곳과 다를 수 있습니다. 글꼴 파일이 있으면 이 목록 끝의 글꼴 추가…로 추가하세요.",
    ),
    (
        "None of the fonts this document is missing can be downloaded free of charge from their publisher.",
        "이 문서에 없는 글꼴 중 배포처에서 무료로 다운로드할 수 있는 것이 없습니다.",
    ),
    (
        "Text in a missing font is shown in another font, so it may not look as it was written. The downloads are free of charge.",
        "없는 글꼴로 된 텍스트는 다른 글꼴로 표시되므로 작성했을 때와 다르게 보일 수 있습니다. 다운로드는 무료입니다.",
    ),
    ("Shown as {fonts} · {publisher}", "{fonts}(으)로 표시 · {publisher}"),
    ("Its licence limits who may use it: read the terms first.", "라이선스에 사용 대상 제한이 있습니다. 먼저 약관을 읽어 보세요."),
    ("Download from {publisher}", "{publisher}에서 다운로드"),
    (
        "Fonts come with licence terms. Read the publisher's terms on the download page: following them is up to you.",
        "글꼴에는 라이선스 약관이 있습니다. 다운로드 페이지에서 배포처의 약관을 읽어 보세요. 약관을 지키는 것은 사용자 책임입니다.",
    ),
    ("Download the font. If it comes as a .zip file, unpack it.", "글꼴을 다운로드하세요. .zip 파일이면 압축을 풀어 주세요."),
    ("Then add it to WordCraft", "그런 다음 WordCraft에 추가하세요"),
    ("Pick the font files:", "글꼴 파일 선택:"),
    ("Don't show this again", "다시 표시 안 함"),
    (
        "Or install it for every app: select the font files, right-click and choose Install, then restart WordCraft.",
        "모든 앱에서 쓰려면 글꼴 파일을 선택하고 마우스 오른쪽 버튼을 눌러 설치를 고른 다음 WordCraft를 다시 시작하세요.",
    ),
    (
        "Or install it for every app: double-click each font file and choose Install in Font Book, then restart WordCraft.",
        "모든 앱에서 쓰려면 글꼴 파일을 각각 더블클릭해 Font Book에서 설치를 선택한 다음 WordCraft를 다시 시작하세요.",
    ),
    (
        "Or install it for every app: copy the font files to the .local/share/fonts folder in your home folder, then restart WordCraft.",
        "모든 앱에서 쓰려면 글꼴 파일을 홈 폴더의 .local/share/fonts 폴더에 복사한 다음 WordCraft를 다시 시작하세요.",
    ),
    // Language packs
    ("Language Packs", "언어 팩"),
    ("Language data", "언어 데이터"),
    ("Install Language Pack", "언어 팩 설치"),
    ("Remove Language Pack", "언어 팩 제거"),
    ("Language packs aren't available in the browser", "브라우저에서는 언어 팩을 사용할 수 없습니다"),
    ("Language packs aren't available in the browser.", "브라우저에서는 언어 팩을 사용할 수 없습니다."),
    (
        "Language packs need the font store folder, and there is none on this computer.",
        "언어 팩에는 글꼴 저장 폴더가 필요한데, 이 컴퓨터에는 없습니다.",
    ),
    (
        "The spelling and hyphenation data WordCraft uses, and their licences.",
        "WordCraft가 사용하는 맞춤법 및 하이픈 넣기 데이터와 그 라이선스입니다.",
    ),
    ("Couldn't get the list of languages: {error}", "언어 목록을 가져오지 못했습니다: {error}"),
    ("Couldn't install: {error}", "설치하지 못했습니다: {error}"),
    (
        "Only you can turn on downloading: click the button yourself. Agents can install once it is on.",
        "다운로드는 사용자만 켤 수 있습니다. 직접 버튼을 클릭하세요. 켜 둔 뒤에는 에이전트가 설치할 수 있습니다.",
    ),
    ("Couldn't turn on downloading: {error}", "다운로드를 켜지 못했습니다: {error}"),
    ("Couldn't remove: {error}", "제거하지 못했습니다: {error}"),
    ("{language} proofing isn't installed.", "{language} 맞춤법 검사가 설치되어 있지 않습니다."),
    ("Installing… {percent}%", "설치 중… {percent}%"),
    ("Install ({size})", "설치({size})"),
    ("Getting the list of languages…", "언어 목록을 가져오는 중…"),
    (
        "Installing turns on downloading of fonts and Language packs from GitHub.",
        "설치하면 GitHub에서 글꼴과 언어 팩을 다운로드하는 기능이 켜집니다.",
    ),
    ("Install", "설치"),
    ("Licence: {licence}. Source: {source}", "라이선스: {licence}. 출처: {source}"),
    ("Language packs come from a test source: {source}", "언어 팩을 테스트용 출처에서 가져옵니다: {source}"),
    ("Installed", "설치됨"),
    ("Get the list of languages again", "언어 목록 다시 가져오기"),
    // Printing
    ("Cancel printing", "인쇄 취소"),
    ("Pages in the range", "범위 안의 페이지"),
    ("Pages per sheet", "용지당 페이지 수"),
    (
        "Booklets: duplex on the short edge. Blank pages are added at the end.",
        "소책자: 짧은 가장자리 기준 양면 인쇄입니다. 빈 페이지는 끝에 추가됩니다.",
    ),
    ("Scale to paper size", "용지 크기에 맞춤"),
    ("Sheet {page} of {pages}", "용지 {page}/{pages}"),
    ("Previous sheet", "이전 용지"),
    ("Next sheet", "다음 용지"),
    (
        "Cancellation could not be confirmed here. The printer driver may still be processing the job. If it appears in the system's print queue, cancel it there.",
        "여기서는 취소를 확인할 수 없습니다. 프린터 드라이버가 아직 작업을 처리 중일 수 있습니다. 시스템 인쇄 대기열에 보이면 거기서 취소하세요.",
    ),
    ("Printing cancelled. Nothing was sent to the printer.", "인쇄를 취소했습니다. 프린터로 보낸 내용은 없습니다."),
    // File browser
    ("Folder path", "폴더 경로"),
    ("File type", "파일 형식"),
    ("No items to show.", "표시할 항목이 없습니다."),
];

/// The framework's dialog words (craft `DialogWords`) Word shows that the Korean catalog hasn't translated, by their
/// English text.
const FRAMEWORK: Table = &[
    ("Do you want to save the changes you made to “{name}” before closing?", "닫기 전에 “{name}”의 변경 사항을 저장하시겠습니까?"),
    ("Do you want to save the changes you made to “{name}” before quitting?", "종료하기 전에 “{name}”의 변경 사항을 저장하시겠습니까?"),
    (
        "Revert “{name}” to the last saved version? Your changes will be lost.",
        "“{name}”을(를) 마지막으로 저장한 버전으로 되돌리시겠습니까? 변경 사항은 사라집니다.",
    ),
    ("Apply to All", "모두에 적용"),
    ("Confirm", "확인"),
    ("Damaged file", "손상된 파일"),
    ("“{name}” has damaged data. Open it anyway?", "“{name}”에 손상된 데이터가 있습니다. 그래도 여시겠습니까?"),
    ("Open Anyway", "그래도 열기"),
    ("File changed", "파일이 변경됨"),
    ("“{name}” was changed by another app after it was opened or saved here.", "“{name}”을(를) 여기서 열거나 저장한 뒤에 다른 앱이 변경했습니다."),
    (
        "Replace it with this document, or cancel and use Save As to keep both.",
        "이 문서로 바꾸거나, 취소한 뒤 다른 이름으로 저장을 사용해 둘 다 보관하세요.",
    ),
    ("Replace file", "파일 바꾸기"),
    ("“{name}” already exists. Replace it?", "“{name}”이(가) 이미 있습니다. 바꾸시겠습니까?"),
    ("The name in the save dialog was “{answered}”.", "저장 대화상자에 입력한 이름은 “{answered}”입니다."),
    ("Recover Unsaved Work", "저장하지 않은 작업 복구"),
    (
        "“{name}” has unsaved changes from an earlier run, autosaved {age}.",
        "“{name}”에 이전 실행에서 저장하지 않은 변경 사항이 있습니다({age} 자동 저장).",
    ),
];

/// The Korean app catalog `app` with a row for each [`FRAMEWORK`] word that neither it nor the shared catalog `shared`
/// has. A row added later upstream is then the one shown, and no row is defined twice.
pub(super) fn with_framework_rows(shared: &str, app: &str) -> String {
    let has = |en: &str| [shared, app].iter().any(|catalog| catalog.lines().any(|line| line.split('\t').nth(1) == Some(en)));
    let mut out = app.to_owned();
    if !out.is_empty() && !out.ends_with('\n') {
        out.push('\n');
    }
    for (en, ko) in FRAMEWORK.iter().filter(|(en, _)| !has(en)) {
        out.push_str(&format!("\t{en}\t{ko}\n"));
    }
    out
}

fn index(table: Table) -> HashMap<&'static str, &'static str> {
    table.iter().copied().collect()
}

/// The Korean of fork string `s`, or `None` when it isn't one.
pub(super) fn get(s: &str) -> Option<&'static str> {
    static INDEX: LazyLock<HashMap<&str, &str>> = LazyLock::new(|| index(FORK));
    INDEX.get(s).copied()
}

/// The Korean of upstream string `s` the catalog doesn't translate, or `None`. The caller asks
/// the catalog first.
pub(super) fn gap(s: &str) -> Option<&'static str> {
    static INDEX: LazyLock<HashMap<&str, &str>> = LazyLock::new(|| index(GAP));
    INDEX.get(s).copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `{name}` placeholders of `s`, sorted.
    fn placeholders(s: &str) -> Vec<&str> {
        let mut found: Vec<&str> = s.split('{').skip(1).filter_map(|rest| rest.split_once('}').map(|(name, _)| name)).collect();
        found.sort_unstable();
        found
    }

    #[test]
    fn translations_keep_placeholders_line_breaks_and_ellipsis() {
        for (name, table) in [("FORK", FORK), ("GAP", GAP), ("FRAMEWORK", FRAMEWORK)] {
            let mut keys = std::collections::HashSet::new();
            for (en, ko) in table {
                assert!(keys.insert(*en), "{name}: duplicate key {en:?}");
                assert_eq!(placeholders(en), placeholders(ko), "{name}: placeholders differ in {en:?}");
                assert_eq!(en.matches('\n').count(), ko.matches('\n').count(), "{name}: line breaks differ in {en:?}");
                assert_eq!(en.ends_with('…'), ko.ends_with('…'), "{name}: trailing ellipsis differs in {en:?}");
            }
        }
    }
}
