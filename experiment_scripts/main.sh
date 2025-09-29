#!/bin/bash

#################################################
#  Main script to orchestrate the experiments.  #
#################################################

node_1="192.168.1.210"
node_2="192.168.1.113"
node_3="192.168.1.126"

su_factor_2="0.5"
su_factor_3="0.5"

# Get the pwd for the nodes.
pwd_ns=$(cat node_user_password.txt)

# Start the low priority task.
echo "START LP TASKS"
sshpass -f node_user_password.txt ssh ubuntu@$node_1 "cd orchestrating-migrating-computations-poc; echo $(cat node_user_password.txt) | sudo -S screen -d -m timeout 3600 experiment_scripts/lp_task_aarch64" &> /dev/null
sshpass -f node_user_password.txt ssh ubuntu@$node_1 "cd orchestrating-migrating-computations-poc; echo $(cat node_user_password.txt) | sudo -S screen -d -m timeout 3600 experiment_scripts/lp_task_aarch64" &> /dev/null
sshpass -f node_user_password.txt ssh ubuntu@$node_1 "cd orchestrating-migrating-computations-poc; echo $(cat node_user_password.txt) | sudo -S screen -d -m timeout 3600 experiment_scripts/lp_task_aarch64" &> /dev/null
echo "LP TASKS STARTED"

declare -a modes=("centralized" "distributed")
for mode in $modes; do

  echo "MODES IS $mode"

  # First experiment.
  echo "START EXPERIMENT 1"
  while read line; do

    echo "LINE IS $line"

    # Read the three states.
    # read state_1 state_2 new_state_2 state_3 request <<< $line

    request="2#[0;100;128;(1.5,2.0);2.0]"
    echo $request

    # Produce the node states.
    # node_state_1="[$state_1;1]"
    # node_state_2="[$state_2;$su_factor_2]"
    # node_state_3="[$state_3;$su_factor_3]"

    # Configure the experiment folder.
    echo "CONFIGURE EXPERIMENT FOLDER"
    sshpass -f node_user_password.txt ssh ubuntu@$node_1 "cd orchestrating-migrating-computations-poc; echo $(cat node_user_password.txt) | sudo -S screen -d -m experiment_scripts/configure_folder.sh 0" &> /dev/null
    sshpass -f node_user_password.txt ssh ubuntu@$node_2 "cd orchestrating-migrating-computations-poc; echo $(cat node_user_password.txt) | sudo -S screen -d -m experiment_scripts/configure_folder.sh 1" &> /dev/null
    sshpass -f node_user_password.txt ssh ubuntu@$node_3 "cd orchestrating-migrating-computations-poc; echo $(cat node_user_password.txt) | sudo -S screen -d -m experiment_scripts/configure_folder.sh 0" &> /dev/null
    echo "EXPERIMENT FOLDER CONFIGURED"

    if [ $mode == "centralized" ]; then
        # Configure the experiment folder.
        sshpass -f node_user_password.txt ssh ubuntu@$node_1 'cd orchestrating-migrating-computations-poc; screen -d -m experiment_scripts/enable_centralized.sh'
        sshpass -f node_user_password.txt ssh ubuntu@$node_2 'cd orchestrating-migrating-computations-poc; screen -d -m experiment_scripts/enable_centralized.sh'
        sshpass -f node_user_password.txt ssh ubuntu@$node_3 'cd orchestrating-migrating-computations-poc; screen -d -m experiment_scripts/enable_centralized.sh'
      else
        # Configure the experiment folder.
        sshpass -f node_user_password.txt ssh ubuntu@$node_1 'cd orchestrating-migrating-computations-poc; screen -d -m experiment_scripts/enable_decentralized.sh'
        sshpass -f node_user_password.txt ssh ubuntu@$node_2 'cd orchestrating-migrating-computations-poc; screen -d -m experiment_scripts/enable_decentralized.sh'
        sshpass -f node_user_password.txt ssh ubuntu@$node_3 'cd orchestrating-migrating-computations-poc; screen -d -m experiment_scripts/enable_decentralized.sh'
      fi

    # Start the orchestrator.
    echo "START ORCHESTRATOR"
    sshpass -f node_user_password.txt ssh ubuntu@$node_1 "cd orchestrating-migrating-computations-poc; echo $(cat node_user_password.txt) | sudo -S screen -d -m experiment_scripts/start_orchestrator.sh 'node_0'"
    sshpass -f node_user_password.txt ssh ubuntu@$node_2 "cd orchestrating-migrating-computations-poc; echo $(cat node_user_password.txt) | sudo -S screen -d -m experiment_scripts/start_orchestrator.sh 'node_1'"
    sshpass -f node_user_password.txt ssh ubuntu@$node_3 "cd orchestrating-migrating-computations-poc; echo $(cat node_user_password.txt) | sudo -S screen -d -m experiment_scripts/start_orchestrator.sh 'node_2'"
    echo "ORCHESTRATION STARTED"

    # Start the experiment.
    mosquitto_pub -h $node_1 -t federation/migration -m "$request"
    # mosquitto_pub -h $node_1 -t node_state_2 -m "$new_state_2"

    # Pause to allow for convergence.
    sleep 5s

    # Stop the orchestrator.
    echo "STOP ORCHESTRATOR"
    sshpass -f node_user_password.txt ssh ubuntu@$node_1 "cd orchestrating-migrating-computations-poc; echo $(cat node_user_password.txt) | sudo -S screen -d -m experiment_scripts/stop_orchestrator.sh"
    sshpass -f node_user_password.txt ssh ubuntu@$node_2 "cd orchestrating-migrating-computations-poc; echo $(cat node_user_password.txt) | sudo -S screen -d -m experiment_scripts/stop_orchestrator.sh"
    sshpass -f node_user_password.txt ssh ubuntu@$node_3 "cd orchestrating-migrating-computations-poc; echo $(cat node_user_password.txt) | sudo -S screen -d -m experiment_scripts/stop_orchestrator.sh"
    echo "ORCHESTRATOR STOPPED"
  done<experiment_data/distances.txt

  echo "END OF EXPERIMENT 1"

done
