#!/bin/bash

for cargofile in `find . -name Cargo.toml -print`; do
  if [ ! -f $(dirname $cargofile)/.skipcargo ]; then
    echo "CLEANING CARGO IN $(dirname $cargofile)"
    (
      cd $(dirname "${cargofile}") && cargo clean

      if [ X$1 == "Xupdate" ]; then
        echo "Updating dependencies"
        cargo update
      fi
    )
  fi
done

echo "CLEANING DOCS"
rm -rf docs/*