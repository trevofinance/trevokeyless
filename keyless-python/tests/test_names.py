import unittest

import trevokeyless


class TestKeylessAddresses(unittest.TestCase):
    def test_valid_name(self: "TestKeylessAddresses") -> None:
        valid_names = ["example123", "abcdEFGHIJ", "1234567890", "name-#test"]
        for name in valid_names:
            with self.subTest(name=name):
                try:
                    trevokeyless.encode_named_address(123, name, 42)
                except ValueError:
                    self.fail(f"Valid name '{name}' was incorrectly rejected.")

    def test_invalid_name(self: "TestKeylessAddresses") -> None:
        invalid_names = [
            "example!23",
            "name@12345",
            "abcd*EFGHI",
            "name with space",
            "name_with_underscore",
            "short",
            "veeeerrrrryyyyloooongg",
        ]
        for name in invalid_names:
            with self.subTest(name=name):  # noqa: SIM117
                with self.assertRaises(ValueError):
                    trevokeyless.encode_named_address(123, name, 42)


if __name__ == "__main__":
    unittest.main()
