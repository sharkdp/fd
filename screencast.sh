#!/bin/sh
set -e

type() {
    printf '\e[32m%s\e[m' "λ "
    echo $1 | pv -qL $[10+(-2 + RANDOM%5)]
    sleep 0.75
    eval $1
    echo ""
}

type "fd"

type "fd mod"

type "fd sh$"

type "fd -H sample"

type "fd -h"