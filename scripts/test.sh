#!/bin/sh

. ./scripts/parse_params.sh "$@"
CCFLAGS=-DTEST
. ./scripts/build_c.sh "$@"
ar rcs .obj/libtest.a .obj/*.o || exit 1;

# Use rustc for tests
RUSTC=rustc
# use rustc

if [ "$OS" = "Linux" ]; then
	ASAN_LINK="-l static=asan"
else
	ASAN_LINK=
fi

# build base
COMMAND="${RUSTC} -C debuginfo=2 \
--crate-name=base \
--crate-type=lib \
-o .obj/libbase.rlib \
${RUSTFLAGS} \
--verbose \
rust/base/lib.rs"

echo ${COMMAND}
${COMMAND} || exit 1;

COMMAND="${RUSTC} -C debuginfo=2 \
--test rust/base/lib.rs \
-o bin/test_base \
-L .obj \
-l static=test \
${RUSTFLAGS} \
--verbose"

echo ${COMMAND}
${COMMAND} || exit 1;

# build macros
COMMAND="${RUSTC} -C debuginfo=2 \
--crate-name=macros \
--crate-type=proc-macro \
--edition=2021 \
--extern base=.obj/libbase.rlib \
-L .obj \
-l static=test \
-o .obj/libmacros${MACRO_EXT} \
${RUSTFLAGS} \
${ASAN_LINK} \
rust/macros/lib.rs";

echo ${COMMAND}
${COMMAND} || exit 1;

COMMAND="${RUSTC} -C debuginfo=2 \
--test rust/bmw/lib.rs \
-o bin/test_bmw \
-L .obj \
-l static=test \
-l static=secp256k1 \
-l static=gmp \
--extern macros=.obj/libmacros${MACRO_EXT} \
${RUSTFLAGS} \
--verbose"

echo ${COMMAND}
${COMMAND} ||  exit 1;

COMMAND="./bin/test_base ${FILTER} --test-threads=1"
${COMMAND} || exit 1;

COMMAND="./bin/test_bmw ${FILTER} --test-threads=1"
${COMMAND} || exit 1;
