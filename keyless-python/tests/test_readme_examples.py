"""Executes every Python example of the package README.

The Rust README is compiled as a doctest (`lib.rs` ReadmeDoctests), so it cannot drift
from the crate; this test gives the Python README the same protection. Every fenced
``python``/``python3`` block must execute as written — including its hardcoded
addresses, which would change if the encoding ever drifted.

Each block runs in a fresh namespace, exactly like a user pasting one example into a
fresh interpreter. The file lives in tests/ (pruned from sdists and wheels), so it only
ever runs from the repository, where ``../README.md`` exists.
"""

import re
import unittest
from pathlib import Path

_README_PATH = Path(__file__).resolve().parents[1] / "README.md"

# A fenced code block whose info string names Python (the repository uses "python3").
_PYTHON_FENCE = re.compile(r"^```\s*python3?\s*$\n(.*?)^```\s*$", re.MULTILINE | re.DOTALL)


@unittest.skipUnless(_README_PATH.is_file(), "README.md not present (not running from the repository?)")
class TestReadmeExamples(unittest.TestCase):
    def test_every_python_block_executes(self: "TestReadmeExamples") -> None:
        blocks = _PYTHON_FENCE.findall(_README_PATH.read_text(encoding="utf-8"))
        # Catches a silent extraction regression (e.g. the fence tags being renamed):
        # the README currently holds seven Python examples.
        self.assertGreaterEqual(len(blocks), 7, "expected the README's Python examples to be found")
        for index, block in enumerate(blocks):
            with self.subTest(block=index, first_line=block.splitlines()[0]):
                # Executing the documented example verbatim is the point of the test.
                exec(compile(block, f"README.md (python block {index})", "exec"), {})  # noqa: S102
