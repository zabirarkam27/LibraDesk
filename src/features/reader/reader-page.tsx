import { BookOpen } from "lucide-react";
import { Link } from "react-router";
import { usePreferences } from "@/features/settings/preferences-store";
import { useResolvedTheme } from "@/shared/hooks/use-resolved-theme";
import { Button } from "@/shared/ui/button";
import { PageHeading } from "@/shared/ui/page-heading";

export function ReaderPage() {
  const theme = useResolvedTheme(usePreferences((state) => state.preferences.reader));

  return (
    <>
      <PageHeading label="Reader" title="Reading workspace" description="Preview the reader's appearance in its own workspace." />
      <section data-theme={theme} aria-label="Reader appearance preview" className="min-h-72 rounded-xl border bg-secondary p-5 text-foreground sm:p-8">
        <div className="mx-auto max-w-md rounded-lg border bg-card px-6 py-8 text-center shadow-sm">
          <BookOpen className="mx-auto mb-4 size-6 text-muted-foreground" />
          <p className="eyebrow">Reader preview</p>
          <h2 className="mt-3 font-display text-2xl">No document open</h2>
          <p className="mt-2 text-sm leading-6 text-muted-foreground">This preview uses your reader theme. PDF viewing will be added in the reader phase.</p>
          <Button variant="outline" asChild className="mt-4"><Link to="/settings">Reader preferences</Link></Button>
        </div>
      </section>
    </>
  );
}
