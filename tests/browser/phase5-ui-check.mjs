import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { once } from "node:events";
import { mkdtemp, readFile, realpath, rm, open, stat } from "node:fs/promises";
import { createServer } from "node:http";
import { basename, dirname, extname, join, resolve } from "node:path";
import { setTimeout as delay } from "node:timers/promises";

const fixture = await readFile(new URL('./phase5-fixture.js', import.meta.url), 'utf8');

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
  await nav("/library/1");
  await check('!!document.querySelector(\'a[href="#/reader?book=1&file=1"]\')');
  await run('document.querySelector(\'a[href="#/reader?book=1&file=1"]\').click()');
  await ready(1);
  await check('fixture.progress[1]?.total === 30');
  assert.ok(await run('fixture.ranges.every(range => range.length <= 262144)'));
  await action("Next page");
  await check(named("Page number") + '.value === "2"');
  await action("Last page");
  await check(named("Page number") + '.value === "30"');
  await action("First page");
  await check(named("Page number") + '.value === "1"');
  await fill(named("Page number"), "7");
  await run(named("Page number") + '.form.requestSubmit()');
  await check(named("Page number") + '.value === "7"');
  await fill(named("Page number"), "10");
  await run(named("Page number") + '.form.requestSubmit()');
  await check('(() => { const c = document.querySelector(\'.page[data-page-number="10"] canvas\'); if (!c || !c.width) return false; return c.getContext("2d").getImageData(Math.floor(c.width/2), Math.floor(c.height/2), 1, 1).data[0] < 250; })()');
  console.log("Multi-chunk image range rendered");
  await action("Zoom in");
  await check('fixture.preferences.zoom === "custom"');
  await fill('document.querySelector("#reader-zoom")', "fit_page");
  await check('fixture.preferences.zoom === "fit_page"');
  await action("Toggle left sidebar");
  await check('document.body.innerText.includes("Chapter one")');
  await click("Chapter two");
  await check(named("Page number") + '.value === "5"');
  await action("Expand Chapter one");
  await click("Nested section");
  await check(named("Page number") + '.value === "3"');
  await click("Pages");
  await check('document.querySelectorAll(\'[aria-label="Page thumbnails"] canvas\').length > 0');
  assert.ok(await run('document.querySelectorAll(\'[aria-label="Page thumbnails"] canvas\').length < 12'));
  await action("Go to page 2");
  await check(named("Page number") + '.value === "2"');
  await fill(named("Find in PDF"), "Needle");
  await action("Search PDF");
  await check('document.body.innerText.includes("of 30 matches")');
  await check('document.querySelectorAll(".textLayer .highlight").length > 0');
  const originalMatch = Number(await run('document.body.innerText.match(/(\\d+) of 30 matches/)?.[1]'));
  await action("Next match");
  await check('document.body.innerText.includes("' + (originalMatch % 30 + 1) + ' of 30 matches")');
  await action("Previous match");
  await check('document.body.innerText.includes("' + originalMatch + ' of 30 matches")');
  await fill(named("Find in PDF"), "বাংলা");
  await action("Search PDF");
  await check('document.body.innerText.includes("of 30 matches")');
  await check('Array.from(document.querySelectorAll(".textLayer .highlight")).map(element => element.textContent).join("").includes("বাংলা")');
  await fill(named("Find in PDF"), "no-such-match-xxx");
  await action("Search PDF");
  await check('document.body.innerText.includes("0 of 0 matches")');
  console.log("Real PDF canvas, text layer, page navigation, zoom, outline, lazy thumbnails, English/Bengali search and highlights verified");

  await action("First page");
  await check(named("Page number") + '.value === "1"');
  await action("Toggle right sidebar");
  await check('document.body.innerText.includes("Add bookmark")');
  await check('!' + button("Add bookmark") + '.disabled');
  await click("Add bookmark");
  await check('!!document.querySelector("[role=dialog]")');
  await fill('document.querySelector("[role=dialog] textarea")', "Saved chapter");
  await click("Save");
  await check('!document.querySelector("[role=dialog]") && document.body.innerText.includes("Saved chapter")');
  await click("Notes");
  await click("Add note");
  await check('!!document.querySelector("[role=dialog]")');
  await fill('document.querySelector("[role=dialog] textarea")', "বাংলা personal note");
  await click("Save");
  await check('!document.querySelector("[role=dialog]") && document.body.innerText.includes("বাংলা personal note")');
  await click("Edit");
  await check('!!document.querySelector("[role=dialog]")');
  await fill('document.querySelector("[role=dialog] textarea")', "Edited personal note");
  await click("Save");
  await check('!document.querySelector("[role=dialog]") && document.body.innerText.includes("Edited personal note")');
  await click("Delete");
  await check('!!document.querySelector("[role=dialog]")');
  await click("Delete");
  await check('!document.querySelector("[role=dialog]") && fixture.annotations.notes.length === 0');
  await click("Settings");
  await fill(field("Page layout"), "single");
  await check('fixture.preferences.layout === "single"');
  await fill(field("Reader appearance"), "dark");
  await check(named("PDF reader") + '.dataset.theme === "dark"');
  await action("Toggle fullscreen");
  await check('fixture.fullscreen === true && fixture.preferences.fullscreen === true');
  assert.equal(await run('document.querySelector(\'[aria-label="PDF reader"]\').getBoundingClientRect().width === innerWidth'), true);
  assert.equal(await run('document.querySelector(\'[aria-label="PDF reader"]\').getBoundingClientRect().height === innerHeight'), true);
  await action("Toggle fullscreen");
  await check('fixture.fullscreen === false');
  await action("Last page");
  await check(named("Page number") + '.value === "30"');
  await run('location.hash = "#/library"');
  await check('Array.from(fixture.sessions.values()).every(session => session.closed)');
  await check('fixture.progress[1]?.page === 30 && fixture.progress[1]?.close');
  await run('location.hash = "#/reader?book=1&file=1"');
  await check('!!document.querySelector(".pdfViewer canvas")');
  await check(named("Page number") + '.value === "30"');
  await action("Toggle right sidebar");
  await check('document.body.innerText.includes("Saved chapter")');
  await click("Page 1");
  await check(named("Page number") + '.value === "1"');
  await cdp("Input.dispatchKeyEvent", { type: "keyDown", key: "ArrowRight", code: "ArrowRight" });
  await cdp("Input.dispatchKeyEvent", { type: "keyUp", key: "ArrowRight", code: "ArrowRight" });
  await check(named("Page number") + '.value === "2"');
  console.log("Bookmarks, notes CRUD, preferences, fullscreen, final flush, auto resume, and keyboard navigation verified");
  await click("Close reader");
  await check('location.hash === "#/library/1" && fixture.progress[1]?.page === 2 && fixture.progress[1]?.close');

  await run('location.hash = "#/reader?book=2&file=2"');
  await ready(1);
  await check(named("Page number") + '.max === "2000"');
  for (const page of [1500, 30, 999, 4, 1999]) {
    await fill(named("Page number"), String(page));
    await run(named("Page number") + '.form.requestSubmit()');
  }
  await check(named("Page number") + '.value === "1999"');
  await check('!!document.querySelector(".pdfViewer canvas")');
  assert.ok(await run('document.querySelectorAll(".pdfViewer canvas").length') < 20);
  await action("Toggle left sidebar");
  await click("Pages");
  await check('document.querySelectorAll(\'[aria-label="Page thumbnails"] canvas\').length > 0');
  assert.ok(await run('document.querySelectorAll(\'[aria-label="Page thumbnails"] canvas\').length') < 12);
  await run('document.querySelector(\'[aria-label="Page thumbnails"]\').scrollTop = 1900 * 152');
  await check('!!document.querySelector(\'button[aria-label="Go to page 1901"]\')');
  await metrics(680, 700);
  await check('document.querySelector("main").scrollWidth === document.querySelector("main").clientWidth');
  await metrics(1360);
  console.log("2,000-page PDF, rapid navigation, bounded canvases and thumbnails, and 680px layout verified");

  await run('location.hash = "#/reader?book=3&file=3"');
  await ready(1);
  await check('fixture.ranges.some(range => range.offset > 256 * 1024 * 1024)');
  const received = await run('fixture.ranges.filter(range => fixture.sessions.get(range.session)?.book === 3).reduce((sum, range) => sum + range.length, 0)');
  assert.ok(received < 8 * 1024 * 1024, "Large PDF range bytes " + received);
  console.log("300 MiB source opened with", received, "range bytes");
  await run('location.hash = "#/reader?book=4&file=4"');
  await check('!!' + named("PDF password"));
  await fill(named("PDF password"), "wrong");
  await click("Unlock PDF");
  await check('document.body.innerText.includes("Incorrect password")');
  await fill(named("PDF password"), "readerpass");
  await click("Unlock PDF");
  await ready(1);
  console.log("Encrypted PDF password and retry verified");
  await run('fixture.failRange = true; location.hash = "#/reader?book=1&file=1"');
  await check('document.body.innerText.includes("Reader source unavailable")');
  await run('fixture.failRange = false');
  await click("Reopen PDF");
  await ready();
  console.log("Changed or unavailable source error and retry verified");

  const shot = await cdp("Page.captureScreenshot", { format: "png" });
  await (await import("node:fs/promises")).writeFile(resolve("src-tauri/target/phase5-reader.png"), Buffer.from(shot.data, "base64"));
  await run('fixture.failProgress = true; fixture.closeWindow()');
  await check('document.body.innerText.includes("Reading progress could not be saved")');
  assert.equal(await run("fixture.closed"), false);
  await run('fixture.failProgress = false; fixture.closeWindow()');
  await check("fixture.closed === true");
  assert.ok(await run('fixture.progress[1]?.close'));
  assert.deepEqual(await run("fixture.failures"), []);
  console.log("Window-close persistence, recoverable save failure, and zero unhandled browser errors verified");
  await run('location.hash = "#/library"');
  await until(async () => (await cdp("Target.getTargets")).targetInfos.filter(target => target.type === "worker").length === 0);
  await cdp("HeapProfiler.collectGarbage");
  console.log("PDF workers after leaving reader", (await cdp("Target.getTargets")).targetInfos.filter(target => target.type === "worker").length);
  console.log("Main-thread heap after cleanup", (await cdp("Runtime.getHeapUsage")).usedSize);

} finally {
  socket?.close();
  chrome.kill();
  server.close();
  const resolvedProfile = await realpath(profile);
  if (dirname(resolvedProfile) === await realpath("src-tauri/target") && basename(resolvedProfile).startsWith("catalog-browser-")) {
    await rm(resolvedProfile, { recursive: true, force: true, maxRetries: 6, retryDelay: 200 }).catch(() => {});
  }
}
