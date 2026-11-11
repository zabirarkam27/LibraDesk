import { desktop } from "@/shared/lib/preferences-client";
import { Button } from "@/shared/ui/button";

export function QueryError({ retry }: { retry: () => void }) {
  return (
    <div role="alert" className="rounded-lg border bg-card p-5">
      <p className="mb-3 text-sm text-destructive">This information could not be loaded. The record may have been removed.</p>
      <Button variant="outline" onClick={retry}>Reload</Button>
    </div>
  );
}
export function DesktopNotice() {
  return <p className="rounded-lg border bg-card p-5 text-sm text-muted-foreground">
    {desktop ? "Your library has no matching records." : "Open the Windows app to use your local library."}
  </p>;
}
export const fieldClass = "min-h-9 w-full min-w-0 rounded-md border bg-background px-3 py-1.5 text-sm focus-visible:outline-2 focus-visible:outline-ring disabled:opacity-50";
