#  Kinic CLI - Trustless Agentic Memory

**OPEN BETA v0.1** - star, share, and provide feedback :)

Python bindings for the Kinic CLI core, enabling you to build AI agents with verifiable, owned memory on the Internet Computer. Give your agents the memory they can prove, own, and carry anywhere.

**For the wizards building trustless agents** - no more lobotomized summons that reset on every quest.

Looking for the CLI docs? See `docs/cli.md`.

Made with ❤️ by [ICME Labs](https://blog.icme.io/).

<img width="983" height="394" alt="icme_labs" src="https://github.com/user-attachments/assets/ffc334ed-c301-4ce6-8ca3-a565328904fe" />

---

## What is Trustless Agentic Memory?

Traditional AI agents face three critical problems:

1. **Memory Without Proof**: TEEs prove computation, but can't verify the memories your agent retrieved were correct.
2. **Memory Without Ownership**: Your agent's identity is onchain, but its memories live in Pinecone, Weaviate, or other centralized providers.
3. **Payment Without Verification**: With x402, agents can pay for memory retrieval - but can't prove they received the right results.

Kinic solves this with **zkTAM** (zero-knowledge Trustless Agentic Memory):
- ✅ **Verifiable**: zkML proofs for embeddings - no black box vectors
- ✅ **Owned**: Your memory lives on-chain in WASM canisters you control
- ✅ **Portable**: Move your agent's memory between any infrastructure

By default we use the Internet Computer as the DA layer—with VetKey encryption and cross-chain signing (tECDSA). You can run it locally or on any WASM-based DA layer. In future versions, full zkML support will be enabled, allowing for trustless verification on nearly all blockchains.

---

## Prerequisites

- **Python 3.9+**
- **dfx 0.28+** with the `arm64` build on Apple Silicon (macOS)
- **KINIC tokens**: At least 1 KINIC to deploy memory canisters
- **dfx identity**: Create or select one with `dfx identity new <name>` or `dfx identity use <name>`

> Note: Do not use the `default` identity with `kinic-cli`—it always fails. Use a named identity instead.

Optional: If you need local launcher/ledger/II canisters, run `./scripts/setup.sh` after `dfx start --clean --background`.

---

## Installation

### From PyPI (Recommended)
```bash
pip install kinic-py

# Or with uv
uv pip install kinic-py
```

### From Source

Requires Rust toolchain for the PyO3 extension:
```bash
pip install -e .

# Or with uv
uv pip install -e .
```

---

## Quickstart

### 1. Create or Select Your Identity

Create (or switch to) a dfx identity before using the library:

```bash
dfx identity new <name>
# or
dfx identity use <name>
```

### 2. Check Your Balance

Make sure you have at least 1 KINIC token:
```bash
# Get your principal
dfx --identity <name> identity get-principal

# Check balance (result is in base units: 100000000 = 1 KINIC)
dfx canister --ic call 73mez-iiaaa-aaaaq-aaasq-cai icrc1_balance_of '(record {owner = principal "<your principal>"; subaccount = null; }, )'

# Example: (100000000 : nat) == 1 KINIC
```
**DM https://x.com/wyatt_benno for KINIC prod tokens** with your principal ID.

Or purchase them from MEXC or swap at https://app.icpswap.com/ . 

### 3. Deploy and Use Memory
```python
from kinic_py import KinicMemories

km = KinicMemories("<name>")  # dfx identity name; use ic=True for mainnet, e.g. KinicMemories("<name>", ic=True)
memory_id = km.create("Python demo", "Created via kinic_py")

km.insert_markdown(memory_id, "notes", "# Hello Kinic!\n\nInserted from Python.")

for score, payload in km.search(memory_id, "Hello"):
    print(f"{score:.4f} -> {payload}")  # payload is the JSON stored in insert
```

---

## API Reference

### Class: `KinicMemories`

Stateful helper that mirrors the CLI behavior.
```python
KinicMemories(identity: str, ic: bool = False)
```

**Parameters:**
- `identity`: Your dfx identity name
- `ic`: Set `True` to target mainnet (default: `False` for local)

### Methods

#### `create(name: str, description: str) -> str`
Deploy a new memory canister.

**Returns:** Canister principal (memory_id)

#### `list() -> List[str]`
List all memory canisters owned by your identity.

#### `insert_markdown(memory_id: str, tag: str, text: str) -> int`
Embed and store markdown text with zkML verification.

**Returns:** Number of chunks inserted

#### `insert_markdown_file(memory_id: str, tag: str, path: str) -> int`
Embed and store markdown from a file.

**Returns:** Number of chunks inserted

#### `search(memory_id: str, query: str) -> List[Tuple[float, str]]`
Search memories with semantic similarity.

**Returns:** List of `(score, payload)` tuples sorted by relevance

### Module-Level Functions

Stateless alternatives available:
- `create_memory(identity, name, description, ic=False)`
- `list_memories(identity, ic=False)`
- `insert_markdown(identity, memory_id, tag, text, ic=False)`
- `insert_markdown_file(identity, memory_id, tag, path, ic=False)`
- `search_memories(identity, memory_id, query, ic=False)`

---

## Example: Full Demo Script

Run the complete example at `python/examples/memories_demo.py`:
```bash
# With existing memory
uv run python python/examples/memories_demo.py \
  --identity <name> \
  --memory-id 

# Deploy new memory
uv run python python/examples/memories_demo.py --identity <name>

# Use mainnet
uv run python python/examples/memories_demo.py --identity <name> --ic
```

---

## Use Cases

### ERC-8004 Agents
Build agents with verifiable memory that works with the ERC-8004 trust model:
```python
km = KinicMemories("agent-identity", ic=True)
memory_id = km.create("Trading Agent Memory", "Market analysis and decisions")

# Store verified context
km.insert_markdown(memory_id, "analysis", market_report)

# Retrieve with proof
results = km.search(memory_id, "BTC trend analysis")
```

### x402 Payment Integration WIP
Agents can pay for memory operations with verifiable results:
```python
# Agent pays for retrieval via x402
# Memory operations return zkML proofs
# Agent can verify it received correct embeddings for payment
```

---

## Building the Wheel

See `docs/python-wheel.md` for packaging, testing, and PyPI upload instructions.

---

## Get Production Tokens

Ready to deploy on mainnet? **DM https://x.com/wyatt_benno for KINIC prod tokens** and start building summons with real memory.

---

## Learn More

- **Blog Post**: [Trustless AI can't work without Trustless AI Memory](https://blog.icme.io/trustless-agents-cant-work-without-trustless-agentic-memory/)
- **Vectune**: WASM-based vector database
- **[JOLT Atlas](https://github.com/ICME-Lab/jolt-atlas)**: zkML framework for embedding verification

---

**Built by wizards, for wizards.** 🧙‍♂️✨

Stop building lobotomized agents. Start building with memory they can prove.
