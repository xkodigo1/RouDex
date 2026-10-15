# Installation

RouDex is local-first: no accounts, no tokens of its own and no remote
services. It drives Codex, so Codex must be installed and signed in.

## From a release

Each release has installers for Windows, macOS and Linux. They are **not
code-signed**, so each system asks before running them the first time.

### Windows

1. Download `RouDex_<version>_x64-setup.exe` and run it. It installs for your
   user only, in `%LOCALAPPDATA%\RouDex`, without administrator rights.
2. SmartScreen may say "Windows protected your PC": choose **More info** and
   then **Run anyway**.

### macOS (Apple Silicon and Intel)

1. Download `RouDex_<version>_universal.dmg`, open it and drag RouDex to
   Applications.
2. The app is ad-hoc signed but not notarized. The first time, macOS refuses
   to open it: go to **System Settings › Privacy & Security** and choose
   **Open Anyway**.

### Linux

- Debian and Ubuntu: `sudo apt install ./RouDex_<version>_amd64.deb`.
- Any distribution: make `RouDex_<version>_amd64.AppImage` executable
  (`chmod +x`) and run it.

The app needs WebKitGTK 4.1, which the `.deb` declares and current desktop
distributions ship.

## Codex

RouDex looks for the Codex executable in this order:

1. `ROUDEX_CODEX_BIN`, for development.
2. `codex_path` in `router.toml`.
3. `codex` on the `PATH`.
4. The places each system installs it in:
   - **Windows:** the newest copy of the Codex desktop app.
   - **macOS:** the Codex app in `/Applications` or `~/Applications`, then
     Homebrew, `/usr/local/bin` and `~/.local/bin`.
   - **Linux:** `~/.local/bin`, `/usr/local/bin` and `~/.npm-global/bin`.

If none of them has it, the app says where it looked. Set the path yourself
in `router.toml`:

```toml
codex_path = "/path/to/codex"
```

## Where RouDex keeps its data

Everything lives in `~/.roudex` (`%USERPROFILE%\.roudex` on Windows), or in
`ROUDEX_HOME` if you set it:

| File | What it is |
| --- | --- |
| `router.toml` | Tiers, risk options, prices, language, `codex_path`. |
| `router.sqlite3` | Threads, turns, specs, task runs and their evidence. |
| `desktop-hook.sqlite3` | Decisions of the Codex Desktop hook (never the prompt text). |
| `hook.log` | Hook failures, with field names only. |
| `bin/router-cli` | The copy of `router-cli` the Codex hook runs. |

**From 0.8.0 or earlier on Windows:** the data lived in
`%LOCALAPPDATA%\RouDex`. The first start of 0.9.0 copies `router.sqlite3`
and `router.toml` to `~/.roudex`, says so once, and leaves the originals as a
backup. Nothing that already exists in `~/.roudex` is overwritten.

## Language

The app starts in your system's language (Spanish or English) and saves it in
`router.toml`. Change it with the **ES / EN** switch in the top bar. The
Codex Desktop hook writes its warning in the same language, and in English
when `router.toml` has none.

## Codex Desktop hook

In **Summary**, the Codex Desktop card has an **Install hook** button. It
copies the bundled `router-cli` to `~/.roudex/bin` and adds one entry to
Codex's `hooks.json`. Codex then asks you to review and trust the hook. See
[desktop-hook.md](desktop-hook.md).

Running from source, use `router-cli hook-install` instead. Uninstalling the
app does not remove the hook: run `~/.roudex/bin/router-cli hook-uninstall`
first if you want it gone.

## Savings estimate

Summary estimates what routing saved once you add model prices to
`router.toml`. See [savings.md](savings.md).

## Uninstall

- **Windows:** Settings › Apps, or `uninstall.exe` in the install folder. It
  removes the program and keeps `~/.roudex`. Windows keeps the install
  location under `HKCU\Software\roudex` unless you tick "delete app data".
- **macOS:** move RouDex from Applications to the Trash.
- **Linux:** `sudo apt remove roudex-desktop`, or delete the AppImage.

Your data stays in `~/.roudex` until you delete it.

## From source

You need Rust stable (with the MSVC toolchain on Windows), Node.js 24 with
`pnpm`, and on Linux the WebKitGTK 4.1 development packages
(`libwebkit2gtk-4.1-dev`, `libgtk-3-dev`, `libayatana-appindicator3-dev`,
`librsvg2-dev`).

```sh
pnpm --dir ui install
cargo test --workspace
pnpm --dir ui dev            # keep running
cargo run -p roudex-desktop  # opens the window
```

Build the installers for your system with `node scripts/bundle.mjs` (add
`--target universal-apple-darwin` on a Mac). The release workflow does the
same for every system.

## Hooks from earlier versions

Versions `v0.3.x`–`v0.5.0` installed a `UserPromptSubmit` hook that blocked
Codex Desktop messages. `router-cli hook-install` removes it from the user
`hooks.json` when it finds it. To remove it by hand:

- **User hook:** delete the `route-prompt` entry from `~/.codex/hooks.json`.
- **Managed hook (Windows):** from an administrator PowerShell, rename
  `%ProgramData%\OpenAI\Codex\requirements.toml` if it holds only the
  `# RouDex managed desktop routing hook` block. If it has other policies,
  remove only that block.

Restart Codex Desktop afterwards.
