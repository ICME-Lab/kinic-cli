"""High-level helpers for managing Kinic memories via the Rust core."""

from __future__ import annotations

import warnings
from typing import List, Sequence, Tuple

from . import _lib as native

ScoreResult = Sequence[Tuple[float, str]]
ExportResult = Sequence[Tuple[int, List[float], str]]


class KinicMemories:
    """Stateful helper that mirrors the Rust CLI behavior."""

    def __init__(self, identity: str, *, ic: bool = False) -> None:
        self.identity = identity
        self.ic = ic

    def create(self, name: str, description: str) -> str:
        """Deploy a new memory canister."""
        return create_memory(self.identity, name, description, ic=self.ic)

    def list(self) -> List[str]:
        """List deployed memories."""
        return list_memories(self.identity, ic=self.ic)

    def insert_markdown(self, memory_id: str, tag: str, text: str) -> int:
        """Insert markdown text directly."""
        return insert_markdown(self.identity, memory_id, tag, text, ic=self.ic)

    def insert_markdown_file(self, memory_id: str, tag: str, path: str) -> int:
        """Insert markdown loaded from disk."""
        return insert_markdown_file(self.identity, memory_id, tag, path, ic=self.ic)

    def insert_raw(self, memory_id: str, tag: str, text: str, embedding: Sequence[float]) -> int:
        """Insert a precomputed embedding with text."""
        return insert_raw(self.identity, memory_id, tag, text, embedding, ic=self.ic)

    def insert_pdf_file(self, memory_id: str, tag: str, path: str) -> int:
        """Convert a PDF to markdown and insert it."""
        return insert_pdf_file(self.identity, memory_id, tag, path, ic=self.ic)

    def insert_pdf(self, memory_id: str, tag: str, path: str) -> int:
        """Deprecated: use insert_pdf_file instead."""
        warnings.warn("insert_pdf is deprecated; use insert_pdf_file", DeprecationWarning, stacklevel=2)
        return self.insert_pdf_file(memory_id, tag, path)

    def insert_text(self, memory_id: str, tag: str, text: str) -> int:
        """Deprecated: use insert_markdown instead."""
        warnings.warn("insert_text is deprecated; use insert_markdown", DeprecationWarning, stacklevel=2)
        return self.insert_markdown(memory_id, tag, text)

    def insert_file(self, memory_id: str, tag: str, path: str) -> int:
        """Deprecated: use insert_markdown_file instead."""
        warnings.warn("insert_file is deprecated; use insert_markdown_file", DeprecationWarning, stacklevel=2)
        return self.insert_markdown_file(memory_id, tag, path)

    def search(self, memory_id: str, query: str) -> ScoreResult:
        """Search the specified memory canister."""
        return search_memories(self.identity, memory_id, query, ic=self.ic)

    def search_raw(self, memory_id: str, embedding: Sequence[float]) -> ScoreResult:
        """Search using a precomputed embedding."""
        return search_raw(self.identity, memory_id, embedding, ic=self.ic)

    def tagged_embeddings(self, memory_id: str, tag: str) -> List[List[float]]:
        """Fetch all embeddings associated with a tag."""
        return tagged_embeddings(self.identity, memory_id, tag, ic=self.ic)

    def export_all(self, memory_id: str) -> ExportResult:
        """Export all embeddings and data from a memory canister."""
        return export_all(self.identity, memory_id, ic=self.ic)

    def ask_ai(
        self,
        memory_id: str,
        query: str,
        *,
        top_k: int | None = None,
        language: str | None = None,
    ) -> tuple[str, str]:
        """Run ask-ai (search + LLM) and return (prompt, answer)."""
        return ask_ai(
            self.identity,
            memory_id,
            query,
            top_k=top_k,
            language=language,
            ic=self.ic,
        )

    def balance(self) -> tuple[int, float]:
        """Return (base_units, kinic) balance for the current identity."""
        return get_balance(self.identity, ic=self.ic)

    def update(self, memory_id: str) -> None:
        """Trigger launcher update_instance for the memory canister."""
        update_instance(self.identity, memory_id, ic=self.ic)

    def reset(self, memory_id: str, dim: int) -> None:
        """Reset a memory canister and set embedding dimension."""
        reset_memory(self.identity, memory_id, dim, ic=self.ic)

    def add_user(self, memory_id: str, user_id: str, role: str) -> None:
        """Configure visibility: add a user (principal or 'anonymous') with a role (admin/writer/reader)."""
        add_user(self.identity, memory_id, user_id, role, ic=self.ic)


def create_memory(
    identity: str,
    name: str,
    description: str,
    *,
    ic: bool | None = None,
) -> str:
    return native.create_memory(identity, name, description, ic=ic)


def list_memories(identity: str, *, ic: bool | None = None) -> List[str]:
    return native.list_memories(identity, ic=ic)


def insert_markdown(
    identity: str,
    memory_id: str,
    tag: str,
    text: str,
    *,
    ic: bool | None = None,
) -> int:
    return native.insert_memory(identity, memory_id, tag, text=text, ic=ic)


def insert_markdown_file(
    identity: str,
    memory_id: str,
    tag: str,
    path: str,
    *,
    ic: bool | None = None,
) -> int:
    return native.insert_memory(identity, memory_id, tag, file_path=path, ic=ic)


def insert_raw(
    identity: str,
    memory_id: str,
    tag: str,
    text: str,
    embedding: Sequence[float],
    *,
    ic: bool | None = None,
) -> int:
    return native.insert_memory_raw(identity, memory_id, tag, text, list(embedding), ic=ic)


def insert_pdf_file(
    identity: str,
    memory_id: str,
    tag: str,
    path: str,
    *,
    ic: bool | None = None,
) -> int:
    return native.insert_memory_pdf(identity, memory_id, tag, path, ic=ic)


def insert_pdf(
    identity: str,
    memory_id: str,
    tag: str,
    path: str,
    *,
    ic: bool | None = None,
) -> int:
    warnings.warn("insert_pdf is deprecated; use insert_pdf_file", DeprecationWarning, stacklevel=2)
    return insert_pdf_file(identity, memory_id, tag, path, ic=ic)


def insert_text(
    identity: str,
    memory_id: str,
    tag: str,
    text: str,
    *,
    ic: bool | None = None,
) -> int:
    warnings.warn("insert_text is deprecated; use insert_markdown", DeprecationWarning, stacklevel=2)
    return insert_markdown(identity, memory_id, tag, text, ic=ic)


def insert_file(
    identity: str,
    memory_id: str,
    tag: str,
    path: str,
    *,
    ic: bool | None = None,
) -> int:
    warnings.warn("insert_file is deprecated; use insert_markdown_file", DeprecationWarning, stacklevel=2)
    return insert_markdown_file(identity, memory_id, tag, path, ic=ic)


def search_memories(
    identity: str,
    memory_id: str,
    query: str,
    *,
    ic: bool | None = None,
) -> ScoreResult:
    return native.search_memories(identity, memory_id, query, ic=ic)


def search_raw(
    identity: str,
    memory_id: str,
    embedding: Sequence[float],
    *,
    ic: bool | None = None,
) -> ScoreResult:
    return native.search_memories_raw(identity, memory_id, list(embedding), ic=ic)


def tagged_embeddings(
    identity: str,
    memory_id: str,
    tag: str,
    *,
    ic: bool | None = None,
) -> List[List[float]]:
    return native.tagged_embeddings(identity, memory_id, tag, ic=ic)


def export_all(
    identity: str,
    memory_id: str,
    *,
    ic: bool | None = None,
) -> ExportResult:
    return native.export_all(identity, memory_id, ic=ic)


def ask_ai(
    identity: str,
    memory_id: str,
    query: str,
    *,
    top_k: int | None = None,
    language: str | None = None,
    ic: bool | None = None,
) -> tuple[str, str]:
    return native.ask_ai(identity, memory_id, query, top_k=top_k, language=language, ic=ic)


def get_balance(identity: str, *, ic: bool | None = None) -> tuple[int, float]:
    return native.get_balance(identity, ic=ic)


def update_instance(identity: str, memory_id: str, *, ic: bool | None = None) -> None:
    return native.update_instance(identity, memory_id, ic=ic)


def reset_memory(
    identity: str,
    memory_id: str,
    dim: int,
    *,
    ic: bool | None = None,
) -> None:
    return native.reset_memory(identity, memory_id, dim, ic=ic)


def add_user(
    identity: str,
    memory_id: str,
    user_id: str,
    role: str,
    *,
    ic: bool | None = None,
) -> None:
    return native.add_user(identity, memory_id, user_id, role, ic=ic)
