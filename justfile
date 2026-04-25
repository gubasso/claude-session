# list available recipes
default:
  @just --list

# install claude-session
install PREFIX="":
  PREFIX="{{PREFIX}}" ./install.sh

# remove files listed in the install manifest
uninstall PREFIX="":
  PREFIX="{{PREFIX}}" ./uninstall.sh

# run unit + integration tests
test: test-unit test-integration

# run unit bats tests
test-unit:
  @if ! command -v bats >/dev/null 2>&1; then printf '%s\n' "bats not found; install bats-core" >&2; exit 127; fi
  bats --filter-tags 'unit,!integration' test/

# run integration bats tests
test-integration:
  @if ! command -v bats >/dev/null 2>&1; then printf '%s\n' "bats not found; install bats-core" >&2; exit 127; fi
  bats --filter-tags integration test/

# run pre-commit on all files
lint:
  pre-commit run --all-files

# run lint + test (CI gate)
check:
  just lint && just test

# install bash completions
completions PREFIX="":
  PREFIX="{{PREFIX}}" ./install.sh --completions-only

# remove generated artifacts (non-destructive)
clean:
  find test -type d -name 'bats-run-*' -prune -exec rm -rf {} +
