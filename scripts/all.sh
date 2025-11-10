#!/bin/sh

. ./scripts/common.sh

for DIR in $SUB_DIRS; do
        build_dir ${DIR} 0 objs || exit $?;
done

mkdir -p ${OBJS_DIR}/asm
if [ ! -f "${OBJS_DIR}/asm/start.o" ] || [ "src/asm/start.s" -nt "${OBJS_DIR}/asm/start.o" ]; then
	COMMAND="${AS} ${ARCH_TARGET} src/asm/start.s -o ${OBJS_DIR}/asm/start.o"
	echo ${COMMAND};
	${COMMAND} || exit $?;
fi

mkdir -p ${BIN_DIR}

OBJ_GLOBS="${OBJS_DIR}/asm/*.o ${OBJS_DIR}/base/*.o ${OBJS_DIR}/main/*.o"
OBJ_FILES=$(eval echo $OBJ_GLOBS)  # expands *.o safely

# Check if we need to link
if [ ! -f "${BIN_DIR}/bmw.elf" ] || \
   [ "src/asm/bmw.ld" -nt "${BIN_DIR}/bmw.elf" ] || \
   find ${OBJS_DIR}/asm ${OBJS_DIR}/base ${OBJS_DIR}/main -name '*.o' -newer "${BIN_DIR}/bmw.elf" | grep -q .
then
    COMMAND="${CC} ${LDFLAGS} -T src/asm/bmw.ld ${ARCH_TARGET} -nostdlib -static -o ${BIN_DIR}/bmw.elf ${OBJ_FILES}"
    echo "${COMMAND}"
    ${COMMAND} || exit $?
fi

if [ ! -f "${BIN_DIR}/bmw" ] || [ "${BIN_DIR}/bmw.elf" -nt "${BIN_DIR}/bmw" ]; then
	COMMAND="${OBJCOPY} -O binary ${BIN_DIR}/bmw.elf ${BIN_DIR}/bmw"
	echo ${COMMAND};
	${COMMAND} || exit $?;
fi


