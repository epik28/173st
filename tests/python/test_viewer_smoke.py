import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT))

from tools import viewer_smoke


class ViewerSmokeTest(unittest.TestCase):
    def test_capture_retries_until_a_nonblack_frame_is_presented(self):
        captures = []
        readiness = iter([False, False, True])
        sleeps = []

        attempts = viewer_smoke.wait_for_rendered_frame(
            lambda: captures.append("capture"),
            lambda: next(readiness),
            timeout=2.0,
            interval=0.1,
            clock=lambda: 0.0,
            sleep=sleeps.append,
        )

        self.assertEqual(len(captures), 3)
        self.assertEqual(attempts, 3)
        self.assertEqual(sleeps, [0.1, 0.1])

    def test_capture_fails_if_window_never_presents_pixels(self):
        clock_values = iter([0.0, 0.0, 0.0, 1.0])
        with self.assertRaisesRegex(TimeoutError, "did not present"):
            viewer_smoke.wait_for_rendered_frame(
                lambda: None,
                lambda: False,
                timeout=0.5,
                interval=0.1,
                clock=lambda: next(clock_values),
                sleep=lambda _: None,
            )


if __name__ == "__main__":
    unittest.main()
