#!/bin/bash
# Stands in for the runner's config.sh: writes down the arguments it was
# called with, then registers, which is writing `.runner`, unless the
# token is `expired`, which fails the way GitHub refuses one.
set -eu
printf '%s\n' "$@" > config.args

token=
while [ $# -gt 0 ]; do
  if [ "$1" = --token ]; then token=$2; fi
  shift
done

if [ "$token" = expired ]; then
  echo "Http response code: NotFound from 'POST https://api.github.com/actions/runner-registration'" >&2
  exit 1
fi
echo '{"agentName": "fake"}' > .runner
echo "Runner successfully added"
