import { convertFileSrc, invoke } from "@tauri-apps/api/core";

/** One capture in ~/Screenshots/MyApp. Mirrors `store::Capture` in Rust. */
export interface Capture {
  /** The image's file name. Commands take this, never a path. */
  id: string;
  path: string;
  /** Milliseconds since the Unix epoch. */
  createdAt: number;
  source: "app" | "mcp" | "unknown" | string;
  kind: "region" | "screen" | "window" | "unknown" | string;
  sizeBytes: number;
  width: number | null;
  height: number | null;
}

export interface AppInfo {
  dir: string;
  hotkey: string;
  hotkeyRegistered: boolean;
}

export const api = {
  appInfo: () => invoke<AppInfo>("app_info"),
  list: () => invoke<Capture[]>("list_captures"),
  capture: () => invoke<Capture | null>("capture_region"),
  copy: (id: string) => invoke<void>("copy_capture", { id }),
  open: (id: string) => invoke<void>("open_capture", { id, reveal: false }),
  reveal: (id: string) => invoke<void>("open_capture", { id, reveal: true }),
  remove: (id: string) => invoke<void>("delete_capture", { id }),
  openFolder: () => invoke<void>("open_folder"),
};

/** A URL the webview can load the image from (Tauri's asset protocol). */
export const imageUrl = (c: Capture) => convertFileSrc(c.path);
