import {readFile} from "node:fs/promises";
import assert from "node:assert/strict";
const built=await readFile("src-tauri/target/release/libradesk.exe");
const installed=await readFile(process.argv[2]);
const marker=Buffer.from("__TAURI_BUNDLE_TYPE_VAR_UNK");
let offset=built.indexOf(marker);
if(offset<0){
 const suffix=Buffer.from("BUNDLE_TYPE_VAR_UNK");offset=built.indexOf(suffix);
 assert.ok(offset>=0,"Tauri bundle marker");
 assert.equal(built.indexOf(suffix,offset+1),-1);
 Buffer.from("BUNDLE_TYPE_VAR_NSS").copy(built,offset);
}else{
 assert.equal(built.indexOf(marker,offset+1),-1);
 Buffer.from("__TAURI_BUNDLE_TYPE_VAR_NSS").copy(built,offset);
}
assert.deepEqual(installed,built,"Installed binary must exactly match release bytes except Tauri NSIS marker");
console.log("Installed executable matches release with expected NSIS bundle marker");
