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
 fixture.health={status:"completed",checked:3,total:3,missing:1,inaccessible:1,invalid:0,changed:1};
 fixture.recoveryFiles=[
 {id:1,bookId:1,title:"Missing বাংলা book",path:"H:\\Books\\missing.pdf",revision:1,availability:"missing",bytes:800,pages:30,checkedAt:1800000000000,result:"missing"},
 {id:2,bookId:2,title:"Denied PDF",path:"H:\\Books\\denied.pdf",revision:1,availability:"inaccessible",bytes:800,pages:10,checkedAt:1800000000000,result:"inaccessible"},
 {id:3,bookId:3,title:"Changed PDF",path:"H:\\Books\\changed.pdf",revision:2,availability:"available",bytes:900,pages:20,checkedAt:1800000000000,result:"changed"},
 ];
 fixture.drafts=0;fixture.cancelled=0;fixture.relinked=0;fixture.actions=[];fixture.pickerCancel=false;
 fixture.dupes=[
 {id:1,revision:1,bookId:1,title:"First copy",author:"Author",path:"D:\\Books\\first.pdf",bytes:800,pages:30,availability:"available",checkedAt:1800000000000,preferred:false,notes:1,bookmarks:2},
 {id:2,revision:1,bookId:2,title:"Second copy",author:"Author",path:"D:\\Books\\second.pdf",bytes:800,pages:30,availability:"available",checkedAt:1800000000000,preferred:false,notes:3,bookmarks:4},
 ];
 const invoke=window.__TAURI_INTERNALS__.invoke.bind(window.__TAURI_INTERNALS__);
 window.__TAURI_INTERNALS__.invoke=async(command,args={})=>{
  if(command==="recovery_snapshot")return structuredClone(fixture.health);
  if(command==="recovery_files")return structuredClone(fixture.recoveryFiles.filter(f=>f.id>args.after&&(!args.unavailableOnly||f.availability!=="available")));
  if(command==="start_file_verification"){fixture.health.status="running";return;}
  if(command==="preview_relink"){
   if(fixture.pickerCancel)return null;
   const file=fixture.recoveryFiles.find(f=>f.id===args.file);
   return {draft:++fixture.drafts,fileId:args.file,title:file.title,originalPath:file.path,replacementPath:"D:\\Recovered\\বাংলা.pdf",oldHash:"a".repeat(64),newHash:"a".repeat(64),identical:true,oldPages:30,newPages:30,oldTitle:"Old title",newTitle:"Recovered title",oldAuthor:"Author",newAuthor:"Author",bytes:800};
  }
  if(command==="cancel_relink"){fixture.cancelled++;return;}
  if(command==="confirm_relink"){
   if(fixture.rejectRelink)throw {message:"Replacement changed after preview. Preview again."};
   fixture.relinked++;return;
  }
  if(command==="list_duplicate_groups")return [{hash:"a".repeat(64),count:fixture.dupes.length,reviewed:!!fixture.ignored,members:fixture.dupes.map(f=>({title:f.title,path:f.path}))}].filter(g=>g.count>1);
  if(command==="duplicate_files")return structuredClone(fixture.dupes.filter(f=>f.id>args.after));
  if(command==="set_duplicate_review"){fixture.ignored=args.reviewed;return;}
  if(command==="duplicate_action"){
   fixture.actions.push(args);
   if(args.action==="prefer")fixture.dupes.forEach(f=>f.preferred=f.id===args.file);
   else {
    if(args.file===args.keep)throw new Error("Kept file removal");
    fixture.dupes=fixture.dupes.filter(f=>f.id!==args.file);
   }
   return;
  }
  return invoke(command,args);
 };
})();
