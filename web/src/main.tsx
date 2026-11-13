import React, { useEffect, useRef, useState } from 'react';
import { createRoot } from 'react-dom/client';
import './style.css';

type Tile = { terrain: string; resources: { vegetation: number; water: number } };
type Genome = { diet: 'herbivore'|'carnivore'|'omnivore'; speed: {first:number;second:number}; metabolism: {first:number;second:number}; fertility: {first:number;second:number}; lifespan_days:number; sex:'asexual'|'female'|'male' };
type Organism = { id: number; x: number; y: number; age_days: number; energy: number; genome: { diet: string; speed: { first: number; second: number }; metabolism: { first: number; second: number }; fertility: { first: number; second: number }; lifespan_days: number; sex: string } };
type OrganismInspection = { organism: Organism & { species_id:number; parent_ids:number[]; water_reserve:number }; lineage:{organism_id:number;parent_ids:number[];species_id:number;born_day:number;died_day:number|null;genome_at_birth:Organism['genome']}; offspring_ids:number[]; ancestry:Array<{organism_id:number;parent_ids:number[];species_id:number;born_day:number;died_day:number|null;genome_at_birth:Organism['genome']}> };
type Sample = { day:number; population:number; herbivores:number; carnivores:number; omnivores:number; average_speed:number; average_metabolism:number; average_fertility:number; births:number; deaths:number };
type AlleleSample = { day:number; species_id:number; population:number; speed:number[]; metabolism:number[]; fertility:number[] };
type SpeciationEvent = { sequence:number;day:number;parent_species_id:number;new_species_id:number;founder_organism_id:number;genetic_distance_basis_points:number;threshold_basis_points:number };
type Statistics = {births:number;deaths:number;predation_deaths:number;starvation_deaths?:number;dehydration_deaths?:number;age_deaths?:number;death_diagnostics_start_day?:number|null};
type ResourceSummary = {land_tiles:number;vegetation:number;water:number;depleted_land_tiles:number;dry_land_tiles:number};
type Summary = { id:string; config:{name:string;seed:number;size:string;tick_duration_ms:number}; state:'created'|'running'|'paused'; tick:number; width:number; height:number; population:number; species_count:number; statistics:Statistics; state_hash:string };
type World = Summary & { tiles:Tile[]; organisms:Organism[]; population_history:Sample[]; allele_frequency_history:AlleleSample[]; organism_lineage:Array<{organism_id:number;parent_ids:number[];born_day:number;died_day:number|null}>;species_lineages:Array<{species_id:number;parent_species_id:number|null;founder_organism_id:number;founded_day:number;extinct_day:number|null}>;speciation_events:SpeciationEvent[];mechanics_version:number;resource_summary:ResourceSummary;resource_history:Array<{day:number;summary:ResourceSummary}> };
type Update = Omit<Summary,'population'|'width'|'height'> & { organisms:Organism[]; population_sample:Sample|null; allele_frequency_samples?:AlleleSample[]; species_lineages?:World['species_lineages']; speciation_events?:SpeciationEvent[];allele_frequency_history?:AlleleSample[];resource_summary:ResourceSummary };
type ScenarioId = 'survival'|'predator_prey'|'evolution'|'recovery';
type RunStatus = 'ready'|'active'|'won'|'failed'|'abandoned';
type PlaytestAction = {at:string;day:number;action:string;details?:string};
type ScenarioRun = {worldId:string;scenario:ScenarioId;attempt:number;createdAt:string;startedAt?:string;completedAt?:string;elapsedSeconds?:number;status:RunStatus;userInterventions:number;pairedWorldId?:string;variant?:string;startingGenomes:Genome[];startingResources?:ResourceSummary;actions:PlaytestAction[]};
const base = (import.meta as any).env.VITE_API_URL ?? 'http://127.0.0.1:3000';
async function api<T>(path: string, init?: RequestInit): Promise<T> {
  const r = await fetch(`${base}/api/v1${path}`, { ...init, headers: { 'content-type': 'application/json', ...init?.headers } });
  const body = await r.json(); if (!r.ok) throw new Error(body.error ?? r.statusText); return body;
}
function MapView({world,onIntroduce}:{world:World;onIntroduce:(x:number,y:number)=>void}) {
  const ref=useRef<HTMLCanvasElement>(null);
  const [inspection,setInspection]=useState<OrganismInspection|null>(null);
  useEffect(()=>{const canvas=ref.current;if(!canvas)return;const ctx=canvas.getContext('2d');if(!ctx)return;
    const scale=Math.max(2,Math.min(8,640/world.width));canvas.width=world.width*scale;canvas.height=world.height*scale;
    const colors:Record<string,string>={ocean:'#25658a',lake:'#347fa1',river:'#438fb2',plains:'#a9b86a',forest:'#52764b',desert:'#d1ad68',mountain:'#888779'};
    world.tiles?.forEach((tile,i)=>{ctx.fillStyle=colors[tile.terrain]??'#888';ctx.fillRect((i%world.width)*scale,Math.floor(i/world.width)*scale,scale,scale)});
    ctx.fillStyle='#fa5947';for(const o of world.organisms??[]){ctx.beginPath();ctx.arc(o.x*scale+scale/2,o.y*scale+scale/2,Math.max(2,scale*.65),0,Math.PI*2);ctx.fill()}
  },[world.tiles,world.organisms,world.width,world.height]);
  async function click(e:React.MouseEvent<HTMLCanvasElement>){const r=e.currentTarget.getBoundingClientRect();const x=Math.min(world.width-1,Math.floor((e.clientX-r.left)/r.width*world.width));const y=Math.min(world.height-1,Math.floor((e.clientY-r.top)/r.height*world.height));const hit=world.organisms.map(o=>({o,d:(o.x-x)**2+(o.y-y)**2})).filter(v=>v.d<=2).sort((a,b)=>a.d-b.d)[0];if(hit){try{setInspection(await api<OrganismInspection>(`/worlds/${world.id}/organisms/${hit.o.id}`))}catch(e){console.error(e)}}else onIntroduce(x,y)}
  return <><canvas ref={ref} className="map" onClick={click} title="Click an organism to inspect it, or an empty tile to select an introduction point"/><div className="population-panel"><div className="map-heading"><h3>Allele-frequency history</h3><small>Per-lineage copy frequency sampled every 10 days</small></div><AlleleFrequencyChart samples={world.allele_frequency_history??[]}/></div>{world.speciation_events?.length>0&&<section className="inspection"><h3>Automatic lineage-branch events</h3>{world.speciation_events.slice(-20).reverse().map(event=><p key={event.sequence}>Day {event.day}: lineage #{event.new_species_id} split from #{event.parent_species_id} · {event.genetic_distance_basis_points} bp distance</p>)}</section>}{inspection&&<section className="inspection"><h3>Organism #{inspection.organism.id}</h3><p>Lineage #{inspection.lineage.species_id} · born day {inspection.lineage.born_day} · age {inspection.organism.age_days} · {inspection.organism.genome.sex} {inspection.organism.genome.diet}</p><p>Energy {inspection.organism.energy} · water {inspection.organism.water_reserve} · parents: {inspection.lineage.parent_ids.join(', ')||'introduced founder'}</p><p>Offspring IDs: {inspection.offspring_ids.join(', ')||'none recorded'}</p><p>Genome: speed [{inspection.organism.genome.speed.first}, {inspection.organism.genome.speed.second}], metabolism [{inspection.organism.genome.metabolism.first}, {inspection.organism.genome.metabolism.second}], fertility [{inspection.organism.genome.fertility.first}, {inspection.organism.genome.fertility.second}]</p><p>Ancestry, nearest first: {inspection.ancestry.map(a=>`#${a.organism_id} (day ${a.born_day})`).join(' → ')||'none'}</p><button className="secondary" onClick={()=>setInspection(null)}>Close inspection</button></section>}</>;
}
function PopulationChart({samples}:{samples:Sample[]}) {
  if(samples.length<2)return <p className="chart-empty">Population history will appear after day 10.</p>;
  const width=720,height=210,pad=28,max=Math.max(1,...samples.map(s=>s.population)),first=samples[0].day,last=samples[samples.length-1].day;
  const points=(key:'population'|'herbivores'|'carnivores'|'omnivores')=>samples.map((s,i)=>`${pad+(last===first?0:(s.day-first)/(last-first))*(width-pad*2)},${height-pad-(s[key]/max)*(height-pad*2)}`).join(' ');
  return <div className="chart-wrap"><svg className="chart" viewBox={`0 0 ${width} ${height}`} role="img" aria-label="Population history by diet"><line x1={pad} y1={height-pad} x2={width-pad} y2={height-pad} className="chart-axis"/><polyline points={points('population')} className="series total"/><polyline points={points('herbivores')} className="series herbivore"/><polyline points={points('carnivores')} className="series carnivore"/><polyline points={points('omnivores')} className="series omnivore"/><text x={pad} y={height-5}>{first.toLocaleString()} days</text><text x={width-pad} y={height-5} textAnchor="end">{last.toLocaleString()} days</text><text x={pad} y={14}>Population · max {max.toLocaleString()}</text></svg><div className="legend"><span className="total-key">Total</span><span className="herb-key">Herbivores</span><span className="carn-key">Carnivores</span><span className="omni-key">Omnivores</span></div></div>;
}
function AlleleFrequencyChart({samples}:{samples:AlleleSample[]}) {
  const [species,setSpecies]=useState<number|null>(null),[locus,setLocus]=useState<'speed'|'metabolism'|'fertility'>('speed'),[allele,setAllele]=useState(1);
  const allSpeciesIds=[...new Set(samples.map(s=>s.species_id))];const speciesIds=allSpeciesIds.slice(0,32);const selectedSpecies=speciesIds.includes(species??-1)?species:speciesIds[0];
  const rows=samples.filter(s=>s.species_id===selectedSpecies);if(rows.length<2)return <p className="chart-empty">Allele-frequency history will appear after day 10.</p>;
  const values=rows.map(s=>({day:s.day,frequency:(s[locus][allele]??0)/Math.max(1,s.population*2)}));const first=values[0].day,last=values[values.length-1].day;const w=720,h=170,p=28;
  const points=values.map(v=>`${p+(last===first?0:(v.day-first)/(last-first))*(w-p*2)},${h-p-v.frequency*(h-p*2)}`).join(' ');
  return <div className="chart-wrap">{allSpeciesIds.length>32&&<small>Showing the first 32 of {allSpeciesIds.length} recorded lineages; all lineage histories remain stored.</small>}<div className="gene-fields"><label>Lineage<select value={selectedSpecies??''} onChange={e=>setSpecies(Number(e.target.value))}>{speciesIds.map(id=><option key={id} value={id}>Lineage #{id}</option>)}</select></label><label>Locus<select value={locus} onChange={e=>{const next=e.target.value as typeof locus;setLocus(next);setAllele(Math.min(allele,next==='speed'?4:next==='metabolism'?20:100))}}><option value="speed">Speed</option><option value="metabolism">Metabolism</option><option value="fertility">Fertility</option></select></label><label>Allele value<input type="number" min="0" max={locus==='speed'?4:locus==='metabolism'?20:100} value={allele} onChange={e=>setAllele(Number(e.target.value))}/></label></div><svg className="chart allele-chart" viewBox={`0 0 ${w} ${h}`} role="img" aria-label={`${locus} allele ${allele} frequency`}><line x1={p} y1={h-p} x2={w-p} y2={h-p} className="chart-axis"/><polyline points={points} className="series allele-series"/><text x={p} y={h-5}>{first} days</text><text x={w-p} y={h-5} textAnchor="end">{last} days</text><text x={p} y={14}>Allele {allele} · frequency</text></svg></div>;
}
type ScenarioInfo = {title:string;summary:string;days:number;objective:string;hint:string};
const scenarioInfo:Record<ScenarioId,ScenarioInfo> = {
  survival:{title:'Survival',summary:'Balance energy use and movement to establish a population.',days:250,objective:'Keep at least 8 organisms alive through day 250.',hint:'A low metabolism spends less energy each day. Choose a habitat with water and vegetation.'},
  predator_prey:{title:'Predator–prey balance',summary:'Keep both diets in the food web while predation occurs.',days:300,objective:'At day 300, retain 5 herbivores and 1 carnivore, with at least one predation death.',hint:'Carnivores need prey. The map shows where founders move and meet.'},
  evolution:{title:'Evolutionary adaptation',summary:'Allow reproduction and mutation to create measurable variation.',days:400,objective:'Reach day 400 with 8 organisms, 20 births, and at least 2 distinct live genomes.',hint:'Fertility increases reproduction chance, but each birth costs the parent energy.'},
  recovery:{title:'Ecosystem recovery',summary:'Intervene after pressure on prey and recover a mixed population.',days:400,objective:'Introduce at least one organism, then retain 12 herbivores and 1 carnivore at day 400.',hint:'Watch deaths and vegetation. Add a founder from the editor when you think the food web needs support.'}
};
const editorGenome = (diet:Genome['diet'], speed:number, metabolism:number, fertility:number, lifespan:number):Genome => ({
  diet, speed:{first:speed,second:speed}, metabolism:{first:metabolism,second:metabolism},
  fertility:{first:fertility,second:fertility}, lifespan_days:lifespan, sex:'asexual'
});
function scenarioFounders(id:ScenarioId):Genome[] {
  if(id==='survival') return Array.from({length:8},()=>editorGenome('herbivore',1,0,20,1500));
  if(id==='predator_prey') return [
    ...Array.from({length:16},()=>editorGenome('herbivore',2,2,18,800)),
    ...Array.from({length:5},()=>editorGenome('carnivore',3,2,10,700))
  ];
  if(id==='evolution') return Array.from({length:10},()=>editorGenome('herbivore',1,3,40,1200));
  return [
    ...Array.from({length:12},()=>editorGenome('herbivore',1,3,16,900)),
    ...Array.from({length:8},()=>editorGenome('carnivore',3,2,12,700))
  ];
}
const variationOptions = [
  {id:'metabolism',label:'Higher metabolism (+5)'},
  {id:'speed',label:'Higher speed (+2)'},
  {id:'fertility',label:'Higher fertility (+20)'},
  {id:'omnivore',label:'Omnivore diet'}
];
function varyGenome(genome:Genome, variant:string):Genome {
  if(variant==='metabolism') { const value=Math.min(20,genome.metabolism.first+5); return {...genome,metabolism:{first:value,second:value}}; }
  if(variant==='speed') { const value=Math.min(4,genome.speed.first+2); return {...genome,speed:{first:value,second:value}}; }
  if(variant==='fertility') { const value=Math.min(100,genome.fertility.first+20); return {...genome,fertility:{first:value,second:value}}; }
  return genome.diet==='herbivore'?{...genome,diet:'omnivore'}:genome;
}
function resourcesFromTiles(tiles:Tile[]):ResourceSummary {
  const wet=new Set(['ocean','lake','river']);
  const land=tiles.filter(tile=>!wet.has(tile.terrain));
  return {
    land_tiles:land.length,
    vegetation:land.reduce((sum,tile)=>sum+tile.resources.vegetation,0),
    water:land.reduce((sum,tile)=>sum+tile.resources.water,0),
    depleted_land_tiles:land.filter(tile=>tile.resources.vegetation===0&&tile.resources.water===0).length,
    dry_land_tiles:land.filter(tile=>tile.resources.water===0).length
  };
}
function hydrateWorld(world:World,baseline?:ResourceSummary):World {
  const summary=resourcesFromTiles(world.tiles??[]);
  return {...world,resource_summary:summary,resource_history:[{day:baseline?0:world.tick,summary:baseline??summary}]};
}
function dietCount(world:World,diet:string):number {
  return world.organisms.filter(organism=>organism.genome.diet===diet).length;
}
function genomeDiversity(world:World):number {
  return new Set(world.organisms.map(o=>JSON.stringify(o.genome))).size;
}
function objectiveState(run:ScenarioRun,world:World):{ready:boolean;success:boolean;requirements:string[]} {
  const counts={herbivore:dietCount(world,'herbivore'),carnivore:dietCount(world,'carnivore')};
  if(run.scenario==='survival') {
    const ready=world.tick>=scenarioInfo.survival.days;
    return {ready,success:ready&&world.organisms.length>=8,requirements:['Population: '+world.organisms.length+' / 8 minimum']};
  }
  if(run.scenario==='predator_prey') {
    const ready=world.tick>=scenarioInfo.predator_prey.days;
    const success=ready&&counts.herbivore>=5&&counts.carnivore>=1&&(world.statistics.predation_deaths??0)>=1;
    return {ready,success,requirements:['Herbivores: '+counts.herbivore+' / 5 minimum','Carnivores: '+counts.carnivore+' / 1 minimum','Predation deaths: '+(world.statistics.predation_deaths??0)+' / 1 minimum']};
  }
  if(run.scenario==='evolution') {
    const diversity=genomeDiversity(world);
    const ready=world.tick>=scenarioInfo.evolution.days;
    const success=ready&&world.organisms.length>=8&&world.statistics.births>=20&&diversity>=2;
    return {ready,success,requirements:['Population: '+world.organisms.length+' / 8 minimum','Births: '+world.statistics.births+' / 20 minimum','Live genome variants: '+diversity+' / 2 minimum']};
  }
  const ready=world.tick>=scenarioInfo.recovery.days;
  const success=ready&&run.userInterventions>0&&counts.herbivore>=12&&counts.carnivore>=1;
  return {ready,success,requirements:['Player introductions: '+run.userInterventions+' / 1 minimum','Herbivores: '+counts.herbivore+' / 12 minimum','Carnivores: '+counts.carnivore+' / 1 minimum']};
}
function loadStored<T>(key:string,fallback:T):T {
  try { const raw=localStorage.getItem(key); return raw?JSON.parse(raw) as T:fallback; } catch { return fallback; }
}
function App() {
  const [worlds,setWorlds]=useState<Summary[]>([]);
  const [selected,setSelected]=useState<World|null>(null);
  const [runs,setRuns]=useState<ScenarioRun[]>(()=>loadStored('simlife.playtest.runs',[]));
  const runsRef=useRef(runs);
  const [compareId,setCompareId]=useState('');
  const [variant,setVariant]=useState('metabolism');
  const [seed,setSeed]=useState(20261113);
  const [name,setName]=useState('Sandbox world');
  const [size,setSize]=useState('small');
  const [tickMs,setTickMs]=useState(1000);
  const [error,setError]=useState('');
  const [connected,setConnected]=useState(false);
  const [tile,setTile]=useState<[number,number]|null>(null);
  const [diet,setDiet]=useState<Genome['diet']>('herbivore');
  const [sex,setSex]=useState<Genome['sex']>('asexual');
  const [speed,setSpeed]=useState(1);
  const [metabolism,setMetabolism]=useState(2);
  const [fertility,setFertility]=useState(20);
  const [lifespan,setLifespan]=useState(700);

  function writeRuns(next:ScenarioRun[]) {
    runsRef.current=next;
    setRuns(next);
    try { localStorage.setItem('simlife.playtest.runs',JSON.stringify(next)); } catch { /* local instrumentation remains optional */ }
  }
  function appendAction(worldId:string,action:string,day:number,details?:string) {
    const next=runsRef.current.map(run=>run.worldId===worldId?{...run,actions:[...run.actions,{at:new Date().toISOString(),day,action,details}]}:run);
    writeRuns(next);
  }
  function updateRun(worldId:string,edit:(run:ScenarioRun)=>ScenarioRun) {
    writeRuns(runsRef.current.map(run=>run.worldId===worldId?edit(run):run));
  }
  useEffect(()=>{runsRef.current=runs;},[runs]);

  useEffect(()=>{
    let closed=false;
    const refresh=async()=>{try{const list=await api<Summary[]>('/worlds');if(!closed)setWorlds(list);}catch(e){if(!closed)setError(String(e));}};
    refresh();
    const timer=window.setInterval(refresh,2500);
    return()=>{closed=true;window.clearInterval(timer);};
  },[]);

  useEffect(()=>{
    const onPageHide=()=>{
      const now=new Date().toISOString();
      const next=runsRef.current.map(run=>{
        if(run.status!=='ready'&&run.status!=='active')return run;
        return {...run,status:'abandoned' as RunStatus,completedAt:now,elapsedSeconds:run.startedAt?Math.round((Date.now()-Date.parse(run.startedAt))/1000):0,actions:[...run.actions,{at:now,day:selected?.tick??0,action:'abandoned',details:'Playtest page closed before objective completion.'}]};
      });
      runsRef.current=next;
      try{localStorage.setItem('simlife.playtest.runs',JSON.stringify(next));}catch{/* best effort */}
    };
    window.addEventListener('pagehide',onPageHide);
    return()=>window.removeEventListener('pagehide',onPageHide);
  },[selected?.tick]);

  useEffect(()=>{
    if(!selected)return;
    const socket=new WebSocket(base.replace(/^http/,'ws')+'/api/v1/worlds/'+selected.id+'/stream');
    socket.onopen=()=>setConnected(true);
    socket.onclose=()=>setConnected(false);
    socket.onmessage=event=>{
      const incoming=JSON.parse(event.data) as Update & Partial<World>;
      if(Array.isArray(incoming.tiles)) {
        const initial=hydrateWorld(incoming as World);
        setSelected(old=>old?.id===initial.id?{...initial,resource_history:old.resource_history?.length?old.resource_history:initial.resource_history}:old);
        return;
      }
      setSelected(old=>{
        if(!old||old.id!==incoming.id)return old;
        const history=old.population_history??[];
        const nextHistory=incoming.population_sample
          ?history[history.length-1]?.day===incoming.population_sample.day
            ?[...history.slice(0,-1),incoming.population_sample]
            :[...history,incoming.population_sample]
          :history;
        const additions=incoming.allele_frequency_samples??[];
        const alleles=[...(old.allele_frequency_history??[]).filter(a=>!additions.some(n=>n.day===a.day&&n.species_id===a.species_id)),...additions];
        const newEvents=incoming.speciation_events??[];
        const speciation=[...(old.speciation_events??[]),...newEvents].filter((item,index,all)=>all.findIndex(other=>other.sequence===item.sequence)===index);
        const resourceHistory=[...(old.resource_history??[]),{day:incoming.tick,summary:incoming.resource_summary}].filter((entry,index,all)=>all.findIndex(other=>other.day===entry.day)===index).slice(-1000);
        return {...old,...incoming,population:incoming.organisms.length,population_history:nextHistory,allele_frequency_history:alleles,speciation_events:speciation,species_lineages:incoming.species_lineages??old.species_lineages,resource_history:resourceHistory};
      });
      setWorlds(old=>old.map(world=>world.id===incoming.id?{...world,...incoming,population:incoming.organisms.length}:world));
    };
    socket.onerror=()=>{setConnected(false);setError('Live update connection failed. Check that the server is running.');};
    return()=>socket.close();
  },[selected?.id]);

  useEffect(()=>{
    if(!selected)return;
    const run=runs.find(item=>item.worldId===selected.id);
    if(!run||run.status==='won'||run.status==='failed'||run.status==='abandoned')return;
    const result=objectiveState(run,selected);
    if(!result.ready)return;
    const now=new Date().toISOString();
    const status:RunStatus=result.success?'won':'failed';
    updateRun(run.worldId,current=>({...current,status,completedAt:now,elapsedSeconds:current.startedAt?Math.round((Date.now()-Date.parse(current.startedAt))/1000):0,actions:[...current.actions,{at:now,day:selected.tick,action:status,details:result.requirements.join('; ')}]}));
  },[selected?.id,selected?.tick,selected?.organisms.length,selected?.statistics.births,runs]);

  async function refreshWorldList() { setWorlds(await api<Summary[]>('/worlds')); }
  async function createEmptyWorld(event:React.FormEvent) {
    event.preventDefault();
    try {
      const snapshot=await api<World>('/worlds',{method:'POST',body:JSON.stringify({name,seed,size,tick_duration_ms:tickMs})});
      setSelected(hydrateWorld(snapshot));setError('');appendAction(snapshot.id,'sandbox_created',0,'Empty sandbox world created.');await refreshWorldList();
    } catch(e) {setError(String(e));}
  }
  async function launchScenario(id:ScenarioId,sourceGenomes?:Genome[],pairedWorldId?:string,variantName?:string,worldSeed=seed,worldTickMs=tickMs) {
    const genomes=(sourceGenomes??scenarioFounders(id)).map(genome=>({...genome}));
    const attempt=runsRef.current.filter(run=>run.scenario===id).length+1;
    const label=scenarioInfo[id].title+(variantName?' · '+variantName:' · attempt '+attempt);
    try {
      let snapshot=await api<World>('/worlds',{method:'POST',body:JSON.stringify({name:label,seed:worldSeed,size:'small',tick_duration_ms:worldTickMs})});
      const [width,height]=[snapshot.width,snapshot.height];
      for(let i=0;i<genomes.length;i++) {
        const colocated=id==='predator_prey'||id==='recovery';
        const x=colocated?Math.floor(width/2):(i*7+3)%width;
        const y=colocated?Math.floor(height/2):(i*13+7)%height;
        snapshot=await api<World>('/worlds/'+snapshot.id+'/commands',{method:'POST',body:JSON.stringify({command:'introduce_organism',x,y,genome:genomes[i]})});
      }
      const now=new Date().toISOString();
      const record:ScenarioRun={worldId:snapshot.id,scenario:id,attempt,createdAt:now,status:'ready',userInterventions:0,pairedWorldId,variant:variantName,startingGenomes:genomes,startingResources:resourcesFromTiles(snapshot.tiles),actions:[{at:now,day:0,action:'scenario_created',details:label+'; '+genomes.length+' founders'}]};
      writeRuns([...runsRef.current,record]);
      setSelected(hydrateWorld(snapshot));setTile(null);setError('');
      if(pairedWorldId)setCompareId(pairedWorldId);
      await refreshWorldList();
    } catch(e) {setError('Could not launch scenario: '+String(e));}
  }
  async function createComparison() {
    const run=runs.find(item=>item.worldId===selected?.id);
    if(!selected||!run)return;
    const changed=run.startingGenomes.map(genome=>varyGenome(genome,variant));
    const option=variationOptions.find(item=>item.id===variant);
    await launchScenario(run.scenario,changed,selected.id,option?.label??variant,selected.config.seed,selected.config.tick_duration_ms);
  }
  async function selectWorld(summary:Summary) {
    try {
      const snapshot=await api<World>('/worlds/'+summary.id);
      const run=runsRef.current.find(item=>item.worldId===summary.id);
      setSelected(hydrateWorld(snapshot,run?.startingResources));setError('');setTile(null);
    } catch(e) {setError(String(e));}
  }
  async function control(command:Record<string,unknown>) {
    if(!selected)return;
    const action=String(command.command??'command');
    try {
      const snapshot=await api<World>('/worlds/'+selected.id+'/commands',{method:'POST',body:JSON.stringify(command)});
      setSelected({...hydrateWorld(snapshot,runsRef.current.find(run=>run.worldId===selected.id)?.startingResources),resource_history:selected.resource_history});
      if(action==='start'||action==='resume') {
        const now=new Date().toISOString();
        updateRun(snapshot.id,run=>({...run,status:'active',startedAt:run.startedAt??now,actions:[...run.actions,{at:now,day:snapshot.tick,action,details:'Simulation clock started.'}]}));
      } else if(action==='introduce_organism') {
        const details=JSON.stringify(command.genome);
        updateRun(snapshot.id,run=>({...run,userInterventions:run.userInterventions+1,actions:[...run.actions,{at:new Date().toISOString(),day:snapshot.tick,action:'introduced_organism',details}]}));
      } else appendAction(snapshot.id,action,snapshot.tick);
      await refreshWorldList();setError('');
    } catch(e) {setError(String(e));}
  }
  async function introduce() {
    if(!selected||!tile)return;
    await control({command:'introduce_organism',x:tile[0],y:tile[1],genome:{speed:{first:speed,second:speed},metabolism:{first:metabolism,second:metabolism},fertility:{first:fertility,second:fertility},lifespan_days:lifespan,diet,sex}});
  }
  function exportPlaytest() {
    const payload={exported_at:new Date().toISOString(),storage:'This log stays in this browser until exported.',runs:runsRef.current};
    const url=URL.createObjectURL(new Blob([JSON.stringify(payload,null,2)],{type:'application/json'}));
    const link=document.createElement('a');link.href=url;link.download='simlife-playtest-log.json';link.click();URL.revokeObjectURL(url);
  }

  const currentRun=runs.find(run=>run.worldId===selected?.id);
  const comparedRun=runs.find(run=>run.worldId===compareId);
  const objective=currentRun&&selected?objectiveState(currentRun,selected):null;
  const compared=worlds.find(world=>world.id===compareId);
  const initialPopulation=selected?.organism_lineage?.filter(record=>record.born_day===0).length??0;
  const laterIntroductions=selected?.organism_lineage?.filter(record=>record.born_day>0&&record.parent_ids.length===0).length??0;
  const netChange=selected?initialPopulation+laterIntroductions+selected.statistics.births-selected.statistics.deaths:0;
  const resource=selected?.resource_summary;
  const baselineResource=selected?.resource_history?.[0]?.summary;
  const vegetationRemaining=baselineResource&&baselineResource.vegetation>0&&resource?Math.round(resource.vegetation/baselineResource.vegetation*100):100;
  const resourceReference=currentRun?'scenario start':'reference level';
  const runsForScenario=currentRun?runs.filter(run=>run.scenario===currentRun.scenario):[];
  const openRuns=runs.filter(run=>run.status==='active'||run.status==='ready');
  const abandonedRuns=runs.filter(run=>run.status==='abandoned');

  return <main>
    <header><div><p className="eyebrow">SERVER-AUTHORITATIVE ECOLOGY · GUIDED PLAYTEST</p><h1>SimLife</h1></div><div className="header-status"><span className={'connection '+(connected?'online':'')}>{connected?'LIVE':'CONNECTING'}</span><span className="pill">DAY {selected?.tick??0}</span></div></header>
    <div className="layout">
      <aside className="panel sidebar">
        <section><p className="eyebrow">START HERE</p><h2>Choose a scenario</h2><p className="muted">Each has a clear target. Change the seed to try a different generated habitat.</p><label className="scenario-seed">Habitat seed<input type="number" min="1" value={seed} onChange={e=>setSeed(Number(e.target.value))}/></label>
          {(Object.keys(scenarioInfo) as ScenarioId[]).map(id=><article className="scenario-card" key={id}><div className="scenario-card-head"><h3>{scenarioInfo[id].title}</h3><span>{scenarioInfo[id].days} days</span></div><p>{scenarioInfo[id].summary}</p><button onClick={()=>launchScenario(id)}>Launch scenario</button></article>)}
        </section>
        <form onSubmit={createEmptyWorld} className="create sandbox-form"><h3>Free sandbox</h3><label>World name<input value={name} onChange={e=>setName(e.target.value)}/></label><label>Seed<input type="number" min="1" value={seed} onChange={e=>setSeed(Number(e.target.value))}/></label><label>Map size<select value={size} onChange={e=>setSize(e.target.value)}><option value="small">Small · 32 × 64</option><option value="medium">Medium · 64 × 128</option><option value="large">Large · 128 × 256</option></select></label><label>Tick interval<input type="number" min="10" max="60000" value={tickMs} onChange={e=>setTickMs(Number(e.target.value))}/></label><button className="secondary">Create empty world</button></form>
        <section className="world-section"><h3>Worlds</h3><ul className="world-list">{worlds.map(world=><li key={world.id}><button className={selected?.id===world.id?'selected':''} onClick={()=>selectWorld(world)}><b>{world.config.name}</b><small>{world.state} · day {world.tick} · {world.population} organisms</small></button></li>)}</ul></section>
        <section className="playtest-tools"><h3>Playtest record</h3><p>{runs.length} attempts · {runs.filter(run=>run.status==='won').length} complete · {abandonedRuns.length} abandoned · {openRuns.length} in progress</p><button className="secondary" onClick={exportPlaytest}>Export local playtest log</button><small>Actions and completion times stay in this browser. Nothing is sent to the server.</small></section>
      </aside>
      {selected?<section className="panel world">
        <div className="world-head"><div><p className="eyebrow">{currentRun?scenarioInfo[currentRun.scenario].title.toUpperCase():'SELECTED WORLD'}</p><h2>{selected.config.name}</h2>{currentRun&&<small>Attempt {currentRun.attempt}{currentRun.variant?' · '+currentRun.variant:''}</small>}</div><span className={'state '+selected.state}>{selected.state}</span></div>
        {currentRun&&objective&&<section className={'objective '+(currentRun.status==='won'?'objective-won':currentRun.status==='failed'?'objective-failed':'')}><div className="objective-head"><div><p className="eyebrow">OBJECTIVE · DAY {scenarioInfo[currentRun.scenario].days}</p><h3>{scenarioInfo[currentRun.scenario].objective}</h3></div><strong>{currentRun.status.replace('_',' ')}</strong></div><div className="progress-track"><div style={{width:Math.min(100,selected.tick/scenarioInfo[currentRun.scenario].days*100)+'%'}}/></div><p>Day {selected.tick} / {scenarioInfo[currentRun.scenario].days}. {scenarioInfo[currentRun.scenario].hint}</p><ul>{objective.requirements.map((requirement,index)=><li key={index}>{requirement}</li>)}</ul>{currentRun.elapsedSeconds!==undefined&&<small>Attempt time: {Math.floor(currentRun.elapsedSeconds/60)}m {currentRun.elapsedSeconds%60}s</small>}</section>}
        <div className="metrics"><div><small>SIMULATED DAY</small><strong>{selected.tick.toLocaleString()}</strong></div><div><small>POPULATION</small><strong>{selected.organisms.length.toLocaleString()}</strong></div><div><small>BIRTHS / DEATHS</small><strong>{selected.statistics.births.toLocaleString()} / {selected.statistics.deaths.toLocaleString()}</strong></div></div>
        <section className="feedback-grid">
          <article><p className="eyebrow">POPULATION CHANGE</p><strong>{netChange>=0?'+':''}{netChange.toLocaleString()}</strong><p>Founders + player introductions + births − deaths. Predation accounts for {selected.statistics.predation_deaths.toLocaleString()} deaths.</p></article>
          <article><p className="eyebrow">RESOURCE SIGNAL</p><strong>{vegetationRemaining}% vegetation</strong><p>{resource?.vegetation.toLocaleString()??0} vegetation · {resource?.water.toLocaleString()??0} land water · {resource?.depleted_land_tiles??0} depleted tiles.</p><p>{vegetationRemaining<25?'Vegetation is critically low compared with the '+resourceReference+'.':vegetationRemaining<60?'Vegetation has fallen substantially; herbivores may face food pressure.':'Vegetation remains above 60% of the '+resourceReference+'.'}</p></article>
          <article><p className="eyebrow">WHY ORGANISMS DIED</p><strong>{(selected.statistics.predation_deaths??0).toLocaleString()} hunted</strong><p>{(selected.statistics.dehydration_deaths??0).toLocaleString()} dehydration · {(selected.statistics.starvation_deaths??0).toLocaleString()} starvation · {(selected.statistics.age_deaths??0).toLocaleString()} reached lifespan.</p><small>Each non-predation death has one primary cause: lifespan, then dehydration, then starvation. Breakdown is complete from day {selected.statistics.death_diagnostics_start_day??'after the next tick'}.</small></article>
          <article><p className="eyebrow">GENETIC VARIATION</p><strong>{genomeDiversity(selected).toLocaleString()} live genomes</strong><p>{selected.speciation_events.length} lineage-branch events recorded. A branch records ancestry, not a reproductively isolated species.</p>{selected.speciation_events.slice(-2).map(event=><small key={event.sequence}>Day {event.day}: branch {event.new_species_id} from {event.parent_species_id}, {event.genetic_distance_basis_points} bp apart (threshold {event.threshold_basis_points}).</small>)}</article>
        </section>
        <section className="comparison"><div><p className="eyebrow">EXPERIMENT COMPARISON</p><h3>Compare worlds side by side</h3><p className="muted">Create a matched world with one changed trait or select an existing world.</p></div><div className="comparison-controls"><label>Change one trait<select value={variant} onChange={e=>setVariant(e.target.value)}>{variationOptions.map(option=><option key={option.id} value={option.id}>{option.label}</option>)}</select></label><button disabled={!currentRun} onClick={createComparison}>Create matched world</button><label>Compare with<select value={compareId} onChange={e=>setCompareId(e.target.value)}><option value="">Choose world</option>{worlds.filter(world=>world.id!==selected.id).map(world=><option key={world.id} value={world.id}>{world.config.name}</option>)}</select></label></div>
          {compared&&<div className="compare-table"><div><span>World</span><b>{selected.config.name}</b><b>{compared.config.name}</b></div><div><span>Objective</span><strong>{currentRun?.status??'sandbox'}</strong><strong>{comparedRun?.status??'sandbox'}</strong></div><div><span>Day</span><strong>{selected.tick}</strong><strong>{compared.tick}</strong></div><div><span>Population</span><strong>{selected.organisms.length}</strong><strong>{compared.population}</strong></div><div><span>Births</span><strong>{selected.statistics.births}</strong><strong>{compared.statistics.births}</strong></div><div><span>Deaths</span><strong>{selected.statistics.deaths}</strong><strong>{compared.statistics.deaths}</strong></div><div><span>Predation deaths</span><strong>{selected.statistics.predation_deaths}</strong><strong>{compared.statistics.predation_deaths}</strong></div></div>}
        </section>
        <div className="controls"><button disabled={selected.state!=='created'} onClick={()=>control({command:'start'})}>Start</button><button disabled={selected.state!=='running'} onClick={()=>control({command:'pause'})}>Pause</button><button disabled={selected.state!=='paused'} onClick={()=>control({command:'resume'})}>Resume</button><button className="secondary" disabled={selected.state!=='running'} onClick={()=>control({command:'advance',ticks:1})}>Advance 1 day</button></div>
        <div className="map-heading"><h3>Habitat and organisms</h3><small>Click an organism to inspect it; click a tile to choose an introduction point.</small></div>
        <MapView world={selected} onIntroduce={(x,y)=>setTile([x,y])}/>
        <div className="genome-editor"><h3>Introduce an organism</h3><p>{tile?'Selected tile: '+tile[0]+', '+tile[1]:'1. Click a habitat tile. 2. Choose a genome. 3. Introduce it.'}</p><div className="trait-notes"><span><b>Diet:</b> sets which food it can use and what it can hunt.</span><span><b>Speed:</b> movement steps per day; changes access to food, water, and encounters.</span><span><b>Metabolism:</b> energy spent per day. Lower values conserve energy.</span><span><b>Fertility:</b> chance to reproduce after age 30; a birth costs parent energy.</span><span><b>Lifespan:</b> maximum age in days.</span></div><div className="gene-fields"><label>Diet<select value={diet} onChange={e=>setDiet(e.target.value as Genome['diet'])}><option value="herbivore">Herbivore</option><option value="carnivore">Carnivore</option><option value="omnivore">Omnivore</option></select></label><label>Sex<select value={sex} onChange={e=>setSex(e.target.value as Genome['sex'])}><option value="asexual">Asexual</option><option value="female">Female</option><option value="male">Male</option></select></label><label>Speed (0–4)<input type="number" min="0" max="4" value={speed} onChange={e=>setSpeed(Number(e.target.value))}/></label><label>Metabolism (0–20)<input type="number" min="0" max="20" value={metabolism} onChange={e=>setMetabolism(Number(e.target.value))}/></label><label>Fertility (0–100)<input type="number" min="0" max="100" value={fertility} onChange={e=>setFertility(Number(e.target.value))}/></label><label>Lifespan (days)<input type="number" min="1" max="65535" value={lifespan} onChange={e=>setLifespan(Number(e.target.value))}/></label></div><button className="secondary" disabled={!tile} onClick={introduce}>Introduce organism</button></div>
        <section className="population-panel"><div className="map-heading"><h3>Population history</h3><small>Sampled every 10 simulated days · {selected.population_history.length} samples</small></div><PopulationChart samples={selected.population_history}/><AlleleFrequencyChart samples={selected.allele_frequency_history}/></section>
        {selected.speciation_events.length>0&&<section className="inspection"><h3>Lineage branch events</h3>{selected.speciation_events.slice(-8).reverse().map(event=><p key={event.sequence}>Day {event.day}: branch {event.new_species_id} split from {event.parent_species_id} · genetic distance {event.genetic_distance_basis_points} bp, threshold {event.threshold_basis_points} bp</p>)}</section>}
        <p className="hash">State hash · {selected.state_hash}</p>
      </section>:<section className="panel empty"><h2>Choose a scenario or create a sandbox</h2><p>Follow the objective, make a hypothesis, then compare a changed genome in a matched world.</p></section>}
    </div>
    {error&&<p className="error">{error}</p>}
  </main>;
}
createRoot(document.getElementById('root')!).render(<React.StrictMode><App/></React.StrictMode>);
