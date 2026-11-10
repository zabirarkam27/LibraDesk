import { Link, Route, Routes } from "react-router";
import { ImportPage } from "@/features/import/import-page";
import { OverviewPage } from "@/features/library/library-pages";
import { LibraryPage } from "@/features/library/catalog-page";
import { ReaderPage } from "@/features/reader/reader-page";
import { SettingsPage } from "@/features/settings/settings-page";
import { Button } from "@/shared/ui/button";
import { PageHeading } from "@/shared/ui/page-heading";
import { Shell } from "./shell";

function NotFound() {
  return (
    <>
      <PageHeading label="Workspace" title="Page not found" description="This address does not match a LibraDesk page." />
      <Button asChild><Link to="/">Back to overview</Link></Button>
    </>
  );
}

export function App() {
  return (
    <Routes>
      <Route element={<Shell />}>
        <Route index element={<OverviewPage />} />
        <Route path="library" element={<LibraryPage />} />
        <Route path="import" element={<ImportPage />} />
        <Route path="reader" element={<ReaderPage />} />
        <Route path="settings" element={<SettingsPage />} />
        <Route path="*" element={<NotFound />} />
      </Route>
    </Routes>
  );
}
