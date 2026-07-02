#!/usr/bin/gawk -f

BEGIN {
  inSection = 0
  inBlock = 0
}


/Command-line options/ { inSection=1 }

inSection && /^```/ {
  inBlock=1
  inSection=0
  # print the starting fence then move to next line
  print
  next
}

inBlock && /^```/ {
  cmd="cargo run --release --quiet -- -h"
  # Output the results of fd -h
  u=0
  while (cmd | getline line) {
    # Skip everything before the usage line
    if (line ~ /^Usage/) {
      u=1;
    }
    if (u) {
      print line
    }
  }
  status = close(cmd)
  if (status) {
    print "failed to generate help output" > "/dev/stderr"
    exit 1
  }
  inBlock = 0
}

!inBlock

