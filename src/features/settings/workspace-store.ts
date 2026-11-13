import { invoke } from "@tauri-apps/api/core";
import { create } from "zustand";
import { desktop,errorMessage } from "@/shared/lib/preferences-client";
import { saveReaderPreferences,type ReaderPreferences } from "@/features/reader/reader-client";

export type WorkspacePreferences = { startPage:"overview"|"library"; libraryView:"grid"|"list"|"compact" };
export type SettingsSnapshot = { workspace:WorkspacePreferences; reader:ReaderPreferences; dataPath:string; version:string };
const initial:SettingsSnapshot = {
  workspace:{startPage:"overview",libraryView:"grid"},
  reader:{zoom:"fit_width",scale:100,layout:"continuous",fullscreen:false,shortcuts:true},
  dataPath:"Browser storage",version:"Preview",
};
const previewKey="libradesk.workspace.v1";
function preview():SettingsSnapshot {
  try {
    const saved=JSON.parse(localStorage.getItem(previewKey)??"{}") as Partial<SettingsSnapshot>;
    return {...initial,workspace:{
      startPage:saved.workspace?.startPage==="library"?"library":"overview",
      libraryView:saved.workspace?.libraryView==="list"||saved.workspace?.libraryView==="compact"?saved.workspace.libraryView:"grid",
    }};
  } catch{return initial;}
}
type Store = {
  snapshot:SettingsSnapshot; status:"idle"|"loading"|"ready"|"error"; error:string|null; saving:boolean;
  load:(force?:boolean)=>Promise<void>;
  updateWorkspace:(patch:Partial<WorkspacePreferences>)=>Promise<void>;
  updateReader:(reader:ReaderPreferences)=>Promise<void>;
};
export const useWorkspaceSettings=create<Store>((set,get)=>({
  snapshot:initial,status:"idle",error:null,saving:false,
  async load(force=false) {
    const current=get();
    if(current.status==="loading"||current.saving||!force&&current.status==="ready")return;
    set({status:"loading",error:null});
    try{
      const snapshot=desktop?await invoke<SettingsSnapshot>("get_settings_snapshot"):preview();
      set({snapshot,status:"ready"});
    }catch(reason){set({status:"error",error:errorMessage(reason)});}
  },
  async updateWorkspace(patch) {
    const {snapshot,status,saving}=get();if(status!=="ready"||saving)return;
    const workspace={...snapshot.workspace,...patch};
    const next={...snapshot,workspace};set({snapshot:next,saving:true});
    try{
      if(desktop)await invoke("save_workspace_preferences",{input:workspace});
      else localStorage.setItem(previewKey,JSON.stringify(next));
    }catch(reason){set({snapshot});throw reason;}
    finally{set({saving:false});}
  },
  async updateReader(reader) {
    const {snapshot,status,saving}=get();if(status!=="ready"||saving||!desktop)return;
    set({snapshot:{...snapshot,reader},saving:true});
    try{await saveReaderPreferences(reader);}
    catch(reason){set({snapshot});throw reason;}
    finally{set({saving:false});}
  },
}));
