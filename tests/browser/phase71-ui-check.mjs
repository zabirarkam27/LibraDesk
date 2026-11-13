import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { once } from "node:events";
import { mkdtemp, readFile, realpath, rm, open, stat } from "node:fs/promises";
import { createServer } from "node:http";
import { basename, dirname, extname, join, resolve } from "node:path";
import { setTimeout as delay } from "node:timers/promises";

const fixture = await readFile(new URL('./phase71-fixture.js', import.meta.url), 'utf8');

const root = resolve("dist");
const server = createServer(async (req, res) => {
  try {
    const path = new URL(req.url, "http://localhost").pathname;
    if (path === "/fixture.js") {
      res.writeHead(200, { "Content-Type": "text/javascript; charset=utf-8" }).end(fixture);
      return;
    }

    if (path === "/fixture-meta" || path === "/fixture-range") {
      const query = new URL(req.url, "http://localhost").searchParams;
      const name = query.get("file");
      if (!["reader-fixture.pdf","reader-long.pdf","reader-large.pdf","reader-locked.pdf"].includes(name)) throw new Error("Invalid fixture");
      const fixturePath = resolve("src-tauri/target", name);
      if (path === "/fixture-meta") {
        res.writeHead(200, {"Content-Type":"application/json"}).end(JSON.stringify({length:(await stat(fixturePath)).size}));
      } else {
        const offset=Number(query.get("offset")), length=Number(query.get("length"));
        if (!Number.isSafeInteger(offset) || offset<0 || !Number.isSafeInteger(length) || length<1 || length>262144) throw new Error("Invalid range");
        const file = await open(fixturePath,"r");
        try {
          const bytes=Buffer.alloc(length);
          const {bytesRead}=await file.read(bytes,0,length,offset);
          res.writeHead(200,{"Content-Type":"application/octet-stream"}).end(bytes.subarray(0,bytesRead));
        } finally { await file.close(); }
      }
      return;
    }
    const file = path === "/" ? "index.html" : path.slice(1);
    const mime = { ".html": "text/html", ".js": "text/javascript", ".mjs": "text/javascript", ".css": "text/css", ".wasm": "application/wasm" };
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

  const metrics = (width, height = 920) => cdp("Emulation.setDeviceMetricsOverride", { width, height, deviceScaleFactor: 1, mobile: false });
  const nav = path => cdp("Page.navigate", { url: base + "/#" + path });
  const check = async expression => {
    try { return await until(() => run(expression).catch(() => false)); }
    catch { throw new Error(expression + "\n" + await run("document.body.innerText") + "\n" + await run("JSON.stringify(fixture.failures)")); }
  };
  const button = label => 'Array.from((document.querySelector("[role=dialog]") || document).querySelectorAll("button")).find(b => b.textContent.trim() === ' + JSON.stringify(label) + ')';
  const click = async label => { await check("!!" + button(label)); await run(button(label) + ".click()"); };
  const named = label => 'document.querySelector(' + JSON.stringify('[aria-label="' + label + '"]') + ')';
  const action = async label => { await check("!!" + named(label)); await run(named(label) + ".click()"); };
  const field = label => 'Array.from(document.querySelectorAll("label")).find(l => l.textContent.trim().startsWith(' + JSON.stringify(label) + '))?.querySelector("input,textarea,select")';
  async function fill(selector, value) {
    await run(`(() => {
      const input = ${selector};
      const type = input.tagName === "TEXTAREA" ? HTMLTextAreaElement.prototype : input.tagName === "SELECT" ? HTMLSelectElement.prototype : HTMLInputElement.prototype;
      Object.getOwnPropertyDescriptor(type, "value").set.call(input, ${JSON.stringify(value)});
      input.dispatchEvent(new Event(input.tagName === "SELECT" ? "change" : "input", { bubbles: true }));
    })()`);
  }
  async function ready(page = null) {
    await check('!!document.querySelector(".pdfViewer canvas") && !' + named("Next page") + '.disabled');
    await check('Array.from(fixture.sessions.values()).some(session => !session.closed)');
    if (page !== null) await check(named("Page number") + '.value === "' + page + '"');
  }


  await metrics(1360);
  await nav("/settings");
  await check('!!document.querySelector("select") && document.body.innerText.includes("Open at startup")');
  for(const label of ["General","Appearance","Library","Reader","Storage","Backup","Privacy"]){
   await click(label);
   await check('document.querySelector("#settings-section-title")?.textContent==='+JSON.stringify(label));
  }
  await click("Storage");
  await check('document.body.innerText.includes("SQLite connected") && document.body.innerText.includes("AppData")');
  await click("Backup");
  await check('document.body.innerText.includes("Backups are manual and are not encrypted.")');
  await click("Appearance");
  await run('document.querySelector(\'input[name="app-theme"]\').closest("fieldset").querySelectorAll("input")[1].click()');
  await check('document.documentElement.dataset.theme==="dark"');
  await click("General");
  await fill(field("Open at startup"),"library");
  await check('fixture.config.startPage==="library"');
  await click("Library");
  await fill(field("Default library view"),"compact");
  await check('fixture.config.libraryView==="compact"');
  await run('fixture.rejectSettings=true');
  await fill(field("Default library view"),"list");
  await check('document.body.innerText.includes("Save rejected")');
  assert.equal(await run(field("Default library view")+'.value'),"compact");
  await run('fixture.rejectSettings=false;location.hash="#/library"');
  await check('!!document.querySelector(\'[aria-label="Catalog view"]\')');
  assert.equal(await run(button("Compact")+'.getAttribute("aria-pressed")'),"true");
  await click("Grid");
  assert.equal(await run(button("Grid")+'.getAttribute("aria-pressed")'),"true");
  await cdp("Page.reload");
  await check('!!document.querySelector(\'[aria-label="Catalog view"]\')');
  await check(button("Compact")+'.getAttribute("aria-pressed")==="true"');
  console.log("Seven settings areas, independent app theme, workspace save rollback, library default persistence and per-page override");

  await run('location.hash="#/settings?section=reader"');
  await check('document.body.innerText.includes("Save reader defaults")');
  await fill(field("Zoom mode"),"custom");
  await fill(field("Custom zoom"),"175");
  await fill(field("Page layout"),"single");
  await run('Array.from(document.querySelectorAll("label")).find(l=>l.textContent.includes("Enable reader keyboard shortcuts")).querySelector("input").click()');
  await click("Save reader defaults");
  await check('fixture.preferences.scale===175 && fixture.preferences.layout==="single" && fixture.preferences.shortcuts===false');
  assert.equal(await run('fixture.appearance.app'),"dark");
  assert.equal(await run('fixture.appearance.reader'),"light");
  await cdp("Page.reload");
  await check(field("Custom zoom")+'.value==="175"');
  await run('location.hash="#/reader?book=1&file=1&page=7&revision=1"');
  await ready(7);
  assert.ok(await run('fixture.preferences.layout==="single"'));
  await run('location.hash="#/settings?section=reader"');
  await check(field("Custom zoom")+'.value==="175"');
  await run('fixture.preferences.scale=125;location.hash="#/library"');
  await check('!!document.querySelector(\'[aria-label="Catalog view"]\')');
  await run('location.hash="#/settings?section=reader"');
  await check(field("Custom zoom")+'.value==="125"');
  console.log("Reader defaults survive reload, apply to a real PDF, preserve deep links, and refresh after reader-side changes");

  await nav("/");
  await delay(350);
  assert.equal(await run('location.hash'),"#/");
  await cdp("Page.reload");
  await check('location.hash==="#/library"');
  await run('localStorage.setItem("settings-delay","1200")');
  await nav("/");
  await cdp("Page.reload");
  await check('!!window.fixture');
  await run('location.hash="#/settings?section=privacy"');
  await check('document.body.innerText.includes("Your library is local")');
  assert.equal(await run('location.hash'),"#/settings?section=privacy");
  await run('localStorage.removeItem("settings-delay")');
  await run('fixture.settingsError=true;location.hash="#/library"');
  await check('!!document.querySelector(\'[aria-label="Catalog view"]\')');
  await run('location.hash="#/settings"');
  await check('document.body.innerText.includes("Settings unavailable")');
  await run('fixture.settingsError=false');
  await click("Reload settings");
  await check('document.body.innerText.includes("Open at startup")');
  await metrics(680);
  await click("Reader");
  await check('document.body.innerText.includes("Save reader defaults")');
  assert.ok(await run('document.documentElement.scrollWidth<=innerWidth'));
  await fill(field("Custom zoom"), "0");
  assert.equal(await run(button("Save reader defaults")+'.disabled'),true);
  await fill(field("Zoom mode"),"fit_width");
  assert.equal(await run(field("Custom zoom")+'.value'),"100");
  assert.deepEqual(await run("fixture.failures"),[]);
  console.log("Startup page, error/retry, compact layout, and zero unhandled browser errors");

} finally {
  socket?.close();
  chrome.kill();
  server.close();
  const resolvedProfile = await realpath(profile);
  if (dirname(resolvedProfile) === await realpath("src-tauri/target") && basename(resolvedProfile).startsWith("catalog-browser-")) {
    await rm(resolvedProfile, { recursive: true, force: true, maxRetries: 6, retryDelay: 200 }).catch(() => {});
  }
}
