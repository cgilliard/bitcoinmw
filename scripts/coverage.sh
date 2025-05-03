#!/bin/sh

. ./scripts/parse_params.sh "$@"

CCFLAGS=-DTEST
. ./scripts/build_c.sh "$@"
ar rcs .obj/libtest.a .obj/*.o || exit 1;

# Use rustc for tests
RUSTC=rustc
RUSTFLAGS="-C instrument-coverage -C opt-level=0"
export LLVM_PROFILE_FILE="/tmp/file.profraw"

# build macros
COMMAND="${RUSTC} --crate-type=proc-macro --edition=2021 rust/macros/lib.rs -o .obj/libbitcoinmw_macros${MACRO_EXT}"
${COMMAND} || exit 1;

COMMAND="${RUSTC} -C debuginfo=2 \
--test rust/bmw/mod.rs \
-o bin/test_bmw \
-L .obj \
-l static=test \
-l static=secp256k1 \
-l static=gmp \
--extern bitcoinmw_macros=.obj/libbitcoinmw_macros${MACRO_EXT} \
${RUSTFLAGS} \
--verbose"
${COMMAND} ||  exit 1;

COMMAND="./bin/test_bmw ${FILTER} --test-threads=1"
${COMMAND} || exit 1;

git log -1 > /tmp/coverage.txt
grcov /tmp/file.profraw --branch --binary-path ./bin -t lcov > /tmp/coverage.txt

cur_file='';
line_count=0;
cov_count=0;
line_count_sum=0;
cov_count_sum=0;

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

echo "Code Coverage Report for commit: $GREEN`git log -1 | grep "^commit " | cut -f2 -d ' '`$RESET";
echo "$BLUE----------------------------------------------------------------------------------------------------$RESET";


for line in $(cat /tmp/coverage.txt)
do
        if [ "`echo $line | grep "^SF:"`" != "" ]; then
                cur_file=`echo $line | cut -f2 -d ':'`;
        fi
        if [ "`echo $line | grep "^DA:"`" != "" ]; then
                #echo "da: $line";
                line_count=$((1 + line_count));
                line_count_sum=$((1 + line_count_sum));
                if [ "`echo $line | cut -f2 -d ','`" != "0" ]; then
                        cov_count=$((1 + cov_count));
                        cov_count_sum=$((1 + cov_count_sum));
                fi
        fi
        if [ "$line" = "end_of_record" ]; then
                percent=100;
                if [ "$line_count" != "0" ]; then
                        percent=$(((cov_count * 100) / line_count));
                fi
                line_fmt="($cov_count/$line_count)";
                printf "Cov: $GREEN%3s%%$RESET Lines: $YELLOW%10s$RESET File: $CYAN%s$RESET\n" "$percent" "$line_fmt" "$cur_file"
                line_count=0;
                cov_count=0;
        fi
done
echo "$BLUE----------------------------------------------------------------------------------------------------$RESET";

percent=100;
if [ "$line_count_sum" != "0" ]; then
        percent=$(((cov_count_sum * 100) / line_count_sum));
fi
echo "Summary: $GREEN$percent%$RESET Lines: $YELLOW($cov_count_sum/$line_count_sum)$RESET!"
codecov=`printf "%.2f" $percent`;
timestamp=`date +%s`
echo "$codecov" > /tmp/cc_final;
echo "$timestamp $codecov $cov_count_sum $line_count_sum" > /tmp/cc.txt

