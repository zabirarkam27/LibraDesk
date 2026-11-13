import {spawn} from "node:child_process";
import {resolve} from "node:path";
import {writeFile} from "node:fs/promises";
import {once} from "node:events";
import {setTimeout as delay} from "node:timers/promises";
import assert from "node:assert/strict";
const app=spawn(resolve(process.argv[2]),[],{windowsHide:true,stdio:"ignore",env:{...process.env,LIBRADESK_QA_DATA_DIR:resolve("src-tauri/target/release-must-ignore-qa-override"),WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS:"--remote-debugging-port=9338"}});
let socket;
try{
 let target;
 for(let i=0;i<100;i++){
  const list=await fetch("http://127.0.0.1:9338/json/list").then(r=>r.json()).catch(()=>[]);
  target=list.find(t=>t.type==="page"&&t.url.includes("tauri.localhost"));if(target)break;await delay(100);
 }
 assert.ok(target,"Installed production WebView");
 socket=new WebSocket(target.webSocketDebuggerUrl);await once(socket,"open");
 let serial=0;const pending=new Map();
 socket.addEventListener("message",event=>{const value=JSON.parse(event.data),task=pending.get(value.id);if(task){pending.delete(value.id);value.error?task.reject(value.error):task.resolve(value.result);}});
 socket.addEventListener("close",()=>{for(const task of pending.values())task.reject(Error("Window closed"));pending.clear();});
 const cdp=(method,params={})=>new Promise((resolve,reject)=>{const id=++serial;pending.set(id,{resolve,reject});socket.send(JSON.stringify({id,method,params}));});
 const run=async expression=>{const value=await cdp("Runtime.evaluate",{expression,returnByValue:true,awaitPromise:true});if(value.exceptionDetails)throw Error(value.exceptionDetails.exception?.description);return value.result.value;};
 for(let i=0;i<100;i++){if(await run('document.body.innerText.includes("Your reading room")'))break;await delay(100);}
 assert.ok(await run('document.body.innerText.includes("Your reading room")'));
 const settings=await run('window.__TAURI_INTERNALS__.invoke("get_settings_snapshot")');assert.equal(settings.version,"0.1.0");assert.ok(!settings.dataPath.includes("release-must-ignore"));
 await run('location.hash="#/settings?section=backup"');await delay(500);
 assert.ok(await run('document.body.innerText.includes("Restore a backup")'));
 await cdp("Network.enable");await cdp("Network.emulateNetworkConditions",{offline:true,latency:0,downloadThroughput:0,uploadThroughput:0});
 await run('location.hash="#/library"');await delay(500);assert.ok(await run('document.body.innerText.includes("Library")'));
 const image=await cdp("Page.captureScreenshot",{format:"png"});await writeFile("src-tauri/target/release-smoke.png",Buffer.from(image.data,"base64"));
 await writeFile("src-tauri/target/release-smoke.json",JSON.stringify({version:settings.version,productionProtocol:target.url,offlineNavigation:true,releaseIgnoresQaProfile:true,settingsBackupRestore:true},null,2));
 console.log("Installed release: production IPC, version, settings/restore, offline library navigation, debug-profile exclusion passed");
 await run('window.__TAURI_INTERNALS__.invoke("plugin:window|destroy",{label:"main"})').catch(()=>{});
}finally{socket?.close();if(app.exitCode===null)app.kill();}
