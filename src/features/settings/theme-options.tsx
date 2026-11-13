import { Monitor,Moon,Sun } from "lucide-react";
import { toast } from "sonner";
import { errorMessage } from "@/shared/lib/preferences-client";
import { themes,type ThemeScope } from "@/shared/types/preferences";
import { usePreferences } from "./preferences-store";
const themeOptions = {
  light: { label: "Light", icon: Sun },
  dark: { label: "Dark", icon: Moon },
  system: { label: "System", icon: Monitor },
};

export function ThemeOptions({ scope, label, description }: { scope: ThemeScope; label: string; description: string }) {
  const selected = usePreferences((state) => state.preferences[scope]);
  const saving = usePreferences((state) => state.saving);
  const change = usePreferences((state) => state.changeTheme);

  return (
    <fieldset aria-busy={saving}>
      <legend className="text-sm font-semibold">{label}</legend>
      <p className="mt-1 text-sm text-muted-foreground">{description}</p>
      <div className="mt-3 grid grid-cols-3 gap-2">
        {themes.map((theme) => {
          const { icon: Icon, label } = themeOptions[theme];
          return (
            <label key={theme} className="flex cursor-pointer flex-wrap items-center gap-2 rounded-md border px-3 py-2.5 text-sm has-checked:border-ring has-checked:bg-accent has-checked:text-accent-foreground has-[:focus-visible]:outline-2 has-[:focus-visible]:outline-ring">
              <input
                className="sr-only"
                type="radio"
                name={scope + "-theme"}
                checked={selected === theme}
                aria-disabled={saving}
                onChange={async () => {
                  if (saving) return;
                  try {
                    await change(scope, theme);
                    toast.success(scope === "app" ? "App appearance saved" : "Reader appearance saved");
                  } catch (error) {
                    toast.error(errorMessage(error));
                  }
                }}
              />
              <Icon className="size-4 shrink-0" />
              {label}
            </label>
          );
        })}
      </div>
    </fieldset>
  );
}

