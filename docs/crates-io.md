# crates.io release notes

Use this flow for the root `kinic-cli` package only.

## Verify package contents

```bash
cargo package --list --allow-dirty
```

## Verify publishability

```bash
cargo publish --dry-run --allow-dirty
```

## Authenticate

```bash
cargo login
```

## Publish

```bash
cargo publish
```
