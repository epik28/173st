"""Structural and provenance checks for the native movement corpus."""
import json
import math
from pathlib import Path
import re
import struct
import unittest

from tools.oracle import movement_corpus as movement

ROOT=Path(__file__).resolve().parents[2]
FIXTURE=ROOT/"tests/fixtures/movement"
RAW64=re.compile(r"^[0-9a-f]{16}$")
RAW32=re.compile(r"^[0-9a-f]{8}$")


def decoded(raw, fmt):
    return struct.unpack(fmt,bytes.fromhex(raw))[0]


class MovementCorpusTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.input=(FIXTURE/"input.tsv").read_bytes()
        cls.reference=(FIXTURE/"reference.tsv").read_bytes()
        cls.audit=json.loads((FIXTURE/"audit.json").read_text())
        cls.registry=(FIXTURE/"registry.stdout").read_bytes()
        cls.inputs=[line.split("\t") for line in cls.input.decode("ascii").splitlines()]
        cls.outputs=[line.split("\t") for line in cls.reference.decode("ascii").splitlines()]

    def test_generator_is_deterministic_and_protocol_bounded(self):
        self.assertEqual(movement.corpus(),self.input)
        self.assertLessEqual(len(self.input),16*1024*1024)
        self.assertLessEqual(len(self.inputs),4096)
        self.assertEqual(self.input[-1:],b"\n")
        self.assertEqual(len({row[0] for row in self.inputs}),len(self.inputs))
        total=0; travel_sequences=[]; modes=set(); input_flag_union=0
        for row in self.inputs:
            self.assertEqual(len(row),6)
            case,mode,initial,boxes,surface,frames=row
            self.assertRegex(case,r"^[A-Za-z0-9_.:-]{1,96}$")
            self.assertIn(mode,{"move","travel"}); modes.add(mode)
            self.assertIn(surface,{"0","1","79"})
            components=initial.split(",")
            self.assertEqual(len(components),18)
            for raw in components[:12]:
                self.assertRegex(raw,RAW64)
                self.assertTrue(math.isfinite(decoded(raw,">d")))
                self.assertLessEqual(abs(decoded(raw,">d")),30_000_000)
            for raw in components[12:17]:
                self.assertRegex(raw,RAW32)
                self.assertTrue(math.isfinite(decoded(raw,">f")))
            flags=int(components[17]); self.assertGreaterEqual(flags,0); self.assertLessEqual(flags,255)
            input_flag_union|=flags
            if boxes!="-":
                self.assertLessEqual(len(boxes.split(":")),128)
                for raw_box in boxes.split(":"):
                    values=raw_box.split(",")
                    self.assertEqual(len(values),6)
                    nums=[decoded(v,">d") for v in values]
                    self.assertTrue(all(math.isfinite(v) and abs(v)<=30_000_000 for v in nums))
                    self.assertTrue(all(nums[i+3]>nums[i] for i in range(3)))
            frame_list=frames.split(";")
            self.assertGreaterEqual(len(frame_list),1); self.assertLessEqual(len(frame_list),256)
            total+=len(frame_list)
            if mode=="move":
                for frame in frame_list:
                    args=frame.split(","); self.assertEqual(len(args),3)
                    for raw in args:
                        self.assertRegex(raw,RAW64)
                        self.assertLessEqual(abs(decoded(raw,">d")),32)
            else:
                travel_sequences.append(frame_list)
                for frame in frame_list:
                    args=frame.split(","); self.assertEqual(len(args),5)
                    for raw in args[:3]: self.assertRegex(raw,RAW32)
                    self.assertIn(args[3],{"0","1"}); self.assertIn(args[4],{"0","1"})
        self.assertGreaterEqual(total,5000); self.assertLessEqual(total,100000)
        self.assertEqual(modes,{"move","travel"})
        self.assertGreaterEqual(sum(len(frames)>=30 for frames in travel_sequences),30)
        self.assertEqual(input_flag_union&0xf1,0xf1)

    def test_reference_has_exact_ordered_states_and_full_flag_coverage(self):
        self.assertEqual(len(self.outputs),sum(len(row[5].split(";")) for row in self.inputs))
        expected=[]
        for row in self.inputs:
            expected.extend((row[0],str(i)) for i,_ in enumerate(row[5].split(";")))
        actual=[]; flag_union=0
        for line in self.outputs:
            self.assertEqual(len(line),3)
            self.assertRegex(line[1],r"^(0|[1-9][0-9]*)$")
            values=line[2].split(",")
            self.assertEqual(len(values),18)
            for raw in values[:12]:
                self.assertRegex(raw,RAW64)
                self.assertTrue(math.isfinite(decoded(raw,">d")))
            for raw in values[12:17]:
                self.assertRegex(raw,RAW32)
                self.assertTrue(math.isfinite(decoded(raw,">f")))
            flags=int(values[17]); self.assertGreaterEqual(flags,0); self.assertLessEqual(flags,255)
            flag_union|=flags
            actual.append((line[0],line[1]))
        self.assertEqual(actual,expected)
        self.assertEqual(flag_union,255)

    def test_native_audit_and_registry_capture_are_pinned(self):
        a=self.audit
        self.assertEqual(a["schema_version"],1); self.assertEqual(a["suite"],"movement")
        self.assertEqual(a["jar_sha256"],movement.JAR_SHA)
        self.assertEqual(a["class_sha256"],movement.CLASS_SHA)
        self.assertEqual(a["method_mapping"],movement.METHOD_MAPPING)
        self.assertEqual(a["field_mapping"],movement.FIELD_MAPPING)
        self.assertEqual(a["surfaces"],movement.SURFACES)
        self.assertEqual(a["case_count"],len(self.inputs)); self.assertEqual(a["state_count"],len(self.outputs))
        self.assertEqual(a["input_sha256"],movement.sha(self.input))
        self.assertEqual(a["output_sha256"],movement.sha(self.reference))
        self.assertEqual(a["repeat_output_sha256"],a["output_sha256"])
        self.assertEqual(a["oracle_sha256"],movement.sha((ROOT/"tools/oracle/src/MovementOracle.java").read_bytes()))
        self.assertEqual(a["invocations"],movement.expected_invocations(self.input,self.reference))
        self.assertEqual(a["native_world_constructors"],0)
        self.assertEqual(a["environment"],"synthetic-static-dry")
        self.assertIsInstance(a["collision_queries"],int); self.assertGreaterEqual(a["collision_queries"],0)
        self.assertEqual(set(a["code_sources"]),set(movement.CLASS_SHA))
        expected_source=movement.paths().as_uri()
        self.assertEqual(set(a["code_sources"].values()),{expected_source})
        self.assertEqual(a["registry_stdout_bytes"],len(self.registry))
        self.assertEqual(a["registry_stdout_sha256"],movement.sha(self.registry))

    def test_boundary_scenarios_include_sneak_step_gap_and_world_flags(self):
        by_id={row[0]:row for row in self.inputs}
        for case in ("move-step-disabled","move-step-tie","move-step-tie-disabled",
                     "move-sneak-corner","move-support-gap","move-no-clip","move-web-scale",
                     "move-allow-slide-off","move-negative-corner","move-ceiling","move-ice-floor"):
            self.assertIn(case,by_id)
        travel=[row for row in self.inputs if row[1]=="travel"]
        self.assertTrue(any(row[4]=="79" for row in travel))
        self.assertTrue(any(row[4]=="1" for row in travel))
        self.assertTrue(any(row[4]=="0" and row[3]=="-" for row in travel))
        self.assertTrue(any(any(frame.split(",")[3]=="1" for frame in row[5].split(";")) for row in travel))
        states_by_id={}
        for row in self.outputs:
            states_by_id.setdefault(row[0],[]).append(bool(int(row[2].split(",")[-1])&1))
        airborne=[]; landed=[]
        for row in travel:
            states=states_by_id[row[0]]
            left_ground=any(states[i] and not states[i+1] for i in range(len(states)-1))
            returned=any(not states[i] and states[i+1] for i in range(len(states)-1))
            if left_ground: airborne.append(row[0])
            if left_ground and returned: landed.append(row[0])
        self.assertGreaterEqual(len(airborne),30)
        self.assertGreaterEqual(len(landed),30)
        frame_text=";".join(row[5] for row in travel)
        frames=[frame.split(",") for frame in frame_text.split(";")]
        yaw_values={decoded(frame[2],">f") for frame in frames}
        self.assertTrue({-90.0,0.0,90.0,180.0}.issubset(yaw_values))
        self.assertTrue(any(decoded(frame[0],">f")!=0 and decoded(frame[1],">f")!=0 for frame in frames))
        self.assertTrue(any(decoded(frame[0],">f")==0 and decoded(frame[1],">f")==0 for frame in frames))
        self.assertTrue(any(decoded(row[2].split(",")[0],">d")<0 for row in self.inputs))
        rows_by_id={row[0]:row for row in self.outputs}
        tied=rows_by_id["move-step-tie"][2].split(",")
        disabled=rows_by_id["move-step-tie-disabled"][2].split(",")
        self.assertEqual(tied[:12],disabled[:12])
        self.assertEqual(tied[17],disabled[17])


if __name__=="__main__": unittest.main()
