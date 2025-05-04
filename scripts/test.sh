#!/bin/sh

. ./scripts/parse_params.sh "$@"
. ./scripts/build_c.sh "$@"
ar rcs .obj/libtest.a .obj/*.o || exit 1;

# Use rustc for tests
RUSTC=rustc
# use rustc

# build base
COMMAND="${RUSTC} -C debuginfo=2 \
--crate-name=base \
--crate-type=lib \
--cfg rustc \
-o .obj/libbase.rlib \
${RUSTFLAGS} \
--verbose \
rust/base/lib.rs"

${COMMAND} || exit 1;

# build macros
COMMAND="${RUSTC} \
--crate-name=macros \
--crate-type=proc-macro \
--edition=2021 \
--extern base=.obj/libbase.rlib \
-o .obj/libmacros${MACRO_EXT} \
rust/macros/lib.rs";

${COMMAND} || exit 1;

COMMAND="${RUSTC} -C debuginfo=2 \
--test rust/bmw/mod.rs \
-o bin/test_bmw \
-L .obj \
-l static=test \
-l static=secp256k1 \
-l static=gmp \
--extern macros=.obj/libmacros${MACRO_EXT} \
${RUSTFLAGS} \
--verbose"

${COMMAND} ||  exit 1;

COMMAND="./bin/test_bmw ${FILTER} --test-threads=1"
${COMMAND} || exit 1;
