#!/bin/sh
# Designed to be executed via svg-term:
# svg-term --command="sh screencast.sh" --out screencast.svg --padding=10
set -e
set -u

PROMPT="▶"

enter() {
    INPUT=$1
    DELAY=$2

    prompt
    sleep $DELAY
    type $INPUT
    sleep 0.5
    printf '%b' " >\n"
    eval $INPUT
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

    enter "fd" 0

    enter "fd -e md" 1

    enter "fd -e md --exec wc -l" 1

    enter "fd mod" 1

    enter "fd sh" 1

    enter "fd -H sample" 1

    enter "fd -h" 1

    prompt

    sleep 3

    unset IFS
}

main