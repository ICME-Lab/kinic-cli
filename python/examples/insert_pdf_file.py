"""Insert a PDF into a Kinic memory via the Python helper.

Run with:
    uv run python python/examples/insert_pdf_file.py \
        --identity alice \
        --memory-id MEMORY_CANISTER_ID \
        --file ./docs/report.pdf
"""

from __future__ import annotations

import argparse
from pathlib import Path

from kinic_py import KinicMemories


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Insert a PDF by converting it to markdown first")
    parser.add_argument("--identity", required=True, help="dfx identity name")
    parser.add_argument("--memory-id", help="existing memory canister id; if omitted, a new one is created")
    parser.add_argument("--file", required=True, help="path to the PDF to insert")
    parser.add_argument("--ic", action="store_true", help="talk to mainnet instead of local replica")
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    pdf_path = Path(args.file)
    if not pdf_path.is_file():
        raise SystemExit(f"PDF file not found: {pdf_path}")

    km = KinicMemories(args.identity, ic=args.ic)
    memory_id = args.memory_id

    if not memory_id:
        memory_id = km.create("PDF demo", "Created via insert_pdf_file example")
        print(f"Created new memory canister: {memory_id}")

    chunks = km.insert_pdf_file(str(memory_id), str(pdf_path))
    print(f"Inserted {chunks} PDF chunks into {memory_id}")

    query = pdf_path.stem.replace("_", " ")
    results = km.search(memory_id, query)
    print(f"Search results for '{query}':")
    for score, payload in results:
        print(f"- [{score:.4f}] {payload}")


if __name__ == "__main__":
    main()
