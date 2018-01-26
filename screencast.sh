#!/bin/sh
# Designed to be executed via svg-term:
# svg-term --command="sh screencast.sh" --out screencast.svg --padding=10 --width=40
set -e

PROMPT="▶"

enter() {
    prompt
    type $1
    sleep 0.5
    printf '%b' " ⏎\n"
    eval $1
    type "\n"
}

prompt() {
  printf $PROMPT
  type " "
}

type() {
    printf '%b' $1 | pv -qL $[10+(-2 + RANDOM%5)]
}

main() {
    IFS='%'

    enter "fd"

    enter "fd -e md"

    enter "fd -e md --exec wc -l"

    enter "fd mod"

    enter "fd sh"

    enter "fd -H sample"

    enter "fd -h"

    prompt

    sleep 3

    unset IFS

    echo ""
}

main