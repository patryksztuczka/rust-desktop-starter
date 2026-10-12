# Rust Desktop Starter

A native macOS app template using the same stack as
[HEX](https://github.com/anomalyco/hex): Rust, [GPUI](https://www.gpui.rs/) for
the UI, `objc2` for macOS APIs, Sparkle for updates, and shell scripts for
signing, notarization, and DMG releases. There is no webview or JavaScript runtime.

What you get out of the box:

- A window with a sidebar and three panes (Home, Settings, About), drawn with
  shared design tokens in `src/desktop_ui.rs`.
- App menu, keyboard shortcuts, and Dock reopen behaviour.
- Settings saved as JSON in `~/Library/Application Support/<bundle id>/`.
- An example of running slow work on a background thread and updating the UI.
- Launch at login through `SMAppService`.
- Optional Sparkle auto-updates.
- A CLI on the same binary (`starter paths`, `starter settings`).
- Scripts to build a signed `.app`, install it, and publish notarized releases.

## Requirements

- macOS 15 or newer on Apple silicon.
- Rust (stable): `curl https://sh.rustup.rs -sSf | sh`
- **Xcode** (full app, not only Command Line Tools) to compile GPUI's Metal
  shaders. Without it, add `--features runtime-shaders` to every cargo command;
  shaders are then compiled when the app starts.

## Run it

```sh
cargo run                    # opens the app (same as `cargo run -- app`)
cargo run -- app --pane settings   # open straight to a pane
cargo run -- paths           # where data, settings, and logs live
cargo run -- settings        # print saved settings
RUST_LOG=starter=debug cargo run

./scripts/check.sh           # fmt, clippy -D warnings, tests
```

Without Xcode: `cargo run --features runtime-shaders` and
`./scripts/check.sh --features runtime-shaders`.

Logs go to stderr and to `~/Library/Application Support/<bundle id>/logs/process.log`.
Set `STARTER_SUPPORT_DIR=/tmp/somewhere` to run against a throwaway data folder.

## Make it yours

1. Change `APP_NAME`, `BUNDLE_ID`, and `EXECUTABLE` in `scripts/app-config.sh`.
2. Change `APP_NAME` and `BUNDLE_ID` in `src/main.rs`, and the `[[bin]] name` and
   package `name` in `Cargo.toml`. `cargo test` fails if these disagree with step 1.
3. Rename the `STARTER_SUPPORT_DIR` variable in `src/app_paths.rs` and the
   `starter=info` log filter if you like.
4. Add `app/AppIcon.icns` for a custom icon.
5. Add usage descriptions to `app/Info.plist` and entitlements to
   `app/App.entitlements` for any protected resources (microphone, camera,
   automation, …).

## Layout

| Path | Purpose |
|---|---|
| `src/main.rs` | CLI entry point and app identity constants |
| `src/app_window.rs` | GPUI application, menus, actions, main window and panes |
| `src/desktop_ui.rs` | Colours, sizes, and shared controls (button, toggle, panel, rows) |
| `src/settings.rs` | Persisted settings with atomic saves and corrupt-file backup |
| `src/app_paths.rs` | Data directories and logging |
| `src/login_item.rs` | Launch at login (`SMAppService`) |
| `src/sparkle.rs` | Sparkle updater, loaded at runtime from the bundle |
| `app/` | `Info.plist` template and entitlements |
| `scripts/` | Build, install, validate, and release scripts |
| `docs/releases/` | Release notes, one Markdown file per version |

## Build the app bundle

Signing has three levels:

| Level | Needs | Good for |
|---|---|---|
| None (`cargo run`) | Nothing | Day-to-day development |
| Ad-hoc | Nothing | Trying the real `.app` on your own Mac |
| Developer ID + notarization | Paid Apple Developer Program | Distributing to other people |

```sh
# Local-only bundle, no Apple account:
APP_ADHOC_SIGN=1 ./scripts/build-app.sh       # → target/app/Starter.app
APP_ADHOC_SIGN=1 ./scripts/install-app.sh     # build, copy to /Applications, open

# Developer ID signed:
APP_TEAM_ID=ABCDE12345 ./scripts/install-app.sh
```

Add `APP_CARGO_FEATURES=runtime-shaders` to either if Xcode is not installed.
Ad-hoc builds skip the Hardened Runtime: its library validation would refuse to
load the bundled Sparkle framework, which has no Team ID to match.

Some features only work from a properly signed app in `/Applications`:
launch at login, and stable permission grants (macOS ties microphone,
accessibility, and similar permissions to the signature, so rebuilding with a
different identity asks again).

## Updates (optional)

Updates use [Sparkle](https://sparkle-project.org/). They stay off until you
configure them, and the bundle is built without Sparkle until then.

1. Get Sparkle's tools: `./scripts/setup-sparkle.sh` prints its folder.
2. Create a signing key once: `<that folder>/bin/generate_keys`. It stores the
   private key in your login keychain and prints the public key. Back up the
   private key (`generate_keys -x private-key-file`); losing it means installed
   apps can no longer accept your updates.
3. In `scripts/app-config.sh`, set `DOWNLOAD_BASE_URL` (where `appcast.xml` and
   `releases/` will be served) and `SPARKLE_PUBLIC_KEY`.

## Release

The release flow is two steps: prepare, test, then publish.

One-time setup:

- A **Developer ID Application** certificate in your keychain (Xcode →
  Settings → Accounts → Manage Certificates, or developer.apple.com).
- A notarization profile:
  `xcrun notarytool store-credentials my-notary --apple-id you@example.com --team-id ABCDE12345`
  (use an app-specific password from appleid.apple.com).
- Sparkle configured as above, and somewhere to host files. The publish step
  uploads to a Cloudflare R2 bucket with `wrangler`; set `RELEASE_BUCKET` and
  point `DOWNLOAD_BASE_URL` at the bucket's public domain. Swap the `put`
  function in `scripts/release-app.sh` for another host.

Each release:

```sh
# 1. Bump `version` in Cargo.toml and add docs/releases/<version>.md. Commit and push.
export APP_TEAM_ID=ABCDE12345 APP_NOTARY_PROFILE=my-notary

# 2. Build, sign, notarize, staple, make the DMG and the signed appcast in dist/.
./scripts/release-app.sh prepare

# 3. Open dist/Starter-<version>-arm64.dmg and test it. Then:
./scripts/release-app.sh publish
```

`prepare` refuses to run with uncommitted or unpushed changes, or when the build
number is not newer than the published one. `publish` uploads the DMG and ZIP,
checks that the public URLs serve identical bytes, and only then replaces the
appcast, so installed apps never see a half-published release.

## Porting from Electron

| Electron | Here |
|---|---|
| `BrowserWindow` + HTML/CSS | `cx.open_window` + a GPUI `Render` view |
| `ipcMain` / `ipcRenderer` | Direct method calls on the view; `cx.spawn` + `background_executor()` for slow work |
| `Menu.setApplicationMenu` | `cx.set_menus` with `actions!` |
| `globalShortcut` (in-app) | `KeyBinding::new` + `cx.bind_keys` |
| `app.setLoginItemSettings` | `src/login_item.rs` |
| `autoUpdater` / electron-updater | `src/sparkle.rs` + `scripts/release-app.sh` |
| `electron-store` | `src/settings.rs` |
| `shell.showItemInFolder` | `cx.reveal_path` |
| `shell.openExternal` | `cx.open_url` |
| `dialog.showMessageBox` | `window.prompt` |
| Native Node modules | Rust crates, or `objc2-*` bindings for Apple frameworks |

GPUI has no browser engine, so screens that render HTML, Markdown, or
embedded web pages need a native rewrite. GPUI's documentation is thin; when
unsure about an API, read how [HEX](https://github.com/anomalyco/hex) or
[Zed](https://github.com/zed-industries/zed) use it.
