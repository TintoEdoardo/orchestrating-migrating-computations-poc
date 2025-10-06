#!/bin/bash

#################################################
#  Main script to orchestrate the experiments.  #
#################################################

node_1="192.168.1.210"
node_2="192.168.1.113"
node_3="192.168.1.126"

# Start the low priority task in each node.
echo "START LP TASKS"
sshpass -f node_user_password.txt ssh ubuntu@$node_1 "cd orchestrating-migrating-computations-poc; echo $(cat node_user_password.txt) | sudo -S screen -d -m timeout 3600 experiment_scripts/lp_task_aarch64"; echo ""
sshpass -f node_user_password.txt ssh ubuntu@$node_2 "cd orchestrating-migrating-computations-poc; echo $(cat node_user_password.txt) | sudo -S screen -d -m timeout 3600 experiment_scripts/lp_task_aarch64"; echo ""
sshpass -f node_user_password.txt ssh ubuntu@$node_3 "cd orchestrating-migrating-computations-poc; echo $(cat node_user_password.txt) | sudo -S screen -d -m timeout 3600 experiment_scripts/lp_task_aarch64"; echo ""
echo "LP TASKS STARTED"

while read line; do

  echo "> CONFIGURATION is $line"

  echo "  distributed"
  sshpass -f node_user_password.txt ssh ubuntu@$node_1 "cd orchestrating-migrating-computations-poc; echo $(cat node_user_password.txt) | sudo -S screen -d -m timeout 10 sudo ./experiment_scripts/start_local.sh 0 distributed node_0"; echo ""
  sshpass -f node_user_password.txt ssh ubuntu@$node_2 "cd orchestrating-migrating-computations-poc; echo $(cat node_user_password.txt) | sudo -S screen -d -m timeout 10 sudo ./experiment_scripts/start_local.sh 1 distributed node_1"; echo ""
  sshpass -f node_user_password.txt ssh ubuntu@$node_3 "cd orchestrating-migrating-computations-poc; echo $(cat node_user_password.txt) | sudo -S screen -d -m timeout 10 sudo ./experiment_scripts/start_local.sh 0 distributed node_2"; echo ""

  mosquitto_pub -h 192.168.1.210 -t federation/migration -m "2#[0;100;128;(1.0,1.0);2.0]"

  sleep 10s

  echo "  centralized"
  sshpass -f node_user_password.txt ssh ubuntu@$node_1 "cd orchestrating-migrating-computations-poc; echo $(cat node_user_password.txt) | sudo -S screen -d -m timeout 10 sudo ./experiment_scripts/start_local.sh 0 centralized node_0"; echo ""
  sshpass -f node_user_password.txt ssh ubuntu@$node_2 "cd orchestrating-migrating-computations-poc; echo $(cat node_user_password.txt) | sudo -S screen -d -m timeout 10 sudo ./experiment_scripts/start_local.sh 1 centralized node_1"; echo ""
  sshpass -f node_user_password.txt ssh ubuntu@$node_3 "cd orchestrating-migrating-computations-poc; echo $(cat node_user_password.txt) | sudo -S screen -d -m timeout 10 sudo ./experiment_scripts/start_local.sh 0 centralized node_2"; echo ""

  mosquitto_pub -h 192.168.1.210 -t federation/migration -m "2#[0;100;128;(1.0,1.0);2.0]"

  sleep 10s

done < experiment_data/distances.txt