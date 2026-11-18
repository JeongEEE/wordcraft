# Word

Word is a personal Windows/macOS word processor maintained in
[JeongEEE/wordcraft](https://github.com/JeongEEE/wordcraft), based on
[WordCraft by the ArtCraft team](https://github.com/storytold/wordcraft).

Rust workspace package names, engine commands and `WORDCRAFT_*` environment variables remain
compatible with upstream. The installed app, preferences and update feed are independent of
WordCraft.

- `main`: unmodified upstream history; synchronize from `upstream/main`.
- `my-custom`: development and personal changes.
- [Build, release and update guide](docs/custom-release.md).
- [Upstream architecture](docs/architecture.md) and [known limitations](docs/gaps.md).

## Build locally

Install Rust 1.95 or later and the platform's native build tools, then run:

```sh
cargo build --release --locked -p wordcraft -p wordcraft-cli
cargo run --release -p wordcraft -- --sample
cargo xtask ci
```

macOS packages: `packaging/macos/package.sh --arch universal`.
Windows packages: `pwsh packaging/windows/package.ps1 -Arch x64` (WiX 5.0.2).

## Downloads and updates

[Published releases](https://github.com/JeongEEE/wordcraft/releases) contain the installers.
Word checks only this fork's published releases; it never installs upstream WordCraft releases.

## License and attribution

Code is available under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your choice.
Copyright (c) 2026 ArtCraft Team and the WordCraft contributors. See [NOTICE](NOTICE)
and [ATTRIBUTION.md](ATTRIBUTION.md) for third-party notices and the original app icon.
ArtCraft logos and wordmarks have been removed from this modified distribution, as the
[brand terms](docs/brand/LICENSE-brand.txt) require of forks.
Word is an independent personal fork and is not an official ArtCraft product.
