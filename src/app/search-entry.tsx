import { useEffect, useState } from "react";
import { Search } from "lucide-react";
import { Button } from "@/shared/ui/button";
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle, DialogTrigger } from "@/shared/ui/dialog";
import { Input } from "@/shared/ui/input";
import { useNavigate } from "react-router";

export function SearchEntry() {
  const [open, setOpen] = useState(false);
  const navigate = useNavigate();

  useEffect(() => {
    function handleShortcut(event: KeyboardEvent) {
      if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "k") {
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
          <DialogDescription>Find a title, author, or tag in বাংলা or English.</DialogDescription>
        </DialogHeader>
        <form className="flex gap-2" onSubmit={(event) => {
          event.preventDefault();
          const query = String(new FormData(event.currentTarget).get("search") ?? "").trim();
          navigate("/library?" + new URLSearchParams({ q: query }));
          setOpen(false);
        }}>
          <Input name="search" aria-label="Title, author, or tag" placeholder="Title, author, or tag" maxLength={256} />
          <Button>Search</Button>
        </form>
      </DialogContent>
    </Dialog>
  );
}
