import fs from 'node:fs'
import path from 'node:path'
import { MANIFEST_PATH, type RunManifest } from '../helpers/run.js'
import { TMP_DIR } from '../helpers/paths.js'

const OUTPUT = path.join(TMP_DIR, 'report.html')
const REL_PREFIX = '../'

interface CardData {
  key: string
  spec: string
  index: number
  title: string
  status: string
  shots: string[]
}

function htmlEscape(s: string): string {
  return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;')
}

function imgSrc(rel: string): string {
  return REL_PREFIX + rel.split(/[\\/]/).join('/')
}

function cardHtml(t: CardData, steps: string[]): string {
  const status = t.status === 'passed'
  const badge = `<span class="status ${status ? 'passed' : 'failed'}">${status ? 'PASS' : 'FAIL'}</span>`

  const lis = steps.map((s) => {
    const m = s.match(/^took screenshot (.+)$/)
    if (m) {
      const fullRel = `screenshots/actual/${t.spec}/${m[1]}`
      if (!fs.existsSync(path.join(TMP_DIR, '..', fullRel))) return ''
      return `<li class="step-shot"><span class="shot-label">took screenshot ${htmlEscape(m[1])}</span>
<div class="shot" data-key="${htmlEscape(t.key)}" title="click: unreviewed / approved / rejected">
  <img src="${htmlEscape(imgSrc(fullRel))}" loading="lazy">
  <div class="overlay"></div>
</div></li>`
    }
    return `<li>${htmlEscape(s)}</li>`
  }).filter(Boolean)

  return `<section class="card" id="t-${t.spec}-${t.index}">
  <div class="head">
    <span class="test-no">TEST ${t.index}</span>
    <span class="spec">${htmlEscape(t.spec)}</span>
    ${badge}
  </div>
  <div class="title">${htmlEscape(t.title)}</div>
  <ol class="steps">${lis.join('')}</ol>
</section>`
}

export function generate(): void {
  if (!fs.existsSync(MANIFEST_PATH)) {
    console.log('no run-manifest.json — run a test suite first (tmp/run-manifest.json)')
    return
  }
  const run: RunManifest = JSON.parse(fs.readFileSync(MANIFEST_PATH, 'utf8'))
  const cards: CardData[] = []
  const bodies: string[] = []

  for (const t of run.tests) {
    const key = `${t.spec}|${t.index}|${t.title}`
    cards.push({ key, spec: t.spec, index: t.index, title: t.title, status: t.status, shots: t.screenshots })
    bodies.push(cardHtml(cards[cards.length - 1], t.steps))
  }

  const passed = run.tests.filter((t) => t.status === 'passed').length
  const failed = run.tests.length - passed
  const summary = `${run.tests.length} tests · ${passed} passed · ${failed} failed`

  const html = `<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>kterm test run — ${htmlEscape(run.generatedAt)}</title>
<style>
:root{--bg:#0d0e11;--card:#181a1f;--fg:#cccccc;--muted:#7a8190;--green:#4ade80;--red:#f87171;--accent:#61afef}
*{box-sizing:border-box}
body{background:var(--bg);color:var(--fg);font:14px/1.5 "Segoe UI",system-ui,sans-serif;margin:0;padding:24px 24px 96px}
h1{font-size:20px;margin:0 0 4px}
.summary{color:var(--muted);margin-bottom:8px}
.hint{color:var(--muted);font-size:12px;margin-bottom:16px}
.card{background:var(--card);border:1px solid #282c34;border-radius:10px;margin:16px 0;padding:16px 20px}
.head{display:flex;gap:10px;align-items:center;flex-wrap:wrap}
.test-no{font-weight:700;color:var(--accent)}
.spec{color:var(--muted);font-size:12px}
.status{font-size:11px;font-weight:700;padding:2px 8px;border-radius:999px}
.status.passed{background:#0f2e1a;color:var(--green)}
.status.failed{background:#3a1111;color:var(--red)}
.title{margin:6px 0 2px;font-weight:600}
.steps{list-style:none;padding:0;margin:8px 0 0}
.steps li{padding:3px 0 3px 18px;position:relative;color:#b6bdc9}
.steps li::before{content:"▸";position:absolute;left:0;color:var(--muted)}
.step-shot{margin-top:6px}
.step-shot::before{display:none}
.shot-label{color:var(--muted);font-size:12px;display:block;margin-left:18px}
.shot{position:relative;display:inline-block;margin:6px 0 6px 18px;cursor:pointer}
.shot img{display:block;max-width:820px;max-height:480px;border:1px solid #333;border-radius:6px}
.overlay{position:absolute;inset:0;display:flex;align-items:center;justify-content:center;pointer-events:none}
.overlay svg{width:38%;height:38%;opacity:.55}
#bar{position:fixed;bottom:0;left:0;right:0;padding:12px 24px;background:var(--card);border-top:1px solid #282c34;display:flex;gap:16px;align-items:center}
#bar button{background:var(--accent);color:#000;border:none;padding:8px 16px;border-radius:6px;font-weight:700;cursor:pointer}
#bar .note{color:var(--muted);font-size:12px}
#prompt{padding:16px 24px 20px}
#prompt pre{background:#0d0e11;border:1px solid #282c34;border-radius:8px;padding:14px;white-space:pre-wrap;color:#b6bdc9;max-height:320px;overflow:auto}
#prompt button{margin-top:8px;background:var(--green);color:#000;border:none;padding:8px 16px;border-radius:6px;font-weight:700;cursor:pointer}
</style>
</head>
<body>
<h1>kterm test run</h1>
<div class="summary">${htmlEscape(summary)} · generated ${htmlEscape(run.generatedAt)}</div>
<div class="hint">Click a screenshot to cycle: blank = unreviewed, ✓ green = approved, ✗ red = rejected. State is remembered in this browser.</div>
${bodies.join('\n')}
<div id="prompt" hidden>
  <div class="summary">Approval prompt — copy it and paste it to the agent.</div>
  <pre id="prompt-pre"></pre>
  <button id="copy">Copy prompt</button>
</div>
<div id="bar">
  <button id="gen-prompt">Generate approval prompt</button>
  <span class="note">approved <b id="n-ok">0</b> · rejected <b id="n-no">0</b> · unreviewed <b id="n-un">0</b></span>
</div>
<script>
const APPROVE_KEY='kterm-approve';
let approvals={};
try{approvals=JSON.parse(localStorage.getItem(APPROVE_KEY)||'{}')}catch(e){}
const TICKS='<svg viewBox="0 0 24 24" fill="none" stroke="#4ade80" stroke-width="3" stroke-linecap="round"><path d="M4 12.5l5 5 11-11"/></svg>';
const CROSS='<svg viewBox="0 0 24 24" fill="none" stroke="#f87171" stroke-width="3" stroke-linecap="round"><path d="M6 6l12 12M18 6L6 18"/></svg>';
const TESTS=${JSON.stringify(cards)};
const RUN_TS=${JSON.stringify(run.generatedAt)};
function stateOf(k){return approvals[k]??0}
function render(shot,s){shot.querySelector('.overlay').innerHTML=s===1?TICKS:s===2?CROSS:'';counts()}
function counts(){let ok=0,no=0;for(const t of TESTS){const s=stateOf(t.key);if(s===1)ok++;else if(s===2)no++}
document.getElementById('n-ok').textContent=ok;document.getElementById('n-no').textContent=no;document.getElementById('n-un').textContent=TESTS.length-ok-no}
function buildPrompt(){
  const groups={approved:[],rejected:[],unset:[]};
  for(const t of TESTS){
    const a=stateOf(t.key);
    const head='  '+t.spec+' TEST '+t.index+' "'+t.title+'" (run: '+t.status.toUpperCase()+')';
    const shots=t.shots.length?t.shots.map(s=>'    '+s).join('\\n'):'';
    groups[a===1?'approved':a===2?'rejected':'unset'].push(shots?head+'\\n'+shots:head);
  }
  const block=(title,items)=>title+'\\n'+(items.length?items.join('\\n\\n'):'  (none)');
  return 'I manually reviewed the kterm autotest run ('+RUN_TS+') by screenshot.\\n\\n'
    +block('APPROVED — use these screenshots as the new visual baselines:',groups.approved)+'\\n\\n'
    +block('REJECTED — result was not as expected (keep old baseline, investigate):',groups.rejected)+'\\n\\n'
    +block('UNSET — not yet reviewed:',groups.unset)+'\\n\\n'
    +'Next step: discuss which of the APPROVED screenshots should replace the existing baselines, then rerun visual comparison.';
}
document.querySelectorAll('.shot').forEach(shot=>{
  const key=shot.dataset.key;
  render(shot,stateOf(key));
  shot.addEventListener('click',()=>{
    const s=(stateOf(key)+1)%3;
    approvals[key]=s;
    try{localStorage.setItem(APPROVE_KEY,JSON.stringify(approvals))}catch(e){}
    render(shot,s);
  });
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

  fs.mkdirSync(TMP_DIR, { recursive: true })
  fs.writeFileSync(OUTPUT, html)
  console.log(`report written: ${OUTPUT}`)
  console.log(`  open with: start ${OUTPUT}`)
}

if (process.argv[1] && path.basename(process.argv[1]).startsWith('report')) {
  generate()
}
