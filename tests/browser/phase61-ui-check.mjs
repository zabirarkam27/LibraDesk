import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { once } from "node:events";
import { mkdtemp, readFile, realpath, rm, open, stat } from "node:fs/promises";
import { createServer } from "node:http";
import { basename, dirname, extname, join, resolve } from "node:path";
import { setTimeout as delay } from "node:timers/promises";

const fixture = await readFile(new URL('./phase61-fixture.js', import.meta.url), 'utf8');

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
      if (!["reader-fixture.pdf","reader-long.pdf","reader-large.pdf","reader-locked.pdf","reader-blank.pdf","reader-text-limit.pdf","reader-invalid.pdf"].includes(name)) throw new Error("Invalid fixture");
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
    catch { throw new Error(expression + "\n" + await run("document.body.innerText") + "\n" + await run("JSON.stringify({failures:fixture.failures,jobs:extraction.jobs})")); }
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
  await run('extraction.add(1,"reader-fixture.pdf")');
  await check('extraction.jobs[1]?.status==="completed"');
  assert.equal(await run('Object.keys(extraction.pages[1]).length'),30);
  assert.ok(await run('extraction.pages[1][1].includes("Needle")'));
  assert.ok(await run('extraction.pages[1][1].includes("বাংলা")'));
  assert.ok(await run('extraction.batches.every(batch=>batch.length<=4)'));
  assert.ok(await run('extraction.ranges.every(range=>range.length<=262144)'));
  await check('document.body.innerText.includes("PDF text: Complete")');
  console.log("Real English/Bengali PDF: 30 ordered pages, bounded binary ranges, status complete");

  await run('extraction.add(2,"reader-locked.pdf")');
  await check('extraction.jobs[2]?.error==="password_required"');
  assert.equal(await run('Object.keys(extraction.pages[2]).length'),0);
  await run('location.hash="#/library/2"');
  await check('document.body.innerText.includes("Password-protected PDF")');
  await click("Retry text extraction");
  await check('extraction.jobs[2]?.lease===2 && extraction.jobs[2]?.status==="failed"');
  console.log("Encrypted PDF: explicit failure, no password prompt, manual retry");

  await run('extraction.stopAt=8; extraction.add(3,"reader-long.pdf")');
  await check('extraction.jobs[3]?.nextPage===9 && extraction.jobs[3]?.status==="queued"');
  assert.equal(await run('Object.keys(extraction.pages[3]).length'),8);
  await cdp("Page.reload");
  await check('!!window.extraction && !!document.querySelector("main")');
  await run('extraction.stopAt=40;extraction.enabled=true');
  await check('extraction.jobs[3]?.nextPage===41 && extraction.jobs[3]?.status==="queued"');
  assert.equal(await run('Object.keys(extraction.pages[3]).length'),40);
  assert.equal(await run('extraction.batches[0][0]'),9);
  assert.equal(await run('new Set(extraction.batches.flat()).size'),32);
  console.log("Reload resumed 2,000-page PDF at page 9; exactly 32 new pages, no duplicates");
  await until(async ()=>(await cdp("Target.getTargets")).targetInfos.filter(t=>t.type==="worker").length===0);

  await run('extraction.jobs[3].status="completed";extraction.forceHidden=false;extraction.enabled=false;location.hash="#/reader?book=1&file=1"');
  await ready(1);
  await run('extraction.add(4,"reader-fixture.pdf")');
  await delay(1800);
  assert.equal(await run('extraction.jobs[4].status'),"queued");
  await action("Next page");
  await check(named("Page number")+'.value==="2"');
  await run('extraction.forceHidden=true;location.hash="#/library/4"');
  await delay(1800);
  assert.equal(await run('extraction.jobs[4].status'),"queued");
  await run('extraction.forceHidden=false;extraction.stopAt=0');
  await check('extraction.jobs[4]?.status==="completed"');
  await check('document.body.innerText.includes("PDF text: Complete")');
  assert.deepEqual(await run("fixture.failures"),[]);
  console.log("Foreground reader and hidden window pause background extraction; navigation responsive; resumes when idle");

  await run('extraction.add(5,"reader-blank.pdf")');
  await check('extraction.jobs[5]?.status==="completed"');
  assert.equal(await run('extraction.pages[5][1]'),"");
  await run('extraction.add(6,"reader-text-limit.pdf")');
  await check('extraction.jobs[6]?.error==="page_text_limit"');
  assert.equal(await run('Object.keys(extraction.pages[6]).length'),0);
  await run('extraction.add(7,"reader-invalid.pdf")');
  await check('extraction.jobs[7]?.error==="invalid_pdf"');
  assert.deepEqual(await run("fixture.failures"),[]);
  console.log("Blank page retained; oversized page fails without truncation; corrupt PDF rejected; zero unhandled errors");

} finally {
  socket?.close();
  chrome.kill();
  server.close();
  const resolvedProfile = await realpath(profile);
  if (dirname(resolvedProfile) === await realpath("src-tauri/target") && basename(resolvedProfile).startsWith("catalog-browser-")) {
    await rm(resolvedProfile, { recursive: true, force: true, maxRetries: 6, retryDelay: 200 }).catch(() => {});
  }
}
