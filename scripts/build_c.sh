#!/bin/sh

# build xxdir and create header
if [ ! -e bin/xxdir ] || [ build_utils/xxdir.c -nt  bin/xxdir ]; then
        echo "${CC} -o bin/xxdir build_utils/xxdir.c"
        ${CC} -o bin/xxdir build_utils/xxdir.c || exit 1;
fi

if [ ! -e c/bin.h ] || [ -n "$(find resources -type f -newer c/bin.h)" ]; then
        ./bin/xxdir ./resources c/bin.h
        touch c/bible.c
fi

# Build secp256k1
./scripts/secp256k1zkp.sh || exit 1;

# Compile c files
cd c
for file in *.c
do
        if [ ! -e ../.obj/${file%.c}.o ] || [ ${file} -nt ../.obj/${file%.c}.o ]; then
                echo "${CC} ${CCFLAGS} -o ../.obj/${file%.c}.o -c -Ic ${file}"
                ${CC} ${CCFLAGS} -o ../.obj/${file%.c}.o -c -Ic ${file} || exit 1;
        fi
done
cd ..
