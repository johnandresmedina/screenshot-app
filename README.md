# Screenshot

A macOS menu bar screenshot app (Tauri v2, Rust + React/TypeScript). Every
capture lands in `~/Screenshots/MyApp`, which an MCP server will also read and
write later so Claude can take and read screenshots too.

What works now (build steps 1 to 4 of the plan):

- **Menu bar icon** with Capture Region, Show History, Open Screenshots Folder and Quit. No Dock icon.
- **Global hotkey ⌘⇧2** for a region capture (macOS keeps ⌘⇧3/4/5 for itself).
  Capturing uses `screencapture -i`: drag to select, press Space to pick a window, Esc to cancel.
- **History window**: captures grouped by day, newest first, with Copy (as an image), Open (Preview),
  Finder (reveal) and Delete (moves to the Trash, so it can be undone). Closing the window just hides it.

## Set up your Mac

You need macOS 11 or later. Do these once, in Terminal. Skip any step whose check already works.

### 1. Apple's command line tools

Rust needs Apple's compilers and linker.

```sh
xcode-select --install
```

If it says they're already installed, move on. Check: `xcode-select -p` prints a path.

### 2. Rust

Install with the official installer from [rustup.rs](https://rustup.rs) and press Enter to accept
the default install:

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Then open a new Terminal window (or run `source "$HOME/.cargo/env"`) so `cargo` is on your PATH.
Check: `rustc --version` and `cargo --version` both print a version. Update later with `rustup update`.

### 3. Node.js

The Node version is pinned in `.nvmrc` (26.13.1). With [nvm](https://github.com/nvm-sh/nvm):

```sh
curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/master/install.sh | bash   # skip if you have nvm
# open a new Terminal window, then, inside the project folder:
nvm install      # installs the version in .nvmrc
nvm use          # switches to it
```

Check: `node --version` prints `v26.13.1`.

### 4. pnpm

The project uses pnpm, pinned in `package.json` (`"packageManager": "pnpm@12.23.0"`). Pick one way
to get it:

- **Corepack** (recommended): it reads that `packageManager` line and runs exactly that pnpm
  version, so everyone uses the same one. Node 25 and later no longer include Corepack, so install
  it first:
  ```sh
  npm install -g corepack
  corepack enable
  ```
- **Or install pnpm directly**: `brew install pnpm` or `npm install -g pnpm`. This works too; pnpm
  warns if its version doesn't match the pinned one.

Check: `pnpm --version` prints a version.

## Run it

1. Get the code and start it in development mode:
   ```sh
   git clone https://github.com/johnandresmedina/screenshot-app.git
   cd screenshot-app
   pnpm install
   pnpm tauri dev
   ```
   `pnpm tauri dev` starts the Vite dev server (`pnpm dev`) by itself, so don't run `pnpm dev` too:
   both want port 1420.
   The first Rust build takes a few minutes. After that, look for the viewfinder icon in the menu bar.
   The history window opens at launch. Closing it only hides it: bring it back from the menu bar icon (**Show History…**) or by opening the app again.
2. Press **⌘⇧2** and drag over part of the screen. Press Space in the overlay to pick a window
   instead, or Esc to cancel.
   - The first time, macOS asks for **Screen Recording** permission. In development the permission
     belongs to your terminal app (Terminal, iTerm or VS Code), not to Screenshot. Turn it on in
     System Settings → Privacy & Security → Screen & System Audio Recording, then quit and reopen the
     terminal and run `pnpm tauri dev` again.
   - Without the permission, captures contain only your wallpaper.
3. To install it as a normal app:
   ```sh
   pnpm tauri build
   open src-tauri/target/release/bundle/macos
   ```
   Drag **Screenshot.app** to Applications and open it. It asks for Screen Recording permission again,
   this time for itself. To start it at login, add it in System Settings → General → Login Items.

## Tests

```sh
pnpm test         # format helpers (node:test) and the folder scanner (cargo test)
pnpm build        # type-checks the frontend
```

## Layout

| Path | What it is |
|---|---|
| `src-tauri/src/lib.rs` | Tray menu, hotkey, window handling, and the commands the UI calls |
| `src-tauri/src/capture.rs` | `screencapture`, clipboard, open/reveal, move to Trash |
| `src-tauri/src/store.rs` | Scanning `~/Screenshots/MyApp`; the storage rules |
| `src/App.tsx`, `src/components/` | The history window (React) |
| `docs/STORAGE.md` | The folder contract the MCP server must follow |
