import csv
import json
import unittest
from pathlib import Path

from tools.oracle import items_corpus


class ItemsCorpusTests(unittest.TestCase):
    def test_native_capture_repeats_and_matches_ordered_fixture(self):
        jar = items_corpus.jar_path()
        with __import__("tempfile").TemporaryDirectory(prefix="items-native-test-") as temp:
            first, audit = items_corpus.capture(jar, Path(temp) / "first")
            second, repeated_audit = items_corpus.capture(jar, Path(temp) / "repeat")
        self.assertEqual(first, second)
        self.assertEqual(audit, repeated_audit)
        self.assertEqual(first, (items_corpus.FIXTURES / "native.tsv").read_bytes())
        pinned = json.loads((items_corpus.FIXTURES / "audit.json").read_text())
        self.assertEqual(audit["jar_sha256"], pinned["jar_sha256"])
        self.assertEqual(audit["classes"], pinned["classes"])
        self.assertEqual(audit["native_methods"], pinned["native_methods"])

    def test_capture_rows_are_ordered_and_cover_all_main_slots(self):
        lines = (items_corpus.FIXTURES / "native.tsv").read_text().splitlines()
        self.assertEqual(len(lines), 21)
        for line in lines:
            scenario, step, result, selected, selected_stack, inventory = line.split("\t")
            self.assertTrue(scenario and step)
            self.assertTrue(selected.lstrip("-").isdigit())
            self.assertEqual(len(inventory.split("|")), 36)
            for stack in inventory.split("|"):
                if stack != "-":
                    item_id, count, damage, limit, name = stack.split(",", 4)
                    self.assertTrue(item_id.isdigit() and count.isdigit() and damage.lstrip("-").isdigit())
                    self.assertTrue(limit.isdigit() and name)
            if selected_stack != "-":
                self.assertEqual(len(selected_stack.split(",", 4)), 5)


if __name__ == "__main__":
    unittest.main()
