# Storage contract

The app and the MCP server never talk to each other. They share one folder:

```
~/Screenshots/MyApp/
  2026-11-24_223105_123.png    the capture
  2026-11-24_223105_123.json   optional sidecar with metadata
```

## Why there is no index.json

The first plan kept history in one shared `index.json`. With two processes writing
it (the app and the MCP server), one can read the file, the other can write, and the
first then overwrites the second's entry. Locking across a Rust app and a Node
process is fiddly. Instead, **history is built by scanning the folder**, and each
writer only ever creates its own files. Nothing is ever rewritten in place, so
nothing can be lost.

## Rules for every writer

1. **Images**: `.png` (also `.jpg`/`.jpeg`). Name new files `YYYY-MM-DD_HHMMSS_mmm.png`
   in local time; if that exists, add `-2`, `-3`, and so on.
2. **Write atomically**: never let a half-written image appear under its final name.
   - `screencapture` refuses hidden (dot-prefixed) output names, so capture into a temp folder
     (the app uses `$TMPDIR/screenshot-app/<name>.png`) and then rename it into the folder.
   - If a rename isn't possible (another volume), copy to `.partial-<name>` in the folder and rename.
   Readers skip every file whose name starts with `.`.
3. **Sidecar** (optional): `<same stem>.json`, also written to `.partial-…` and renamed:
   ```json
   { "version": 1, "createdAt": 1764023465123, "source": "mcp", "kind": "window" }
   ```
   - `createdAt`: milliseconds since the Unix epoch (UTC).
   - `source`: `"app"` or `"mcp"`. The history marks `"mcp"` captures with a Claude badge.
   - `kind`: `"region"`, `"screen"` or `"window"`.
   - Every field is optional and unknown fields are ignored, so it can grow
     (for example `display`, `windowOwner`, `windowTitle`).
4. **No sidecar is fine**: the image still shows, with its file time as `createdAt`.
5. **Delete** the image and its sidecar together. The app moves both to the Trash.

## Reading

List the folder, keep non-hidden image files, read each sidecar if present,
sort by `createdAt` newest first. `list_recent` in the MCP server should do the same.
The Rust version is `store::list` in `src-tauri/src/store.rs`.
