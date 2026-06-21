import unittest

from nvbes_foundation import health_status


class RuntimeFoundationTest(unittest.TestCase):
    def test_health_status(self) -> None:
        self.assertEqual(health_status(), "ok")


if __name__ == "__main__":
    unittest.main()
