# Contribuir

Use Rust estável (mínimo declarado: 1.88), mantenha Cargo.lock versionado e execute:

```sh
cargo fmt --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
```

Mudanças em layout, parser, figuras ou Mermaid devem incluir casos que exercitem o comportamento. Os testes ignorados por padrão requerem fontes TrueType, Tectonic e/ou Mermaid/Chromium; execute `cargo test --locked -- --include-ignored` após instalar esses runtimes.

Compile os exemplos com `python scripts/check-examples.py --binary target/release/carbon` (Windows: `target/release/carbon.exe`). Não versione target, output, dist, node_modules, caches nem runtimes baixados.

A gramática e as mensagens de erro precisam permanecer coerentes com o README. Evite fallback silencioso quando uma dependência ou funcionalidade solicitada não estiver disponível.
