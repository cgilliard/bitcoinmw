#!/bin/sh

. ./scripts/parse_params.sh "$@"
. ./scripts/build_c.sh "$@"
ar rcs .obj/libtest.a .obj/*.o || exit 1;

# Use rustc for tests
RUSTC=rustc

# build macros
COMMAND="${RUSTC} --crate-type=proc-macro --edition=2021 rust/macros/lib.rs -o .obj/libbitcoinmw_macros${MACRO_EXT}"
${COMMAND} || exit 1;

COMMAND="${RUSTC} -C debuginfo=2 \
--test rust/bmw/mod.rs \
-o bin/test_bmw \
-L .obj \
-l static=test \
-l static=secp256k1 \
-l static=gmp \
--extern bitcoinmw_macros=.obj/libbitcoinmw_macros${MACRO_EXT} \
${RUSTFLAGS} \
--verbose"
${COMMAND} ||  exit 1;

COMMAND="./bin/test_bmw ${FILTER} --test-threads=1"
${COMMAND} || exit 1;
