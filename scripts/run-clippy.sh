#!/usr/bin/env bash

set -e

COMMAND=$1

ALLOW_CLIPPY_RULES=(
	"clippy::identity_op" # this helps the code to be consistant
	"clippy::blacklisted-name" # TODO: allow them in test only
	"clippy::ptr-arg" # TODO: decide if we want to fix those
	"clippy::match_single_binding" # TODO: fix those
)

CLIPPY_RULE=$(printf " -A %s" "${ALLOW_CLIPPY_RULES[@]}")

cargo clippy $@ -- $CLIPPY_RULE

