#!/bin/sh
set -e

PROMPT="▶ "

enter() {
    IFS='%'
    type $PROMPT
    type $1
    sleep 0.5
    type " "
    sleep 0.25
    type "\n"
    eval $1
    type "\n"
    unset IFS
}

type() {
    printf '%b' $1 | pv -qL $[10+(-2 + RANDOM%5)]
}

enter "fd"

enter "fd mod"

enter "fd sh"

enter "fd -H sample"

enter "fd -h"