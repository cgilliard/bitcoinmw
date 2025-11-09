#!/bin/sh

. ./scripts/common.sh

for DIR in $SUB_DIRS; do
        build_dir ${DIR} 0 objs || exit $?;
done

mkdir -p target/objs/asm
COMMAND="${AS} -march=rv64i -mabi=lp64 src/asm/main.s -o target/objs/asm/main.o"
echo ${COMMAND};
${COMMAND};

mkdir -p target/bin
COMMAND="${CC} ${LDFLAGS} \
	-T src/asm/main.ld \
	-march=rv64i \
	-mabi=lp64 \
	-nostdlib \
	-static \
	-o target/bin/bmw.elf \
	target/objs/asm/* \
	target/objs/base/*";
echo ${COMMAND};
${COMMAND};

COMMAND="${OBJCOPY} -O binary target/bin/bmw.elf target/bin/bmw"
echo ${COMMAND};
${COMMAND};


