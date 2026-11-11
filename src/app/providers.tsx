import { useEffect, useLayoutEffect, type ReactNode } from "react";
import { HashRouter } from "react-router";
import { Toaster } from "sonner";
import { usePreferences } from "@/features/settings/preferences-store";
import { startCoverWorker } from "@/features/metadata/cover-worker";
import { useResolvedTheme } from "@/shared/hooks/use-resolved-theme";
import { TooltipProvider } from "@/shared/ui/tooltip";
import { desktop } from "@/shared/lib/preferences-client";
import { guardReaderWindowClose } from "@/features/reader/window-close";

export function AppProviders({ children }: { children: ReactNode }) {
  const load = usePreferences((state) => state.load);
  const theme = useResolvedTheme(usePreferences((state) => state.preferences.app));

  useEffect(() => {
    void load();
    startCoverWorker();
    if (!desktop) return;
    const listening = guardReaderWindowClose().catch(() => null);
    return () => { void listening.then((remove) => remove?.()); };
  }, [load]);

  useLayoutEffect(() => {
    document.documentElement.dataset.theme = theme;
  }, [theme]);

  return (
    <HashRouter>
      <TooltipProvider delayDuration={300}>
        {children}
        <Toaster theme={theme} position="bottom-right" closeButton />
      </TooltipProvider>
    </HashRouter>
  );
}
