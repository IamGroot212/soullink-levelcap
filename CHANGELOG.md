# Changelog

Alle nennenswerten Änderungen werden hier dokumentiert. Format orientiert sich an
[Keep a Changelog](https://keepachangelog.com/), Versionierung folgt
[SemVer](https://semver.org/).

## [Unreleased]

### Added
- Projektgerüst, Cargo-Manifest, CI/Release-Workflows
- `caps.txt`-Parser mit Validierung
- Memory-Abstraktion (Trait + Windows/Linux-Backends, Skeleton)
- Citra-Prozess-Detection via `sysinfo`
- Daemon-Loop-Skelett mit Reconnect-Logik

### TODO (blocking für v0.1.0)
- Recherche R1: FCRAM-Base-Detection in Citra-Prozess (Linux + Windows)
- Recherche R3: Badge-Byte-Offset im 3DS-Adressraum
- Recherche R2: Party-Base-Offset
- Recherche R5: Vollständige `species_growth.json` aus PokéAPI generieren
