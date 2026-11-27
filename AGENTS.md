# Project instructions

- Read `PRD.md` for the current product scope before making changes.
- Keep related code together in small, focused modules with clear interfaces. Split a module when it takes on a separate responsibility.
- Use the simplest implementation and the smallest configuration that make the requested workflow work. Reuse existing patterns and add dependencies, options, or abstractions only when the current change needs them.
- Run the development app with `dx serve --desktop`. For Rust changes, run `cargo fmt --check`, `cargo clippy --locked --all-targets -- -D warnings`, and `cargo test --locked` before committing.

## Commits

- Make a focused commit as soon as each milestone works and has been verified. Push to `origin` on the working branch of `vimzh/xuy` and report any checks or push that could not complete.
- Use `(type) subject` with a short imperative subject. Use `(feat)` for new behavior, `(fix)` for bug fixes, and `(chore)` for maintenance. Examples include `(feat) Add notebook settings`, `(fix) Restore the last open page`, and `(chore) Update project instructions`.
