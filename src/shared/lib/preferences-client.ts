import { invoke, isTauri } from "@tauri-apps/api/core";
import { isTheme, type Preferences, type Theme, type ThemeScope } from "@/shared/types/preferences";

const storageKey: Record<ThemeScope, string> = {
  app: "libradesk.appearance.v1",
  reader: "libradesk.reader-appearance.v1",
};

function readPreviewTheme(scope: ThemeScope) {
  try {
    const value = localStorage.getItem(storageKey[scope]);
    return isTheme(value) ? value : null;
  } catch {
    return null;
  }
}

export const desktop = isTauri();

export async function getPreferences(): Promise<Preferences> {
  const legacyTheme = readPreviewTheme("app");
  if (desktop) {
    const preferences = await invoke<Preferences>("get_preferences", { legacyTheme });
    if (legacyTheme) {
      try {
        localStorage.removeItem(storageKey.app);
      } catch {
        // Import is idempotent if the webview keeps the old value.
      }
    }
    return preferences;
  }
  return { app: legacyTheme ?? "system", reader: readPreviewTheme("reader") ?? "system", schemaVersion: null };
}

export async function saveTheme(scope: ThemeScope, theme: Theme) {
  if (desktop) {
    await invoke("set_theme", { scope, theme });
  } else {
    localStorage.setItem(storageKey[scope], theme);
  }
}

export function errorMessage(error: unknown) {
  if (typeof error === "object" && error !== null && "message" in error && typeof error.message === "string") {
    return error.message;
  }
  return "LibraDesk could not access its preferences. Try again.";
}

export function isRetryable(error: unknown) {
  return typeof error !== "object" || error === null || !("retryable" in error) || error.retryable !== false;
}
