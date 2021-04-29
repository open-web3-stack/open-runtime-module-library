#!/usr/bin/env bash

set -e

COMMAND=$1
shift

set -x

FILES=$(find . -type d \( -name xtokens -o -name xcm-support -o -name unknown-tokens \) -prune -false -o -name Cargo.toml);

for file in $FILES; do
	cargo $COMMAND --manifest-path "$file" $@;
done

