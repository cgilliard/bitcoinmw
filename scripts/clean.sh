#!/bin/sh

nosecp="false";

# Check all arguments for --nosecp
for arg in "$@"; do
    case "$arg" in
        --nosecp)
            nosecp="true"
            ;;
    esac
done

if [ "$nosecp" = "false" ]; then
	cd secp256k1-zkp
	make mostlyclean-compile
	cd ..
fi

rm -rf .obj/* bin/* c/bin.h
