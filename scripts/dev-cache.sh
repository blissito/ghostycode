#!/bin/sh
# Portable opt-in Cargo cache topology for Ghosty worktrees.
#
# Isolated per-workspace intermediates (Cargo build-dir + {workspace-path-hash})
# plus optional shared sccache. Never hard-codes a machine path. Does not
# change product behavior. This file is safe to source from other /bin/sh
# helpers; the CLI path is the only one that calls `set -eu` and `exit`.
#
# Usage:
#   . scripts/dev-cache.sh && ghosty_dev_cache_apply
#   scripts/dev-cache.sh --status
#   scripts/dev-cache.sh --self-check
#   scripts/dev-cache.sh --print-exports
#   scripts/dev-cache.sh --help
#
# Environment:
#   GHOSTY_CACHE_ROOT        cache root (default:
#                               ${XDG_CACHE_HOME:-$HOME/.cache}/ghosty)
#   GHOSTY_DEV_CACHE         auto|1|force|local|0   (default auto)
#                               auto/1/force = isolated build-dir (still
#                                       respects an already-set
#                                       CARGO_TARGET_DIR / CARGO_BUILD_BUILD_DIR)
#                               local = keep compiling into ./target
#                               0     = do nothing
#   GHOSTY_SCCACHE           auto|1|0         (default auto)
#                               auto  = wrap rustc only when incremental is
#                                       already off and sccache is on PATH
#                               1     = request CARGO_INCREMENTAL=0 (unless
#                                       the caller already set it to on) and
#                                       wrap if sccache exists
#                               0     = never wrap
#   GHOSTY_DEV_CACHE_QUIET   1 to suppress the one-line status
#   GHOSTY_DEV_CACHE_REPO_ROOT
#                               workspace root used for ./target detection
#                               (default: $PWD)
#   GHOSTY_RUSTC_COMMIT      override the rustc commit used to namespace
#                               SCCACHE_DIR (tests / offline hosts)
#
# sccache cannot cache incremental units. This helper never enables the
# wrapper while CARGO_INCREMENTAL is on, and it never turns incremental off
# unless GHOSTY_SCCACHE=1. Everyday edits stay on cargo's incremental
# default. New-worktree / CI-like rebuilds that already set
# CARGO_INCREMENTAL=0 get the measured shared-compiler-output path.

ghosty_dev_cache_truthy() {
  case ${1:-} in
    1|true|yes|on|force) return 0 ;;
    *) return 1 ;;
  esac
}

ghosty_dev_cache_falsey() {
  case ${1:-} in
    0|false|no|off|never|disabled) return 0 ;;
    *) return 1 ;;
  esac
}

ghosty_dev_cache_root() {
  if [ -n "${GHOSTY_CACHE_ROOT:-}" ]; then
    printf '%s\n' "$GHOSTY_CACHE_ROOT"
    return
  fi
  if [ -n "${XDG_CACHE_HOME:-}" ]; then
    printf '%s\n' "$XDG_CACHE_HOME/ghosty"
    return
  fi
  if [ -n "${HOME:-}" ]; then
    printf '%s\n' "$HOME/.cache/ghosty"
    return
  fi
  printf '%s\n' "${TMPDIR:-/tmp}/ghosty-cache"
}

ghosty_dev_cache_repo_root() {
  if [ -n "${GHOSTY_DEV_CACHE_REPO_ROOT:-}" ]; then
    printf '%s\n' "$GHOSTY_DEV_CACHE_REPO_ROOT"
    return
  fi
  pwd
}

ghosty_dev_cache_rustc_commit() {
  if [ -n "${GHOSTY_RUSTC_COMMIT:-}" ]; then
    printf '%s\n' "$GHOSTY_RUSTC_COMMIT"
    return
  fi
  # rustc -vV is cheap and does not start a compile.
  _cw_out=$(rustc -vV 2>/dev/null || true)
  _cw_commit=$(printf '%s\n' "$_cw_out" | awk '/^commit-hash:/{print $2; exit}')
  if [ -n "$_cw_commit" ]; then
    printf '%s\n' "$_cw_commit"
  else
    printf '%s\n' "unknown"
  fi
}

ghosty_dev_cache_cargo_version_mm() {
  # cargo 1.97.0 -> 1097. Empty if cargo is missing or unparsable.
  cargo --version 2>/dev/null | awk '{
    split($2, a, ".")
    if (a[1] ~ /^[0-9]+$/ && a[2] ~ /^[0-9]+$/)
      printf "%d\n", a[1]*1000+a[2]
  }'
}

# Stable 16-hex-char fingerprint of a path. Used only for the pre-1.91
# CARGO_TARGET_DIR fallback; Cargo 1.91+ expands {workspace-path-hash}.
ghosty_dev_cache_path_fingerprint() {
  if command -v sha256sum >/dev/null 2>&1; then
    printf '%s' "$1" | sha256sum | awk '{print substr($1,1,16)}'
  elif command -v shasum >/dev/null 2>&1; then
    printf '%s' "$1" | shasum -a 256 | awk '{print substr($1,1,16)}'
  elif command -v openssl >/dev/null 2>&1; then
    printf '%s' "$1" | openssl dgst -sha256 | awk '{print substr($NF,1,16)}'
  else
    printf '%s' "$1" | cksum | awk '{printf "%s%08s\n", $1, $2}' | tr ' ' '0'
  fi
}

ghosty_dev_cache_incremental_off() {
  case ${CARGO_INCREMENTAL:-} in
    0|false|no|off) return 0 ;;
    *) return 1 ;;
  esac
}

ghosty_dev_cache_incremental_on() {
  case ${CARGO_INCREMENTAL:-} in
    1|true|yes|on) return 0 ;;
    *) return 1 ;;
  esac
}

ghosty_dev_cache_sccache_bin() {
  command -v sccache 2>/dev/null || true
}

ghosty_dev_cache_log() {
  if [ "${GHOSTY_DEV_CACHE_QUIET:-}" = 1 ]; then
    return 0
  fi
  printf 'dev-cache: %s\n' "$*" >&2
}

# Free space (in GiB, integer, truncated) on the filesystem holding $1, or
# empty when it cannot be determined. `df -Pk` is the POSIX-portable form and
# behaves the same on macOS and Linux.
ghosty_dev_cache_free_gib() {
  _cw_probe=$1
  # Walk up to the nearest existing ancestor: the cache root usually does not
  # exist yet on a first run, and df needs a real path.
  while [ -n "$_cw_probe" ] && [ ! -d "$_cw_probe" ]; do
    _cw_parent=$(dirname -- "$_cw_probe")
    [ "$_cw_parent" = "$_cw_probe" ] && break
    _cw_probe=$_cw_parent
  done
  [ -d "$_cw_probe" ] || return 0
  df -Pk -- "$_cw_probe" 2>/dev/null | awk 'NR==2 { print int($4 / 1048576) }'
}

# A cold workspace build-dir is ~6 GB, and cargo writes it incrementally, so a
# volume that runs out mid-build leaves a half-written cache and a failed run.
# Worse, on the host where this was added the low volume was also $TMPDIR's:
# the shell tool's spill `tempfile` fails on a full disk, which is the exact
# wedge #5465 was filed for. Warn before the build starts rather than after it
# dies, and name the override that fixes it.
GHOSTY_DEV_CACHE_MIN_FREE_GIB=${GHOSTY_DEV_CACHE_MIN_FREE_GIB:-15}

ghosty_dev_cache_warn_low_space() {
  _cw_root=$1
  _cw_free=$(ghosty_dev_cache_free_gib "$_cw_root")
  [ -n "$_cw_free" ] || return 0
  [ "$_cw_free" -lt "$GHOSTY_DEV_CACHE_MIN_FREE_GIB" ] || return 0
  ghosty_dev_cache_log \
    "WARNING: only ${_cw_free}GiB free on the volume holding ${_cw_root}; a cold build-dir is ~6GB."
  ghosty_dev_cache_log \
    "         Set GHOSTY_CACHE_ROOT=<dir on a roomier volume> (threshold: GHOSTY_DEV_CACHE_MIN_FREE_GIB)."
}

# Apply the topology to the current shell. Idempotent. Never overrides an
# already-set CARGO_TARGET_DIR, CARGO_BUILD_BUILD_DIR, RUSTC_WRAPPER, or
# SCCACHE_DIR.
ghosty_dev_cache_apply() {
  GHOSTY_DEV_CACHE_MODE=${GHOSTY_DEV_CACHE_MODE:-}
  # Re-entry after a previous apply in this shell: keep the first decision
  # unless the caller unset the mode (tests do that between cases).
  if [ -n "${GHOSTY_DEV_CACHE_APPLIED:-}" ] && [ -n "${GHOSTY_DEV_CACHE_MODE:-}" ]; then
    return 0
  fi

  _cw_want=${GHOSTY_DEV_CACHE:-auto}
  _cw_root=$(ghosty_dev_cache_root)
  _cw_sccache_want=${GHOSTY_SCCACHE:-auto}
  _cw_commit=$(ghosty_dev_cache_rustc_commit)
  GHOSTY_DEV_CACHE_SCCACHE_DIR=${SCCACHE_DIR:-${_cw_root}/sccache/${_cw_commit}}
  GHOSTY_DEV_CACHE_BUILD_DIR=${_cw_root}/build/'{workspace-path-hash}'
  GHOSTY_DEV_CACHE_LEGACY_TARGET_DIR=${_cw_root}/target/$(ghosty_dev_cache_path_fingerprint "$(ghosty_dev_cache_repo_root)")

  if ghosty_dev_cache_falsey "$_cw_want"; then
    GHOSTY_DEV_CACHE_MODE=disabled
    GHOSTY_DEV_CACHE_SCCACHE=disabled
    GHOSTY_DEV_CACHE_APPLIED=1
    export GHOSTY_DEV_CACHE_MODE GHOSTY_DEV_CACHE_SCCACHE
    ghosty_dev_cache_log "disabled (GHOSTY_DEV_CACHE=${_cw_want})"
    return 0
  fi

  if [ -n "${CARGO_TARGET_DIR:-}" ]; then
    GHOSTY_DEV_CACHE_MODE=inherited-target-dir
  elif [ -n "${CARGO_BUILD_BUILD_DIR:-}" ]; then
    GHOSTY_DEV_CACHE_MODE=inherited-build-dir
  elif [ "$_cw_want" = local ]; then
    # Explicit stay-in-./target. A stub target/ created by a previous
    # isolated build-dir run is not a reason to abandon isolation: Cargo
    # still writes CACHEDIR.TAG (and sometimes finals) under ./target
    # when build-dir is split, and treating that as "warm" made the
    # second command recompile everything into a second tree.
    GHOSTY_DEV_CACHE_MODE=existing-target
  else
    GHOSTY_DEV_CACHE_MODE=isolated-build-dir
  fi

  case $GHOSTY_DEV_CACHE_MODE in
    force-isolated|isolated-build-dir)
      _cw_mm=$(ghosty_dev_cache_cargo_version_mm)
      if [ -n "$_cw_mm" ] && [ "$_cw_mm" -lt 1091 ]; then
        # Cargo 1.91 introduced build.build-dir. Older cargo only isolates
        # via CARGO_TARGET_DIR; keep that fallback per-workspace.
        if [ -z "${CARGO_TARGET_DIR:-}" ]; then
          CARGO_TARGET_DIR=$GHOSTY_DEV_CACHE_LEGACY_TARGET_DIR
          export CARGO_TARGET_DIR
        fi
        GHOSTY_DEV_CACHE_MODE=${GHOSTY_DEV_CACHE_MODE}-legacy-target-dir
      else
        CARGO_BUILD_BUILD_DIR=$GHOSTY_DEV_CACHE_BUILD_DIR
        export CARGO_BUILD_BUILD_DIR
        mkdir -p "$_cw_root/build" 2>/dev/null || true
      fi
      ;;
  esac

  # sccache: never wrap incremental rustc. Missing binary is a fallback,
  # not an error.
  if ghosty_dev_cache_falsey "$_cw_sccache_want"; then
    GHOSTY_DEV_CACHE_SCCACHE=disabled
  elif [ -n "${RUSTC_WRAPPER:-}" ]; then
    case $RUSTC_WRAPPER in
      *sccache*) GHOSTY_DEV_CACHE_SCCACHE=external-sccache ;;
      *) GHOSTY_DEV_CACHE_SCCACHE=external-wrapper ;;
    esac
  else
    if ghosty_dev_cache_truthy "$_cw_sccache_want" && ! ghosty_dev_cache_incremental_on; then
      # Request the measured new-worktree / CI-like topology.
      if [ -z "${CARGO_INCREMENTAL:-}" ]; then
        CARGO_INCREMENTAL=0
        export CARGO_INCREMENTAL
      fi
    fi
    if ! ghosty_dev_cache_incremental_off; then
      GHOSTY_DEV_CACHE_SCCACHE=skipped-incremental
    else
      _cw_bin=$(ghosty_dev_cache_sccache_bin)
      if [ -z "$_cw_bin" ]; then
        GHOSTY_DEV_CACHE_SCCACHE=not-found
      else
        RUSTC_WRAPPER=$_cw_bin
        export RUSTC_WRAPPER
        if [ -z "${SCCACHE_DIR:-}" ]; then
          SCCACHE_DIR=$GHOSTY_DEV_CACHE_SCCACHE_DIR
          export SCCACHE_DIR
        fi
        mkdir -p "$SCCACHE_DIR" 2>/dev/null || true
        GHOSTY_DEV_CACHE_SCCACHE=enabled
      fi
    fi
  fi

  GHOSTY_DEV_CACHE_APPLIED=1
  export GHOSTY_DEV_CACHE_MODE GHOSTY_DEV_CACHE_SCCACHE
  export GHOSTY_DEV_CACHE_SCCACHE_DIR

  _cw_line="mode=${GHOSTY_DEV_CACHE_MODE} sccache=${GHOSTY_DEV_CACHE_SCCACHE}"
  case $GHOSTY_DEV_CACHE_MODE in
    isolated-build-dir*|force-isolated*)
      _cw_line="${_cw_line} build-dir=${CARGO_BUILD_BUILD_DIR:-${CARGO_TARGET_DIR:-unset}}"
      ;;
    inherited-target-dir)
      _cw_line="${_cw_line} CARGO_TARGET_DIR=${CARGO_TARGET_DIR}"
      ;;
    inherited-build-dir)
      _cw_line="${_cw_line} CARGO_BUILD_BUILD_DIR=${CARGO_BUILD_BUILD_DIR}"
      ;;
    existing-target)
      _cw_line="${_cw_line} using ./target (GHOSTY_DEV_CACHE=local)"
      ;;
  esac
  if [ "${GHOSTY_DEV_CACHE_SCCACHE}" = enabled ]; then
    _cw_line="${_cw_line} SCCACHE_DIR=${SCCACHE_DIR} rustc=${_cw_commit}"
  fi
  ghosty_dev_cache_log "$_cw_line"
  case $GHOSTY_DEV_CACHE_MODE in
    disabled|existing-target) ;;
    *) ghosty_dev_cache_warn_low_space "$(ghosty_dev_cache_root)" ;;
  esac
}

ghosty_dev_cache_status() {
  _cw_tmpl=${GHOSTY_DEV_CACHE_BUILD_DIR:-}
  if [ -z "$_cw_tmpl" ]; then
    _cw_tmpl=$(ghosty_dev_cache_root)/build/'{workspace-path-hash}'
  fi
  printf '%s\n' \
    "cache_root=$(ghosty_dev_cache_root)" \
    "repo_root=$(ghosty_dev_cache_repo_root)" \
    "mode=${GHOSTY_DEV_CACHE_MODE:-unset}" \
    "sccache=${GHOSTY_DEV_CACHE_SCCACHE:-unset}" \
    "build_dir_template=${_cw_tmpl}" \
    "CARGO_BUILD_BUILD_DIR=${CARGO_BUILD_BUILD_DIR:-<unset>}" \
    "CARGO_TARGET_DIR=${CARGO_TARGET_DIR:-<unset>}" \
    "CARGO_INCREMENTAL=${CARGO_INCREMENTAL:-<unset>}" \
    "RUSTC_WRAPPER=${RUSTC_WRAPPER:-<unset>}" \
    "SCCACHE_DIR=${SCCACHE_DIR:-<unset>}" \
    "rustc_commit=$(ghosty_dev_cache_rustc_commit)" \
    "sccache_bin=$(ghosty_dev_cache_sccache_bin)"
}

# Extra cargo CLI flags that force template expansion of build-dir.
# CARGO_BUILD_BUILD_DIR is still exported for nested tools; --config is
# what Cargo documents as supporting {workspace-path-hash}.
ghosty_dev_cache_exec_cargo() {
  case ${GHOSTY_DEV_CACHE_MODE:-} in
    isolated-build-dir|force-isolated)
      if [ -n "${CARGO_BUILD_BUILD_DIR:-}" ]; then
        exec cargo --config "build.build-dir = \"${CARGO_BUILD_BUILD_DIR}\"" "$@"
      fi
      ;;
  esac
  exec cargo "$@"
}

ghosty_dev_cache_print_exports() {
  [ -n "${CARGO_BUILD_BUILD_DIR:-}" ] && printf 'export CARGO_BUILD_BUILD_DIR=%s\n' "$CARGO_BUILD_BUILD_DIR"
  [ -n "${CARGO_TARGET_DIR:-}" ] && printf 'export CARGO_TARGET_DIR=%s\n' "$CARGO_TARGET_DIR"
  [ -n "${CARGO_INCREMENTAL:-}" ] && printf 'export CARGO_INCREMENTAL=%s\n' "$CARGO_INCREMENTAL"
  [ -n "${RUSTC_WRAPPER:-}" ] && printf 'export RUSTC_WRAPPER=%s\n' "$RUSTC_WRAPPER"
  [ -n "${SCCACHE_DIR:-}" ] && printf 'export SCCACHE_DIR=%s\n' "$SCCACHE_DIR"
  [ -n "${GHOSTY_DEV_CACHE_MODE:-}" ] && printf 'export GHOSTY_DEV_CACHE_MODE=%s\n' "$GHOSTY_DEV_CACHE_MODE"
  [ -n "${GHOSTY_DEV_CACHE_SCCACHE:-}" ] && printf 'export GHOSTY_DEV_CACHE_SCCACHE=%s\n' "$GHOSTY_DEV_CACHE_SCCACHE"
}

ghosty_dev_cache_self_check() {
  _cw_fail=0
  _cw_root=$(ghosty_dev_cache_root)
  _cw_home=${HOME:-}
  _cw_xdg=${XDG_CACHE_HOME:-}

  if [ -z "${GHOSTY_CACHE_ROOT:-}" ]; then
    if [ -n "$_cw_xdg" ]; then
      case $_cw_root in
        "$_cw_xdg"/ghosty) ;;
        *)
          printf 'self-check: expected XDG default %s/ghosty, got %s\n' "$_cw_xdg" "$_cw_root" >&2
          _cw_fail=1
          ;;
      esac
    elif [ -n "$_cw_home" ]; then
      case $_cw_root in
        "$_cw_home"/.cache/ghosty) ;;
        *)
          printf 'self-check: expected HOME default %s/.cache/ghosty, got %s\n' "$_cw_home" "$_cw_root" >&2
          _cw_fail=1
          ;;
      esac
    fi
  fi

  case ${CARGO_BUILD_BUILD_DIR:-} in
    *'{workspace-path-hash}'*)
      ;;
    '')
      case ${GHOSTY_DEV_CACHE_MODE:-} in
        isolated-build-dir|force-isolated)
          printf 'self-check: isolated mode did not set CARGO_BUILD_BUILD_DIR\n' >&2
          _cw_fail=1
          ;;
      esac
      ;;
    *)
      case ${GHOSTY_DEV_CACHE_MODE:-} in
        isolated-build-dir|force-isolated)
          printf 'self-check: isolated build-dir lacks {workspace-path-hash}: %s\n' "$CARGO_BUILD_BUILD_DIR" >&2
          _cw_fail=1
          ;;
      esac
      ;;
  esac

  if [ "${GHOSTY_DEV_CACHE_SCCACHE:-}" = enabled ]; then
    if ! ghosty_dev_cache_incremental_off; then
      printf 'self-check: sccache enabled while incremental is not off\n' >&2
      _cw_fail=1
    fi
    case ${RUSTC_WRAPPER:-} in
      *sccache*) ;;
      *)
        printf 'self-check: sccache enabled but RUSTC_WRAPPER=%s\n' "${RUSTC_WRAPPER:-<unset>}" >&2
        _cw_fail=1
        ;;
    esac
  fi

  if [ "${GHOSTY_DEV_CACHE_SCCACHE:-}" = not-found ]; then
    if [ -n "${RUSTC_WRAPPER:-}" ]; then
      printf 'self-check: missing sccache must not set RUSTC_WRAPPER (%s)\n' "$RUSTC_WRAPPER" >&2
      _cw_fail=1
    fi
  fi

  if [ "$_cw_fail" -eq 0 ]; then
    printf 'dev-cache: self-check ok\n'
    return 0
  fi
  printf 'dev-cache: self-check FAILED\n' >&2
  return 1
}

ghosty_dev_cache_usage() {
  cat <<'EOF'
usage: scripts/dev-cache.sh --status|--self-check|--print-exports|--help

Portable opt-in Cargo cache topology. Source this file and call
ghosty_dev_cache_apply from scripts/dev-test.sh / scripts/dev-cargo.sh.

  --status          apply (if needed) and print key=value status
  --self-check      apply and check portable defaults / sccache fallback
  --print-exports   apply and print export statements for eval
  --help            this message

Environment overrides are documented at the top of this file. Defaults
never contain a machine-specific absolute path.
EOF
}

# CLI only when this file is the executed program, not when sourced.
case ${0##*/} in
  dev-cache.sh)
    set -eu
    _cw_cmd=${1:---status}
    case $_cw_cmd in
      --status|-s)
        ghosty_dev_cache_apply
        ghosty_dev_cache_status
        ;;
      --self-check)
        ghosty_dev_cache_apply
        ghosty_dev_cache_status
        ghosty_dev_cache_self_check
        ;;
      --print-exports)
        ghosty_dev_cache_apply
        ghosty_dev_cache_print_exports
        ;;
      --help|-h)
        ghosty_dev_cache_usage
        ;;
      *)
        ghosty_dev_cache_usage >&2
        exit 2
        ;;
    esac
    ;;
esac
