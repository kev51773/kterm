import fs from 'node:fs'
import path from 'node:path'
import { MANIFEST_PATH, type RunManifest, type RunTest, type RunShot, type CompareStatus } from '../helpers/run.js'
import { TMP_DIR } from '../helpers/paths.js'

const OUTPUT = path.join(TMP_DIR, 'report.html')

export interface ReportShot {
  name: string
  spec: string
  compare: CompareStatus | undefined
  mismatchPct?: number
}

export interface ReportTest {
  key: string
  spec: string
  index: number
  title: string
  status: string
  error?: string
  shots: ReportShot[]
  steps: string[]
}

function htmlEscape(s: string): string {
  return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;')
}

function jsString(s: string): string {
  return JSON.stringify(s).replace(/</g, '\\u003c')
}

export function readManifest(): RunManifest | null {
  if (!fs.existsSync(MANIFEST_PATH)) return null
  try {
    return JSON.parse(fs.readFileSync(MANIFEST_PATH, 'utf8'))
  } catch {
    return null
  }
}

// Legacy manifests stored plain path strings; normalize to shot objects.
function normalizeRun(run: RunManifest): RunManifest {
  return {
    generatedAt: run.generatedAt,
    tests: run.tests.map((t) => ({
      ...t,
      screenshots: t.screenshots.map((s) => (typeof s === 'string' ? { rel: s } : s)),
    })),
  }
}

function shotSpecAndName(rel: string): { spec: string; name: string } {
  const parts = rel.split('/')
  const file = parts[parts.length - 1] ?? ''
  const spec = parts[parts.length - 2] ?? 'unknown'
  return { spec, name: file.replace(/\.png$/, '') }
}

export function renderReportHtml(run: RunManifest, base: string): string {
  const norm = normalizeRun(run)
  const cards: ReportTest[] = []

  for (const t of norm.tests) {
    const key = `${t.spec}|${t.index}|${t.title}`
    const shots: ReportShot[] = t.screenshots.map((s) => {
      const { spec, name } = shotSpecAndName(s.rel)
      return { name, spec, compare: s.compare, mismatchPct: s.mismatchPct }
    })
    cards.push({ key, spec: t.spec, index: t.index, title: t.title, status: t.status, error: t.error, shots, steps: t.steps })
  }

  const passed = norm.tests.filter((t) => t.status === 'passed').length
  const failed = norm.tests.filter((t) => t.status === 'failed').length
  const skipped = norm.tests.filter((t) => t.status === 'skipped').length
  const summary = `${norm.tests.length} tests · ${passed} passed · ${failed} failed · ${skipped} skipped`

  const bodies = cards.map((t) => cardHtml(t, base)).join('\n')

  return `<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>kterm review — ${htmlEscape(run.generatedAt)}</title>
<style>
:root{--bg:#0d0e11;--card:#181a1f;--fg:#cccccc;--muted:#7a8190;--green:#4ade80;--red:#f87171;--amber:#fbbf24;--accent:#61afef}
*{box-sizing:border-box}
body{background:var(--bg);color:var(--fg);font:14px/1.5 "Segoe UI",system-ui,sans-serif;margin:0;padding:24px 24px 96px}
h1{font-size:20px;margin:0 0 4px}
.summary{color:var(--muted);margin-bottom:8px}
.hint{color:var(--muted);font-size:12px;margin-bottom:16px}
.card{background:var(--card);border:1px solid #282c34;border-radius:10px;margin:16px 0;padding:16px 20px}
.card.skipped{opacity:.65}
.head{display:flex;gap:10px;align-items:center;flex-wrap:wrap}
.test-no{font-weight:700;color:var(--accent)}
.spec{color:var(--muted);font-size:12px}
.status{font-size:11px;font-weight:700;padding:2px 8px;border-radius:999px}
.status.passed{background:#0f2e1a;color:var(--green)}
.status.failed{background:#3a1111;color:var(--red)}
.status.skipped{background:#3a2f0e;color:var(--amber)}
.title{margin:6px 0 2px;font-weight:600}
.steps{list-style:none;padding:0;margin:8px 0 0}
.steps>li{padding:3px 0 3px 18px;position:relative;color:#b6bdc9}
.steps>li::before{content:"▸";position:absolute;left:0;color:var(--muted)}
.shot{display:flex;flex-direction:column;gap:6px;margin:8px 0 8px 18px}
.shot-label{color:var(--muted);font-size:12px}
.chip{display:inline-block;font-size:11px;font-weight:700;padding:1px 8px;border-radius:999px;margin-left:6px}
.chip.match{background:#0f2e1a;color:var(--green)}
.chip.diff{background:#3a1111;color:var(--red)}
.chip.missing{background:#3a2f0e;color:var(--amber)}
.chip.updated{background:#0f2e1a;color:var(--green)}
.pair{display:flex;gap:10px;flex-wrap:wrap}
.pair figure{margin:0;flex:1 1 380px;max-width:430px}
.pair img{display:block;width:100%;border:1px solid #333;border-radius:6px}
.pair figcaption{color:var(--muted);font-size:11px;margin-bottom:4px}
.placeholder{display:flex;align-items:center;justify-content:center;height:120px;border:1px dashed #333;border-radius:6px;color:var(--muted);font-size:12px}
.verdict{display:flex;gap:18px;align-items:flex-start;margin-top:10px;flex-wrap:wrap}
.verdict .opts{display:flex;gap:14px}
.verdict label{display:flex;align-items:center;gap:5px;cursor:pointer;font-size:13px}
.verdict input[type=radio]{accent-color:var(--accent)}
.verdict textarea{flex:1 1 100%;min-height:56px;background:#0d0e11;border:1px solid #282c34;border-radius:6px;color:var(--fg);font:12px/1.4 "Consolas",monospace;padding:8px;display:none}
.verdict textarea.show{display:block}
.skipped-note{color:var(--amber);font-size:12px;margin-top:8px}
#bar{position:fixed;bottom:0;left:0;right:0;padding:12px 24px;background:var(--card);border-top:1px solid #282c34;display:flex;gap:16px;align-items:center;flex-wrap:wrap}
#bar button{background:var(--accent);color:#000;border:none;padding:8px 16px;border-radius:6px;font-weight:700;cursor:pointer}
#bar button:disabled{opacity:.4;cursor:default}
#bar button.update{background:var(--green)}
#bar .note{color:var(--muted);font-size:12px}
#bar #status-msg{color:var(--green);font-size:12px}
#prompt{padding:16px 24px 20px}
#prompt pre{background:#0d0e11;border:1px solid #282c34;border-radius:8px;padding:14px;white-space:pre-wrap;color:#b6bdc9;max-height:360px;overflow:auto}
#prompt button{margin-top:8px;background:var(--green);color:#000;border:none;padding:8px 16px;border-radius:6px;font-weight:700;cursor:pointer}
</style>
</head>
<body>
<h1>kterm review</h1>
<div class="summary">${htmlEscape(summary)} · run ${htmlEscape(run.generatedAt)}</div>
<div class="hint">Per test, pick a verdict. Defaults come from the pixel comparison (baseline vs current). Failing tests take a description — it feeds the failure prompt. Click "Update selected baselines" to promote approved runs, then "Generate failure prompt" to hand failures to the agent.</div>
${bodies}
<div id="prompt" hidden>
  <div class="summary">Failure prompt — copy and paste it to the agent.</div>
  <pre id="prompt-pre"></pre>
  <button id="copy">Copy prompt</button>
</div>
<div id="bar">
  <button id="gen-prompt">Generate failure prompt</button>
  <button id="update-baselines" class="update">Update selected baselines</button>
  <span class="note">fail <b id="n-fail">0</b> · update <b id="n-update">0</b> · pass <b id="n-pass">0</b></span>
  <span id="status-msg"></span>
</div>
<script>
const RUN_TS=${jsString(run.generatedAt)};
const TESTS=${JSON.stringify(cards).replace(/</g,'\\u003c')};
const FILE_MODE=location.protocol==='file:';
const STORE_KEY='kterm-verdict-'+RUN_TS;
let state={}; // key -> {v:'fail'|'pass'|'update',note:string,updated:bool}
try{state=JSON.parse(localStorage.getItem(STORE_KEY)||'{}')}catch(e){}
function shotsOf(t){return t.shots}
function defaultVerdict(t){
  if(t.status==='failed')return 'fail';
  if(t.status==='skipped')return null;
  const s=shotsOf(t);
  if(s.some(x=>x.compare==='diff'))return 'fail';
  if(s.some(x=>x.compare==='baseline-missing'||x.compare===undefined))return 'update';
  return 'pass';
}
function save(){try{localStorage.setItem(STORE_KEY,JSON.stringify(state))}catch(e){}}
function setVerdict(key,v){state[key]=state[key]||{};state[key].v=v;save();counts()}
function setNote(key,n){state[key]=state[key]||{};state[key].note=n;save()}
function verdictOf(t){const st=state[t.key];if(st&&st.v)return st.v;const d=defaultVerdict(t);return d}
function counts(){
  let f=0,u=0,p=0;
  for(const t of TESTS){if(t.status==='skipped')continue;const v=verdictOf(t);if(v==='fail')f++;else if(v==='update')u++;else p++}
  document.getElementById('n-fail').textContent=f;
  document.getElementById('n-update').textContent=u;
  document.getElementById('n-pass').textContent=p;
}
function applyCard(t,card){
  const radios=card.querySelectorAll('input[name="v-'+t.key+'"]');
  radios.forEach(r=>{r.addEventListener('change',()=>{setVerdict(t.key,r.value);syncCard(t,card)})});
  const ta=card.querySelector('textarea');
  ta.addEventListener('input',()=>setNote(t.key,ta.value));
  syncCard(t,card);
}
function syncCard(t,card){
  const st=state[t.key]||{};
  const v=st.v||defaultVerdict(t);
  card.querySelectorAll('input[name="v-'+t.key+'"]').forEach(r=>{r.checked=(r.value===v)});
  const ta=card.querySelector('textarea');
  const show=ta&&(v==='fail'&&!st.updated);
  if(show){ta.classList.add('show');ta.value=st.note!==undefined?st.note:(t.status==='failed'?(t.error||''):'')}
  else{ta.classList.remove('show')}
  if(st.updated&&!card.querySelector('.updated-chip'))card.querySelector('.verdict').insertAdjacentHTML('beforeend','<span class="chip updated updated-chip" style="margin-left:8px">baselines updated</span>');
  counts();
}
document.querySelectorAll('.card').forEach(card=>{
  const t=TESTS.find(x=>x.key===card.dataset.key);
  if(!t)return;
  if(t.status==='skipped')return; // skipped tests carry no verdict buttons
  applyCard(t,card);
});
function shotLine(s){return s.name+(s.compare==='diff'?(' DIFF '+((s.mismatchPct||0)).toFixed(2)+'%'):s.compare==='match'?' MATCH':s.compare==='baseline-missing'?' NO BASELINE':s.compare==='updated-baseline'?' UPDATED':' no-compare')}
function buildPrompt(){
  const fail=[],skip=[],pass=[];
  for(const t of TESTS){
    if(t.status==='skipped'){skip.push(t);continue}
    const v=verdictOf(t);
    if(v==='fail'){fail.push(t)}else if(v==='pass'){pass.push(t)}
    // 'update' verdicts are not failures and not listed
  }
  const lines=[];
  lines.push('FAILURES ('+fail.length+'):');
  if(fail.length){
    fail.forEach((t,i)=>{
      const st=state[t.key]||{};
      lines.push('');
      lines.push((i+1)+'. '+t.spec+' TEST '+t.index+' "'+t.title+'" — run: '+(t.status==='failed'?'FAILED':'PASSED'));
      if(t.shots.length)lines.push('   screenshots: '+t.shots.map(shotLine).join('; '));
      const note=st.note!==undefined&&st.note.trim()?st.note.trim():(t.status==='failed'?(t.error||'no run error captured'):'');
      lines.push('   failure description: '+note);
      lines.push('   evidence: '+t.shots.map(s=>'screenshots/actual/'+s.spec+'/'+s.name+'.png').join(', '));
    });
  } else lines.push('   (none)');
  lines.push('');
  lines.push('SKIPPED ('+skip.length+'): '+(skip.length?skip.map(t=>t.spec+' TEST '+t.index+' "'+t.title+'" ('+(t.error||'no reason')+')').join('; '):'none'));
  lines.push('');
  lines.push('PASSED ('+pass.length+'): '+(pass.length?pass.map(t=>t.spec+' T'+t.index).join(', '):'none'));
  lines.push('');
  lines.push('Next step: fix or re-run the FAILURES above. '+(skip.length?'Re-run the skipped tests when UAC approval is available (they were not counted as failures).':''));
  return lines.join('\\n');
}
const updBtn=document.getElementById('update-baselines');
if(FILE_MODE){updBtn.disabled=true;updBtn.textContent='Update selected baselines (run: npm run review)';updBtn.title='Baseline updates need the review server (npm run review).'}
updBtn.addEventListener('click',async()=>{
  const updates=[];
  for(const t of TESTS){
    if(t.status==='skipped')continue;
    const st=state[t.key]||{};
    const v=st.v||defaultVerdict(t);
    if(v==='update'&&!st.updated)for(const s of t.shots)updates.push({spec:s.spec,name:s.name});
  }
  if(!updates.length){const m=document.getElementById('status-msg');m.textContent='nothing to update';return}
  try{
    const res=await fetch('/api/update-baselines',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({updates})});
    const body=await res.json();
    for(const t of TESTS){
      if(state[t.key]&&state[t.key].v==='update'&&!state[t.key].updated){state[t.key].updated=true;state[t.key].v='pass'}
    }
    save();counts();
    document.getElementById('status-msg').textContent='baselines updated: '+body.updated.length+' shot(s)';
    document.querySelectorAll('.card').forEach(card=>{
      const t=TESTS.find(x=>x.key===card.dataset.key);
      if(t&&state[t.key]&&state[t.key].updated)syncCard(t,card);
    });
  }catch(e){document.getElementById('status-msg').textContent='update failed: '+e}
});
document.getElementById('gen-prompt').addEventListener('click',()=>{
  document.getElementById('prompt-pre').textContent=buildPrompt();
  document.getElementById('prompt').hidden=false;
  document.getElementById('prompt').scrollIntoView({behavior:'smooth'});
});
document.getElementById('copy').addEventListener('click',()=>{
  const txt=document.getElementById('prompt-pre').textContent;
  if(navigator.clipboard&&navigator.clipboard.writeText){navigator.clipboard.writeText(txt)}
  else{const ta=document.createElement('textarea');ta.value=txt;document.body.appendChild(ta);ta.select();document.execCommand('copy');ta.remove()}
});
counts();
</script>
</body>
</html>`
}

function chipHtml(compare: CompareStatus | undefined, mismatchPct?: number): string {
  if (compare === 'match') return `<span class="chip match">MATCH</span>`
  if (compare === 'diff') return `<span class="chip diff">DIFF ${(mismatchPct ?? 0).toFixed(2)}%</span>`
  if (compare === 'updated-baseline') return `<span class="chip updated">BASELINE UPDATED</span>`
  return `<span class="chip missing">NO BASELINE</span>`
}

function shotBlockHtml(shot: ReportShot, base: string): string {
  const actSrc = `${base}screenshots/actual/${shot.spec}/${shot.name}.png`
  const baseSrc = `${base}screenshots/baseline/${shot.spec}/${shot.name}.png`
  const hasBaseline = shot.compare !== 'baseline-missing' && shot.compare !== undefined
  return `<div class="shot">
  <div class="shot-label">${htmlEscape(shot.name)}.png${chipHtml(shot.compare, shot.mismatchPct)}</div>
  <div class="pair">
    <figure>${hasBaseline
      ? `<img src="${htmlEscape(baseSrc)}" loading="lazy">`
      : `<div class="placeholder">no baseline</div>`}<figcaption>baseline</figcaption></figure>
    <figure><img src="${htmlEscape(actSrc)}" loading="lazy"><figcaption>current</figcaption></figure>
  </div>
</div>`
}

function cardHtml(t: ReportTest, base: string): string {
  const badge = `<span class="status ${htmlEscape(t.status)}">${t.status.toUpperCase()}</span>`

  const shotByName = new Map(t.shots.map((s) => [s.name, s]))
  const lis = t.steps.map((s) => {
    const m = s.match(/^took screenshot (.+?)\.png$/)
    if (m) {
      const shot = shotByName.get(m[1])
      if (shot) return shotBlockHtml(shot, base)
      return ''
    }
    return `<li>${htmlEscape(s)}</li>`
  }).filter(Boolean)

  const verdict = t.status === 'skipped'
    ? `<div class="skipped-note">skipped: ${htmlEscape(t.error ?? 'no reason')}</div>`
    : `<div class="verdict" data-key="${htmlEscape(t.key)}">
        <div class="opts">
          <label><input type="radio" name="v-${htmlEscape(t.key)}" value="fail"> Fail</label>
          <label><input type="radio" name="v-${htmlEscape(t.key)}" value="pass"> Pass</label>
          <label><input type="radio" name="v-${htmlEscape(t.key)}" value="update"> Pass (and update baseline)</label>
        </div>
        <textarea placeholder="Describe the failure — it goes into the failure prompt..."></textarea>
      </div>`

  return `<section class="card${t.status === 'skipped' ? ' skipped' : ''}" data-key="${htmlEscape(t.key)}">
  <div class="head">
    <span class="test-no">TEST ${t.index}</span>
    <span class="spec">${htmlEscape(t.spec)}</span>
    ${badge}
  </div>
  <div class="title">${htmlEscape(t.title)}</div>
  <ol class="steps">${lis.join('')}</ol>
  ${verdict}
</section>`
}

export function generate(): void {
  const run = readManifest()
  if (!run) {
    console.log('no run-manifest.json — run a test suite first (tmp/run-manifest.json)')
    return
  }
  fs.mkdirSync(TMP_DIR, { recursive: true })
  fs.writeFileSync(OUTPUT, renderReportHtml(run, '../'))
  console.log(`report written: ${OUTPUT}`)
  console.log(`  review with: npm run review (baseline compare + verdict buttons)`)
}

if (process.argv[1] && path.basename(process.argv[1]).startsWith('report')) {
  generate()
}
