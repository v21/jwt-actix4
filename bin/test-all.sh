#!/bin/bash

for cargofile in `find . -name Cargo.toml -print`; do
  if [ ! -f $(dirname $cargofile)/.skipcargo ]; then

    echo "RUNNING TESTS IN $(dirname $cargofile)"
    (
      cd $(dirname "${cargofile}")

      # Reset the test helper functions
      pre_test() {
        echo "Null pre-test script"
      }
      reset_test() {
        echo "Null test reset script"
      }
      post_test() {
        echo "Null post-test script"
      }

      # Load the test helpers for this specific test run
      if [ -e testhelpers.sh ]; then
        source testhelpers.sh
      fi

      pre_test
      cargo test --all-features $@ -- --test-threads=1 || exit -1
      reset_test
      cargo tarpaulin --all-features $@ -v --out Xml -- --test-threads=1 || true
      post_test
    ) || exit -1
  fi
done
