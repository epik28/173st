import unittest
from pathlib import Path
import sys
sys.path.insert(0, str(Path(__file__).resolve().parents[2]))
from tools.oracle.world_select_audit import audit

class WorldSelectAuditTest(unittest.TestCase):
    def test_pinned_native_behavior(self):
        result = audit()
        self.assertTrue(all(result["checks"].values()))
        self.assertEqual(result["mutations"], [])

if __name__ == "__main__":
    unittest.main()
