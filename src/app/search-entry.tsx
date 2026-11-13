import { useEffect, useState } from "react";
import { Search } from "lucide-react";
import { Button } from "@/shared/ui/button";
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle, DialogTrigger } from "@/shared/ui/dialog";
import { Input } from "@/shared/ui/input";
import { useNavigate } from "react-router";
import { fieldClass } from "@/features/library/query-feedback";

export function SearchEntry() {
  const [open, setOpen] = useState(false);
  const navigate = useNavigate();

  useEffect(() => {
    function handleShortcut(event: KeyboardEvent) {
      if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "k") {
        if(document.querySelector('[role="dialog"]'))return;
        event.preventDefault();
        setOpen(true);
      }
    }
    window.addEventListener("keydown", handleShortcut);
    return () => window.removeEventListener("keydown", handleShortcut);
  }, []);

  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogTrigger asChild>
        <Button variant="outline" aria-keyshortcuts="Control+K Meta+K" className="w-44 justify-start gap-2.5 text-muted-foreground sm:w-64">
          <Search />
          <span>Search library</span>
          <kbd className="ml-auto hidden rounded border px-1.5 py-0.5 font-sans text-[10px] sm:inline">Ctrl K</kbd>
        </Button>
      </DialogTrigger>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Search library</DialogTitle>
          <DialogDescription>Search metadata or extracted PDF contents in বাংলা or English.</DialogDescription>
        </DialogHeader>
        <form className="grid gap-3" onSubmit={(event) => {
          event.preventDefault();
          const query = String(new FormData(event.currentTarget).get("search") ?? "").trim();
          const scope = new FormData(event.currentTarget).get("scope");
          navigate((scope === "contents" ? "/search?" : "/library?") + new URLSearchParams({ q: query }));
          setOpen(false);
        }}>
          <select name="scope" aria-label="Search in" className={fieldClass}>
            <option value="metadata">Titles, authors, and tags</option>
            <option value="contents">PDF contents</option>
          </select>
          <Input name="search" aria-label="Search terms" placeholder="বাংলা or English words" maxLength={256} />
          <Button>Search</Button>
        </form>
      </DialogContent>
    </Dialog>
  );
}
