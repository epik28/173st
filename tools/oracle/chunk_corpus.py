#!/usr/bin/env python3
"""CHUNK-001 inputs and original-JAR reflection audit, never Rust-derived answers."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import tomllib
import zipfile

ROOT = Path(__file__).resolve().parents[2]
FIXTURE = ROOT/'tests/fixtures/chunk/layout.tsv'
JAR_SHA256 = 'af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c'


def corpus():
    ops = [f'coord:{value}' for value in [-2147483648,-17,-16,-15,-1,0,1,15,16,17,2147483647]]
    for x in [0,1,15]:
        for z in [0,1,15]:
            for y in [0,1,126,127]:
                position = y+z*128+x*2048
                ops.extend([f'index:{x}:{y}:{z}',f'get:{x}:{y}:{z}'])
                for nibble in range(16):
                    block_id = [0,127,128,255][nibble%4]
                    ops.extend([f'set:{x}:{y}:{z}:{block_id}:{nibble}:{15-nibble}:{(nibble+7)%16}',
                                f'get:{x}:{y}:{z}', f'get:{x}:{y^1}:{z}',
                                f'byte:blocks:{position}',f'byte:data:{position//2}',
                                f'byte:block_light:{position//2}',f'byte:sky_light:{position//2}'])
    # Exercise both ends of adjacent columns, and rejected writes followed by reads.
    for x,y,z in [(0,127,15),(1,0,0),(14,127,15),(15,0,0),(15,127,15)]:
        ops.extend([f'set:{x}:{y}:{z}:255:3:5:9', f'get:{x}:{y}:{z}'])
    ops.append('set:0:0:0:255:3:7:11')
    for invalid in ['set:16:0:0:0:0:0:0','set:0:128:0:0:0:0:0','set:0:0:16:0:0:0:0',
                    'set:0:0:0:0:16:0:0','set:0:0:0:0:0:16:0','set:0:0:0:0:0:0:255',
                    'set:255:255:255:255:255:255:255']:
        ops.extend([invalid,'get:0:0:0','byte:blocks:0','byte:data:0','byte:block_light:0','byte:sky_light:0'])
    for axis in [(16,0,0),(0,128,0),(0,0,16),(255,255,255)]:
        ops.extend(['index:'+':'.join(map(str,axis)),'get:'+':'.join(map(str,axis))])
    lines=['# CHUNK-001 inputs, original wi nibble calls; generator tools/oracle/chunk_corpus.py',
           'layout\t'+';'.join(ops),
           'fresh-zero\tget:0:0:0;get:15:127:15;byte:blocks:32767;byte:data:16383;byte:block_light:16383;byte:sky_light:16383']
    return ('\n'.join(lines)+'\n').encode()


def verify():
    data = corpus()
    assert FIXTURE.read_bytes()==data, 'fixture differs from generator'
    config=tomllib.loads((ROOT/'config/local.toml').read_text())
    jar=(ROOT/config['paths']['client_jar']).resolve()
    jar_hash=hashlib.sha256(jar.read_bytes()).hexdigest()
    assert jar_hash==JAR_SHA256
    build=ROOT/'artifacts/oracle-chunk-check'; build.mkdir(parents=True,exist_ok=True)
    subprocess.run(['javac','-d',str(build),str(ROOT/'tools/oracle/src/ChunkOracle.java')],check=True,timeout=30)
    command=['java','-cp',str(build),'ChunkOracle']
    description=subprocess.run(command+['--describe',str(jar)],capture_output=True,check=True,timeout=10)
    metadata=json.loads(description.stdout)
    assert metadata['class']=='wi' and metadata['native_sets']==2 and metadata['native_gets']==2
    def execute(value, args=None):
        return subprocess.run(command+([str(jar)] if args is None else args),input=value,capture_output=True,timeout=20)
    audit=build/'native-invocations.json'
    first=execute(data,[str(jar),str(audit)]);second=execute(data)
    total=sum(len(line.split('\t')[1].split(';')) for line in data.decode().splitlines() if not line.startswith('#'))
    assert total>=1000 and first.returncode==second.returncode==0 and not first.stderr and not second.stderr
    assert first.stdout==second.stdout and len(first.stdout.splitlines())==total
    (build/'reference.stdout.tsv').write_bytes(first.stdout)
    invocation=json.loads(audit.read_text())
    assert invocation['native_gets']>0 and invocation['native_sets']>0 and invocation['raw_reads']>0
    rows=[line.split('\t') for line in first.stdout.decode().splitlines()]
    assert [row[-1] for row in rows if row[0]=='fresh-zero']==['0,0,0,0','0,0,0,0','0','0','0','0']
    # Each rejected write is followed by the unchanged block and all four raw bytes.
    for index,row in enumerate(rows):
        if row[-1] in ('invalid_coordinate','invalid_nibble') and row[2]=='set':
            assert rows[index+1][-1]=='255,3,7,11'
    invalid=[b' \n',b'\t\n',b'x\tunknown\n',b'x\t\n',b'x\tget:0:0:0;\n',b'x\tget:0:0:0;;coord:0\n',
             b'x\tget:-1:0:0\n',b'x\tget:256:0:0\n',b'x\tset:0:0:0:256:0:0:0\n',
             b'x\tcoord:2147483648\n',b'x\tbyte:data:16384\n',b'x\tbyte:blocks:-1\n',
             b'x\tbyte:unknown:0\n',b'x\tget:0:0\n',b'x\tcoord:0\nx\tcoord:1\n',
             b'x\t'+b';'.join([b'coord:0']*100001)+b'\n']
    for value in invalid:
        result=execute(value)
        assert result.returncode!=0 and result.stderr,value[:80]
    maximum=execute(b'max\t'+b';'.join([b'coord:0']*100000)+b'\n')
    assert maximum.returncode==0 and len(maximum.stdout.splitlines())==100000
    missing=execute(b'x\tget:0:0:0\n',[str(build/'absent.jar')]);assert missing.returncode!=0 and missing.stderr
    wrong=build/'wrong.jar'
    with zipfile.ZipFile(wrong,'w') as file: file.writestr('dummy','not the original JAR')
    result=execute(b'x\tget:0:0:0\n',[str(wrong)]);assert result.returncode!=0 and result.stderr
    assert hashlib.sha256(jar.read_bytes()).hexdigest()==jar_hash
    report={'kind':'original client nibble adapter audit, not Rust parity','value_count':total,'case_count':2,
            'input_sha256':hashlib.sha256(data).hexdigest(),'jar_sha256':jar_hash,'native':invocation,
            'invalid_inputs_rejected':len(invalid),'maximum_accepted':100000,
            'missing_and_wrong_jar_rejected':True,'reflection_selftest':metadata}
    (build/'audit.json').write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps(report,indent=2))


def main():
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--write',action='store_true');args=parser.parse_args()
    if args.write:
        FIXTURE.parent.mkdir(parents=True,exist_ok=True);FIXTURE.write_bytes(corpus())
    verify()

if __name__=='__main__':main()
