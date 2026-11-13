import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { once } from "node:events";
import { mkdtemp, readFile, realpath, rm } from "node:fs/promises";
import { createServer } from "node:http";
import { basename, dirname, extname, join, resolve } from "node:path";
import { setTimeout as delay } from "node:timers/promises";

const fixture = await readFile(new URL('./phase4-fixture.js', import.meta.url), 'utf8');

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
  const rows = 'document.querySelector(\'ul[aria-label="Catalog books"]\')';
  await cdp("Runtime.enable");
  await metrics(1140);
  await nav();
  await check('document.body.textContent.includes("Loading books")');
  await check(rows + '?.querySelector("h2")?.textContent === "Book 10000"');
  await check(rows + '?.querySelector("img")?.naturalWidth === 1');
  for (const mode of ["Grid", "List", "Compact"]) {
    await click(mode);
    await check(button(mode) + '.getAttribute("aria-pressed") === "true"');
    const count = await run(rows + ".childElementCount");
    assert.ok(count > 0 && count < 35, mode);
    await run('document.querySelector("main").scrollTop = document.querySelector("main").scrollHeight');
    await check(rows + '?.lastElementChild?.getAttribute("aria-posinset") === "50"');
    assert.ok(await run(rows + ".childElementCount") < 35, mode + " scrolled");
    console.log(mode + " virtualized");
  }
  await run('document.querySelector(\'button[aria-label="Next page"]\').click()');
  await check(rows + '?.querySelector("h2")?.textContent === "Book 9950"');
  await run('document.querySelector(\'button[aria-label="Previous page"]\').click()');
  await check(rows + '?.querySelector("h2")?.textContent === "Book 10000"');
  await click("Grid");
  await metrics(520);
  await check('getComputedStyle(' + rows + ').gridTemplateColumns.split(" ").length === 2');
  assert.equal(await run('document.querySelector("main").scrollWidth === document.querySelector("main").clientWidth'), true);
  await metrics(1140);
  await run('document.querySelector("main").scrollTop = 300');
  await run('document.querySelector(\'a[href="#/library/10000"]\').click()');
  await check('document.body.innerText.includes("9780000000002") && document.body.innerText.includes("Edit book")');
  assert.equal(await run('document.querySelector("main").scrollTop'), 0);
  await click("Edit book");
  await check('!!document.querySelector("[role=dialog]")');
  await fill(field("Title"), "Edited বাংলা book");
  await fill(field("Authors"), "New Bengali author\nSecond author");
  await fill(field("Reading status"), "reading");
  await run(field("Favorite") + ".click()");
  await click("Save book");
  await check('!document.querySelector("[role=dialog]") && document.body.innerText.includes("Edited বাংলা book")');
  await check('document.body.innerText.includes("Second author") && document.body.innerText.includes("Reading")');
  const saved = await run('fixture.writes.find(write => write.command === "update_book").args');
  assert.equal(saved.input.title, "Edited বাংলা book");
  assert.deepEqual(saved.input.authorNames, ["New Bengali author", "Second author"]);
  assert.equal(saved.input.isFavorite, true);
  assert.deepEqual(saved.input.categoryIds, [1]);
  await click("Edit book");
  await fill(field("Title"), "Cancelled change");
  await run('fixture.state = "refresh_error"; fixture.publish()');
  await check('!!document.querySelector("[role=dialog]") && document.body.textContent.includes("latest book data could not be loaded")');
  assert.equal(await run(field("Title") + ".value"), "Cancelled change");
  await run('fixture.state = "ready"');
  await click("Cancel");
  await check('!document.querySelector("[role=dialog]") && document.body.innerText.includes("Edited বাংলা book")');
  console.log("Book details, edit, authors, flags, assignments, cancellation verified");

  await run('location.hash = "#/organize"');
  await check('!!Array.from(document.querySelectorAll("button")).find(button => button.textContent === "Create category")');
  await click("Create category");
  await check('!!document.querySelector("[role=dialog]")');
  await fill(field("Name"), "Nested category");
  await fill(field("Parent category"), "1");
  await click("Save");
  await check('!document.querySelector("[role=dialog]") && document.body.innerText.includes("Nested category")');
  await run('document.querySelector(\'button[aria-label="Edit Nested category"]\').click()');
  await check('!!document.querySelector("[role=dialog]")');
  await fill(field("Name"), "Renamed category");
  await click("Save");
  await check('!document.querySelector("[role=dialog]") && document.body.innerText.includes("Renamed category")');
  await run('document.querySelector(\'button[aria-label="Delete Renamed category"]\').click()');
  await check('!!document.querySelector("[role=dialog]")');
  await click("Delete");
  await check('!document.querySelector("[role=dialog]") && !document.body.innerText.includes("Renamed category")');
  await click("Tags");
  await click("Create tag");
  await check('!!document.querySelector("[role=dialog]")');
  await fill(field("Name"), "Test tag");
  await click("Save");
  await check('!document.querySelector("[role=dialog]") && document.body.innerText.includes("Test tag")');
  await click("Collections");
  await click("Create collection");
  await check('!!document.querySelector("[role=dialog]")');
  await fill(field("Name"), "Weekend");
  await fill(field("Description"), "Two books this weekend");
  await fill(field("Collection order"), "title");
  await click("Save");
  await check('!document.querySelector("[role=dialog]") && document.body.innerText.includes("Weekend")');
  console.log("Category CRUD, parent choice, tag and collection creation verified");

  await run('location.hash = "#/collections/1"');
  await check('document.body.innerText.includes("Reading queue") && document.body.innerText.includes("Book 9999")');
  await run('document.querySelector(\'button[aria-label="Move Book 9999 up"]\').click()');
  await check('Array.from(document.querySelectorAll("main ul a"))[0]?.textContent === "Book 9999"');
  await run('document.querySelector(\'button[aria-label="Remove Book 9999"]\').click()');
  await check('!document.querySelector(\'button[aria-label="Remove Book 9999"]\')');
  assert.equal(await run('fixture.records.some(book => book.id === 9999)'), true);
  console.log("Collection reorder and removal verified");

  await run('location.hash = "#/library"');
  await check(rows + '?.querySelector("h2")?.textContent === "Edited বাংলা book"');
  await fill(field("Category"), "1");
  await check('fixture.requests.at(-1)?.categoryId === 1');
  await fill(field("Tag"), "1");
  await check('fixture.requests.at(-1)?.tagId === 1');
  await fill(field("Reading status"), "reading");
  await check('fixture.requests.at(-1)?.readingStatus === "reading" && !!document.querySelector(\'a[href="#/library/10000"]\')');
  await run(field("Favorites") + ".click()");
  await check('fixture.requests.at(-1)?.favoritesOnly === true');
  await fill(field("Sort by"), "year");
  await check('fixture.requests.at(-1)?.sort === "year"');
  await run('document.querySelector(\'button[aria-label="Remove Edited বাংলা book from favorites"]\').click()');
  await check('fixture.records[0].isFavorite === false');
  await check('document.body.innerText.includes("No matching books")');
  await run('location.hash = "#/library?q=বাংলা"');
  await check('fixture.requests.at(-1)?.text === "বাংলা" && !!document.querySelector(\'a[href="#/library/10000"]\')');
  console.log("Combined filters, search parameters, favorites toggle verified");

  await run('location.hash = "#/"');
  await check('document.body.innerText.includes("10,000") && document.body.innerText.includes("Recently read")');
  await check('document.body.innerText.includes("No books to show yet.")');
  const shot = await cdp("Page.captureScreenshot", { format: "png" });
  await (await import("node:fs/promises")).writeFile(resolve("src-tauri/target/phase4-dashboard.png"), Buffer.from(shot.data, "base64"));
  await cdp("Input.dispatchKeyEvent", { type: "keyDown", key: "k", code: "KeyK", modifiers: 2 });
  await cdp("Input.dispatchKeyEvent", { type: "keyUp", key: "k", code: "KeyK", modifiers: 2 });
  await check('!!document.querySelector("[role=dialog]")');
  await fill('document.querySelector(\'input[aria-label="Search terms"]\')', "বাংলা");
  await run('document.querySelector("[role=dialog] form").requestSubmit()');
  await check('location.hash.includes("q=") && !document.querySelector("[role=dialog]")');
  console.log("Database-backed dashboard presentation and Ctrl+K search flow verified");

  await nav("/library", "error");
  await check('document.querySelector("[role=alert]")?.textContent.includes("could not load")');
  await run('fixture.state = "ready"');
  await click("Try again");
  await check(rows + '?.querySelector("h2")?.textContent === "Book 10000"');
  await nav("/library", "empty");
  await check('document.body.innerText.includes("Your library is empty")');
  await nav("/library/999999");
  await check('document.querySelector("[role=alert]")?.textContent.includes("record may have been removed")');
  await nav("/library", "preview");
  await check('document.body.innerText.includes("Desktop library")');
  console.log("Retry, empty library, missing record, and browser preview verified");

} finally {
  socket?.close();
  chrome.kill();
  server.close();
  const resolvedProfile = await realpath(profile);
  if (dirname(resolvedProfile) === await realpath("src-tauri/target") && basename(resolvedProfile).startsWith("catalog-browser-")) {
    await rm(resolvedProfile, { recursive: true, force: true, maxRetries: 6, retryDelay: 200 }).catch(() => {});
  }
}
