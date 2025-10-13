#!/bin/bash

# $1 : has enqueued requests?
# $2 : centralized/distributed
# $3 : node_0/node_1/node_2
# $4 : node state

node_1="192.168.1.210"
node_2="192.168.1.113"
node_3="192.168.1.126"

su_factor_2="0.5"
su_factor_3="0.5"

# Get the pwd for the nodes.
pwd_ns=$(cat node_user_password.txt)

# Configure the experiment folder.
./experiment_scripts/configure_folder.sh $1

if [ $2 == "centralized" ]; then
  # Configure the experiment folder.
  echo $(cat node_user_password.txt) | sudo -S ./experiment_scripts/enable_centralized.sh
  else
    # Configure the experiment folder.
    echo $(cat node_user_password.txt) | sudo -S ./experiment_scripts/enable_distributed.sh
  fi

# Alter the node state.
sed -i "3s/.*/$4/" experiment_folder/$3

# Start the experiment.
cd experiment_folder/
sudo ./app_lev_orc $3
