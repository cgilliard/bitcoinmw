#!/bin/bash

export cc_final=`cat /tmp/cc_final`;
cp .templates/code_coverage.html docs/code_coverage.html
cp .templates/README.md README.md
perl -pi -e 's/CODE_COVERAGE/$ENV{cc_final}/g' README.md

entries=`cat docs/cc.txt`;
rm -f /tmp/timestamps
rm -f /tmp/values
rm -f /tmp/lines
rm -f /tmp/covered
declare -a timestamps;
declare -a values;
declare -a lines;
declare -a covered
i=0;
for entry in $entries
do
        if [ $(expr $i % 4) == 0 ]
        then
                echo "format_date($entry * 1000 )," >> /tmp/timestamps
	elif [ $(expr $i % 4) == 1 ]; then
                last_value=`echo $entry`;
                echo "$entry," >> /tmp/values
	elif [ $(expr $i % 4) == 2 ]; then
                echo "$entry," >> /tmp/covered
	elif [ $(expr $i % 4) == 3 ]; then
		echo "$entry," >> /tmp/lines
        fi
        let i=i+1;
done

export coverage=`cat /tmp/values`;
export timestampsv=`cat /tmp/timestamps`;
export linesrep=`cat /tmp/lines`;
export coveredrep=`cat /tmp/covered`;
export cc_final=`cat /tmp/cc_final`;
export gcov="unimplemented";

perl -pi -e 's/REPLACESUMMARY/$ENV{gcov}/g' docs/code_coverage.html
perl -pi -e 's/REPLACECOVERAGE_SINGLE/$ENV{cc_final}/g' docs/code_coverage.html
perl -pi -e 's/REPLACECOVERAGE/$ENV{coverage}/g' docs/code_coverage.html
perl -pi -e 's/REPLACETIMESTAMP/$ENV{timestampsv}/g' docs/code_coverage.html
perl -pi -e 's/REPLACELINES/$ENV{linesrep}/g' docs/code_coverage.html
perl -pi -e 's/REPLACECCLINES/$ENV{coveredrep}/g' docs/code_coverage.html

