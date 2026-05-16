# list available recipes
default:
  @just --list

# install claude-session
install PREFIX="":
  #!/usr/bin/env bash
  set -euo pipefail
  if [[ $EUID -ne 0 && -z "{{PREFIX}}" ]]; then
    mode="user mode"
  else
    mode="system mode"
  fi
  printf '==> Installing claude-session (%s)\n' "$mode"
  PREFIX="{{PREFIX}}" ./install.sh
  if [[ $EUID -ne 0 && -z "{{PREFIX}}" ]]; then
    state_dir="${XDG_STATE_HOME:-$HOME/.local/state}/claude-session"
  else
    state_dir="/var/lib/claude-session"
  fi
  manifest="$state_dir/install-manifest"
  if [[ -f "$manifest" ]]; then
    file_count=0
    bin_path=""
    completion_path=""
    lib_files=0
    cmd_files=0
    fn_files=0
    while IFS= read -r line; do
      [[ -n "$line" ]] || continue
      [[ -d "$line" ]] && continue
      file_count=$((file_count + 1))
      case "$line" in
        */bin/claude-session)
          bin_path="$line"
          ;;
        */bash-completion/completions/claude-session.bash)
          completion_path="$line"
          ;;
        */claude-session/commands/*)
          cmd_files=$((cmd_files + 1))
          ;;
        */claude-session/functions/*)
          fn_files=$((fn_files + 1))
          ;;
        */claude-session/*.sh)
          lib_files=$((lib_files + 1))
          ;;
      esac
    done <"$manifest"
    printf '==> Installed %d files\n' "$file_count"
    [[ -n "$bin_path" ]] && printf '  -> bin: %s\n' "$bin_path"
    [[ $lib_files -gt 0 ]] && printf '  -> lib: %d files\n' "$lib_files"
    [[ $cmd_files -gt 0 ]] && printf '  -> lib/commands: %d files\n' "$cmd_files"
    [[ $fn_files -gt 0 ]] && printf '  -> lib/functions: %d files\n' "$fn_files"
    [[ -n "$completion_path" ]] && printf '  -> completions: %s\n' "$completion_path"
  fi
  printf '==> Done. Try: claude-session --help\n'

# remove files listed in the install manifest
uninstall PREFIX="":
  #!/usr/bin/env bash
  set -euo pipefail
  printf '==> Uninstalling claude-session\n'
  if [[ $EUID -ne 0 && -z "{{PREFIX}}" ]]; then
    state_dir="${XDG_STATE_HOME:-$HOME/.local/state}/claude-session"
  else
    state_dir="/var/lib/claude-session"
  fi
  manifest="$state_dir/install-manifest"
  if [[ -f "$manifest" ]]; then
    entries=$(grep -c . "$manifest" 2>/dev/null || true)
    printf '  -> manifest: %s (%s entries)\n' "$manifest" "${entries:-0}"
  fi
  PREFIX="{{PREFIX}}" ./uninstall.sh
  printf '==> Removed.\n'

# run unit + integration tests
test: test-unit test-integration
  @printf '==> Tests passed.\n'

# run unit bats tests
test-unit:
  @if ! command -v bats >/dev/null 2>&1; then printf '%s\n' "bats not found; install bats-core" >&2; exit 127; fi
  @printf '==> Running unit tests\n'
  bats --filter-tags 'unit,!integration' test/
  @printf '==> Unit tests passed.\n'

# run integration bats tests
test-integration:
  @if ! command -v bats >/dev/null 2>&1; then printf '%s\n' "bats not found; install bats-core" >&2; exit 127; fi
  @printf '==> Running integration tests\n'
  bats --filter-tags integration test/
  @printf '==> Integration tests passed.\n'

# run pre-commit on all files
lint:
  @printf '==> Running pre-commit on all files\n'
  pre-commit run --all-files
  @printf '==> Lint clean.\n'

# run lint + test (CI gate)
check:
  @printf '==> CI gate: lint + test\n'
  just lint
  just test
  @printf '==> Check passed.\n'

# install bash completions
completions PREFIX="":
  #!/usr/bin/env bash
  set -euo pipefail
  printf '==> Installing bash completions only\n'
  PREFIX="{{PREFIX}}" ./install.sh --completions-only
  if [[ $EUID -ne 0 && -z "{{PREFIX}}" ]]; then
    data_dir="${XDG_DATA_HOME:-$HOME/.local/share}"
    completion_path="$data_dir/bash-completion/completions/claude-session.bash"
    printf '==> Done. Completion file: %s\n' "$completion_path"
  else
    printf '==> Done. Completion file installed under system bash-completion dir.\n'
  fi

# remove generated artifacts (non-destructive)
clean:
  #!/usr/bin/env bash
  set -euo pipefail
  printf '==> Cleaning generated artifacts\n'
  count=$(find test -type d -name 'bats-run-*' 2>/dev/null | wc -l | tr -d ' ')
  find test -type d -name 'bats-run-*' -prune -exec rm -rf {} +
  printf '  -> removed %s test/bats-run-* dir(s)\n' "$count"
  printf '==> Done.\n'
