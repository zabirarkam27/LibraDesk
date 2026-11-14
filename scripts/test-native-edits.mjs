import {spawn} from "node:child_process";
import {readFile,writeFile,realpath} from "node:fs/promises";
import {resolve} from "node:path";
import {once} from "node:events";
import {setTimeout as delay} from "node:timers/promises";
import {createHash} from "node:crypto";
import assert from "node:assert/strict";
const profile=JSON.parse(await readFile("src-tauri/target/native-edit-profile.json","utf8"));
const targetRoot=await realpath("src-tauri/target");
assert.ok((await realpath(profile.root)).startsWith(targetRoot+"\\pdf-edit-profile-"));
assert.ok((await realpath(profile.pdf)).startsWith(targetRoot+"\\pdf-edit-profile-"));
const expectedVersion=JSON.parse(await readFile("src-tauri/tauri.conf.json","utf8")).version;
const port=9340;
await fetch("http://127.0.0.1:"+port+"/json/list").then(()=>{throw Error("QA port already in use");},()=>{});
const app=spawn(resolve("src-tauri/target/debug/libradesk.exe"),[],{windowsHide:true,stdio:"ignore",env:{...process.env,LIBRADESK_QA_DATA_DIR:profile.root,WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS:"--remote-debugging-port="+port}});
let socket;
try{
 let page;
 for(let i=0;i<150;i++){
  const targets=await fetch("http://127.0.0.1:"+port+"/json/list").then(r=>r.json()).catch(()=>[]);
  page=targets.find(p=>p.type==="page"&&p.url.includes("tauri.localhost"));
  if(page)break;assert.equal(app.exitCode,null);await delay(100);
 }
 assert.ok(page,"Embedded frontend native window");
 socket=new WebSocket(page.webSocketDebuggerUrl);await once(socket,"open");
 let serial=0;const pending=new Map();
 socket.addEventListener("message",event=>{const value=JSON.parse(event.data),task=pending.get(value.id);if(task){pending.delete(value.id);value.error?task.reject(value.error):task.resolve(value.result);}});
 socket.addEventListener("close",()=>{for(const task of pending.values())task.reject(Error("Window closed"));pending.clear();});
 const cdp=(method,params={})=>new Promise((resolve,reject)=>{const id=++serial;pending.set(id,{resolve,reject});socket.send(JSON.stringify({id,method,params}));});
 const run=async expression=>{const value=await cdp("Runtime.evaluate",{expression,returnByValue:true,awaitPromise:true});if(value.exceptionDetails)throw Error(value.exceptionDetails.exception?.description||value.exceptionDetails.text);return value.result.value;};
 const invoke=(command,args={})=>run('window.__TAURI_INTERNALS__.invoke('+JSON.stringify(command)+','+JSON.stringify(args)+')');
 const until=async expression=>{for(let i=0;i<150;i++){if(await run(expression))return;await delay(100);}throw Error("Timed out: "+expression);};
 await until('!!window.__TAURI_INTERNALS__');
 const settings=await invoke("get_settings_snapshot");
 assert.equal(settings.version,expectedVersion);assert.equal((await realpath(settings.dataPath)).toLowerCase(),(await realpath(profile.root)).toLowerCase());
 const parent=await invoke("save_group",{kind:"category",input:{name:"বাংলা"}});
 const category=await invoke("save_group",{kind:"category",input:{name:"Fiction",parentId:parent}});
 const tag=await invoke("save_group",{kind:"tag",input:{name:"Native tag"}});
 const collection=await invoke("save_group",{kind:"collection",input:{name:"Native collection"}});
 const before=await invoke("get_book",{id:profile.book});
 await invoke("update_book",{id:profile.book,revision:before.revision,input:{title:"Native বাংলা title",subject:"Native subject",genre:"উপন্যাস",publisher:"Native publisher",isbn:"9780000000002",language:"bn",publicationYear:2026,authorNames:["লেখক","Native author"],categoryIds:[category],tagIds:[tag],collectionIds:[collection],isFavorite:true,readingStatus:"reading"}});
 let statuses=[];
 for(let i=0;i<150;i++){
  statuses=await invoke("pdf_metadata_status",{book:profile.book});
  if(statuses.every(s=>s.status==="completed"))break;
  assert.ok(statuses.every(s=>s.status!=="failed"),JSON.stringify(statuses));await delay(100);
 }
 assert.ok(statuses.every(s=>s.status==="completed"));
 const hash=bytes=>createHash("sha256").update(bytes).digest("hex");
 const sourceHash=hash(await readFile(profile.pdf));
 await run('location.hash="#/library/'+profile.book+'"');
 await until('document.body.innerText.includes("Native বাংলা title")&&document.body.innerText.includes("Saved to PDF")');
 await run('Array.from(document.querySelectorAll("button")).find(b=>b.textContent==="Change thumbnail").click()');
 await until('!!document.querySelector("input[type=file]")');
 await run('(()=>{const canvas=document.createElement("canvas");canvas.width=1200;canvas.height=800;const c=canvas.getContext("2d");c.fillStyle="#416b48";c.fillRect(0,0,1200,800);const data=Uint8Array.from(atob(canvas.toDataURL("image/jpeg").split(",")[1]),c=>c.charCodeAt(0));const transfer=new DataTransfer();transfer.items.add(new File([data],"native-cover.jpg",{type:"image/jpeg"}));const input=document.querySelector("input[type=file]");input.files=transfer.files;input.dispatchEvent(new Event("change",{bubbles:true}));})()');
 await until('Array.from(document.images).some(i=>i.alt==="Selected custom thumbnail"&&i.naturalWidth===240)');
 await run('Array.from(document.querySelectorAll("button")).find(b=>b.textContent==="Save thumbnail").click()');
 await until('!document.querySelector("[role=dialog]")&&Array.from(document.images).some(i=>i.naturalWidth===240&&i.naturalHeight===160)');
 const after=await invoke("get_book",{id:profile.book});assert.ok(after.customCoverRevision>0);
 assert.ok(await run('window.__TAURI_INTERNALS__.invoke("get_book_cover",{fileId:'+profile.file+',revision:-'+after.customCoverRevision+'}).then(b=>b.byteLength>50)'));
 await invoke("clear_cache",{kind:"thumbnails"});
 const retained=await invoke("get_book",{id:profile.book});assert.equal(retained.customCoverRevision,after.customCoverRevision);
 assert.equal(hash(await readFile(profile.pdf)),sourceHash,"Custom thumbnail and cache cleanup do not alter the PDF");
 const backup=await invoke("create_backup",{includeThumbnails:false,chooseDestination:false});assert.equal(backup.books,1);
 await run('location.hash="#/reader?book='+profile.book+'&file='+profile.file+'"');
 await until('!!document.querySelector(".pdfViewer canvas")');
 await run('location.hash="#/library/'+profile.book+'"');
 await until('document.body.innerText.includes("Native বাংলা title")&&document.body.innerText.includes("Saved to PDF")');
 const screenshot=await cdp("Page.captureScreenshot",{format:"png"});await writeFile("src-tauri/target/native-edits.png",Buffer.from(screenshot.data,"base64"));
 const report={version:expectedVersion,profile:"isolated generated PDF/library under target",nativeMetadataSync:"completed",standardAndCustomFieldsRequested:true,jpegPreviewAndNativeThumbnailSave:true,customCoverSurvivesCacheClear:true,thumbnailDoesNotModifySource:true,editedPdfRenderedInNativeReader:true,backupWithoutGeneratedCoversBooks:backup.books,productionProtocol:page.url};
 await writeFile("src-tauri/target/native-edits.json",JSON.stringify(report,null,2));console.log(JSON.stringify(report,null,2));
 await invoke("plugin:window|destroy",{label:"main"}).catch(()=>{});
}finally{
 socket?.close();
 if(app.exitCode===null)await Promise.race([once(app,"exit"),delay(1000)]);
 if(app.exitCode===null)app.kill();
}
