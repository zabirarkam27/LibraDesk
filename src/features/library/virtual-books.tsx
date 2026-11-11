import { useLayoutEffect, useRef, useState } from "react";
import { BookCard, type CatalogView } from "./book-card";
import type { BookSummary } from "./catalog-client";

const rowHeights = { grid: 312, list: 132, compact: 54 };
const minimumCardWidth = 168;
const columnGap = 12;
const overscan = 2;

export function VirtualBooks({ books, view }: { books: BookSummary[]; view: CatalogView }) {
  const track = useRef<HTMLDivElement>(null);
  const [windowRows, setWindowRows] = useState({ columns: 1, start: 0, end: 1 });
  const rowHeight = rowHeights[view];

  useLayoutEffect(() => {
    const element = track.current;
    const viewport = element?.closest("main");
    if (!element || !viewport) return;

    const measure = () => {
      const columns = view === "grid"
        ? Math.max(1, Math.floor((element.clientWidth + columnGap) / (minimumCardWidth + columnGap)))
        : 1;
      const rows = Math.ceil(books.length / columns);
      const offset = viewport.getBoundingClientRect().top - element.getBoundingClientRect().top;
      const start = Math.min(rows, Math.max(0, Math.floor(offset / rowHeight) - overscan));
      const end = Math.min(rows, Math.max(start, Math.ceil((offset + viewport.clientHeight) / rowHeight) + overscan));
      setWindowRows((current) =>
        current.columns === columns && current.start === start && current.end === end
          ? current
          : { columns, start, end },
      );
    };

    const observer = new ResizeObserver(measure);
    observer.observe(element);
    observer.observe(viewport);
    viewport.addEventListener("scroll", measure, { passive: true });
    measure();
    return () => {
      observer.disconnect();
      viewport.removeEventListener("scroll", measure);
    };
  }, [books.length, rowHeight, view]);

  const { columns, start, end } = windowRows;
  const firstBook = start * columns;

  return (
    <div ref={track} className="relative" style={{ height: Math.ceil(books.length / columns) * rowHeight }}>
      <ul
        role="list"
        aria-label="Catalog books"
        className="absolute inset-x-0 grid"
        style={{
          top: start * rowHeight,
          columnGap,
          gridTemplateColumns: `repeat(${columns}, minmax(0, 1fr))`,
          gridAutoRows: rowHeight,
        }}
      >
        {books.slice(firstBook, end * columns).map((book, index) => (
          <li key={book.id} aria-posinset={firstBook + index + 1} aria-setsize={books.length} className="min-w-0 pb-2">
            <BookCard book={book} view={view} />
          </li>
        ))}
      </ul>
    </div>
  );
}
