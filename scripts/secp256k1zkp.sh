#!/bin/sh

cd secp256k1-zkp
if [ ! -f "./configure" ]; then
	./autogen.sh || exit 1;
	./configure \
		--enable-module-schnorrsig \
		--enable-module-rangeproof \
		--enable-module-generator \
		--enable-experimental \
		--enable-module-aggsig \
		--enable-module-commitment \
		CC=${CC} || exit 1;
fi
make || exit 1;
cp .libs/libsecp256k1.a ../.obj
cd ..
