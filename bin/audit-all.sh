#!/bin/bash

# Construct the command line parameters to ignore Rust IDs we have accepted
# the risk on.
IGNORECMD=""

for rvid in `cat accepted-vulnerabilities.rvid`; do
  echo "Ignoring vulnerability ${rvid}"
  IGNORECMD="--ignore ${rvid} ${IGNORECMD}"
done
echo ""

for cargofile in `find . -name Cargo.toml -print`; do
  if [ ! -f $(dirname $cargofile)/.skipcargo ]; then
    echo "AUDITING DEPENDENCIES IN $(dirname $cargofile)"
    (
      cd $(dirname "${cargofile}") && cargo audit --deny warnings ${IGNORECMD}
    ) || exit -1
  fi
done
