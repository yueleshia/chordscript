#!/bin/sh

NAME="$( basename "${0}"; printf a )"; NAME="${NAME%?a}"
dir="$( dirname "${0}"; printf a )"; dir="${dir%?a}"
cd "${dir}" || exit "$?"
dir="$( pwd -P; printf a )"; dir="${dir?a}"

SHORTCUTS="${XDG_CONFIG_HOME}/rc/wm-shortcuts"
BEGIN_MARKER="########## automatically generated begin ##########"
CLOSE_MARKER="########## automatically generated close ##########"


main() {
  # Make sure we support all the subcommands
  for command in $( run_parser 'subcommands' ); do
    DEBUG='true' my_make "${command}"
  done

  TEMP="$( mktemp )"
  trap "rm -f \"${TEMP}\"" EXIT
  DEBUG='false' my_make i3
}

my_make() {
  case "${1}"
    # @TODO implement -r/--runner
    in i3) parse "${XDG_CONFIG_HOME}/i3/config" i3 \
      -c "${SHORTCUTS}" -s "${SCRIPTS}/shortcuts.sh" #-r "shortcuts.sh"
    ;; sh|shortcuts|shortcuts-debug|keyspaces)
      parse "${XDG_CONIG_HOME}" "${1}" -c "${SHORTCUTS}"
    ;; *)
      if "${DEBUG}"
        then die DEV 1 "'${1}' is needs to be supported by ${NAME}"
        else die FATAL 1 "'${1}' is not a valid command"
      fi
  esac
}

parse() {
  # $1: filepath of config file

  "${DEBUG}" && return

  [ -r "${1}" ] || die FATAL 1 "File '${1}' does not exist"
  filepath="${1}"
  shift 1

  # Verify it is formatted correctly
  <"${filepath}" awk -v marker="${BEGIN_MARKER}" '
    $0 == marker { found = 1; }
    END { if (found) {} else { exit 1; } }
  ' || die FATAL 1 "Could not found BEGIN_MARKER" "${BEGIN_MARKER}"

  <"${filepath}" awk -v marker="${CLOSE_MARKER}" '
    $0 == marker { found = 1; }
    END { if (found) {} else { exit 1; } }
  ' || die FATAL 1 "Could not found CLOSE_MARKER" "${CLOSE_MARKER}"

  # Replace
  <"${filepath}" replace_text_between_markers "$@" >"${TEMP}"
  cp "${TEMP}" "${filepath}"
}

replace_text_between_markers() {
  <&0 eat_till "${BEGIN_MARKER}"
  printf %s\\n "${BEGIN_MARKER}"
  # Eat the core
  <&0 eat_till "${CLOSE_MARKER}" >/dev/null

  <&0 run_parser "$@"
  printf %s\\n "${CLOSE_MARKER}"
  <&0 eat_rest
}

run_parser() {
  cargo run -- "$@"
}

eat_till() {
  # $1: the comment method
  while IFS= read -r _line; do
    [ "${_line}" != "${_line#"${1}"}" ] && return 0
    printf %s\\n "${_line}"
  done
  printf %s "${_line}"
}

eat_rest() {
  while IFS= read -r _line; do
    printf %s\\n "${_line}"
  done
  printf %s "${_line}"
}

die() { printf %s "${1}: " >&2; shift 1; printf %s\\n "$@" >&2; exit "${1}"; }

main "$@"
