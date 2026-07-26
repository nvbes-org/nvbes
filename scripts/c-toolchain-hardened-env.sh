#!/usr/bin/env bash
set -euo pipefail

profile="${1:-release}"
target_os="$(uname -s | tr '[:upper:]' '[:lower:]')"

warning_flags=(
  -Wall
  -Wextra
  -Wformat=2
  -Wformat-security
  -Werror=format-security
  -Wconversion
  -Wno-unused-parameter
)

common_c_flags=(
  "${warning_flags[@]}"
  -D_FORTIFY_SOURCE=3
  -fstack-protector-strong
  -fPIE
  -fno-omit-frame-pointer
)

case "$profile" in
  debug)
    profile_c_flags=(-O0 -g3 -ggdb -DDEBUG -UNDEBUG)
    ;;
  release)
    profile_c_flags=(-O2 -g2 -DNDEBUG -UDEBUG -fvisibility=hidden)
    ;;
  test)
    profile_c_flags=(-O1 -g3 -DDEBUG -UNDEBUG -fsanitize=address,undefined)
    ;;
  *)
    echo "usage: source scripts/c-toolchain-hardened-env.sh [debug|release|test]" >&2
    return 2 2>/dev/null || exit 2
    ;;
esac

ld_flags=(-pie)
case "$target_os" in
  linux*)
    ld_flags+=(-Wl,-z,relro -Wl,-z,now -Wl,-z,noexecstack)
    ;;
  darwin*)
    ld_flags=(-Wl,-dead_strip)
    ;;
esac

joined_c_flags="${profile_c_flags[*]} ${common_c_flags[*]}"
export NVBES_C_TOOLCHAIN_HARDENING=required
export CFLAGS="${joined_c_flags} ${CFLAGS:-}"
export CXXFLAGS="${joined_c_flags} ${CXXFLAGS:-}"
export LDFLAGS="${ld_flags[*]} ${LDFLAGS:-}"
