"""Word's install identity stays apart from upstream WordCraft; nothing is installed or registered.

    python3 packaging/test_fork_contract.py
"""

import json
from pathlib import Path
import plistlib
import re
import unittest
import xml.etree.ElementTree as ET


ROOT = Path(__file__).resolve().parents[1]
WIX = "{http://wixtoolset.org/schemas/v4/wxs}"
# Upstream WordCraft's MSI identity: reusing any of these would upgrade or remove its install.
UPSTREAM_GUIDS = {
    "075497F9-E861-45D0-9C7F-A74F8C34E6BA",  # UpgradeCode
    "855DCEC6-9285-444F-9382-9654B83B34F6",  # WordcraftApp component
    "EFF89E6A-F9B8-49F9-B3AE-E243347484B4",  # WordcraftFileTypes component
}


def wxs():
    return ET.parse(ROOT / "packaging/windows/wordcraft.wxs").getroot()


def policy():
    return json.loads((ROOT / "packaging/windows/install-policy.json").read_text(encoding="utf-8"))


class MacBundle(unittest.TestCase):
    def test_bundle_is_word_and_only_an_open_with_candidate(self):
        info = plistlib.loads((ROOT / "packaging/macos/Info.plist.in").read_bytes())
        self.assertEqual(info["CFBundleDisplayName"], "Word")
        self.assertEqual(info["CFBundleName"], "Word")
        self.assertEqual(info["CFBundleExecutable"], "Word")
        self.assertEqual(info["CFBundleIdentifier"], "io.github.jeongeee.word")
        self.assertTrue(all(t["LSHandlerRank"] == "Alternate" for t in info["CFBundleDocumentTypes"]))

    def test_package_names(self):
        script = (ROOT / "packaging/macos/package.sh").read_text(encoding="utf-8")
        self.assertIn('APP="$WORK/Word.app"', script)
        self.assertIn('DMG="$DIST/word-$VERSION-macos-$ARCH.dmg"', script)
        self.assertIn('CLI_ZIP="$DIST/word-cli-$VERSION-macos-$ARCH.zip"', script)
        self.assertNotIn("WordCraft.app", script)


class WindowsInstall(unittest.TestCase):
    def test_policy_has_its_own_identity_and_payloads(self):
        p = policy()
        self.assertEqual(p["id"], "word")
        self.assertEqual(p["name"], "Word")
        self.assertNotIn(p["upgradeCode"], UPSTREAM_GUIDS)
        self.assertEqual(sorted(p["payloads"]), ["word-cli.exe", "word.exe"])
        for row in p["registry"]:
            self.assertNotIn("WordCraft", row[1])
            self.assertNotIn("wordcraft.exe", row[1])

    def test_wxs_does_not_reuse_upstream_guids(self):
        root = wxs()
        package = root.find(f"{WIX}Package")
        self.assertEqual(package.attrib["Name"], "Word")
        self.assertEqual(package.attrib["Manufacturer"], "JeongEEE")
        self.assertEqual(package.attrib["UpgradeCode"], policy()["upgradeCode"])
        guids = {node.attrib["Guid"] for node in root.iter() if "Guid" in node.attrib}
        guids.add(package.attrib["UpgradeCode"])
        self.assertFalse(guids & UPSTREAM_GUIDS)

    def test_install_folder_registry_and_progids(self):
        root = wxs()
        dirs = {d.attrib["Id"]: d.attrib["Name"] for d in root.iter(f"{WIX}Directory")}
        self.assertEqual((dirs["CraftSuiteFolder"], dirs["INSTALLFOLDER"]), ("JeongEEE", "Word"))
        wxi = (ROOT / "packaging/windows/install-policy.wxi").read_text(encoding="utf-8")
        self.assertIn("[LocalAppDataFolder]Programs\\JeongEEE\\Word\\", wxi)
        self.assertNotIn("WordCraft", wxi)
        keys = {v.attrib["Key"] for v in root.iter(f"{WIX}RegistryValue")}
        self.assertIn("Software\\JeongEEE\\Word", keys)
        # Microsoft Word owns `Word.Document.*`: every ProgId carries the fork's prefix.
        progids = [p.attrib["Id"] for p in root.iter(f"{WIX}ProgId")]
        self.assertTrue(progids)
        self.assertTrue(all(p.startswith("JeongEEE.Word.") for p in progids), progids)

    def test_file_types_are_open_with_candidates_only(self):
        # A value without a name on `Software\Classes\.<ext>` would take over the default app.
        for v in wxs().iter(f"{WIX}RegistryValue"):
            if re.fullmatch(r"Software\\Classes\\\.[^\\]+", v.attrib["Key"]):
                self.fail(f"sets the default handler of {v.attrib['Key']}")

    def test_policy_registry_matches_the_wxs(self):
        # check-install-policy.ps1 compares the compiled MSI with the policy on Windows only.
        rows = {
            (v.attrib["Key"], v.attrib.get("Name", ""), v.attrib["Value"])
            for v in wxs().iter(f"{WIX}RegistryValue")
        }
        for row in policy()["registry"]:
            self.assertIn((row[1], row[2], row[3]), rows)


if __name__ == "__main__":
    unittest.main()
