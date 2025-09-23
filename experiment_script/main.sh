#!/bin/bash

#################################################
#  Main script to orchestrate the experiments.  #
#################################################

node_1="192.168.1.210"
node_2="192.168.1.113"
node_3="192.168.1.126"

su_factor_2="0.5"
su_factor_3="0.5"

# Start the low priority task.
sshpass -f node_user_password.txt ssh ubuntu@$node_1 'cd orchestrating-migrating-computations-poc; sudo timeout 3600 experiments_scripts/lp_task_aarch64'
sshpass -f node_user_password.txt ssh ubuntu@$node_1 'cd orchestrating-migrating-computations-poc; sudo timeout 3600 experiments_scripts/lp_task_aarch64'
sshpass -f node_user_password.txt ssh ubuntu@$node_1 'cd orchestrating-migrating-computations-poc; sudo timeout 3600 experiments_scripts/lp_task_aarch64'

declare -a modes=("centralized" "distributed")
for mode in $modes; do
  if [ $mode == "centralized" ]; then
    # Configure the experiment folder.
    sshpass -f node_user_password.txt ssh ubuntu@$node_1 'cd orchestrating-migrating-computations-poc; experiment_script/enable_centralized.sh'
    sshpass -f node_user_password.txt ssh ubuntu@$node_2 'cd orchestrating-migrating-computations-poc; experiment_script/enable_centralized.sh'
    sshpass -f node_user_password.txt ssh ubuntu@$node_3 'cd orchestrating-migrating-computations-poc; experiment_script/enable_centralized.sh'
  else
    # Configure the experiment folder.
    sshpass -f node_user_password.txt ssh ubuntu@$node_1 'cd orchestrating-migrating-computations-poc; experiment_script/enable_decentralized.sh'
    sshpass -f node_user_password.txt ssh ubuntu@$node_2 'cd orchestrating-migrating-computations-poc; experiment_script/enable_decentralized.sh'
    sshpass -f node_user_password.txt ssh ubuntu@$node_3 'cd orchestrating-migrating-computations-poc; experiment_script/enable_decentralized.sh'
  fi

  # First experiment. 
  for line in $(cat experiment_data/distances.txt); do
    # Read the three states.
    read state_1 state_2 new_state_2 state_3 request <<< $line

    # Produce the node states.
    node_state_1="[$state_1;1]"
    node_state_2="[$state_2;$su_factor_2]"
    node_state_3="[$state_3;$su_factor_3]"

    # Configure the experiment folder.
    sshpass -f node_user_password.txt ssh ubuntu@$node_1 'cd orchestrating-migrating-computations-poc; experiment_script/configure_folder.sh 0'
    sshpass -f node_user_password.txt ssh ubuntu@$node_2 'cd orchestrating-migrating-computations-poc; experiment_script/configure_folder.sh 1'
    sshpass -f node_user_password.txt ssh ubuntu@$node_3 'cd orchestrating-migrating-computations-poc; experiment_script/configure_folder.sh 0'

    # Start the orchestrator.
    sshpass -f node_user_password.txt ssh ubuntu@$node_1 "cd orchestrating-migrating-computations-poc; sudo experiment_script/start_orchestrator.sh 0 $node_state_1 1"
    sshpass -f node_user_password.txt ssh ubuntu@$node_2 "cd orchestrating-migrating-computations-poc; sudo experiment_script/start_orchestrator.sh 0 $node_state_2 0"
    sshpass -f node_user_password.txt ssh ubuntu@$node_3 "cd orchestrating-migrating-computations-poc; sudo experiment_script/start_orchestrator.sh 0 $node_state_3 0"

    # Start the experiment.
    mosquitto_pub -h $node_1 -t federation/migration -m "$request"
    # mosquitto_pub -h $node_1 -t node_state_2 -m "$new_state_2"

    # Pause to allow for convergence.
    sleep 5s

    # Stop the orchestrator.
    sshpass -f node_user_password.txt ssh ubuntu@$node_1 'cd orchestrating-migrating-computations-poc; sudo experiment_script/stop_orchestrator.sh'
    sshpass -f node_user_password.txt ssh ubuntu@$node_2 'cd orchestrating-migrating-computations-poc; sudo experiment_script/stop_orchestrator.sh'
    sshpass -f node_user_password.txt ssh ubuntu@$node_3 'cd orchestrating-migrating-computations-poc; sudo experiment_script/stop_orchestrator.sh'
  done

done
