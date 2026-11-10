import { Book } from "lucide-react";
import { useEffect, useState } from "react";
import { getCover } from "./client";

export function BookCover({ fileId, revision }: { fileId: number | null; revision: number | null }) {
  const [image, setImage] = useState<{ url: string; fileId: number; revision: number } | null>(null);
  useEffect(() => {
    if (fileId === null || revision === null) return;
    let active = true;
    let url: string | undefined;
    void getCover(fileId, revision).then((bytes) => {
      if (!active) return;
      url = URL.createObjectURL(new Blob([bytes], { type: "image/png" }));
      setImage({ url, fileId, revision });
    }).catch(() => undefined);
    return () => {
      active = false;
      if (url) URL.revokeObjectURL(url);
    };
  }, [fileId, revision]);

  return (
    <span className="grid h-16 w-12 shrink-0 place-items-center overflow-hidden rounded-md border bg-accent text-accent-foreground">
      {image && image.fileId === fileId && image.revision === revision
        ? <img src={image.url} alt="" className="h-full w-full object-contain" onError={() => setImage(null)} />
        : <Book className="size-4" aria-hidden />}
    </span>
  );
}
