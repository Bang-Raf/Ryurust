# Contributing to Ryurust

Thanks for helping. This guide covers how to set up, what a good pull request
looks like, and how it gets reviewed and merged.

## Ground rules

- Be respectful. Disagree about code, not people.
- One topic per pull request. A bug fix and a refactor are two PRs.
- Performance claims need numbers from `cargo bench`, not estimates.
- Don't add hard-coded "baseline" or "target" numbers to code or docs. If you
  compare against something, measure both sides in the same run.
- Never commit Nintendo keys, firmware, game files, or anything copyrighted
  that you don't own.

## 1. Set up

```bash
git clone git@github.com:Bang-Raf/Ryurust.git
cd Ryurust
rustup toolchain install stable
rustup component add rustfmt clippy
cargo build --workspace
cargo test --workspace
```

See [`docs/DEVELOPMENT.md`](docs/DEVELOPMENT.md) for the layout and conventions.

## 2. Pick or open an issue

- For anything bigger than a typo, open (or comment on) an issue first so the
  approach can be agreed before you write code.
- Bug reports and feature requests use the templates in
  `.github/ISSUE_TEMPLATE/`.

## 3. Branch

Fork the repository, then branch from an up-to-date `main`:

```bash
git remote add upstream git@github.com:Bang-Raf/Ryurust.git
git fetch upstream
git checkout -b perf/texture-bcn-bc7 upstream/main
```

Branch name format: `<type>/<short-description>`.

| Prefix | Use for |
| :--- | :--- |
| `feat/` | new functionality or a new crate |
| `fix/` | bug fixes |
| `perf/` | performance work (must include benchmark results) |
| `refactor/` | code changes with no behavior change |
| `docs/` | documentation only |
| `test/` | tests or benchmarks only |
| `ci/` | CI and tooling |

## 4. Commit

Use [Conventional Commits](https://www.conventionalcommits.org/). The scope is
the crate name without the `ryu-` prefix.

```text
perf(texture-bcn): decode BC7 mode 6 with AVX2 shuffles
fix(memory-pagetable): handle unmapped L2 entry on read
feat(ipc-parser): parse type-0x5 control messages
docs(readme): update benchmark table
```

- Subject line: imperative, lower case, no trailing period, 72 characters max.
- Body: explain *why* when it isn't obvious.
- Breaking changes to an exported C ABI: add `!` after the scope and a
  `BREAKING CHANGE:` footer describing what callers must change.

## 5. Check before you push

All of these must pass; CI runs the same commands.

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo bench -p ryurust-bench -- --test      # every benchmark compiles and runs once
```

For changes to code on a hot path, also run the relevant benchmark before and
after your change on the same machine:

```bash
git stash && cargo bench -p ryurust-bench --bench texture_bcn -- --save-baseline before
git stash pop && cargo bench -p ryurust-bench --bench texture_bcn -- --baseline before
```

Paste Criterion's change summary into the PR, plus your CPU model and OS.

## 6. Open the pull request

- Target branch: `main`.
- Fill in every section of the PR template (it loads automatically).
- Keep the PR small enough to review in one sitting; split it otherwise.
- Mark it as a draft while it's still in progress.

### What reviewers check

1. **Correctness** — tests cover the change, including edge cases (empty input,
   misaligned sizes, maximum values). Output matches the Ryujinx C#
   implementation for the same input.
2. **Safety** — every `unsafe` block has a `// SAFETY:` comment explaining why
   it's sound. FFI functions validate null pointers and lengths.
3. **ABI stability** — exported `extern "C"` signatures don't change without a
   version bump and a matching update in `bindings/csharp/`.
4. **Performance** — for `perf/` PRs, the benchmark shows an improvement
   outside Criterion's noise threshold and nothing else regressed.
5. **Portability** — SIMD paths have a scalar fallback and runtime feature
   detection (`is_x86_feature_detected!`); the crate still builds on non-x86.

## 7. Review and merge

- At least one maintainer approval is required.
- CI must be green.
- Address review comments with new commits; don't force-push during review
  unless asked. Resolve a conversation only after the reviewer agrees.
- Maintainers **squash-merge**: the PR title becomes the commit subject, so it
  must follow the Conventional Commits format too.
- After merge, delete your branch.

## Adding a new crate

1. `cargo new --lib crates/ryu-<name>` and add it to `members` in the root
   `Cargo.toml`.
2. Set `crate-type = ["cdylib", "rlib"]` if C# will call it, and put every
   exported function in an `ffi` module with `#[no_mangle] pub unsafe extern "C"`.
3. Add unit tests that compare against known-good output from Ryujinx.
4. Add `benches/benches/<name>.rs` and a `[[bench]]` entry in
   `benches/Cargo.toml`.
5. If it has a C# wrapper, add it under `bindings/csharp/`, with a managed
   fallback when the library isn't found (see the existing wrappers).
6. Document it in the crate table in `README.md`.

## Reporting security issues

Don't open a public issue for memory-safety bugs that could be triggered by a
game file or save data. Contact the maintainer through GitHub
([@Bang-Raf](https://github.com/Bang-Raf)) privately first.

## License

By contributing you agree that your contributions are licensed under the MIT
License in [`LICENSE`](LICENSE).
