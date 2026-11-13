let paused=false;
let active=0;
const waiters=new Set<()=>void>();
export const backgroundPaused=()=>paused;
export function beginBackgroundWork(){
  if(paused)return null;active++;
  let ended=false;
  return ()=>{if(ended)return;ended=true;active--;if(active===0)for(const done of waiters)done();};
}
export async function pauseBackgroundWork(){
  paused=true;if(active===0)return;
  let finish!:()=>void;
  let timer:number|undefined;
  try{
    await new Promise<void>((resolve,reject)=>{
      finish=resolve;waiters.add(finish);
      timer=window.setTimeout(()=>reject(new Error("Background PDF work did not stop in time. Close the reader and retry restore.")),60_000);
    });
  }finally{window.clearTimeout(timer);waiters.delete(finish);}
}
export function resumeBackgroundWork(){paused=false;}
