#!/bin/sh

export CC=clang


if [ "clean" = "$1" ]; then
	echo "Cleaning"
        cd secp256k1-zkp
        make mostlyclean-compile
        cd ..
        rm -rf .obj/* libtest.a bin/* rust/test_deps/*/target
else
	echo "Building BitcoinMW"
	./scripts/secp256k1zkp.sh || exit 1;
fi
