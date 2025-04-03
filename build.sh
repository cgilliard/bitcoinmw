#!/bin/sh

export CC=clang
export CCFLAGS=-O3


if [ "clean" = "$1" ]; then
	echo "Cleaning"
        cd secp256k1-zkp
        make mostlyclean-compile
        cd ..
        rm -rf .obj/* libtest.a bin/* 
else
	echo "Building BitcoinMW"
	./scripts/secp256k1zkp.sh || exit 1;
	cd c
        for file in *.c
        do
                if [ ! -e ../.obj/${file%.c}.o ] || [ ${file} -nt ../.obj/${file%.c}.o ]; then
                        echo "${CC} ${CCFLAGS} -o ../.obj/${file%.c}.o -c -Ic ${file}";
                        ${CC} ${CCFLAGS} -o ../.obj/${file%.c}.o -c -Ic ${file} || exit 1;
                fi
        done
	cd ..
	${CC} ${CCFLAGS} -o bin/bmw .obj/*.o -L.obj -lsecp256k1 || exit 1;
fi
