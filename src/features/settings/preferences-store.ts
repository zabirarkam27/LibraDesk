import { create } from "zustand";
import { errorMessage, getPreferences, isRetryable, saveTheme } from "@/shared/lib/preferences-client";
import type { Preferences, Theme, ThemeScope } from "@/shared/types/preferences";

type PreferenceStore = {
  preferences: Preferences;
  status: "idle" | "loading" | "ready" | "error";
  error: { message: string; retryable: boolean } | null;
  saving: boolean;
  load: () => Promise<void>;
  changeTheme: (scope: ThemeScope, theme: Theme) => Promise<void>;
};

export const usePreferences = create<PreferenceStore>((set, get) => ({
  preferences: { app: "system", reader: "system", schemaVersion: null },
  status: "idle",
  error: null,
  saving: false,
  async load() {
    const { status } = get();
    if (status === "loading" || status === "ready") return;
    set({ status: "loading", error: null });
    try {
      set({ preferences: await getPreferences(), status: "ready" });
    } catch (error) {
      set({ status: "error", error: { message: errorMessage(error), retryable: isRetryable(error) } });
    }
  },
  async changeTheme(scope, theme) {
    const { preferences, saving, status } = get();
    if (status !== "ready" || saving || preferences[scope] === theme) return;
    set({ preferences: { ...preferences, [scope]: theme }, saving: true });
    try {
      await saveTheme(scope, theme);
    } catch (error) {
      set({ preferences });
      throw error;
    } finally {
      set({ saving: false });
    }
  },
}));
