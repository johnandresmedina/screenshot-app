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

## Run it on your Mac

You need macOS 11 or later.

1. Install the tools (skip any you already have):
   ```sh
   xcode-select --install                                         # Apple's compilers
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh  # Rust, then open a new terminal
   ```
   Node is pinned in `.nvmrc` (26.13.1) and pnpm in `package.json`. With
   [nvm](https://github.com/nvm-sh/nvm):
   ```sh
   nvm install                                # reads .nvmrc
   npm install -g corepack && corepack enable  # Node 25+ no longer ships Corepack; it provides the pinned pnpm
   ```
2. Get the code and start it in development mode:
   ```sh
   git clone https://github.com/johnandresmedina/screenshot-app.git
   cd screenshot-app
   pnpm install
   pnpm tauri dev
   ```
   The first Rust build takes a few minutes. After that, look for the viewfinder icon in the menu bar.
   There is no window at first: click the icon, then **Show History…**
3. Press **⌘⇧2** and drag over part of the screen.
   - The first time, macOS asks for **Screen Recording** permission. In development the permission
     belongs to your terminal app (Terminal, iTerm or VS Code), not to Screenshot. Turn it on in
     System Settings → Privacy & Security → Screen & System Audio Recording, then quit and reopen the
     terminal and run `pnpm tauri dev` again.
   - Without the permission, captures contain only your wallpaper.
4. To install it as a normal app:
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
