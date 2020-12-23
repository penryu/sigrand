#!/bin/sh
set -eux

FIFO="$HOME/.signature"

if [ ! -e "$FIFO" ]; then
  mkfifo -m u=rw "$FIFO"
fi

/app/sigrand & pid=$!
cat "$FIFO"
kill "$pid"
