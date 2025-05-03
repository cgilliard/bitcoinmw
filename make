#!/bin/sh

if [ "clean" = "$1" ]; then
	./scripts/clean.sh "$@"
elif [ "test" = "$1" ]; then
	./scripts/test.sh "$@"
elif [ "coverage" = "$1" ]; then
	./scripts/coverage.sh "$@"
elif [ "all" = "$1" ] || [ "--*" = "$1" ] || [ "" = "$1" ]; then
	./scripts/all.sh "$@"
elif case "$1" in --*) true;; *) false;; esac; then
	./scripts/all.sh "$@"
else
	echo "Usage: ./make [clean | test | coverage | all] <options>"
fi
