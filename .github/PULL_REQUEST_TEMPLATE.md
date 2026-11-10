## Summary

<!-- What does this PR change, and why? Link the issue: "Closes #123". -->

## Type

- [ ] `feat` — new functionality
- [ ] `fix` — bug fix
- [ ] `perf` — performance improvement
- [ ] `refactor` — no behavior change
- [ ] `docs` / `test` / `ci`

## Crates touched

<!-- e.g. ryu-texture-bcn, bindings/csharp/NativeTextureMatcher.cs -->

## Testing

<!-- Which tests did you add or change? How did you check output matches Ryujinx? -->

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] `cargo test --workspace`
- [ ] `cargo bench -p ryurust-bench -- --test`

## Benchmarks

<!-- Required for perf PRs and for changes on hot paths. Paste Criterion's
     before/after change summary. Delete this section otherwise. -->

- CPU:
- OS / kernel:
- Rust version:

```text
paste criterion output here
```

## ABI / breaking changes

- [ ] No exported `extern "C"` function changed
- [ ] Exported ABI changed — `bindings/csharp/` updated and version bumped

<!-- Describe what callers must change. -->

## Checklist

- [ ] Every new `unsafe` block has a `// SAFETY:` comment
- [ ] SIMD code has a scalar fallback and runtime feature detection
- [ ] Docs / README updated if behavior or public API changed
- [ ] PR title follows Conventional Commits (it becomes the squash commit)
