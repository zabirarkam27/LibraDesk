import { useEffect } from "react";
import { Link,useSearchParams } from "react-router";
import { toast } from "sonner";
import { desktop,errorMessage } from "@/shared/lib/preferences-client";
import { Button } from "@/shared/ui/button";
import { PageHeading } from "@/shared/ui/page-heading";
import { Skeleton } from "@/shared/ui/skeleton";
import { fieldClass } from "@/features/library/query-feedback";
import { usePreferences } from "./preferences-store";
import { useWorkspaceSettings,type WorkspacePreferences } from "./workspace-store";
import { ThemeOptions } from "./theme-options";
import { BackupSettings } from "./backup-settings";
import { StorageSettings } from "./storage-settings";
import { ReaderSettings } from "./reader-settings";

const sections=["general","appearance","library","reader","storage","backup","privacy"] as const;
type Section=typeof sections[number];
const names:Record<Section,string>={general:"General",appearance:"Appearance",library:"Library",reader:"Reader",storage:"Storage",backup:"Backup",privacy:"Privacy"};
function WorkspaceSelect({field,label,options}:{field:keyof WorkspacePreferences;label:string;options:readonly [string,string][]}) {
  const value=useWorkspaceSettings(state=>state.snapshot.workspace[field]);
  const saving=useWorkspaceSettings(state=>state.saving);
  const update=useWorkspaceSettings(state=>state.updateWorkspace);
  return <label className="grid max-w-md gap-2 text-sm font-medium">{label}
    <select className={fieldClass} value={value} disabled={saving} onChange={event=>{
      void update({[field]:event.target.value} as Partial<WorkspacePreferences>).then(()=>toast.success("Preference saved"),reason=>toast.error(errorMessage(reason)));
    }}>{options.map(([value,label])=><option key={value} value={value}>{label}</option>)}</select>
  </label>;
}
function Panel({section}:{section:Section}) {
  const snapshot=useWorkspaceSettings(state=>state.snapshot);
  switch(section) {
    case "general":return <div className="space-y-5">
      <p className="text-sm text-muted-foreground">LibraDesk {snapshot.version} · {desktop?"Windows desktop":"Browser preview"}</p>
      <WorkspaceSelect field="startPage" label="Open at startup" options={[["overview","Overview"],["library","Library"]]}/>
      <p className="text-sm text-muted-foreground">Applies the next time the app opens. Links to a specific book or page still open directly.</p>
    </div>;
    case "appearance":return <ThemeOptions scope="app" label="App theme" description="Choose Light, Dark, or your system appearance for the workspace."/>;
    case "library":return <div className="space-y-5">
      <WorkspaceSelect field="libraryView" label="Default library view" options={[["grid","Grid"],["list","List"],["compact","Compact"]]}/>
      <p className="text-sm text-muted-foreground">Used when opening Library. Its view buttons can change the current view.</p>
      <div className="flex flex-wrap gap-2">
        <Button asChild variant="outline"><Link to="/import">Manage scan folders</Link></Button>
        <Button asChild variant="outline"><Link to="/organize">Organize library</Link></Button>
        <Button asChild variant="outline"><Link to="/recovery">File recovery</Link></Button>
      </div>
    </div>;
    case "reader":return <ReaderSettings/>;
    case "storage":return <StorageSettings/>;
    case "backup":return <BackupSettings/>;
    case "privacy":return <div className="space-y-4 text-sm">
      <p className="font-medium">Your library is local to this device.</p>
      <p className="text-muted-foreground">Core library features work offline. No cloud account or synchronization is configured.</p>
      <p className="text-muted-foreground">Extracted PDF text, metadata, notes, bookmarks, custom thumbnails and reading activity are stored locally. Import and reading use PDFs in place. Book metadata edits also queue atomic updates to the original PDFs without a safety backup; check each book's sync status for failures.</p>
      <p className="text-muted-foreground">PDF scripting and external PDF links are disabled in the built-in reader.</p>
    </div>;
  }
}
export function SettingsPage() {
  const theme=usePreferences();
  const status=useWorkspaceSettings(state=>state.status);
  const error=useWorkspaceSettings(state=>state.error);
  const load=useWorkspaceSettings(state=>state.load);
  const [params,setParams]=useSearchParams();
  const section=sections.find(value=>value===params.get("section"))??"general";
  useEffect(()=>{void load(true);},[load]);
  const loading=theme.status==="idle"||theme.status==="loading"||status==="idle"||status==="loading";
  return <>
    <PageHeading label="Settings" title="Preferences" description="Workspace, library, reader, and local data settings."/>
    <nav aria-label="Settings sections" className="mb-5 flex flex-wrap gap-2">{sections.map(value=>
      <Button key={value} variant={value===section?"default":"outline"} aria-current={value===section?"page":undefined} onClick={()=>setParams({section:value})}>{names[value]}</Button>
    )}</nav>
    {loading?<Skeleton className="h-56" role="status" aria-label="Loading settings"/>:
      theme.status==="error"||status==="error"?<section role="alert" className="space-y-3 rounded-xl border p-5">
        <p className="text-sm text-destructive">{error??theme.error?.message??"Could not load settings."}</p>
        <Button variant="outline" onClick={()=>{void theme.load();void load(true);}}>Reload settings</Button>
      </section>:
      <section className="space-y-5 rounded-xl border bg-card p-5 sm:p-6" aria-labelledby="settings-section-title">
        <h2 id="settings-section-title" className="font-display text-2xl">{names[section]}</h2>
        <Panel section={section}/>
        {!desktop&&<p className="border-t pt-4 text-xs text-muted-foreground">Browser preview uses browser storage. Open the Windows app for local library tools.</p>}
      </section>}
  </>;
}
