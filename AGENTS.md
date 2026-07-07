# Repository Instructions

- Put behavior, regression, and integration-style tests under `tests/`.
- Keep `src/` focused on production code; do not leave ad hoc test modules there when a `tests/` file can cover the behavior.
- When adding a new test for existing runtime behavior, prefer extending an existing file in `tests/` before creating a new inline unit test in `src/`.
