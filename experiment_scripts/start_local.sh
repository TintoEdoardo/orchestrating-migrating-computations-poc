#!/bin/bash

# $1 : has enqueued requests?
# $2 : centralized/distributed
# $3 : node_0/node_1/node_2
# $4 : node state
# $5 : number of iteration

node_1="192.168.1.210"
node_2="192.168.1.113"
node_3="192.168.1.126"

su_factor_2="0.5"
su_factor_3="0.5"

experiment_time="35"

# Get the pwd for the nodes.
pwd_ns=$(cat node_user_password.txt)

# Configure the experiment folder.
./experiment_scripts/configure_folder.sh $1

if [ $2 == "centralized" ]; then
  # Prepare the log file.
  echo -n "$5 " >> experiment_data/convergence_c.txt
  echo -n "$5 " >> experiment_data/iterations_c.txt
  # Configure the experiment folder.
  echo $(cat node_user_password.txt) | sudo -S ./experiment_scripts/enable_centralized.sh
  else
    # Prepare the log file.
    echo -n "$5 " >> experiment_data/convergence_d.txt
    echo -n "$5 " >> experiment_data/iterations_d.txt
    # Configure the experiment folder.
    echo $(cat node_user_password.txt) | sudo -S ./experiment_scripts/enable_distributed.sh
  fi

# Alter the node state.
sed -i "4s/.*/$4/" experiment_folder/$3

# Prepare the log file.
# echo -n "$5 " >> experiment_data/send.txt
# echo -n "$5 " >> experiment_data/receive.txt

# Start the lp_task.
# sudo timeout $experiment_time ./experiment_scripts/lp_task_aarch64

# Start the experiment.
cd experiment_folder/
sudo timeout $experiment_time ./app_lev_orc $3
