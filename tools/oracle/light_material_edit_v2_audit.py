#!/usr/bin/env python3
"""Rebuild and verify full-state native material edit V2 captures."""
from __future__ import annotations
import argparse, hashlib, json, pathlib, shutil, subprocess, tempfile, zipfile

ROOT=pathlib.Path(__file__).resolve().parents[2]
FIXTURE=ROOT/'tests/fixtures/light_material/native_edits_v2.tsv'
MANIFEST=ROOT/'tests/fixtures/light_material/edit_v2_audit.json'
SOURCE=ROOT/'tools/oracle/src/LightMaterialEditV2Oracle.java'
HELPER=ROOT/'tools/oracle/src/LightUpdateOracle.java'
JAR=ROOT/'../.local/minecraft/client-b1.7.3.jar'
JAR_SHA='af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c'
REQUEST=b'material-edits-v2\n'
BOOTSTRAP_SHA='0b280c253ec80f2690447bfc42cadac3ed78f1d388bdbe2d10b340c76daacc4e'
CASES=('canopy-log-sequence','leaf-attenuation','water-attenuation','glowstone-loaded-chunk-edge')
CLASS_NAMES=('fd','lm','wi','uu','eb','st')

def sha(b): return hashlib.sha256(b).hexdigest()
def valid_digest(s): return len(s)==64 and all(c in '0123456789abcdef' for c in s)
def pinned_jar(path):
    path=path.resolve()
    if not path.is_file() or sha(path.read_bytes())!=JAR_SHA: raise ValueError('wrong or missing pinned client JAR')
    return path
def validate(data):
    if not data or len(data)>5_000_000 or not data.endswith(b'\n') or b'\r' in data: raise ValueError('V2 output size/line ending invalid')
    try: lines=data.decode('ascii').splitlines()
    except UnicodeDecodeError as e: raise ValueError('V2 output must be ASCII') from e
    cases={}; order=[]; last_case=None; last_phase={}; counts={}; edit_rows=[]
    for n,line in enumerate(lines,1):
        f=line.split('\t'); tag=f[0]
        if tag=='case' and len(f)==3:
            name=f[1]
            if name not in CASES or name in cases or not f[2].startswith('steps='): raise ValueError(f'bad case row {n}')
            steps=int(f[2][6:]); cases[name]={'steps':steps,'domain':False,'seed':[],'phases':[],'snap':{},'state':{},'height':{},'deltas':{},'hdeltas':{},'snap_order':[]}
            order.append(name); last_case=name; counts[name]={'states':0,'heights':0,'deltas':0,'hdeltas':0}
        elif tag=='domain' and len(f)==13:
            name=f[1]
            if name not in cases or cases[name]['domain']: raise ValueError(f'orphan/duplicate domain row {n}')
            d=dict(x.split('=',1) for x in f[2:])
            expected={'chunks_x':'-1,0,1','chunks_z':'-1,0,1','y':'0..127','default_block':'0','default_meta':'0','default_block_light':'0','default_sky':'0@y0;15@y1..127(native-lm.c-all-air)','default_height':'0(native-lm.c-all-air)','material':'uu.q[id],uu.s[id]','init':'xa.a(0);lm.c();fd.j-quiescent'}
            if any(d.get(k)!=v for k,v in expected.items()) or not valid_digest(d.get('digest','')): raise ValueError(f'domain contract mismatch {name}')
            cases[name]['domain']=True;cases[name]['digest']=d['digest']
        elif tag=='seed' and len(f)==8:
            name=f[1]; vals=tuple(map(int,f[2:]))
            if name not in cases or not cases[name]['domain'] or vals[0]!=len(cases[name]['seed']): raise ValueError(f'bad seed row {n}')
            _,x,y,z,bid,meta=vals
            if not(-16<=x<=31 and 0<=y<=127 and -16<=z<=31 and 1<=bid<=255 and 0<=meta<=15): raise ValueError(f'seed out of bounds {n}')
            cases[name]['seed'].append((x,y,z,bid,meta))
        elif tag=='snapshot' and len(f)==4:
            name,phase=f[1:3]; digest=f[3].removeprefix('sha256=')
            if name not in cases or not valid_digest(digest) or phase in cases[name]['snap']: raise ValueError(f'bad snapshot row {n}')
            if phase not in ('before','after-place','after-remove') or (not cases[name]['phases'] and phase!='before'): raise ValueError(f'phase order error {n}')
            cases[name]['snap'][phase]=digest;cases[name]['phases'].append(phase);cases[name]['snap_order'].append((phase,digest));last_phase[name]=phase
        elif tag=='state' and len(f)==10:
            name,phase=f[1:3]
            if name not in cases or phase not in cases[name]['snap']: raise ValueError(f'state precedes snapshot {n}')
            x,y,z,bid,meta,block,sky=map(int,f[3:]);key=(x,y,z)
            if not(-16<=x<=31 and -16<=z<=31 and 0<=y<=127 and 0<=bid<=255 and 0<=meta<=15 and 0<=block<=15 and 0<=sky<=15): raise ValueError(f'state range {n}')
            if key in cases[name]['state'].setdefault(phase,{}): raise ValueError(f'duplicate state {n}')
            if (bid,meta,block,sky)==(0,0,0,0 if y==0 else 15): raise ValueError(f'redundant state default {n}')
            cases[name]['state'][phase][key]=(bid,meta,block,sky);counts[name]['states']+=1
        elif tag=='height' and len(f)==6:
            name,phase=f[1:3];x,z,h=map(int,f[3:])
            if name not in cases or phase not in cases[name]['snap'] or not(-16<=x<=31 and -16<=z<=31 and 1<=h<=128): raise ValueError(f'height range/order {n}')
            key=(x,z)
            if key in cases[name]['height'].setdefault(phase,{}): raise ValueError(f'duplicate height {n}')
            cases[name]['height'][phase][key]=h;counts[name]['heights']+=1
        elif tag=='edit' and len(f)==9:
            name,phase,op=f[1:4];x,y,z,bid,meta=map(int,f[4:]);
            if name not in cases or phase not in ('after-place','after-remove') or op not in ('place','remove') or not(0<=y<=127 and -16<=x<=31 and -16<=z<=31 and 0<=bid<=255 and 0<=meta<=15): raise ValueError(f'edit invalid {n}')
            if phase not in cases[name]['phases'] and phase not in ('after-place','after-remove'): raise ValueError(f'edit phase invalid {n}')
            edit_rows.append((name,phase,op,x,y,z,bid,meta))
        elif tag=='delta' and len(f)==15:
            name,fr,to=f[1:4];x,y,z=map(int,f[4:7]);vals=tuple(map(int,f[7:15]));key=(x,y,z)
            if name not in cases or fr not in ('before','after-place') or to not in ('after-place','after-remove') or (fr,to) not in (('before','after-place'),('after-place','after-remove')): raise ValueError(f'delta order {n}')
            if key in cases[name]['deltas'].setdefault((fr,to),{}): raise ValueError(f'duplicate delta {n}')
            if any(v<0 or v>255 for v in vals) or vals[1]>15 or vals[2]>15 or vals[3]>15 or vals[5]>15 or vals[6]>15 or vals[7]>15: raise ValueError(f'delta range {n}')
            cases[name]['deltas'][(fr,to)][key]=vals;counts[name]['deltas']+=1
        elif tag=='hdelta' and len(f)==8:
            name,fr,to=f[1:4];x,z,old,new=map(int,f[4:]);key=(x,z)
            if name not in cases or fr not in ('before','after-place') or to not in ('after-place','after-remove') or (fr,to) not in (('before','after-place'),('after-place','after-remove')) or not(-16<=x<=31 and -16<=z<=31 and 0<=old<=128 and 0<=new<=128): raise ValueError(f'height delta invalid {n}')
            if key in cases[name]['hdeltas'].setdefault((fr,to),{}): raise ValueError(f'duplicate height delta {n}')
            cases[name]['hdeltas'][(fr,to)][key]=(old,new);counts[name]['hdeltas']+=1
        elif tag=='sample' and len(f)==13:
            name,phase=f[1:3];x,y,z=map(int,f[3:6]);vals=tuple(map(int,f[6:]));
            if name not in cases or phase not in cases[name]['snap'] or not(-16<=x<=31 and -16<=z<=31 and 0<=y<=127) or len(vals)!=7: raise ValueError(f'sample invalid {n}')
        else: raise ValueError(f'unknown or malformed row {n}: {tag}')
    if tuple(order)!=CASES: raise ValueError('case list/order mismatch')
    expected_phases={'before','after-place','after-remove'}
    for name,c in cases.items():
        if not c['domain'] or set(c['phases'])!=expected_phases or c['steps']!=2 or c['snap_order'][0][1]!=c['digest']: raise ValueError(f'incomplete case phases {name}')
        if len(c['seed'])!=len(set((x,y,z) for x,y,z,*_ in c['seed'])): raise ValueError(f'duplicate seed coordinate {name}')
        if 'before' not in c['snap']: raise ValueError(f'missing state snapshot {name}')
        before_states=c['state'].get('before',{})
        for x,y,z,bid,meta in c['seed']:
            if before_states.get((x,y,z),(0,0,0,0 if y==0 else 15))[:2]!=(bid,meta): raise ValueError(f'seed differs from native before snapshot {name}')
    for name,phase,op,x,y,z,bid,meta in edit_rows:
        if (phase,op,bid) not in (('after-place','place',17),('after-place','place',18),('after-place','place',9),('after-place','place',89),('after-remove','remove',0)): raise ValueError(f'unexpected edit transition {name}')
    if len(edit_rows)!=8: raise ValueError('expected exactly eight sequential edit rows')
    return {'cases':cases,'counts':counts,'edit_rows':edit_rows,'row_count':len(lines)}

def capture(jar,build,req=REQUEST):
    javac,java=shutil.which('javac'),shutil.which('java')
    if not javac or not java: raise ValueError('javac/java required')
    cp=str(build)+':'+str(jar)
    with tempfile.TemporaryDirectory(prefix='light-material-v2-') as tmp:
        outdir=pathlib.Path(tmp)
        q=subprocess.run([javac,'-cp',str(jar),'-d',str(outdir),str(HELPER),str(SOURCE)],capture_output=True,timeout=60)
        if q.returncode: raise ValueError('javac failed: '+q.stderr.decode(errors='replace'))
        q=subprocess.run([java,'-cp',str(outdir)+':'+str(jar),'LightMaterialEditV2Oracle',str(jar)],input=req,capture_output=True,timeout=60)
        if q.returncode: raise ValueError('native oracle failed: '+q.stderr.decode(errors='replace'))
        return q.stdout

def main():
    ap=argparse.ArgumentParser();ap.add_argument('--write',action='store_true');args=ap.parse_args()
    jar=pinned_jar(JAR)
    out1=capture(jar,pathlib.Path('/tmp'))
    out2=capture(jar,pathlib.Path('/tmp'))
    if out1!=out2: raise SystemExit('two independent native captures differ')
    parsed=validate(out1)
    if args.write:
        FIXTURE.write_bytes(out1)
    elif not FIXTURE.exists() or FIXTURE.read_bytes()!=out1: raise SystemExit('V2 native output differs from frozen fixture')
    with zipfile.ZipFile(jar) as z: cls={n:sha(z.read(n+'.class')) for n in CLASS_NAMES}
    manifest={'jar_sha256':sha(jar.read_bytes()),'source_sha256':sha(SOURCE.read_bytes()),'helper_source_sha256':sha(HELPER.read_bytes()),'request_sha256':sha(REQUEST),'output_sha256':sha(out1),'two_capture_output_sha256':[sha(out1),sha(out2)],'native_class_sha256':cls,'bootstrap_sha256':BOOTSTRAP_SHA,'row_count':parsed['row_count'],'byte_count':len(out1),'case_counts':parsed['counts'],'capture_contract':{'domain':'9 loaded chunks x/z -1..1, y 0..127; all-air native defaults; full state and height exceptions for every phase','transitions':'seed with native fd.a and queue settle; each edit row precedes resulting state snapshot','state':'state case phase x y z id meta blocklight skylight; sparse against native all-air defaults','height':'height case phase x z height; hdelta includes native changed height columns','delta':'complete cell diff between adjacent native phases; includes block, metadata, block-light and sky-light before/after','emission_case':'ID89 glowstone at x=15 across loaded chunks 0/1; verifies native emission propagation and removal','metadata_scope':'native leaf metadata mutations are retained in snapshots but no full metadata parity claim'} }
    if MANIFEST.exists() and not args.write:
        if json.loads(MANIFEST.read_text())!=manifest: raise SystemExit('V2 audit manifest mismatch')
    else: MANIFEST.write_text(json.dumps(manifest,indent=2,sort_keys=True)+'\n')
    print(f'V2 native captures identical: {len(out1)} bytes, {parsed["row_count"]} rows, sha256={sha(out1)}')
if __name__=='__main__': main()
