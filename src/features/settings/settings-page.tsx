import { Database, Monitor, Moon, Sun } from "lucide-react";
import { toast } from "sonner";
import { desktop, errorMessage } from "@/shared/lib/preferences-client";
import { themes, type ThemeScope } from "@/shared/types/preferences";
import { Button } from "@/shared/ui/button";
import { PageHeading } from "@/shared/ui/page-heading";
import { Skeleton } from "@/shared/ui/skeleton";
import { usePreferences } from "./preferences-store";

const themeOptions = {
  light: { label: "Light", icon: Sun },
  dark: { label: "Dark", icon: Moon },
  system: { label: "System", icon: Monitor },
};

function ThemeOptions({ scope, label, description }: { scope: ThemeScope; label: string; description: string }) {
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

export function SettingsPage() {
  const { status, error, preferences, load } = usePreferences();

  return (
    <>
      <PageHeading label="Settings" title="Preferences" description="Choose how the workspace and the reader look on this device." />
      {status === "idle" || status === "loading" ? (
        <div role="status" className="space-y-4">
          <span className="sr-only">Loading preferences</span>
          <Skeleton className="h-44 w-full motion-reduce:animate-none" />
          <Skeleton className="h-28 w-full motion-reduce:animate-none" />
        </div>
      ) : status === "error" ? (
        <section role="alert" className="rounded-xl border border-destructive/40 bg-card p-5">
          <h2 className="text-base font-semibold">Storage unavailable</h2>
          <p className="mt-2 text-sm text-muted-foreground">{error?.message}</p>
          {error?.retryable && (
            <Button variant="outline" className="mt-4" onClick={() => void load()}>Try again</Button>
          )}
        </section>
      ) : (
        <div className="grid gap-4">
          <section className="space-y-5 rounded-xl border bg-card p-5 sm:p-6" aria-labelledby="appearance-heading">
            <h2 id="appearance-heading" className="font-display text-2xl">Appearance</h2>
            <ThemeOptions scope="app" label="App theme" description="Applies to the workspace and its controls." />
            <div className="border-t pt-5">
              <ThemeOptions scope="reader" label="Reader theme" description="Applies inside the reader, independently of the app theme." />
            </div>
          </section>
          <section className="flex items-start gap-4 rounded-xl border bg-card p-5 sm:p-6" aria-labelledby="storage-heading">
            <span className="grid size-10 shrink-0 place-items-center rounded-lg bg-accent text-accent-foreground"><Database className="size-5" /></span>
            <div>
              <h2 id="storage-heading" className="font-semibold">Local storage</h2>
              {desktop ? (
                <>
                  <p className="mt-1 text-sm text-muted-foreground">SQLite database connected · Schema {preferences.schemaVersion}</p>
                  <p className="mt-2 font-mono text-xs text-muted-foreground">%LOCALAPPDATA%\LibraDesk</p>
                </>
              ) : (
                <p className="mt-1 text-sm text-muted-foreground">Browser preview saves appearance in this browser. Open the Windows app to use local SQLite storage.</p>
              )}
            </div>
          </section>
        </div>
      )}
    </>
  );
}
