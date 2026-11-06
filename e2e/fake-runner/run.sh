#!/bin/bash
# Stands in for the runner's run.sh: writes down the environment it was
# started with, then listens for jobs until it is stopped, or, when it
# is ephemeral, takes its one job and exits as the runner does after it.
env > run.env
echo "Listening for Jobs"

if [ "${RUNNER_EPHEMERAL:-false}" = true ]; then
  sleep 3
  echo "Job completed"
  exit 0
fi

trap 'echo "Runner stopped"; exit 0' TERM INT
while :; do
  sleep 1 &
  wait $!
done
