#!/bin/bash

echo "Reason=$3";
if [ "$3" == "Schedule" ] || [ "$3" == "Manual" ]; then
  echo "updating"
  cargo install cargo-tarpaulin
  cargo tarpaulin --all > /tmp/tarpaulin.out
  echo "cat /tmp/tarpaulin.out"
  cat /tmp/tarpaulin.out
  cd ~
  git clone https://anything:$1@github.com/cgilliard/bitcoinmw.git bmw_new
  cd bmw_new
  git config user.name "Pipelines-Bot"
  git checkout main
  last=$( tail -n 1 /tmp/tarpaulin.out )
  spl=( $last )
  str=${spl[0]}
  IFS='%';
  read -rasplitIFS<<< "$str"
  cur=${splitIFS[0]}
  re='^[0-9]+([.][0-9]+)?$'
  if ! [[ $cur =~ $re ]] ; then
    echo "error: Not a number" >&2; exit 1
  else
    echo "number ok $cur"
    IFS=' ';
    echo "$timestamp ${splitIFS[0]}" >> docs/tarpaulin_summary.txt
    cp README.md.template README.md
    export ccvalue=${splitIFS[0]}
    perl -pi -e 's/CODECOVERAGE/$ENV{ccvalue}/g' README.md
    chmod 755 ./.ci/make_cc_graph.sh
    ./.ci/make_cc_graph.sh

    git config --global user.email "pipelinesbot.noreply@example.com"
    git config --global user.name "Pipelines-Bot"

    git fetch
    if [ `git diff --exit-code origin/main..main | wc -l | xargs` = "0" ]; then
      git pull
      git add --all
      git commit -m"Pipelines-Bot: Updated repo (via tarpaulin script) Source Version is $2";
      git push https://$1@github.com/cgilliard/bitcoinmw.git
    else
      echo "There are changes after this checkout. Not committing!"
      git diff origin/main
      git diff origin/main | wc -l
    fi
  fi
else
  echo "Only executed on nightly build";
fi

