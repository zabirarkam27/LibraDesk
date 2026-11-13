import { BookOpen, Ellipsis, FileCheck2, FolderInput, FolderTree, LayoutGrid, LibraryBig, Search, Settings2 } from "lucide-react";
import { Link, NavLink, Outlet, useLocation } from "react-router";
import { usePreferences } from "@/features/settings/preferences-store";
import { desktop } from "@/shared/lib/preferences-client";
import { cn } from "@/shared/lib/utils";
import { Button } from "@/shared/ui/button";
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuLabel, DropdownMenuSeparator, DropdownMenuTrigger } from "@/shared/ui/dropdown-menu";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/shared/ui/tooltip";
import { SearchEntry } from "./search-entry";
import { Suspense, useLayoutEffect, useRef } from "react";
import { Skeleton } from "@/shared/ui/skeleton";

const navigation = [
  { to: "/", name: "Overview", icon: LayoutGrid },
  { to: "/library", name: "Library", icon: LibraryBig },
  { to: "/search", name: "PDF contents", icon: Search },
  { to: "/organize", name: "Organize", icon: FolderTree },
  { to: "/import", name: "Import", icon: FolderInput },
  { to: "/metadata", name: "Metadata", icon: FileCheck2 },
  { to: "/reader", name: "Reader", icon: BookOpen },
  { to: "/settings", name: "Settings", icon: Settings2 },
];

function Sidebar() {
  const status = usePreferences((state) => state.status);
  const storageStatus = !desktop ? "Browser preview" : {
    idle: "Opening workspace",
    loading: "Opening workspace",
    ready: "Local workspace",
    error: "Storage needs attention",
  }[status];

  return (
    <aside className="flex min-h-0 flex-col border-r bg-sidebar px-2.5 pb-4 md:px-4">
      <Link to="/" aria-label="LibraDesk home" className="flex h-16 items-center justify-center gap-3 rounded-md focus-visible:outline-2 focus-visible:outline-ring md:justify-start md:px-1">
        <span className="grid size-9 shrink-0 place-items-center rounded-xl bg-primary text-primary-foreground"><BookOpen className="size-5" /></span>
        <span className="hidden text-lg font-semibold tracking-tight md:inline">LibraDesk</span>
      </Link>
      <p className="mt-5 mb-2 hidden px-3 text-[10px] font-semibold tracking-[0.18em] text-muted-foreground uppercase md:block">Workspace</p>
      <nav aria-label="Workspace" className="mt-4 grid gap-1 md:mt-0">
        {navigation.map(({ to, name, icon: Icon }) => (
          <Tooltip key={to}>
            <TooltipTrigger asChild>
              <NavLink end={to === "/"} to={to} aria-label={name} className="flex h-10 items-center justify-center gap-3 rounded-md px-2 text-sm font-medium text-muted-foreground transition-colors hover:bg-accent hover:text-foreground focus-visible:outline-2 focus-visible:outline-ring aria-[current=page]:bg-accent aria-[current=page]:text-accent-foreground md:justify-start md:px-3">
                <Icon className="size-4 shrink-0" />
                <span className="hidden md:inline">{name}</span>
              </NavLink>
            </TooltipTrigger>
            <TooltipContent side="right" className="md:hidden">{name}</TooltipContent>
          </Tooltip>
        ))}
      </nav>
      <div role="status" className="mt-auto flex items-center justify-center gap-2 border-t pt-4 text-xs text-muted-foreground md:justify-start md:px-2">
        <span className={cn("size-2 shrink-0 rounded-full", status === "error" ? "bg-destructive" : "bg-ring")} />
        <span className="sr-only md:not-sr-only">{storageStatus}</span>
      </div>
    </aside>
  );
}

export function Shell() {
  const { pathname, search } = useLocation();
  const main = useRef<HTMLElement>(null);
  useLayoutEffect(() => { main.current?.scrollTo({ top: 0 }); }, [pathname, search]);
  const title = navigation.find((item) => item.to === pathname || item.to !== "/" && pathname.startsWith(item.to + "/"))?.name ?? "Workspace";

  return (
    <div className="grid h-dvh grid-cols-[3.75rem_minmax(0,1fr)] overflow-hidden md:grid-cols-[13rem_minmax(0,1fr)]">
      <Button className="sr-only focus:not-sr-only focus:absolute focus:top-2 focus:left-2 focus:z-50" onClick={() => document.getElementById("main-content")?.focus()}>
        Skip to content
      </Button>
      <Sidebar />
      <div className="grid min-h-0 min-w-0 grid-rows-[4rem_minmax(0,1fr)]">
        <header className="flex items-center justify-between gap-3 border-b bg-background px-4 sm:px-6 lg:px-8">
          <span className="truncate text-sm font-medium">{title}</span>
          <div className="flex items-center gap-2">
            <SearchEntry />
            <DropdownMenu>
              <Tooltip>
                <TooltipTrigger asChild>
                  <DropdownMenuTrigger asChild>
                    <Button variant="outline" size="icon" aria-label="Application menu"><Ellipsis /></Button>
                  </DropdownMenuTrigger>
                </TooltipTrigger>
                <TooltipContent>Application menu</TooltipContent>
              </Tooltip>
              <DropdownMenuContent align="end" className="w-44">
                <DropdownMenuLabel>LibraDesk</DropdownMenuLabel>
                <DropdownMenuSeparator />
                <DropdownMenuItem asChild><Link to="/library"><LibraryBig /> Open library</Link></DropdownMenuItem>
                <DropdownMenuItem asChild><Link to="/import"><FolderInput /> Import PDFs</Link></DropdownMenuItem>
                <DropdownMenuItem asChild><Link to="/settings"><Settings2 /> Settings</Link></DropdownMenuItem>
              </DropdownMenuContent>
            </DropdownMenu>
          </div>
        </header>
        <main ref={main} id="main-content" tabIndex={-1} className="overflow-y-auto focus:outline-none">
          <div className="mx-auto w-full max-w-6xl px-4 py-8 sm:px-6 lg:px-8">
            <Suspense fallback={<Skeleton className="h-64" role="status" aria-label="Loading page" />}>
              <Outlet />
            </Suspense>
          </div>
        </main>
      </div>
    </div>
  );
}
