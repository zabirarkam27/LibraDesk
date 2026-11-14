(() => {
  const state = new URLSearchParams(location.search).get("case") || "ready";
  const organization = {
    categories: [{ id: 1, name: "Fiction", parentId: null, description: null, sortMode: "manual", revision: 1, count: 2 }],
    tags: [{ id: 1, name: "বাংলা", parentId: null, description: null, sortMode: "manual", revision: 1, count: 2 }],
    collections: [{ id: 1, name: "Reading queue", parentId: null, description: "Read this month", sortMode: "manual", revision: 1, count: 2 }],
    languages: ["bn", "en"],
  };
  const records = Array.from({ length: Number(new URLSearchParams(location.search).get("records")) || 10000 }, (_, index) => ({
    id: 10000 - index, title: "Book " + (10000 - index), path: "D:/Fixture/book-" + index + ".pdf",
    bytes: 2048, author: "বাংলা লেখক", pages: 37, fileId: 10000 - index,
    coverRevision: index % 2 === 0 ? 1 : null, duplicate: index === 0,
    isFavorite: false, readingStatus: "unread", revision: 1, publicationYear: 2026,
    language: "bn", categoryIds: [1], tagIds: [1], collectionIds: index < 2 ? [1] : [],
    subject: "Fiction", publisher: "Press", isbn: "9780000000002",
  }));
  const members = [10000, 9999];
  let callbackId = 0;
  let groupId = 10;
  const callbacks = new Map();
  const subscriptions = new Map();
  window.fixture = { state, requests: [], writes: [], organization, records };
  if (state === "preview") return;
  function publish() {
    for (const [id, event] of subscriptions) {
      if (event === "catalog-updated") callbacks.get(id)?.({ event, payload: null });
    }
  }
  fixture.publish = publish;
  function book(id) {
    const record = records.find(record => record.id === id);
    if (!record) throw { message: "This record no longer exists." };
    return record;
  }
  function details(record) {
    return {
      ...record,
      authors: record.author ? record.author.split(", ").map((displayName, id) => ({ id: id + 1, displayName })) : [],
      files: [{
        id: record.fileId, bookId: record.id, path: record.path, bytes: record.bytes,
        pages: record.pages, availability: "available", revision: 1, isPrimary: true,
        coverRevision: record.coverRevision, modifiedAt: 0, lastVerifiedAt: 0,
      }],
    };
  }
  const pixel = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+/l2sAAAAASUVORK5CYII=";
  window.isTauri = true;
  window.__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener: (_event, id) => subscriptions.delete(id) };
  window.__TAURI_INTERNALS__ = {
    metadata: { currentWindow: { label: "main" }, currentWebview: { label: "main" } },
    transformCallback(fn) { callbacks.set(++callbackId, fn); return callbackId; },
    unregisterCallback(id) { callbacks.delete(id); },
    async invoke(command, args = {}) {
      if(command==="pdf_metadata_status")return [];
      if (command === "plugin:event|listen") { subscriptions.set(args.handler, args.event); return args.handler; }
      if (command === "plugin:event|unlisten") return;
      if (command === "get_preferences") return { app: "system", reader: "system", schemaVersion: 5 };
      if (command === "get_settings_snapshot") return {workspace:{startPage:"overview",libraryView:"grid"},reader:{zoom:"fit_width",scale:100,layout:"continuous",fullscreen:false,shortcuts:true},dataPath:"Fixture",version:"0.1.0"};
      if (command === "claim_text_extraction") return null;
      if (command === "reading_analytics") return {completed:0,reading:0,pages:0,activeMs:0,sessions:0,activity:[],history:[],page:1,hasMore:false};
      if (command === "next_cover_task") return null;
      if (command === "get_book_cover") return Uint8Array.from(atob(pixel), c => c.charCodeAt(0)).buffer;
      await new Promise(resolve => setTimeout(resolve, 50));
      if (fixture.state === "refresh_error" && ["get_book", "get_organization"].includes(command)) throw { message: "Refresh failed" };
      if (command === "query_catalog") {
        fixture.requests.push(args.query);
        if (fixture.state === "error") throw { message: "fixture failure" };
        let rows = fixture.state === "empty" ? [] : records;
        const q = args.query;
        if (q.text) rows = rows.filter(record => (record.title + " " + record.author).toLowerCase().includes(q.text.toLowerCase()));
        if (q.collectionId) rows = rows.filter(record => members.includes(record.id));
        if (q.favoritesOnly) rows = rows.filter(record => record.isFavorite);
        if (q.readingStatus) rows = rows.filter(record => record.readingStatus === q.readingStatus);
        if (q.sort === "collection") rows = [...rows].sort((a, b) => members.indexOf(a.id) - members.indexOf(b.id));
        const page = Math.min(q.page || 0, Math.max(0, Math.ceil(rows.length / 50) - 1));
        return { books: structuredClone(rows.slice(page * 50, page * 50 + 50)), total: rows.length, page };
      }
      if (command === "get_organization") return structuredClone(organization);
      if (command === "get_book") return structuredClone(details(book(args.id)));
      if (command === "get_dashboard") return {
        totalBooks: records.length, totalStorage: 20480000, totalPages: 370000,
        favorites: records.filter(record => record.isFavorite).length,
        reading: records.filter(record => record.readingStatus === "reading").length,
        categories: [{ id: 1, name: "Fiction", count: 2 }],
        recentlyAdded: records.slice(0, 2), recentlyRead: [], favoriteBooks: records.filter(record => record.isFavorite).slice(0, 6),
        currentlyReading: records.filter(record => record.readingStatus === "reading").slice(0, 6),
      };
      fixture.writes.push({ command, args: structuredClone(args) });
      let result;
      if (command === "update_book") {
        const current = book(args.id);
        if (args.revision !== current.revision) throw { message: "This record has changed. Reload it before making more changes." };
        Object.assign(current, args.input, { revision: current.revision + 1 });
        if (args.input.authorNames) current.author = args.input.authorNames.join(", ");
        result = details(current);
      } else if (command === "set_book_flags") {
        const current = book(args.id);
        if (args.favorite !== undefined) current.isFavorite = args.favorite;
        if (args.status !== undefined) current.readingStatus = args.status;
        current.revision++;
      } else if (command === "save_group") {
        const list = organization[args.kind === "category" ? "categories" : args.kind + "s"];
        if (args.input.id) Object.assign(list.find(group => group.id === args.input.id), args.input);
        else list.push({ ...args.input, id: ++groupId, count: 0, revision: 1, sortMode: args.input.sortMode || "manual", parentId: args.input.parentId || null, description: args.input.description || null });
        result = args.input.id || groupId;
      } else if (command === "delete_group") {
        const key = args.kind === "category" ? "categories" : args.kind + "s";
        organization[key] = organization[key].filter(group => group.id !== args.id);
      } else if (command === "edit_collection_member") {
        const index = members.indexOf(args.book);
        if (args.action === "remove") members.splice(index, 1);
        else {
          const other = args.action === "up" ? index - 1 : index + 1;
          [members[index], members[other]] = [members[other], members[index]];
        }
        organization.collections[0].revision++;
      } else throw new Error(command);
      setTimeout(publish, 0);
      return result;
    },
  };
})();
