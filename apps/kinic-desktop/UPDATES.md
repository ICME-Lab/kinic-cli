# Kinic Memory Desktop Updates

## Development Reinstall

For local development, the fastest update path is rebuilding and replacing the installed app:

```sh
pnpm --filter @kinic/kinic-desktop tauri build --debug
open target/debug/bundle/dmg/Kinic\ Memory_0.1.0_aarch64.dmg
```

Drag the rebuilt app over the installed `Kinic Memory.app`.

## Manual Updater Check

The Settings screen has a manual update check. It uses the Tauri updater plugin and checks:

```txt
https://github.com/ICME-Lab/kinic-cli/releases/latest/download/kinic-memory-latest.json
```

Tauri updater installs only signed updater artifacts. To create local debug updater artifacts:

```sh
export TAURI_SIGNING_PRIVATE_KEY="$(cat apps/kinic-desktop/.local/feat/desktop-tauri-spec/tauri-updater-dev.key)"
export TAURI_SIGNING_PRIVATE_KEY_PASSWORD=""
pnpm build:desktop:update
```

The update build uses `--bundles app` because the updater needs the app archive, not the DMG. macOS output includes:

```txt
target/debug/bundle/macos/Kinic Memory.app.tar.gz
target/debug/bundle/macos/Kinic Memory.app.tar.gz.sig
```

Publish a `kinic-memory-latest.json` release asset that points to the `.tar.gz` URL and contains the `.sig` file content. The public key is in `src-tauri/tauri.conf.json`; the private key must stay outside git.
