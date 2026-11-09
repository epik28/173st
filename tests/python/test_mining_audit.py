import unittest
from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[2]))
from tools.oracle.mining_audit import audit


class MiningAuditTest(unittest.TestCase):
    def test_pinned_native_behavior(self):
        result = audit()
        self.assertTrue(all(result["checks"].values()))
        self.assertEqual(result["mutations"], [])
        self.assertEqual(result["descriptors"]["ob.place_item"], "(Lgs;Lfd;Liz;IIII)Z")
        self.assertEqual(result["descriptors"]["dn.break_overlay"], "(IIII)V")
        self.assertEqual(result["class_sha256"]["dn.class"],
                         "a4992db8f7765066458ef19fd471933a0ac5af61257bd1dbe08f67a5be69b57d")
        self.assertIn("empty mutation", "empty mutation list")


if __name__ == "__main__":
    unittest.main()
