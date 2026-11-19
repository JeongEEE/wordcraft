//! Korean for the strings only this personal fork (Word) has: the update card, prompt and
//! window, and the About window's own sentences. The craft-i18n catalogs don't have them, so
//! [`super::tr`] looks here first when the interface is Korean. Upstream strings stay with the
//! catalogs: nothing here overrides them.

/// The Korean of fork string `s`, or `None` when it isn't one.
pub(super) fn get(s: &str) -> Option<&'static str> {
    Some(match s {
        // About
        "About Word" => "Word 정보",
        "A personal word processor based on WordCraft by the ArtCraft team." => "ArtCraft 팀의 WordCraft를 바탕으로 만든 개인용 워드 프로세서입니다.",
        "Word releases and source: github.com/JeongEEE/wordcraft" => "Word 릴리스와 소스: github.com/JeongEEE/wordcraft",
        // Ribbon
        "Collapse the ribbon" => "리본 접기",
        // Updates: the corner card
        "Checking for updates" => "업데이트 확인 중",
        "Looking for a new version" => "새 버전을 확인하고 있습니다",
        "Downloading the update" => "업데이트 다운로드 중",
        "Trying to connect again" => "연결을 다시 시도하고 있습니다",
        "Preparing the update download" => "업데이트 다운로드 준비 중",
        "You can keep working" => "작업을 계속하셔도 됩니다",
        "Verifying the update" => "업데이트 검증 중",
        "Checking the downloaded file" => "다운로드한 파일을 확인하고 있습니다",
        "Update ready" => "업데이트 준비됨",
        "Open the restart options" => "재시작 옵션 열기",
        // Updates: the restart prompt
        "Update ready to install" => "업데이트 준비 완료",
        "The new version is ready.\nRestart Word now?" => "새 버전을 사용할 준비가 되었습니다.\n지금 Word를 재시작할까요?",
        "Documents with unsaved changes are asked about first,\nand the open files open again after the restart." => {
            "저장하지 않은 문서는 먼저 확인하고,\n열려 있던 파일은 재시작 후 다시 엽니다."
        }
        "Restart to Update" => "재시작하여 업데이트",
        "Later" => "나중에",
        // Updates: the window and the notices
        "Word Update" => "Word 업데이트",
        "You can check for updates." => "업데이트를 확인할 수 있습니다.",
        "Word {version} is up to date." => "Word {version} — 최신 버전입니다.",
        "Word {version} is available." => "Word {version} 새 버전이 있습니다.",
        "Get the installer from the releases page" => "릴리스에서 설치 파일 받기",
        "Couldn't prepare the update." => "업데이트를 준비하지 못했습니다.",
        "Try Again" => "다시 시도",
        "Download from the releases page" => "릴리스에서 직접 받기",
        "Word was updated to {version}." => "Word {version} 업데이트가 완료되었습니다.",
        "Couldn't install the update. {reason}" => "업데이트를 설치하지 못했습니다. {reason}",
        _ => return None,
    })
}
