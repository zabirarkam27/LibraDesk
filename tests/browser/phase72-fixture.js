(() => {
  const stored = JSON.parse(localStorage.getItem("reader-fixture-state") || "{}");
  const callbacks = new Map();
  const subscriptions = new Map();
  let callback = 0;
  let session = 0;
  const sessions = new Map();
  const records = {
    1: { file: "reader-fixture.pdf", length: 16752, title: "Reader fixture" },
    2: { file: "reader-long.pdf", length: 1, title: "Long reader fixture" },
    3: { file: "reader-large.pdf", length: 1, title: "Large reader fixture" },
    4: { file: "reader-locked.pdf", length: 1, title: "Locked reader fixture" },
  };
  window.fixture = {
    ranges: [], writes: [], sessions, annotations: stored.annotations || { bookmarks: [], notes: [] },
    preferences: stored.preferences || { zoom: "fit_width", scale: 100, layout: "continuous", fullscreen: false, shortcuts: true },
    progress: stored.progress || {}, closeCallbacks: [], fullscreen: false, closed: false, failures: [],
  };
  function persist() {
    localStorage.setItem("reader-fixture-state", JSON.stringify({
      annotations: fixture.annotations, preferences: fixture.preferences, progress: fixture.progress,
    }));
  }
  function publish(event) {
    for (const [id, name] of subscriptions) if (name === event) callbacks.get(id)?.({ event, payload: null });
  }
  window.isTauri = true;
  window.__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener: (_name, id) => subscriptions.delete(id) };
  window.__TAURI_INTERNALS__ = {
    metadata: { currentWindow: { label: "main" }, currentWebview: { label: "main" } },
    transformCallback(fn) { callbacks.set(++callback, fn); return callback; },
    unregisterCallback(id) { callbacks.delete(id); },
    async invoke(command, args = {}) {
      if (command === "plugin:event|listen") { subscriptions.set(args.handler, args.event); return args.handler; }
      if (command === "plugin:event|unlisten") return;
      if (command === "get_preferences") return { app: "light", reader: stored.theme || "light", schemaVersion: 6 };
      if (command === "set_theme") { stored.theme = args.theme; return; }
      if (command === "next_cover_task") return null;
      if (command === "plugin:window|is_fullscreen") return fixture.fullscreen;
      if (command === "plugin:window|set_fullscreen") { fixture.fullscreen = args.value; return; }
      if (command === "plugin:window|destroy") { fixture.closed = true; return; }
      if (command === "open_reader") {
        const record = records[args.book];
        if (!record) throw { message: "The PDF is unavailable." };
        const meta = await (await fetch("/fixture-meta?file=" + record.file)).json();
        sessions.set(++session, { book: args.book, file: record.file, closed: false });
        return {
          sessionId: session, bookId: args.book, fileId: args.book, revision: 1,
          length: meta.length, title: record.title, resumePage: fixture.progress[args.book]?.page || 1,
          staleProgress: false, preferences: fixture.preferences,
        };
      }
      if (command === "read_reader_range") {
        const record = sessions.get(args.session);
        if (!record || record.closed) throw { message: "Reader session closed" };
        if (fixture.failRange) throw { message: "Reader source unavailable" };
        fixture.ranges.push({ ...args });
        return (await fetch("/fixture-range?file=" + record.file + "&offset=" + args.offset + "&length=" + args.length)).arrayBuffer();
      }
      fixture.writes.push({ command, args });
      if (command === "save_reader_progress") {
        const record = sessions.get(args.input.sessionId);
        if (fixture.failProgress) throw { message: "Unable to save progress" };
        if (record && !record.closed) { fixture.progress[record.book] = args.input; if (args.input.close) record.closed = true; }
        persist(); publish("catalog-updated"); return;
      }
      if (command === "close_reader") { if (sessions.has(args.session)) sessions.get(args.session).closed = true; return; }
      if (command === "save_reader_preferences") { fixture.preferences = args.input; persist(); return; }
      if (command === "reader_annotations") return structuredClone(fixture.annotations);
      if (command === "save_reader_annotation") {
        const list = fixture.annotations[args.kind === "note" ? "notes" : "bookmarks"];
        if (args.input.id) {
          const record = list.find(record => record.id === args.input.id);
          record.text = args.input.text; record.revision++;
        } else list.push({ id: Date.now(), fileId: 1, fileRevision: 1, page: args.input.page, text: args.input.text, revision: 1 });
        persist(); return list.at(-1)?.id;
      }
      if (command === "delete_reader_annotation") {
        const key = args.kind === "note" ? "notes" : "bookmarks";
        fixture.annotations[key] = fixture.annotations[key].filter(record => record.id !== args.id);
        persist(); return;
      }
      if (command === "query_catalog") return { books: [], total: 0, page: 0 };
      if (command === "get_book") return {
        id: args.id, title: "Reader fixture", author: null, authors: [], subject: null, publisher: null,
        isbn: null, publicationYear: null, language: "bn", revision: 1, isFavorite: false, readingStatus: "unread",
        categoryIds: [], tagIds: [], collectionIds: [], files: [{
          id: args.id, bookId: args.id, path: "fixture.pdf", bytes: 40000, pages: 30, availability: "available",
          revision: 1, isPrimary: true, coverRevision: null, modifiedAt: 0, lastVerifiedAt: 0,
        }],
      };
      if (command === "get_organization") return { categories: [], tags: [], collections: [], languages: [] };
      throw new Error(command);
    },
  };
  window.addEventListener("error", event => fixture.failures.push(event.message));
  window.addEventListener("unhandledrejection", event => fixture.failures.push(String(event.reason)));
  fixture.closeWindow = () => publish("tauri://close-requested");
})();

(() => {
 fixture.searchRequests=[];fixture.searchError=false;
 const invoke=window.__TAURI_INTERNALS__.invoke.bind(window.__TAURI_INTERNALS__);
 window.__TAURI_INTERNALS__.invoke=async(command,args={})=>{
  if(command==="claim_text_extraction"||command==="text_extraction_status")return null;
  if(command==="search_page_text"){
   fixture.searchRequests.push(args.query);
   if(fixture.searchError)throw {message:"Search unavailable"};
   const text=args.query.text,page=args.query.page;
   if(text==="slow")await new Promise(resolve=>setTimeout(resolve,1800));
   const total=!text.trim()||text==="absent"?0:text==="many"?105:2;
   return {total,page,coverage:{files:3,completedFiles:1,failedFiles:1,extractedPages:40},
    hits:Array.from({length:Math.max(0,Math.min(50,total-page*50))},(_,index)=>({
     id:page*50+index+1,bookId:1,title:text==="slow"?"OLD SEARCH":"Reader fixture",author:"বাংলা Author",
     fileId:1,revision:text==="stale"?99:1,path:"D:\\Books\\বাংলা fixture.pdf",
     pageNumber:index===0?7:12,snippet:'বাংলা needle excerpt <img src=x onerror="window.excerptInjected=true"> & literal text',
    }))};
  }
  return invoke(command,args);
 };
})();

(() => {
 fixture.config=JSON.parse(localStorage.getItem("settings-config")||'{"startPage":"overview","libraryView":"grid"}');
 fixture.appearance=JSON.parse(localStorage.getItem("settings-themes")||'{"app":"light","reader":"light"}');
 fixture.settingsCalls=[];fixture.settingsError=false;fixture.rejectSettings=false;
 const invoke=window.__TAURI_INTERNALS__.invoke.bind(window.__TAURI_INTERNALS__);
 window.__TAURI_INTERNALS__.invoke=async(command,args={})=>{
  if(command==="get_settings_snapshot"){
   const delay=Number(localStorage.getItem("settings-delay"))||fixture.settingsDelay; if(delay)await new Promise(resolve=>setTimeout(resolve,delay));
   if(fixture.settingsError)throw {message:"Settings unavailable"};
   return {workspace:structuredClone(fixture.config),reader:structuredClone(fixture.preferences),dataPath:"C:\\Users\\Fixture\\AppData\\Local\\LibraDesk",version:"0.1.0"};
  }
  if(command==="save_workspace_preferences"){
   if(fixture.rejectSettings)throw {message:"Save rejected"};
   fixture.config=structuredClone(args.input);fixture.settingsCalls.push(args.input);
   localStorage.setItem("settings-config",JSON.stringify(fixture.config));return;
  }
  if(command==="get_preferences")return {...fixture.appearance,schemaVersion:9};
  if(command==="set_theme"){
   fixture.appearance[args.scope]=args.theme;localStorage.setItem("settings-themes",JSON.stringify(fixture.appearance));return;
  }
  return invoke(command,args);
 };
})();

(() => {
 fixture.disk={database:1048576,thumbnails:524288,cache:1024,temporary:2048,backups:2097152,logs:256,files:12,skipped:0};
 fixture.clears=[];
 const invoke=window.__TAURI_INTERNALS__.invoke.bind(window.__TAURI_INTERNALS__);
 window.__TAURI_INTERNALS__.invoke=async(command,args={})=>{
  if(command==="storage_usage")return structuredClone(fixture.disk);
  if(command==="clear_cache"){
   if(fixture.failCleanup)throw {message:"Cleanup unavailable"};
   fixture.clears.push(args.kind);
   const field=args.kind==="temporary"?"temporary":args.kind;
   const bytes=fixture.disk[field];fixture.disk[field]=0;
   return {removed:2,bytes,retained:1,usage:structuredClone(fixture.disk)};
  }
  return invoke(command,args);
 };
})();
