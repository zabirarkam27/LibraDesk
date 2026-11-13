import { useState } from "react";
import { Button } from "@/shared/ui/button";
import { Dialog,DialogContent,DialogHeader,DialogTitle,DialogDescription,DialogFooter } from "@/shared/ui/dialog";
import { errorMessage } from "@/shared/lib/preferences-client";
import { formatBytes } from "@/shared/lib/file-size";
import { previewRelink,confirmRelink,cancelRelink,type RelinkPreview } from "./client";
export function RelinkButton({file}:{file:number}) {
  const [preview,setPreview]=useState<RelinkPreview|null>(null);
  const [busy,setBusy]=useState(false);
  const [error,setError]=useState<string|null>(null);
  async function choose() {
    setBusy(true);setError(null);
    try {setPreview(await previewRelink(file));}
    catch(reason){setError(errorMessage(reason));}
    finally{setBusy(false);}
  }
  async function cancel() {
    if(!preview||busy)return;
    setBusy(true);
    try{await cancelRelink(preview.draft);setPreview(null);setError(null);}
    catch(reason){setError(errorMessage(reason));}
    finally{setBusy(false);}
  }
  async function confirm() {
    if(!preview||busy)return;
    setBusy(true);setError(null);
    try{await confirmRelink(preview.draft);setPreview(null);}
    catch(reason){setError(errorMessage(reason));}
    finally{setBusy(false);}
  }
  return <div className="mt-2 space-y-2">
    <Button variant="outline" size="sm" disabled={busy} onClick={()=>void choose()}>{busy?"Checking PDF…":"Relink PDF"}</Button>
    {error&&!preview&&<p role="alert" className="text-xs text-destructive">{error}</p>}
    <Dialog open={!!preview} onOpenChange={(open)=>{if(!open)void cancel();}}>
      <DialogContent className="max-h-[85dvh] overflow-y-auto sm:max-w-2xl" onEscapeKeyDown={(event)=>{if(busy)event.preventDefault();}} onPointerDownOutside={(event)=>{if(busy)event.preventDefault();}}>
        <DialogHeader><DialogTitle>Confirm PDF relink</DialogTitle><DialogDescription>Only the library link changes. Source PDFs stay in place. Notes, bookmarks, and reading progress are retained.</DialogDescription></DialogHeader>
        {preview&&<div className="space-y-4 text-sm">
          <p className="font-semibold wrap-anywhere">{preview.title}</p>
          <dl className="space-y-3">
            <div><dt className="font-medium">Current location</dt><dd className="wrap-anywhere text-muted-foreground">{preview.originalPath}</dd></div>
            <div><dt className="font-medium">Replacement</dt><dd className="wrap-anywhere text-muted-foreground">{preview.replacementPath} · {formatBytes(preview.bytes)}</dd></div>
          </dl>
          <table className="w-full table-fixed text-left text-xs"><caption className="mb-2 text-left font-semibold">PDF metadata comparison</caption>
            <thead><tr><th className="p-1">Field</th><th className="p-1">Previous PDF</th><th className="p-1">Replacement PDF</th></tr></thead>
            <tbody>{[["Title",preview.oldTitle,preview.newTitle],["Author",preview.oldAuthor,preview.newAuthor],["Pages",preview.oldPages,preview.newPages]].map(([label,before,after])=><tr key={String(label)}><th className="p-1">{label}</th><td className="p-1 wrap-anywhere">{before??"Unknown"}</td><td className="p-1 wrap-anywhere">{after??"Unknown"}</td></tr>)}</tbody>
          </table>
          <p className={preview.identical?"text-primary":"font-semibold"}>{preview.identical?"SHA-256 matches: identical content.":"Different or unknown original hash. Page-linked notes and progress may refer to different pages."}</p>
          <p className="break-all text-xs text-muted-foreground">Previous SHA-256: {preview.oldHash??"Unknown"}<br/>Replacement SHA-256: {preview.newHash}</p>
          <p className="text-xs text-muted-foreground">Your catalog metadata stays as saved. Replacement metadata may be unavailable for encrypted, malformed, or large PDFs. Preview expires after 15 minutes.</p>
        </div>}
        {error&&<p role="alert" className="text-sm text-destructive">{error}</p>}
        <DialogFooter><Button variant="outline" disabled={busy} onClick={()=>void cancel()}>Cancel</Button><Button disabled={busy} onClick={()=>void confirm()}>{busy?"Verifying…":"Confirm relink"}</Button></DialogFooter>
      </DialogContent>
    </Dialog>
  </div>;
}
