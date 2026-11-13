import { useEffect,useRef } from "react";
import { useLocation,useNavigate } from "react-router";
import { useWorkspaceSettings } from "./workspace-store";
export function StartupRoute() {
  const location=useLocation();
  const navigate=useNavigate();
  const initial=useRef(location);
  const applied=useRef(false);
  const status=useWorkspaceSettings(state=>state.status);
  const startPage=useWorkspaceSettings(state=>state.snapshot.workspace.startPage);
  useEffect(()=>{
    if(location.key!==initial.current.key||location.pathname!==initial.current.pathname||location.search!==initial.current.search){
      applied.current=true;
    }
    if(applied.current||status!=="ready"&&status!=="error")return;
    applied.current=true;
    // Only the original root entry can redirect; late loads cannot override navigation or deep links.
    if(status==="ready"&&startPage==="library"&&initial.current.pathname==="/"&&!initial.current.search&&location.pathname==="/"&&!location.search){
      navigate("/library",{replace:true});
    }
  },[location,navigate,status,startPage]);
  return null;
}
