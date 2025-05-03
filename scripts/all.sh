#!/bin/sh

. ./scripts/parse_params.sh "$@"
. ./scripts/build_c.sh "$@"

if [ "${RUSTC}" = "" ]; then
	# use famc
	# build macros
	COMMAND="${FAMC} -C panic=abort \
--crate-type proc-macro \
-L${OUTPUT} \
--cfg mrustc \
-o .obj/libbitcoinmw_macros.rlib \
rust/macros/lib.rs";

	echo "${COMMAND}"
	${COMMAND} || exit 1;

	COMMAND="${FAMC} -C panic=abort \
-O \
--crate-type=lib \
-L${OUTPUT} \
--cfg mrustc \
-o .obj/rust \
--extern bitcoinmw_macros=.obj/libbitcoinmw_macros.rlib \
rust/bmw/mod.rs"
	echo "${COMMAND}"
       	${COMMAND} || exit 1;

	COMMAND="${CC} ${CCFLAGS} ${STATIC} -o bin/bmw .obj/*.o ${OUTPUT}/libcore.rlib.o -L.obj ${LINK_GMP} ${LINK_SECP256K1}"
	echo "${COMMAND}"
	${COMMAND} || exit 1;
else
	# use rustc

	# build macros
	COMMAND="${RUSTC} --crate-type=proc-macro --edition=2021 rust/macros/lib.rs -o .obj/libbitcoinmw_macros${MACRO_EXT}"
	echo ${COMMAND}
	${COMMAND} || exit 1;

	COMMAND="${RUSTC} -C panic=abort \
-C opt-level=3 \
--emit=obj \
--crate-type=lib \
--cfg rustc \
-o .obj/rust.o \
--extern bitcoinmw_macros=.obj/libbitcoinmw_macros${MACRO_EXT} \
rust/bmw/mod.rs"
	echo "${COMMAND}"
	${COMMAND} || exit 1;
    COMMAND="${CC} ${CCFLAGS} ${STATIC} -o bin/bmw .obj/*.o -L.obj ${LINK_GMP} ${LINK_SECP256K1}"
    echo "${COMMAND}"
    ${COMMAND} || exit 1;
fi
