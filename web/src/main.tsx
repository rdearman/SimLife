import React, { useEffect, useState } from 'react';
import { createRoot } from 'react-dom/client';
import './style.css';

type World = { id: string; config: { name: string; seed: number; tick_duration_seconds: number }; state: 'created'|'running'|'paused'; tick: number; state_hash: string };
const base = (import.meta as any).env.VITE_API_URL ?? 'http://127.0.0.1:3000';
async function api<T>(path: string, init?: RequestInit): Promise<T> {
  const r = await fetch(`${base}/api/v1${path}`, { ...init, headers: { 'content-type': 'application/json', ...init?.headers } });
  const body = await r.json(); if (!r.ok) throw new Error(body.error ?? r.statusText); return body;
}
function App() {
  const [worlds,setWorlds]=useState<World[]>([]), [selected,setSelected]=useState<World|null>(null), [name,setName]=useState('New world'), [seed,setSeed]=useState(1), [error,setError]=useState('');
  const refresh=async()=>{const data=await api<World[]>('/worlds');setWorlds(data);if(selected){const current=data.find(w=>w.id===selected.id);if(current)setSelected(current)}};
  useEffect(()=>{refresh().catch(e=>setError(String(e)))},[]);
  useEffect(()=>{if(!selected)return;const ws=new WebSocket(`${base.replace(/^http/,'ws')}/api/v1/worlds/${selected.id}/stream`);ws.onmessage=e=>{const world=JSON.parse(e.data) as World;setSelected(world);setWorlds(old=>[world,...old.filter(w=>w.id!==world.id)])};ws.onerror=()=>setError('WebSocket connection failed');return()=>ws.close()},[selected?.id]);
  async function create(e:React.FormEvent){e.preventDefault();try{const w=await api<World>('/worlds',{method:'POST',body:JSON.stringify({name,seed,tick_duration_seconds:1})});setSelected(w);await refresh()}catch(e){setError(String(e))}}
  async function control(command:object){if(!selected)return;try{setSelected(await api<World>(`/worlds/${selected.id}/commands`,{method:'POST',body:JSON.stringify(command)}));setError('')}catch(e){setError(String(e))}}
  return <main><h1>SimLife</h1><section><h2>Worlds</h2><form onSubmit={create}><input value={name} onChange={e=>setName(e.target.value)} aria-label="World name"/><input type="number" value={seed} onChange={e=>setSeed(Number(e.target.value))} aria-label="Seed"/><button>Create world</button></form><ul>{worlds.map(w=><li key={w.id}><button onClick={()=>setSelected(w)}>{w.config.name} · {w.state} · tick {w.tick}</button></li>)}</ul></section>
  {selected&&<section><h2>{selected.config.name}</h2><p>Status: <b>{selected.state}</b> · Tick: <b>{selected.tick}</b></p><p className="hash">State hash: {selected.state_hash}</p><button onClick={()=>control({command:'start'})}>Start</button><button onClick={()=>control({command:'pause'})}>Pause</button><button onClick={()=>control({command:'resume'})}>Resume</button><button onClick={()=>control({command:'advance',ticks:1})}>Advance 1 tick</button></section>}{error&&<p className="error">{error}</p>}</main>
}
createRoot(document.getElementById('root')!).render(<React.StrictMode><App/></React.StrictMode>);
