import { Link, Route, Routes } from "react-router";
import { lazy } from "react";
import { ImportPage } from "@/features/import/import-page";
import { OverviewPage } from "@/features/library/dashboard-page";
import { LibraryPage } from "@/features/library/catalog-page";
import { MetadataPage } from "@/features/metadata/metadata-page";
import { ReaderPage } from "@/features/reader/reader-page";
import { SettingsPage } from "@/features/settings/settings-page";
import { Button } from "@/shared/ui/button";
import { PageHeading } from "@/shared/ui/page-heading";
import { Shell } from "./shell";

const BookDetailsPage = lazy(() => import("@/features/library/book-details-page").then((module) => ({ default: module.BookDetailsPage })));
const OrganizationPage = lazy(() => import("@/features/library/organization-page").then((module) => ({ default: module.OrganizationPage })));
const CollectionPage = lazy(() => import("@/features/library/collection-page").then((module) => ({ default: module.CollectionPage })));

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
        <Route path="library/:id" element={<BookDetailsPage />} />
        <Route path="organize" element={<OrganizationPage />} />
        <Route path="collections/:id" element={<CollectionPage />} />
        <Route path="import" element={<ImportPage />} />
        <Route path="metadata" element={<MetadataPage />} />
        <Route path="reader" element={<ReaderPage />} />
        <Route path="settings" element={<SettingsPage />} />
        <Route path="*" element={<NotFound />} />
      </Route>
    </Routes>
  );
}
