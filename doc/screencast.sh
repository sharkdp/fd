#!/bin/bash
# Designed to be executed via svg-term from the fd root directory:
# svg-term --command="bash doc/screencast.sh" --out doc/screencast.svg --padding=10
set -e
set -u

PROMPT="▶"

enter() {
    INPUT=$1
    DELAY=1

    prompt
    sleep "$DELAY"
    type "$INPUT"
    sleep 0.5
    printf '%b' "\\n"
    eval "$INPUT"
    type "\\n"
}

prompt() {
    printf '%b ' "$PROMPT" | pv -q
}

type() {
    printf '%b' "$1" | pv -qL $((10+(-2 + RANDOM%5)))
}

main() {
    IFS='%'

    enter "fd"

    enter "fd app"

    enter "fd fi"

    enter "fd fi --type f"

    enter "fd --type d"

    enter "fd -e md"

    enter "fd -e md --exec wc -l"

    enter "fd '^[A-Z]'"

    enter "fd --exclude src"

    enter "fd --hidden sample"

    prompt

    sleep 3

    echo ""

    unset IFS
}

main
