#!/bin/sh
# Verifier and the demo meters in one machine. The simulator resumes from the verifier's state.
set -e
sunmint-verifier &
VPID=$!
sunmint-simulator --backfill "${BACKFILL_HOURS:-24}" &
wait $VPID
