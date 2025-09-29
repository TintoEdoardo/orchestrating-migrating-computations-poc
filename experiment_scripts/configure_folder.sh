#!/bin/bash

# This script should be started from the
#  root directory (".." from here).

# $1 : number of pending requests.

chmod u+x experiment_scripts/start_orchestrator.sh
chmod u+x experiment_scripts/stop_orchestrator.sh
chmod u+x experiment_scripts/main.sh
chmod u+x experiment_scripts/enable_centralized.sh
chmod u+x experiment_scripts/enable_distributed.sh

# Remove any existing folder.
rm -r expericment_folder

# Create a folder for running the experiments.
mkdir experiment_folder

# Copy the requests folder and the binaries.
if [ $1 -gt 0 ]; then
  cp -r requests experiment_folder/requests
else
  cp -r requests_empty experiment_folder/requests
fi
cp out/app_lev_orc_aarch64_c experiment_folder/app_lev_orc_c
cp out/app_lev_orc_aarch64_d experiment_folder/app_lev_orc_d

cp experiment_data/nodes_conf/node_0 experiment_folder/node_0
cp experiment_data/nodes_conf/node_1 experiment_folder/node_1
cp experiment_data/nodes_conf/node_2 experiment_folder/node_2