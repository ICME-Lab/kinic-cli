from __future__ import annotations

import importlib
import sys
import unittest
from pathlib import Path


class NativeStub:
    def __init__(self) -> None:
        self.calls: list[tuple[str, tuple[object, ...], dict[str, object]]] = []

    def rename_memory(self, *args: object, **kwargs: object) -> None:
        self.calls.append(("rename_memory", args, kwargs))

    def insert_memory(self, *args: object, **kwargs: object) -> int:
        self.calls.append(("insert_memory", args, kwargs))
        return 1

    def insert_memory_pdf(self, *args: object, **kwargs: object) -> int:
        self.calls.append(("insert_memory_pdf", args, kwargs))
        return 1


class MemoriesApiTests(unittest.TestCase):
    def setUp(self) -> None:
        repo_root = Path(__file__).resolve().parents[2]
        sys.path.insert(0, str(repo_root / "python"))
        self.native = NativeStub()
        sys.modules["kinic_py._lib"] = self.native
        sys.modules.pop("kinic_py", None)
        sys.modules.pop("kinic_py.memories", None)
        self.memories = importlib.import_module("kinic_py.memories")

    def test_rename_memory_passes_description_update(self) -> None:
        self.memories.rename_memory(
            "default",
            "aaaaa-aa",
            "Docs",
            description="reference",
            ic=True,
        )

        self.assertEqual(
            self.native.calls,
            [
                (
                    "rename_memory",
                    ("default", "aaaaa-aa", "Docs"),
                    {
                        "description": "reference",
                        "clear_description": False,
                        "ic": True,
                    },
                )
            ],
        )

    def test_rename_memory_rejects_description_conflict(self) -> None:
        with self.assertRaises(ValueError):
            self.memories.rename_memory(
                "default",
                "aaaaa-aa",
                "Docs",
                description="reference",
                clear_description=True,
            )

    def test_insert_markdown_file_accepts_auto_tag_form(self) -> None:
        result = self.memories.insert_markdown_file("default", "aaaaa-aa", "./note.md")

        self.assertEqual(result, 1)
        self.assertEqual(
            self.native.calls,
            [
                (
                    "insert_memory",
                    ("default", "aaaaa-aa", None),
                    {"file_path": "./note.md", "ic": None},
                )
            ],
        )

    def test_insert_markdown_file_accepts_legacy_tag_keyword(self) -> None:
        result = self.memories.insert_markdown_file(
            "default",
            "aaaaa-aa",
            tag="manual",
            path="./note.md",
        )

        self.assertEqual(result, 1)
        self.assertEqual(
            self.native.calls,
            [
                (
                    "insert_memory",
                    ("default", "aaaaa-aa", "manual"),
                    {"file_path": "./note.md", "ic": None},
                )
            ],
        )

    def test_insert_pdf_file_keeps_legacy_tag_path_form(self) -> None:
        result = self.memories.insert_pdf_file(
            "default",
            "aaaaa-aa",
            "manual",
            "./paper.pdf",
        )

        self.assertEqual(result, 1)
        self.assertEqual(
            self.native.calls,
            [
                (
                    "insert_memory_pdf",
                    ("default", "aaaaa-aa", "manual", "./paper.pdf"),
                    {"ic": None},
                )
            ],
        )

    def test_insert_pdf_file_accepts_auto_tag_form(self) -> None:
        result = self.memories.insert_pdf_file("default", "aaaaa-aa", "./paper.pdf")

        self.assertEqual(result, 1)
        self.assertEqual(
            self.native.calls,
            [
                (
                    "insert_memory_pdf",
                    ("default", "aaaaa-aa", None, "./paper.pdf"),
                    {"ic": None},
                )
            ],
        )

    def test_insert_pdf_file_accepts_legacy_tag_keyword(self) -> None:
        result = self.memories.insert_pdf_file(
            "default",
            "aaaaa-aa",
            tag="manual",
            path="./paper.pdf",
        )

        self.assertEqual(result, 1)
        self.assertEqual(
            self.native.calls,
            [
                (
                    "insert_memory_pdf",
                    ("default", "aaaaa-aa", "manual", "./paper.pdf"),
                    {"ic": None},
                )
            ],
        )

    def test_insert_markdown_file_rejects_ambiguous_tag_forms(self) -> None:
        with self.assertRaises(ValueError):
            self.memories.insert_markdown_file(
                "default",
                "aaaaa-aa",
                "manual",
                path="./note.md",
                tag="other",
            )

    def test_kinic_memories_insert_pdf_file_accepts_legacy_tag_keyword(self) -> None:
        client = self.memories.KinicMemories("default")

        result = client.insert_pdf_file(
            "aaaaa-aa",
            tag="manual",
            path="./paper.pdf",
        )

        self.assertEqual(result, 1)
        self.assertEqual(
            self.native.calls,
            [
                (
                    "insert_memory_pdf",
                    ("default", "aaaaa-aa", "manual", "./paper.pdf"),
                    {"ic": False},
                )
            ],
        )

    def test_deprecated_insert_file_accepts_legacy_tag_keyword(self) -> None:
        with self.assertWarns(DeprecationWarning):
            result = self.memories.insert_file(
                "default",
                "aaaaa-aa",
                tag="manual",
                path="./note.md",
            )

        self.assertEqual(result, 1)
        self.assertEqual(
            self.native.calls,
            [
                (
                    "insert_memory",
                    ("default", "aaaaa-aa", "manual"),
                    {"file_path": "./note.md", "ic": None},
                )
            ],
        )


if __name__ == "__main__":
    unittest.main()
