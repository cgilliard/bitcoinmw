#!/bin/sh

. ./scripts/parse_params.sh "$@"
. ./scripts/build_c.sh "$@"

if [ "${RUSTC}" = "" ]; then
	# use famc

	# build base
	COMMAND="${FAMC} -O \
--crate-type lib \
--crate-name=base \
-L${OUTPUT} \
--cfg mrustc \
-o .obj/libbase.rlib \
rust/base/lib.rs";
	echo "${COMMAND}"
	${COMMAND} || exit 1;

# create library for linking to work
ar rcs .obj/libdeps.a .obj/*.o || exit 1;

	# build macros
	COMMAND="${FAMC} -O \
--crate-type proc-macro \
-L${OUTPUT} \
--cfg mrustc \
-o .obj/libmacros.rlib \
-L.obj -ldeps \
--extern base=.obj/libbase.rlib \
rust/macros/lib.rs";


	echo "${COMMAND}"
	${COMMAND} || exit 1;

	COMMAND="${FAMC} -C panic=abort -O \
--crate-type=lib \
-L${OUTPUT} \
--cfg mrustc \
-o .obj/rust \
--extern macros=.obj/libmacros.rlib \
--extern base=.obj/libbase.rlib \
rust/bmw/lib.rs"
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
--crate-name=base \
--crate-type=lib \
--cfg rustc \
-o .obj/libbase.rlib \
rust/base/lib.rs"

        echo "${COMMAND}"
        ${COMMAND} || exit 1;

	# build base obj
        COMMAND="${RUSTC} \
-C opt-level=3 \
--crate-name=base \
--crate-type=lib \
--cfg rustc \
-o .obj/libbase.o \
--emit=obj
rust/base/lib.rs"

        echo "${COMMAND}"
        ${COMMAND} || exit 1;

	# build macros
	COMMAND="${RUSTC} \
--crate-name=macros \
--crate-type=proc-macro \
--edition=2021 \
--extern base=.obj/libbase.rlib \
-o .obj/libmacros${MACRO_EXT} \
rust/macros/lib.rs"

	echo ${COMMAND}
	${COMMAND} || exit 1;

	COMMAND="${RUSTC} -C panic=abort \
-C opt-level=3 \
--emit=obj \
--crate-type=staticlib \
--cfg rustc \
-o .obj/rust.o \
--extern macros=.obj/libmacros${MACRO_EXT} \
--extern base=.obj/libbase.rlib \
rust/bmw/lib.rs";
	echo "${COMMAND}"
	${COMMAND} || exit 1;
    COMMAND="${CC} ${CCFLAGS} ${STATIC} -o bin/bmw .obj/*.o -L.obj ${LINK_GMP} ${LINK_SECP256K1}"
    echo "${COMMAND}"
    ${COMMAND} || exit 1;
fi
