# PD2 x64 Converter

Converts legacy PAYDAY 2 mod assets to the x64 Diesel 3.0 format.

Supported:

- `.font`
- `.animation`
- `.massunit`
- `.stream`
- `.bnk`
- ScriptData: `.sequence_manager`, `.continent`, `.environment`,
  `.world`, `.mission`, `.continents`, `.cover_data`, `.nav_data`,
  `.world_cameras`, `.world_sounds`

## CLI

```bash
pd2-x64-converter-cli scan <root> [--jobs N] [--json] [--report]
pd2-x64-converter-cli convert <root> [--jobs N] [--json] [--report] [--dry-run]
```

## Building

Requirements:

- Rust 1.88 or newer
- Node.js 24 and pnpm 11
- The [Tauri system dependencies](https://v2.tauri.app/start/prerequisites/) for GUI builds

From the repository root, install the frontend dependencies:

```bash
pnpm install --frozen-lockfile
```

Build the CLI:

```bash
pnpm run build:cli:linux   # Linux
pnpm run build:cli:windows # Windows
```

Build the desktop application:

```bash
pnpm run build:gui:linux            # Linux executable
pnpm run build:gui:linux:appimage   # Linux AppImage
pnpm run build:gui:windows          # Windows executable
pnpm run build:gui:windows:installer # Windows installer
```

## Licensing

Third-party attributions are listed in [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
