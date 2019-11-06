#!/usr/bin/env bash

COMMAND=$1
shift
find . -name 'Cargo\.toml' -print -exec cargo $COMMAND --manifest-path {} $@ \;
