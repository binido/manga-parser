import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import { revealItemInDir } from "@tauri-apps/plugin-opener";

const ARCHIVE_FILTER = [{ name: "Архив с главами", extensions: ["zip"] }];

export function pickSource(mode) {
  return open({
    multiple: false,
    directory: mode === "folder",
    filters: mode === "folder" ? undefined : ARCHIVE_FILTER,
  });
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
