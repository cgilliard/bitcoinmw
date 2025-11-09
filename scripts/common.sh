
TAG=`git describe --tags`
echo "#define BMW_VERSION \"${TAG}\"" > src/include/bmw/version.h

build_dir() {
        local DIR="$1";
        local IS_TEST="$2";
        local OUT_DIR="$3";
        cd src/${DIR}
        mkdir -p ../../target/${OUT_DIR}/${DIR}

        for FILE in *.c
        do
                if [ "${FILE}" != "test.c" ] || [ "${IS_TEST}" = "1" ]; then
                        DEST_OBJ=../../target/${OUT_DIR}/${DIR}/${FILE%.c}.o
                        if [ ! -e ${DEST_OBJ} ] || [ ${FILE} -nt ${DEST_OBJ} ]; then
                                COMMAND="${CC} \
                                        -I../../${INCDIR} \
                                        ${CFLAGS} \
                                        ${CDEFS} \
                                        -o ${DEST_OBJ} \
                                        ${FILE} -c"
                                if [ "$SILENT" != "1" ]; then
                                        echo ${COMMAND};
                                fi
                                ${COMMAND} || exit $?;
                        fi
                fi
        done
        cd ../..
}

needs_linking() {
	local lib_file="$1"
	shift
	local obj_files="$@"

	# If LIB_NAME doesn't exist, linking is needed
	if [ ! -e "$lib_file" ]; then
		return 0  # 0 means true (needs linking)
	fi

	# Check each object file
	for obj in $obj_files; do
		# Ensure the object file exists before comparing
		if [ -e "$obj" ] && [ "$obj" -nt "$lib_file" ]; then
			return 0  # An object file is newer, needs linking
		fi
	done
	return 1  # No object files are newer, no linking needed
}



if [ "$NO_COLOR" = "" ]; then
   GREEN="\033[32m";
   CYAN="\033[36m";
   YELLOW="\033[33m";
   BRIGHT_RED="\033[91m";
   RESET="\033[0m";
   BLUE="\033[34m";
else
   GREEN="";
   CYAN="";
   YELLOW="";
   BRIGHT_RED="";
   RESET="";
   BLUE="";
fi

INCDIR="src/include";
SUB_DIRS="base";
BIN_DIR="./target/bin";

if [ "${CC}" = "" ]; then
	CC=clang
fi

CFLAGS="${CFLAGS} \
	-march=rv64i \
	-mabi=lp64 \
       	-fvisibility=hidden \
       	-fno-pie \
       	-fPIC \
	-fno-builtin \
       	-Wno-pointer-sign";

if [ "${LDFLAGS}" = "" ]; then
        LDFLAGS="-O2 \
		-ffreestanding \
		-nostdlib \
		-fstack-protector \
		-fvisibility=hidden \
		-Wl,--no-warn-rwx-segments";
fi
if [ "${FLTO}" = "1" ]; then
	LDFLAGS="${LDFLAGS} -flto=auto";
	CFLAGS="${CFLAGS} -flto=auto";
fi

