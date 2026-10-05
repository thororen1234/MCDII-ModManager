# MCDII Mod Manager

A mod manager for Minecraft Dungeons II (Blueprint Loader mods). Install, toggle, rename and delete mods without digging through the game folder.

## Mod loader

Mods need a loader to run. The app installs [BetterBlueprintLoader](https://github.com/thororen1234/MCDII-Mods/tree/main/BetterBlueprintLoader) for you, unless you already use [Blueprint Loader](https://www.nexusmods.com/minecraftdungeons2/mods/2). Only one loader can work at a time: to switch from Blueprint Loader, delete it in the app and BetterBlueprintLoader takes its place.

### Requirements

- [Rust](https://rustup.rs/) (stable)
- [Node.js](https://nodejs.org/) 22 or newer
- [pnpm](https://pnpm.io/installation): run `corepack enable` (comes with Node.js) or `npm install -g pnpm`
- The Tauri system dependencies for your OS, listed at <https://v2.tauri.app/start/prerequisites/>. On Windows that's the Microsoft C++ Build Tools ("Desktop development with C++") and WebView2, which Windows 10 and 11 already include.

### Build

```sh
git clone https://github.com/thororen1234/MCDII-ModManager.git
cd MCDII-ModManager
pnpm install
pnpm build:unsigned
```

The installers end up in `src-tauri/target/release/bundle/` (`msi/` and `nsis/` on Windows), and the app itself is `src-tauri/target/release/mcdii-modmanager.exe`.

`pnpm build:unsigned` skips signing the auto-update files.

To run the app in development mode instead, use `pnpm dev`. To run the tests, use `pnpm test`.

### Bundled files

- `betterBlueprintLoader/`: [BetterBlueprintLoader](https://github.com/thororen1234/MCDII-Mods/tree/main/BetterBlueprintLoader), built from its own repo. The app installs this copy when no mod loader is installed, and downloads newer versions from its GitHub releases.
- `noIntro/`: the blank intro videos for the No Intro option.

## Credits

- The app icon is from Minecraft Dungeons II. Minecraft Dungeons II is a trademark of Mojang Studios / Microsoft; this project is unofficial and not affiliated with or endorsed by them.
- Includes No Intro Button to install <https://www.nexusmods.com/minecraftdungeons2/mods/69> with permission from its creator: TPPr0
