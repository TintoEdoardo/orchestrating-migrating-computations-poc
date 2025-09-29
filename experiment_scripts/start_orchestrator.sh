#!/bin/bash

# This script should be started from the
#  root directory (".." from here).

# Start the orchestrator (from the nodes in my cluster).
# $1 : node_i.conf

cd experiment_folder
./app_lev_orc $1 & pid=$!

echo $pid > pid.txt
