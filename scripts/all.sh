#!/bin/sh

. ./scripts/parse_params.sh "$@"
. ./scripts/build_c.sh "$@"

if [ "${RUSTC}" = "" ]; then
	# use famc
	
	# build base
	COMMAND="${FAMC} -C panic=abort -O \
--crate-type lib \
--crate-name=bitcoinmw_base \
-L${OUTPUT} \
--cfg mrustc \
-o .obj/libbitcoinmw_base.rlib \
rust/base/lib.rs";
	echo "${COMMAND}"
	${COMMAND} || exit 1;


	# build macros
	COMMAND="${FAMC} -C panic=abort -O \
--crate-type proc-macro \
-L${OUTPUT} \
--cfg mrustc \
-o .obj/libbitcoinmw_macros.rlib \
--extern bitcoinmw_base=.obj/libbitcoinmw_base.rlib \
rust/macros/lib.rs";

	echo "${COMMAND}"
	${COMMAND} || exit 1;

	COMMAND="${FAMC} -C panic=abort -O \
--crate-type=lib \
-L${OUTPUT} \
--cfg mrustc \
-o .obj/rust \
--extern bitcoinmw_macros=.obj/libbitcoinmw_macros.rlib \
--extern bitcoinmw_base=.obj/libbitcoinmw_base.rlib \
rust/bmw/mod.rs"
	echo "${COMMAND}"
       	${COMMAND} || exit 1;

	COMMAND="${CC} ${CCFLAGS} ${STATIC} -o bin/bmw .obj/*.o ${OUTPUT}/libcore.rlib.o -L.obj ${LINK_GMP} ${LINK_SECP256K1}"
	echo "${COMMAND}"
	${COMMAND} || exit 1;
else
	# use rustc
	
	# build base
	COMMAND="${RUSTC} \
-C opt-level=3 \
--crate-name=bitcoinmw_base \
--crate-type=lib \
--cfg rustc \
-o .obj/libbitcoinmw_base.rlib \
rust/base/lib.rs"

        echo "${COMMAND}"
        ${COMMAND} || exit 1;

	# build macros
	COMMAND="${RUSTC} \
--crate-name=bitcoinmw_macros \
--crate-type=proc-macro \
--edition=2021 \
--extern bitcoinmw_base=.obj/libbitcoinmw_base.rlib \
-o .obj/libbitcoinmw_macros${MACRO_EXT} \
rust/macros/lib.rs"

	echo ${COMMAND}
	${COMMAND} || exit 1;

	COMMAND="${RUSTC} -C panic=abort \
-C opt-level=3 \
--emit=obj \
--crate-type=lib \
--cfg rustc \
-o .obj/rust.o \
--extern bitcoinmw_macros=.obj/libbitcoinmw_macros${MACRO_EXT} \
--extern bitcoinmw_base=.obj/libbitcoinmw_base.rlib \
rust/bmw/mod.rs";
	echo "${COMMAND}"
	${COMMAND} || exit 1;
    COMMAND="${CC} ${CCFLAGS} ${STATIC} -o bin/bmw .obj/*.o -L.obj ${LINK_GMP} ${LINK_SECP256K1}"
    echo "${COMMAND}"
    ${COMMAND} || exit 1;
fi
