import {spawn,spawnSync} from "node:child_process";
import {readFile,writeFile,realpath} from "node:fs/promises";
import {resolve} from "node:path";
import {once} from "node:events";
import {setTimeout as delay} from "node:timers/promises";
import assert from "node:assert/strict";

const profile=(await readFile("src-tauri/target/qa-profile-path.txt","utf8")).trim();
assert.ok((await realpath(profile)).startsWith((await realpath("src-tauri/target"))+"\\performance-profile-"));
const port=9337;
await fetch("http://127.0.0.1:"+port+"/json/list").then(()=>{throw Error("QA debugging port already in use");},()=>{});
const started=performance.now();
const app=spawn(resolve("src-tauri/target/debug/libradesk.exe"),[],{windowsHide:true,env:{...process.env,LIBRADESK_QA_DATA_DIR:profile,WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS:"--remote-debugging-port="+port},stdio:"ignore"});
let socket;
try{
 let targets=[];
 for(let i=0;i<100;i++){
  targets=await fetch("http://127.0.0.1:"+port+"/json/list").then(r=>r.json()).catch(()=>[]);
  if(targets.some(t=>t.type==="page"&&t.url.includes("tauri.localhost")))break;
  if(app.exitCode!==null)throw Error("QA app exited before initialization");
  await delay(100);
 }
 const target=targets.find(t=>t.type==="page"&&t.url.includes("tauri.localhost"));
 assert.ok(target,"Native production-protocol WebView target");
 socket=new WebSocket(target.webSocketDebuggerUrl);await once(socket,"open");
 let serial=0;const pending=new Map();
 socket.addEventListener("close",()=>{for(const task of pending.values())task.reject(Error("QA window closed"));pending.clear();});
 socket.addEventListener("message",event=>{const value=JSON.parse(event.data);const task=pending.get(value.id);if(task){pending.delete(value.id);value.error?task.reject(value.error):task.resolve(value.result);}});
 const cdp=(method,params={})=>new Promise((resolve,reject)=>{const id=++serial;pending.set(id,{resolve,reject});socket.send(JSON.stringify({id,method,params}));});
 const run=async expression=>{const result=await cdp("Runtime.evaluate",{expression,returnByValue:true,awaitPromise:true});if(result.exceptionDetails)throw Error(result.exceptionDetails.exception?.description||result.exceptionDetails.text);return result.result.value;};
 for(let i=0;i<100;i++){if(await run('document.body.innerText.includes("10,001")'))break;await delay(100);}
 assert.ok(await run('document.body.innerText.includes("10,001")'));
 const initialUiMs=performance.now()-started;
 const settings=await run('window.__TAURI_INTERNALS__.invoke("get_settings_snapshot")');
 assert.equal((await realpath(settings.dataPath)).toLowerCase(),(await realpath(profile)).toLowerCase());
 const times=[];
 for(let i=0;i<10;i++){
  const start=performance.now();
  const result=await run('window.__TAURI_INTERNALS__.invoke("query_catalog",{query:{text:"বই",page:0}})');
  assert.equal(result.total,10001);assert.equal(result.books.length,50);
  times.push(performance.now()-start);
 }
 await run('location.hash="#/library"');await delay(600);
 assert.ok(await run('document.querySelector(\'ul[aria-label="Catalog books"]\')?.childElementCount>0'));
 const memory=[];
 for(let i=0;i<5;i++){
  const pid=app.pid;
  const command='$all=Get-CimInstance Win32_Process;$ids=[System.Collections.Generic.List[int]]::new();$ids.Add('+pid+');do{$added=$false;foreach($item in $all){if($ids.Contains([int]$item.ParentProcessId)-and -not $ids.Contains([int]$item.ProcessId)){$ids.Add([int]$item.ProcessId);$added=$true}}}while($added);$sum=0L;foreach($id in $ids){$p=Get-Process -Id $id -ErrorAction SilentlyContinue;if($p){$sum+=$p.WorkingSet64}};$sum';
  const result=spawnSync("powershell.exe",["-NoProfile","-Command",command],{encoding:"utf8",windowsHide:true});
  assert.equal(result.status,0,result.stderr);memory.push(Number(result.stdout.trim()));await delay(200);
 }
 const backup=await run('window.__TAURI_INTERNALS__.invoke("create_backup",{includeThumbnails:false,chooseDestination:false})');
 const manifest=JSON.parse(await readFile(resolve(backup.path,"manifest.json"),"utf8"));assert.equal(manifest.database.books,10001);
 const report={profile,scope:"Embedded-frontend debug native app, real SQLite 10001 records, idle catalog with parsing queues removed; working set includes app and descendant WebView2 processes",initialUiMs,searchMaxMs:Math.max(...times),searchTimesMs:times,workingSetSamplesBytes:memory,maxWorkingSetMiB:Math.max(...memory)/1048576,backupBooks:manifest.database.books,budgets:{initialUiMs:3000,warmSearchMs:500,normalMemoryMiB:500}};
 await writeFile("src-tauri/target/performance-native.json",JSON.stringify(report,null,2));console.log(JSON.stringify(report,null,2));
 assert.ok(initialUiMs<=3000,"Native initial UI budget");assert.ok(Math.max(...times)<=500,"Warm search budget");assert.ok(Math.max(...memory)/1048576<=500,"Normal library memory budget");
 await run('window.__TAURI_INTERNALS__.invoke("plugin:window|destroy",{label:"main"})').catch(()=>{});
}finally{
 socket?.close();
 if(app.exitCode===null)app.kill();
}
