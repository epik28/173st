"""Structural checks for native geometry fixtures and operation coverage."""
import hashlib
import json
from pathlib import Path
import re
import unittest

from tools.oracle.geometry_corpus import OPS, corpus

ROOT=Path(__file__).resolve().parents[2]
FIXTURE=ROOT/"tests/fixtures/geometry"


class GeometryCorpusTests(unittest.TestCase):
    def test_input_is_deterministic_valid_and_covers_every_operation(self):
        data=corpus()
        self.assertEqual(data,(FIXTURE/"input.tsv").read_bytes())
        self.assertLessEqual(len(data),32*1024*1024)
        rows=data.decode("ascii").splitlines()
        self.assertGreaterEqual(len(rows),5000)
        ids=[]
        counts={op:0 for op in OPS}
        for row in rows:
            fields=row.split("\t")
            ids.append(fields[0])
            self.assertIn(fields[1],OPS)
            counts[fields[1]]+=1
            for arg in fields[2:]:
                self.assertRegex(arg,r"^[0-9a-f]{16}$")
                value=__import__("struct").unpack(">d",bytes.fromhex(arg))[0]
                self.assertTrue(__import__("math").isfinite(value))
                self.assertLessEqual(abs(value),30_000_000)
        self.assertEqual(len(ids),len(set(ids)))
        self.assertTrue(all(counts.values()))

    def test_native_audit_binds_classes_methods_and_counts(self):
        audit=json.loads((FIXTURE/"audit.json").read_text())
        input_data=(FIXTURE/"input.tsv").read_bytes()
        output=(FIXTURE/"reference.tsv").read_bytes()
        self.assertEqual(audit["suite"],"geometry")
        self.assertEqual(audit["schema_version"],1)
        self.assertEqual(audit["case_count"],len(input_data.splitlines()))
        self.assertEqual(audit["input_sha256"],hashlib.sha256(input_data).hexdigest())
        self.assertEqual(audit["output_sha256"],hashlib.sha256(output).hexdigest())
        self.assertEqual(audit["output_sha256"],audit["repeat_output_sha256"])
        self.assertEqual(set(audit["class_sha256"]),{"eq","bt","vf","in"})
        self.assertTrue(all(re.fullmatch(r"[0-9a-f]{64}",v) for v in audit["class_sha256"].values()))
        expected={op:sum(row.split("\t",2)[1]==op for row in input_data.decode().splitlines()) for op in OPS}
        self.assertEqual(audit["invocations"],expected)
        self.assertEqual(set(audit["method_mapping"]),set(OPS))
        lines=output.decode("ascii").splitlines()
        self.assertEqual(len(lines),len(input_data.splitlines()))
        self.assertEqual(len({line.split("\t",1)[0] for line in lines}),len(lines))
        ray_values=[line.split("\t",1)[1] for row,line in zip(input_data.decode().splitlines(),lines) if "\tbox_ray\t" in row]
        self.assertTrue(any(value=="none" for value in ray_values))
        faces={int(value.split(",",1)[0]) for value in ray_values if value!="none"}
        self.assertEqual(faces,set(range(6)))
        output_by_id={line.split("\t",1)[0]:line.split("\t",1)[1]
                      for line in lines if line.split("\t",1)[0].startswith("ray-neartie-")}
        expected_faces={
            "ray-neartie-x-positive-negative":5,
            "ray-neartie-x-negative-positive":4,
            "ray-neartie-y-positive-negative":1,
            "ray-neartie-y-negative-positive":0,
            "ray-neartie-z-positive-negative":3,
            "ray-neartie-z-negative-positive":2,
        }
        self.assertEqual(set(output_by_id),set(expected_faces))
        self.assertEqual({case:int(value.split(",",1)[0]) for case,value in output_by_id.items()},expected_faces)
        output_by_id={line.split("\t",1)[0]:line.split("\t",1)[1]
                      for line in lines if line.split("\t",1)[0].startswith("ray-neartie-")}
        expected_faces={
            "ray-neartie-x-positive-negative":5,
            "ray-neartie-x-negative-positive":4,
            "ray-neartie-y-positive-negative":1,
            "ray-neartie-y-negative-positive":0,
            "ray-neartie-z-positive-negative":3,
            "ray-neartie-z-negative-positive":2,
        }
        self.assertEqual(set(output_by_id),set(expected_faces))
        self.assertEqual({case:int(value.split(",",1)[0]) for case,value in output_by_id.items()},expected_faces)


if __name__=="__main__": unittest.main()
