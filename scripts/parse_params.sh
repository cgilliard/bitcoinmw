#!/bin/sh

CC=clang
RUSTC=
FAMC=../famc/bin/famc
OUTPUT=../famc/output-1.29.0
FILTER=
CCFLAGS=
RUSTFLAGS=

# Parse command-line arguments
for arg in "$@"; do
    case "$arg" in
	test)
	;;
	clean)
	;;
	coverage)
	;;
	-f=*)
	    FILTER=${arg#*=}  # Extract value after =
	    if [ -z "$FILTER" ]; then
                echo "Error: --with-cc requires a non-empty value" >&2
                exit 1
            fi
            ;;
	--with-asan)
	    CCFLAGS=-fsanitize=address
	    RUSTFLAGS=-Zsanitizer=address
	    ;;
        --with-cc=*)
            CC=${arg#*=}  # Extract value after =
            if [ -z "$CC" ]; then
                echo "Error: --with-cc requires a non-empty value" >&2
                exit 1
            fi
            ;;
        --with-rustc=*)
            RUSTC=${arg#*=}
            if [ -z "$RUSTC" ]; then
                echo "Error: --with-rustc requires a non-empty value" >&2
                exit 1
            fi
            ;;
        --with-famc=*)
            FAMC=${arg#*=}
            if [ -z "$FAMC" ]; then
                echo "Error: --with-famc requires a non-empty value" >&2
                exit 1
            fi
            ;;
        --with-output=*)
            OUTPUT=${arg#*=}
            if [ -z "$OUTPUT" ]; then
                echo "Error: --with-output requires a non-empty value" >&2
                exit 1
            fi
            ;;
        *)
            echo "Warning: Ignoring unknown option: $arg" >&2
            ;;
    esac
done

if [ "${CCFLAGS}" = "" ]; then
	case "$($CC --version 2>/dev/null)" in
    		*clang*) CCFLAGS="-O3 -flto" ;;
    		*gcc*) CCFLAGS="-O3 -flto=2" ;;
    		*) CCFLAGS="-O3 -flto" ;;
	esac
fi

OS=$(uname -s)
if [ "$OS" = "Linux" ]; then
    MACRO_EXT=.so
    STATIC=-static
    LINK_GMP="-lgmp -lm"
    LINK_SECP256K1="-lsecp256k1"
elif [ "$OS" = "Darwin" ]; then
    if ! command -v brew >/dev/null 2>&1; then
        echo "Error: Homebrew is required on macOS to locate GMP" >&2
        exit 1
    fi
    GMP_PATH="$(brew --prefix gmp)/lib/libgmp.a"
    if [ ! -f "$GMP_PATH" ]; then
        echo "Error: libgmp.a not found at $GMP_PATH. Install with 'brew install gmp'" >&2
        exit 1
    fi
    MACRO_EXT=.dylib
    STATIC=
    LINK_SECP256K1="-lsecp256k1"
    LINK_GMP="$GMP_PATH"
else
    echo "Unsupported platform: $OS"
    exit 1
fi

export CC CCFLAGS RUSTC FAMC OUTPUT MACRO_EXT STATIC FILTER ASAN RUSTFLAGS
