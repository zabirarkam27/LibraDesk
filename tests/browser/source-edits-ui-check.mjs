import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { once } from "node:events";
import { mkdtemp, readFile, realpath, rm } from "node:fs/promises";
import { createServer } from "node:http";
import { basename, dirname, extname, join, resolve } from "node:path";
import { setTimeout as delay } from "node:timers/promises";

const fixture = await readFile(new URL('./source-edits-fixture.js', import.meta.url), 'utf8');

const root = resolve("dist");
const server = createServer(async (req, res) => {
  try {
    const path = new URL(req.url, "http://localhost").pathname;
    if (path === "/fixture.js") {
      res.writeHead(200, { "Content-Type": "text/javascript; charset=utf-8" }).end(fixture);
      return;
    }
    const file = path === "/" ? "index.html" : path.slice(1);
    const mime = { ".html": "text/html", ".js": "text/javascript", ".mjs": "text/javascript", ".css": "text/css" };
    let contents = await readFile(join(root, file));
    if (file === "index.html") {
      contents = contents.toString().replace("<head>", '<head><script src="/fixture.js"></script>');
    }
    res.writeHead(200, { "Content-Type": mime[extname(file)] || "application/octet-stream" }).end(contents);
  } catch {
    res.writeHead(404).end();
  }
});
server.listen(0, "127.0.0.1");
await once(server, "listening");
const base = "http://127.0.0.1:" + server.address().port;
const profile = await mkdtemp(resolve("src-tauri/target/catalog-browser-"));
const chrome = spawn("C:/Program Files/Google/Chrome/Application/chrome.exe", [
  "--headless", "--disable-gpu", "--disable-background-networking", "--no-first-run",
  "--remote-debugging-port=0", "--user-data-dir=" + profile, "about:blank",
], { windowsHide: true, stdio: ["ignore", "ignore", "pipe"] });
chrome.stderr.on("data", (data) => {
  if (process.env.CATALOG_BROWSER_LOG) process.stderr.write(data);
});

async function until(fn) {
  for (let i = 0; i < 150; i++) {
    const value = await fn();
    if (value) return value;
    await delay(100);
  }
  throw new Error("Timed out waiting for " + fn);
}

let socket;
try {
  const port = await until(async () => (await readFile(join(profile, "DevToolsActivePort"), "utf8").catch(() => "")).split("\n")[0]);
  const targets = await (await fetch("http://127.0.0.1:" + port + "/json/list")).json();
  socket = new WebSocket(targets.find(target => target.type === "page").webSocketDebuggerUrl);
  await once(socket, "open");
  let id = 0;
  const pending = new Map();
  socket.addEventListener("message", event => {
    const message = JSON.parse(event.data);
    if (message.method === "Runtime.exceptionThrown") console.error(message.params.exceptionDetails);
    const { id, result, error } = JSON.parse(event.data);
    if (!pending.has(id)) return;
    const { resolve, reject } = pending.get(id);
    pending.delete(id);
    if (error) reject(error);
    else resolve(result);
  });
  function cdp(method, params = {}) {
    return new Promise((resolve, reject) => {
      pending.set(++id, { resolve, reject });
      socket.send(JSON.stringify({ id, method, params }));
    });
  }
  async function run(expression) {
    const { result, exceptionDetails } = await cdp("Runtime.evaluate", { expression, returnByValue: true });
    if (exceptionDetails) throw new Error(exceptionDetails.exception?.description || exceptionDetails.text);
    return result.value;
  }

  const metrics = (width, height = 900) => cdp("Emulation.setDeviceMetricsOverride", { width, height, deviceScaleFactor: 1, mobile: false });
  const nav = (path = "/library", mode = "") => cdp("Page.navigate", { url: base + "/?" + (mode ? "case=" + mode : "") + "#" + path });
  const check = async (expression) => {
    try { return await until(() => run(expression).catch(() => false)); }
    catch { throw new Error(expression + "\n" + await run("document.body.innerText")); }
  };
  const button = label => 'Array.from((document.querySelector("[role=dialog]") || document).querySelectorAll("button")).find(b => b.textContent.trim() === ' + JSON.stringify(label) + ')';
  const click = async label => {
    await check("!!" + button(label));
    return run(button(label) + ".click()");
  };
  const field = label => 'Array.from(document.querySelectorAll("label")).find(l => l.textContent.trim().startsWith(' + JSON.stringify(label) + '))?.querySelector("input,textarea,select")';
  async function fill(selector, value) {
    await run(`(() => {
      const input = ${selector};
      const prototype = input.tagName === "TEXTAREA" ? HTMLTextAreaElement.prototype : input.tagName === "SELECT" ? HTMLSelectElement.prototype : HTMLInputElement.prototype;
      Object.getOwnPropertyDescriptor(prototype, "value").set.call(input, ${JSON.stringify(value)});
      input.dispatchEvent(new Event(input.tagName === "SELECT" ? "change" : "input", { bubbles: true }));
    })()`);
  }

  await metrics(1140);await nav("/library/10000");
  await check('document.body.innerText.includes("PDF metadata sync")&&document.body.innerText.includes("No sync requested")');
  await click("Edit book");await check('document.querySelector("[role=dialog]").innerText.includes("without a safety backup")');
  await fill(field("Genre"),"বাংলা উপন্যাস");await fill(field("Title"),"Updated বাংলা title");await click("Save book");
  await check('document.body.innerText.includes("Updated বাংলা title")&&document.body.innerText.includes("Queued")');
  assert.equal(await run('fixture.records[0].genre'),"বাংলা উপন্যাস");
  await run('fixture.pdfStatus="failed";fixture.publish()');await check('document.body.innerText.includes("PDF save failed")');
  await click("Retry PDF save");await check('document.body.innerText.includes("Saved to PDF")');
  await run('fixture.pdfStatus="queued";fixture.publish()');await check('document.body.innerText.includes("Stop sync")');
  await click("Stop sync");await check('document.body.innerText.includes("No sync requested")');
  await run('location.hash="#/library"');await check('document.body.innerText.includes("All books")');
  await run('fixture.publishPdfFailure()');await check('document.body.innerText.includes("PDF metadata was not saved")&&document.body.innerText.includes("Fixture sync failure")');
  await click("View book");await check('document.body.innerText.includes("PDF metadata sync")');
  const preview='Array.from(document.images).find(i=>i.alt==="Selected custom thumbnail")';
  async function selectImage(invalid=false){
    const expression='(()=>{const dt=new DataTransfer();if('+invalid+'){dt.items.add(new File(["bad"],"not-image.txt",{type:"text/plain"}));}else{const canvas=document.createElement("canvas");canvas.width=1200;canvas.height=800;const context=canvas.getContext("2d");context.fillStyle="#d47440";context.fillRect(0,0,1200,800);const bytes=Uint8Array.from(atob(canvas.toDataURL("image/jpeg").split(",")[1]),c=>c.charCodeAt(0));dt.items.add(new File([bytes],"my-cover.jpg",{type:"image/jpeg"}));}const input=document.querySelector("input[type=file]");input.files=dt.files;input.dispatchEvent(new Event("change",{bubbles:true}));})()';
    await run(expression);
  }
  await click("Change thumbnail");await selectImage(true);
  await check('document.querySelector("[role=dialog]").innerText.includes("Choose a PNG, JPEG or WebP")');
  await selectImage();await check("!!"+preview);
  await check(preview+'.naturalWidth>0');assert.ok(await run(preview+'.naturalWidth<=240'));
  await click("Cancel");assert.equal(await run("fixture.coverChanges.length"),0);
  await click("Change thumbnail");await selectImage();await check("!!"+preview);
  await run('fixture.coverError=true');await click("Save thumbnail");await check('document.body.innerText.includes("Thumbnail storage unavailable")');
  await run('fixture.coverError=false');await click("Save thumbnail");
  await check('!document.querySelector("[role=dialog]")&&fixture.coverChanges.length===1');
  await check('Array.from(document.images).some(i=>i.naturalWidth===240&&i.naturalHeight===160)');
  assert.ok(await run('fixture.coverChanges[0].png.length>0&&fixture.coverChanges[0].png.length<=524288'));
  await cdp("Page.reload");await check('document.body.innerText.includes("Change thumbnail")');
  await check('Array.from(document.images).some(i=>i.naturalWidth===240&&i.naturalHeight===160)');
  await click("Change thumbnail");await check('document.body.innerText.includes("Use automatic thumbnail")');
  await click("Use automatic thumbnail");await check('!document.querySelector("[role=dialog]")&&fixture.manualRevision===null');
  await check('Array.from(document.images).some(i=>i.naturalWidth===1)');
  assert.deepEqual(await run("fixture.failures"),[]);
  console.log("Genre edit, no-backup disclosure, PDF queued/failed/retry/stop status; JPEG normalization, preview/cancel/error, manual cover save/reload/reset: passed");

} finally {
  socket?.close();
  chrome.kill();
  server.close();
  const resolvedProfile = await realpath(profile);
  if (dirname(resolvedProfile) === await realpath("src-tauri/target") && basename(resolvedProfile).startsWith("catalog-browser-")) {
    await rm(resolvedProfile, { recursive: true, force: true, maxRetries: 6, retryDelay: 200 }).catch(() => {});
  }
}
