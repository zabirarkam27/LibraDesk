import { useSyncExternalStore } from "react";
import type { Theme } from "@/shared/types/preferences";

const scheme = window.matchMedia("(prefers-color-scheme: dark)");

function subscribe(callback: () => void) {
  scheme.addEventListener("change", callback);
  return () => scheme.removeEventListener("change", callback);
}

export function useResolvedTheme(theme: Theme) {
  const dark = useSyncExternalStore(subscribe, () => scheme.matches);
  return theme === "system" ? (dark ? "dark" : "light") : theme;
}
