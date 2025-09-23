#!/bin/bash

# This script should be started from the
#  root directory (".." from here).

# $1 : number of pending requests.

chmod u+x experiment_script/start_orchestrator.sh
chmod u+x experiment_script/stop_orchestrator.sh
chmod u+x experiment_script/main.sh
chmod u+x experiment_script/enable_centralized.sh
chmod u+x experiment_script/enable_distributed.sh

# Remove any existing folder.
rm -r experiments

# Create a folder for running the experiments.
mkdir experiments

# Copy the requests folder and the binaries.
if [ $1 -gt 0 ]; then
  cp -r requests experiments/requests
else
  cp -r requests_empty experiments/requests
fi
cp out/app_lev_orc_aarch64_c experiments/app_lev_orc_c
cp out/app_lev_orc_aarch64_d experiments/app_lev_orc_d
