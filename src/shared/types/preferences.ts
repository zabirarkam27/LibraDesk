export const themes = ["light", "dark", "system"] as const;
export type Theme = (typeof themes)[number];
export type ThemeScope = "app" | "reader";

export type Preferences = Record<ThemeScope, Theme> & {
  schemaVersion: number | null;
};

export function isTheme(value: unknown): value is Theme {
  return themes.some((theme) => theme === value);
}
