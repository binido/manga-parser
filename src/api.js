import { getVersion } from "@tauri-apps/api/app";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import { openUrl, revealItemInDir } from "@tauri-apps/plugin-opener";

const ARCHIVE_FILTER = [{ name: "Архив с главами", extensions: ["zip"] }];
const IMAGE_FILTER = [
  { name: "Изображение", extensions: ["jpg", "jpeg", "png", "gif", "webp"] },
];

export function pickSource(mode) {
  return open({
    multiple: false,
    directory: mode === "folder",
    filters: mode === "folder" ? undefined : ARCHIVE_FILTER,
  });
}

export function pickCover() {
  return open({ multiple: false, filters: IMAGE_FILTER });
}

export async function readCover(path) {
  const bytes = await invoke("cover_preview", { path });
  return new Blob([new Uint8Array(bytes)]);
}

export function pickDirectory() {
  return open({ multiple: false, directory: true });
}

export function prepare(job) {
  return invoke("prepare", { job });
}

export function cancel() {
  return invoke("cancel");
}

export function onPipelineEvent(handler) {
  return listen("pipeline://event", (message) => handler(message.payload));
}

export function reveal(path) {
  return revealItemInDir(path);
}

export function openExternal(url) {
  return openUrl(url);
}

export { getVersion };
