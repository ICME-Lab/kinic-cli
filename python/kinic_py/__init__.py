"""Pythonic API for the Kinic memory tooling."""

from __future__ import annotations

from . import _lib as native
from .memories import (
    KinicMemories,
    create_memory,
    insert_file,
    insert_markdown,
    insert_markdown_file,
    insert_raw,
    insert_pdf,
    insert_pdf_file,
    insert_text,
    list_memories,
    search_memories,
    search_raw,
    tagged_embeddings,
    export_all,
    ask_ai,
    get_balance,
    update_instance,
    reset_memory,
    add_user,
)

__all__ = [
    "KinicMemories",
    "create_memory",
    "insert_file",
    "insert_markdown",
    "insert_markdown_file",
    "insert_raw",
    "insert_pdf_file",
    "insert_pdf",
    "insert_text",
    "list_memories",
    "search_memories",
    "search_raw",
    "tagged_embeddings",
    "export_all",
    "ask_ai",
    "get_balance",
    "update_instance",
    "reset_memory",
    "add_user",
    "native",
    "__version__",
]
__version__ = "0.1.2"
