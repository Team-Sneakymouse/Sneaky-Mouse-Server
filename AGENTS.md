# AGENTS.md

## Repo-specific rules

- **Confirm it is still used before editing.** This Rust server (`sm-server`) may have been superseded by the Node `Sneakymouse-Server`; ask before investing in it.
- **Don't run `cargo update` blindly.** `Cargo.toml` uses `*` for `rand`, `rand_pcg`, `chrono` and `redis`; the code matches `redis::Value::Bulk` (`com.rs`), which breaks on newer redis. Keep `Cargo.lock` committed and bump redis only together with code changes.
- **Redis streams and the `layer::key` constants (`config.rs`) are a shared contract** with other services and the overlays. Never rename them without checking Sneakymouse-Server and the overlays.
- **Don't panic in the event loop.** Return `LayerError`/`Result` rather than `unwrap`/`expect`/`panic!`; a panic halts game state processing.
