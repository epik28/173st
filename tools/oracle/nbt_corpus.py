#!/usr/bin/env python3
"""Deterministic NBT wire corpus for the original Beta adapter."""
from pathlib import Path
import struct, subprocess, hashlib, json, sys
ROOT=Path(__file__).resolve().parents[2]; OUT=ROOT/'tests/fixtures/nbt'; FIX=OUT/'corpus.tsv'
def mu(s):
    b=bytearray()
    units=[]
    for c in s:
        n=ord(c)
        if n>0xffff:
            n-=0x10000; units.extend((0xd800+(n>>10),0xdc00+(n&1023)))
        else: units.append(n)
    for n in units:
        if n==0: b += b'\xc0\x80'
        elif n<=0x7f: b.append(n)
        elif n<=0x7ff: b += bytes((0xc0|(n>>6),0x80|(n&63)))
        else: b += bytes((0xe0|(n>>12),0x80|((n>>6)&63),0x80|(n&63)))
    return struct.pack('>H',len(b))+b
def tag(t,name,payload): return bytes([t])+mu(name)+payload
def scalar(t,n): return struct.pack({1:'>b',2:'>h',3:'>i',4:'>q',5:'>f',6:'>d'}[t],n)
def compound(i):
    fields=[tag(1,'byte',scalar(1,(i%255)-127)),tag(2,'short',scalar(2,-32768+(i%65536))),tag(3,'int',scalar(3,i)),tag(4,'long',scalar(4,-i)),tag(5,'float',struct.pack('>I',0x80000000 if i%7==0 else 0x3f800000)),tag(6,'double',struct.pack('>Q',0x7ff8000000000001 if i%11==0 else 0x3ff0000000000000)),tag(7,'bytes',struct.pack('>i',3)+bytes((i&255,(i>>8)&255,255))),tag(8,'text',mu('nul\x00/emoji\U0001f600'))]
    vals=bytes([1])+struct.pack('>i',3)+bytes((i&255,(i+1)&255,(i+2)&255)); fields.append(tag(9,'list',vals))
    nested=tag(10,'nested',bytes([8])+mu('empty')+mu(b'')+bytes([0]))
    return bytes([10])+mu('root')+b''.join(fields)+nested+bytes([0])
def main():
    OUT.mkdir(parents=True,exist_ok=True); rows=[]
    for i in range(5200): rows.append(f'case-{i:05d}\t{compound(i).hex()}')
    FIX.write_text('\n'.join(rows)+'\n',encoding='ascii')
    bad=[('negative-list',bytes([10,0,0,0,9,255,255,255,255,0]).hex()),('unknown',bytes([12]).hex()),('truncated',bytes([10,0,0]).hex()),('trailing',(compound(0)+b'\x00').hex())]
    (OUT/'invalid.tsv').write_text('\n'.join(f'{a}\t{b}' for a,b in bad)+'\n')
    int_array=bytes([10,0,4,114,111,111,116,11,0,4,105,110,116,115,0,0,0,2,0,0,0,1,255,255,255,255,0])
    (OUT/'wire-only-int-array.tsv').write_text('int-array-wire-only\t'+int_array.hex()+'\n')
    audit={'cases':len(rows),'sha256':hashlib.sha256(FIX.read_bytes()).hexdigest(),'invalid_cases':len(bad),'types':[0,1,2,3,4,5,6,7,8,9,10],'wire_only_types':[11],'adapter_classes':['ij','nu'],'jar_sha256':'af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c','limitation':'Beta 1.7.3 ij factory supports tag ids 0..10; TAG_Int_Array (11) is retained as wire-only evidence and is rejected by the original class.'}
    (OUT/'audit.json').write_text(json.dumps(audit,indent=2)+'\n')
if __name__=='__main__': main()
