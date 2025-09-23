#!/bin/bash

# This script should be started from the
#  root directory (".." from here).

# Start the orchestrator (from the nodes in my cluster).
# $1 : index of the current node.
# $2 : node state => "[(1.2,3.4);0.5]"
# $3 : is controller? (only for centralized)

./app_lev_orc_aarch64 3 "$(hostname -I):8080" "192.168.1.210" $1 0 $2 2 $3 & pid=$!

echo $pid > experiments/pid.txt
