import { invoke } from "@tauri-apps/api/core";
import { listen, type EventCallback, type UnlistenFn } from "@tauri-apps/api/event";

export type TranslationHistoryEntry = {
  original: string;
  translation: string;
};

const isTauri =
  typeof window !== "undefined" &&
  ("__TAURI_METADATA__" in window || "__TAURI_INTERNALS__" in window || "__TAURI_IPC__" in window);

export async function recordTranslationHistory(original: string, translation: string) {
  if (!isTauri) return;

  try {
    await invoke("record_translation_history", { original, translation });
  } catch (error) {
    console.error("Failed to record translation history:", error);
  }
}

export async function fetchTranslationHistory(): Promise<TranslationHistoryEntry[]> {
  if (!isTauri) return [];

  try {
    return await invoke<TranslationHistoryEntry[]>("get_translation_history");
  } catch (error) {
    console.error("Failed to load translation history:", error);
    return [];
  }
}

export async function listenTranslationHistoryUpdates(handler: EventCallback<unknown>): Promise<UnlistenFn | null> {
  if (!isTauri) return null;

  try {
    return await listen("translation_history_updated", handler);
  } catch (error) {
    console.error("Failed to subscribe to translation history updates:", error);
    return null;
  }
}
