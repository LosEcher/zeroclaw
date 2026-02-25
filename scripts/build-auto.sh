#!/usr/bin/env bash
set -euo pipefail

# Auto-select build profile/jobs based on host resources.
# Supports optional channel feature auto-detection from config.toml.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

usage() {
  cat <<'USAGE'
Usage:
  scripts/build-auto.sh [options]

Options:
  --mode <safe|balanced|fast>
                          Build policy. Default: balanced
  --auto-channels         Auto-enable channel features from config (default)
  --no-auto-channels      Disable channel feature auto-detection
  --config <path>         Config file path for channel auto-detection
  --with-feishu           Enable channel-lark feature
  --with-matrix           Enable channel-matrix feature
  --features "a,b"        Append extra Cargo features
  --profile <name>        Force profile (release|release-fast|dev)
  --jobs <n>              Force CARGO_BUILD_JOBS
  --locked                Use --locked (default)
  --no-locked             Do not pass --locked
  --dry-run               Print resolved command and exit
  -h, --help              Show help

Environment overrides:
  ZEROCLAW_BUILD_PROFILE  Same as --profile
  ZEROCLAW_BUILD_JOBS     Same as --jobs
  ZEROCLAW_BUILD_MODE     Same as --mode
  ZEROCLAW_CONFIG         Same as --config
USAGE
}

have_cmd() { command -v "$1" >/dev/null 2>&1; }

get_mem_mb() {
  case "$(uname -s)" in
    Darwin)
      local bytes
      bytes="$(sysctl -n hw.memsize 2>/dev/null || echo 0)"
      echo $((bytes / 1024 / 1024))
      ;;
    Linux)
      awk '/MemTotal:/ {print int($2/1024)}' /proc/meminfo 2>/dev/null || echo 0
      ;;
    *)
      echo 0
      ;;
  esac
}

get_cpu_count() {
  case "$(uname -s)" in
    Darwin)
      sysctl -n hw.logicalcpu 2>/dev/null || echo 1
      ;;
    Linux)
      if have_cmd nproc; then
        nproc
      else
        getconf _NPROCESSORS_ONLN 2>/dev/null || echo 1
      fi
      ;;
    *)
      echo 1
      ;;
  esac
}

append_feature() {
  local f="$1"
  [[ -z "$f" ]] && return 0
  if [[ -z "$features" ]]; then
    features="$f"
  elif [[ ",$features," != *",$f,"* ]]; then
    features="${features},${f}"
  fi
}

resolve_config_path() {
  if [[ -n "$config_path" ]]; then
    echo "$config_path"
    return 0
  fi
  if [[ -n "${ZEROCLAW_CONFIG:-}" ]]; then
    echo "$ZEROCLAW_CONFIG"
    return 0
  fi
  if [[ -f "$ROOT_DIR/config.toml" ]]; then
    echo "$ROOT_DIR/config.toml"
    return 0
  fi
  if [[ -f "$HOME/.zeroclaw/config.toml" ]]; then
    echo "$HOME/.zeroclaw/config.toml"
    return 0
  fi
  echo ""
}

toml_has_section() {
  local file="$1" section="$2"
  awk -v section="$section" '
    /^[[:space:]]*\[/ {
      gsub(/^[[:space:]]*\[/, "", $0)
      gsub(/\][[:space:]]*$/, "", $0)
      in_target = ($0 == section)
    }
    in_target { found=1; exit }
    END { exit found ? 0 : 1 }
  ' "$file"
}

toml_whatsapp_has_session_path() {
  local file="$1"
  awk '
    /^[[:space:]]*\[/ {
      line=$0
      gsub(/^[[:space:]]*\[/, "", line)
      gsub(/\][[:space:]]*$/, "", line)
      in_target = (line == "channels_config.whatsapp")
      next
    }
    in_target {
      line=$0
      sub(/[[:space:]]*#.*/, "", line)
      if (line ~ /^[[:space:]]*session_path[[:space:]]*=/) {
        split(line, parts, "=")
        value=parts[2]
        gsub(/^[[:space:]]+|[[:space:]]+$/, "", value)
        gsub(/^"/, "", value)
        gsub(/"$/, "", value)
        if (value != "") {
          found=1
          exit
        }
      }
    }
    END { exit found ? 0 : 1 }
  ' "$file"
}

detect_channel_features_from_config() {
  local file="$1"
  if [[ ! -f "$file" ]]; then
    return 0
  fi

  if toml_has_section "$file" "channels_config.matrix"; then
    append_feature "channel-matrix"
  fi

  if toml_has_section "$file" "channels_config.lark" || toml_has_section "$file" "channels_config.feishu"; then
    append_feature "channel-lark"
  fi

  if toml_whatsapp_has_session_path "$file"; then
    append_feature "whatsapp-web"
  fi
}

pick_defaults() {
  local mem_mb="$1" cpu="$2" mode="$3"

  case "$mode" in
    safe)
      profile="release"
      jobs=1
      ;;
    balanced)
      # Prioritize low disturbance when other tasks run on the same host.
      profile="release"
      if (( mem_mb <= 4096 )); then
        jobs=1
      elif (( mem_mb <= 8192 )); then
        jobs=2
      elif (( cpu >= 8 )); then
        jobs=3
      else
        jobs=2
      fi
      ;;
    fast)
      if (( mem_mb >= 16384 && cpu >= 8 )); then
        profile="release-fast"
      else
        profile="release"
      fi

      if (( mem_mb <= 4096 )); then
        jobs=1
      elif (( mem_mb <= 8192 )); then
        jobs=2
      elif (( mem_mb <= 16384 )); then
        jobs=4
      else
        jobs=8
      fi
      ;;
    *)
      echo "Unsupported mode: $mode (expected safe|balanced|fast)" >&2
      exit 2
      ;;
  esac

  if (( jobs > cpu )); then jobs="$cpu"; fi
  if (( jobs < 1 )); then jobs=1; fi
}

with_feishu=false
with_matrix=false
auto_channels=true
config_path=""
resolved_config_path=""
features=""
mode="${ZEROCLAW_BUILD_MODE:-balanced}"
profile="${ZEROCLAW_BUILD_PROFILE:-}"
jobs="${ZEROCLAW_BUILD_JOBS:-}"
locked=true
dry_run=false

while (($#)); do
  case "$1" in
    --mode)
      shift
      mode="$1"
      ;;
    --auto-channels)
      auto_channels=true
      ;;
    --no-auto-channels)
      auto_channels=false
      ;;
    --config)
      shift
      config_path="$1"
      ;;
    --with-feishu)
      with_feishu=true
      ;;
    --with-matrix)
      with_matrix=true
      ;;
    --features)
      shift
      features="${features:+$features,}$1"
      ;;
    --profile)
      shift
      profile="$1"
      ;;
    --jobs)
      shift
      jobs="$1"
      ;;
    --locked)
      locked=true
      ;;
    --no-locked)
      locked=false
      ;;
    --dry-run)
      dry_run=true
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown option: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
  shift
done

mem_mb="$(get_mem_mb)"
cpu="$(get_cpu_count)"

if [[ -z "$profile" || -z "$jobs" ]]; then
  pick_defaults "$mem_mb" "$cpu" "$mode"
fi

if [[ "$with_feishu" == true ]]; then
  append_feature "channel-lark"
fi
if [[ "$with_matrix" == true ]]; then
  append_feature "channel-matrix"
fi
if [[ "$auto_channels" == true ]]; then
  resolved_config_path="$(resolve_config_path)"
  if [[ -n "$resolved_config_path" && -f "$resolved_config_path" ]]; then
    detect_channel_features_from_config "$resolved_config_path"
  fi
fi

cmd=(cargo build)
if [[ "$profile" == "release" ]]; then
  cmd+=(--release)
elif [[ "$profile" == "release-fast" ]]; then
  cmd+=(--profile release-fast)
elif [[ "$profile" != "dev" ]]; then
  echo "Unsupported profile: $profile" >&2
  exit 2
fi
if [[ "$locked" == true ]]; then
  cmd+=(--locked)
fi
if [[ -n "$features" ]]; then
  cmd+=(--features "$features")
fi

printf 'Host detected: mem=%sMB cpu=%s\n' "$mem_mb" "$cpu"
printf 'Build plan: mode=%s profile=%s jobs=%s features=%s locked=%s\n' \
  "$mode" "$profile" "$jobs" "${features:-<none>}" "$locked"
if [[ "$auto_channels" == true ]]; then
  printf 'Channel auto-detect: %s\n' "${resolved_config_path:-<no config found>}"
else
  printf 'Channel auto-detect: disabled\n'
fi
printf 'Command: CARGO_BUILD_JOBS=%s %s\n' "$jobs" "${cmd[*]}"

if [[ "$dry_run" == true ]]; then
  exit 0
fi

CARGO_BUILD_JOBS="$jobs" "${cmd[@]}"
