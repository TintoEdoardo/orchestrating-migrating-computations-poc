#!/bin/bash

# $1 : has enqueued requests?
# $2 : centralized/distributed
# $3 : node_0/node_1/node_2

node_1="192.168.1.210"
node_2="192.168.1.113"
node_3="192.168.1.126"

su_factor_2="0.5"
su_factor_3="0.5"

# Get the pwd for the nodes.
pwd_ns=$(cat node_user_password.txt)

# Configure the experiment folder.
./experiment_scripts/configure_folder.sh $1 ; echo ""

if [ $2 == "centralized" ]; then
  # Configure the experiment folder.
  sudo ./experiment_scripts/enable_centralized.sh; echo ""
  else
    # Configure the experiment folder.
    sudo ./experiment_scripts/enable_distributed.sh; echo ""
  fi

# Start the orchestrator.
sudo ./experiment_scripts/start_orchestrator.sh "'$3'"
