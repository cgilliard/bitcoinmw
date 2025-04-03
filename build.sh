#!/bin/sh

export CC=clang


echo "Building BitcoinMW"

./scripts/secp256k1zkp.sh || exit 1;
