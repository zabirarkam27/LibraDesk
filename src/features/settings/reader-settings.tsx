import { useEffect,useState } from "react";
import { toast } from "sonner";
import { desktop,errorMessage } from "@/shared/lib/preferences-client";
import { Button } from "@/shared/ui/button";
import { Input } from "@/shared/ui/input";
import { fieldClass } from "@/features/library/query-feedback";
import { useWorkspaceSettings } from "./workspace-store";
import { ThemeOptions } from "./theme-options";
export function ReaderSettings() {
  const saved=useWorkspaceSettings(state=>state.snapshot.reader);
  const saving=useWorkspaceSettings(state=>state.saving);
  const update=useWorkspaceSettings(state=>state.updateReader);
  const [draft,setDraft]=useState(saved);
  const [error,setError]=useState<string|null>(null);
  useEffect(()=>{setDraft(saved);},[saved]);
  const valid=Number.isInteger(draft.scale)&&draft.scale>=25&&draft.scale<=400;
  return <div className="space-y-5">
    <ThemeOptions scope="reader" label="Reader theme" description="Reader appearance is saved independently of the workspace."/>
    {!desktop&&<p className="text-sm text-muted-foreground">Open the Windows app to save PDF reader defaults.</p>}
    <form className="space-y-4 border-t pt-5" onSubmit={(event)=>{
      event.preventDefault();if(!valid)return;setError(null);
      void update(draft).then(()=>toast.success("Reader defaults saved"),reason=>setError(errorMessage(reason)));
    }}>
      <fieldset disabled={saving||!desktop} className="grid gap-4 sm:grid-cols-2" aria-busy={saving}>
        <legend className="mb-3 text-sm font-semibold">Defaults for the next reader you open</legend>
        <label className="grid gap-1 text-sm">Zoom mode
          <select className={fieldClass} value={draft.zoom} onChange={event=>setDraft({...draft,zoom:event.target.value as typeof draft.zoom,scale:valid?draft.scale:100})}>
            <option value="fit_width">Fit width</option><option value="fit_page">Fit page</option><option value="custom">Custom percentage</option>
          </select>
        </label>
        <label className="grid gap-1 text-sm">Custom zoom (%)
          <Input type="number" min={25} max={400} step={1} value={draft.scale} disabled={draft.zoom!=="custom"}
            onChange={event=>setDraft({...draft,scale:Number(event.target.value)})}/>
        </label>
        <label className="grid gap-1 text-sm">Page layout
          <select className={fieldClass} value={draft.layout} onChange={event=>setDraft({...draft,layout:event.target.value as typeof draft.layout})}>
            <option value="continuous">Continuous</option><option value="single">Single page</option>
          </select>
        </label>
        <div className="space-y-3 text-sm">
          <label className="flex items-center gap-2"><input type="checkbox" checked={draft.fullscreen} onChange={event=>setDraft({...draft,fullscreen:event.target.checked})}/>Open reader fullscreen</label>
          <label className="flex items-center gap-2"><input type="checkbox" checked={draft.shortcuts} onChange={event=>setDraft({...draft,shortcuts:event.target.checked})}/>Enable reader keyboard shortcuts</label>
        </div>
      </fieldset>
      {error&&<p role="alert" className="text-sm text-destructive">{error}</p>}
      {!valid&&<p role="alert" className="text-sm text-destructive">Enter a whole-number zoom between 25% and 400%.</p>}
      <Button disabled={saving||!desktop||!valid}>{saving?"Saving…":"Save reader defaults"}</Button>
    </form>
  </div>;
}
