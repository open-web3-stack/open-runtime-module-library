#!/usr/bin/env bash

set -e

COMMAND=$1
shift

set -x

for file in **/Cargo.toml; do
	if [ "$file" == "xtokens/Cargo.toml" ] || [ "$file" == "xcm-support/Cargo.toml" ] || [ "$file" == "unknown-tokens/Cargo.toml" ]
		then
			continue
	fi
	cargo $COMMAND --manifest-path "$file" $@;
done

