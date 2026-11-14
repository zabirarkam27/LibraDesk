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
      if(command==="pdf_metadata_status")return [];
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
 const saved = JSON.parse(localStorage.getItem("extraction-fixture") || "{}");
 window.extraction = { enabled:false, jobs:saved.jobs || {}, pages:saved.pages || {}, batches:[], ranges:[], forceHidden:false, stopAt:0, heartbeats:0 };
 Object.defineProperty(document,"hidden",{ get:()=>extraction.forceHidden });
 const callbacks=new Map(), events=new Map();
 const original=window.__TAURI_INTERNALS__;
 const transform=original.transformCallback.bind(original);
 original.transformCallback=fn=>{const id=transform(fn);callbacks.set(id,fn);return id;};
 const persist=()=>localStorage.setItem("extraction-fixture",JSON.stringify({ jobs:extraction.jobs,pages:extraction.pages }));
 const publish=file=>{for(const [id,event] of events) if(event==="extraction-updated") callbacks.get(id)?.({event,payload:file});};
 extraction.add=(fileId,name)=>{
  extraction.jobs[fileId]={fileId,revision:1,lease:0,file:name,status:"queued",nextPage:1,totalPages:null,error:null};
  extraction.pages[fileId]={}; extraction.enabled=true;persist();publish(fileId);
 };
 const invoke=original.invoke.bind(original);
 original.invoke=async(command,args={})=>{
  if(command==="plugin:event|listen")events.set(args.handler,args.event);
  if(command==="plugin:event|unlisten")events.delete(args.eventId);
  if(command==="claim_text_extraction"){
   if(!extraction.enabled)return null;
   const job=Object.values(extraction.jobs).find(j=>j.status==="queued");
   if(!job)return null;
   job.status="running";job.lease++;
   const {length}=await(await fetch("/fixture-meta?file="+job.file)).json();
   persist();publish(job.fileId);return {...job,length};
  }
  if(command==="text_extraction_status"){
   const job=extraction.jobs[args.file];if(!job)return null;
   return {revision:job.revision,status:job.status,completedPages:job.nextPage-1,totalPages:job.totalPages,
    pagesWithText:Object.values(extraction.pages[args.file]).filter(t=>t.trim()).length,error:job.error};
  }
  if(command==="retry_text_extraction"){
   const job=extraction.jobs[args.file];if(!job||job.revision!==args.revision||job.status!=="failed")return false;
   job.status="queued";job.error=null;persist();publish(job.fileId);return true;
  }
  if(["read_extraction_range","renew_text_extraction","commit_page_text","release_text_extraction","fail_text_extraction"].includes(command)){
   const job=extraction.jobs[args.key.fileId];
   if(!job||job.lease!==args.key.lease||job.revision!==args.key.revision)throw {code:"extraction_stale"};
   if(command==="read_extraction_range"){
    extraction.ranges.push({offset:args.offset,length:args.length});
    return(await fetch("/fixture-range?file="+job.file+"&offset="+args.offset+"&length="+args.length)).arrayBuffer();
   }
   if(command==="renew_text_extraction"){extraction.heartbeats++;return job.status==="running";}
   if(command==="commit_page_text"){
    if(job.status!=="running")return false;
    if(args.pages.length>4)throw new Error("Unbounded batch");
    for(const page of args.pages){
     if(page.number!==job.nextPage)throw new Error("Duplicate or out of order page");
     if(new TextEncoder().encode(page.text).length>262144)throw new Error("Unbounded page");
     extraction.pages[job.fileId][page.number]=page.text;job.nextPage++;
    }
    extraction.batches.push(args.pages.map(p=>p.number));
    job.totalPages=args.total;if(job.nextPage>args.total)job.status="completed";
    if(extraction.stopAt&&job.nextPage>extraction.stopAt)extraction.forceHidden=true;
   }
   if(command==="release_text_extraction"&&job.status==="running")job.status="queued";
   if(command==="fail_text_extraction"){job.status="failed";job.error=args.reason;}
   persist();publish(job.fileId);return true;
  }
  return invoke(command,args);
 };
})();
