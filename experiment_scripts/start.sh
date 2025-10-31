#!/bin/bash

#################################################
#  Main script to orchestrate the experiments.  #
#################################################

node_1="192.168.1.210"
node_2="192.168.1.113"
node_3="192.168.1.126"

# Start the low priority task in each node.
# echo "START LP TASKS"
# sshpass -f node_user_password.txt ssh ubuntu@$node_1 "cd orchestrating-migrating-computations-poc; echo $(cat node_user_password.txt) | sudo -S screen -d -m timeout 28800 experiment_scripts/lp_task_aarch64"; echo ""
# sshpass -f node_user_password.txt ssh ubuntu@$node_2 "cd orchestrating-migrating-computations-poc; echo $(cat node_user_password.txt) | sudo -S screen -d -m timeout 28800 experiment_scripts/lp_task_aarch64"; echo ""
# sshpass -f node_user_password.txt ssh ubuntu@$node_3 "cd orchestrating-migrating-computations-poc; echo $(cat node_user_password.txt) | sudo -S screen -d -m timeout 28800 experiment_scripts/lp_task_aarch64"; echo ""
# echo "LP TASKS STARTED"

# Save and clear logs.
echo "CLEAR THE LOG FILES"
sshpass -f node_user_password.txt ssh ubuntu@$node_1 "cd orchestrating-migrating-computations-poc; cd experiment_scripts; chmod u+x clear_log.sh; ./clear_log.sh"; echo ""
sshpass -f node_user_password.txt ssh ubuntu@$node_2 "cd orchestrating-migrating-computations-poc; cd experiment_scripts; chmod u+x clear_log.sh; ./clear_log.sh"; echo ""
sshpass -f node_user_password.txt ssh ubuntu@$node_3 "cd orchestrating-migrating-computations-poc; cd experiment_scripts; chmod u+x clear_log.sh; ./clear_log.sh"; echo ""
echo "LOG FILES CLEARED"

# Experiment 1.
while read -u 9 line; do

  read state_0 state_1 state_2 request <<< $line

  echo "> CONFIGURATION is $state_0 $state_1 $state_2 $request"

  echo "  distributed"
  sshpass -f node_user_password.txt ssh ubuntu@$node_3 "cd orchestrating-migrating-computations-poc; echo $(cat node_user_password.txt) | sudo -S screen -d -m ./experiment_scripts/start_local.sh 0 distributed node_2 \"$state_2\""; echo ""
  sshpass -f node_user_password.txt ssh ubuntu@$node_2 "cd orchestrating-migrating-computations-poc; echo $(cat node_user_password.txt) | sudo -S screen -d -m ./experiment_scripts/start_local.sh 1 distributed node_1 \"$state_1\""; echo ""
  sshpass -f node_user_password.txt ssh ubuntu@$node_1 "cd orchestrating-migrating-computations-poc; echo $(cat node_user_password.txt) | sudo -S screen -d -m ./experiment_scripts/start_local.sh 0 distributed node_0 \"$state_0\""; echo ""

  sleep 7s

  mosquitto_pub -h 192.168.1.210 -t federation/migration -m "$request"

  sleep 35s

  echo "  centralized"
  sshpass -f node_user_password.txt ssh ubuntu@$node_1 "cd orchestrating-migrating-computations-poc; echo $(cat node_user_password.txt) | sudo -S screen -d -m ./experiment_scripts/start_local.sh 0 centralized node_0 \"$state_0\""; echo ""
  sshpass -f node_user_password.txt ssh ubuntu@$node_2 "cd orchestrating-migrating-computations-poc; echo $(cat node_user_password.txt) | sudo -S screen -d -m ./experiment_scripts/start_local.sh 1 centralized node_1 \"$state_1\""; echo ""
  sshpass -f node_user_password.txt ssh ubuntu@$node_3 "cd orchestrating-migrating-computations-poc; echo $(cat node_user_password.txt) | sudo -S screen -d -m ./experiment_scripts/start_local.sh 0 centralized node_2 \"$state_2\""; echo ""

  sleep 7s

  mosquitto_pub -h 192.168.1.210 -t federation/migration -m "$request"

  sleep 35s

done 9< experiment_data/distances.txt


: <<'COMMENT'
# Experiment 2.
for (( i=1; i<=200; i++))
do

  echo "RUN $i"

  state_0="[(21,46);1]"
  state_1="[(91,87.4);0.4]"
  state_2="[(1,2.2);0.6]"

  sshpass -f node_user_password.txt ssh ubuntu@$node_1 "cd orchestrating-migrating-computations-poc; echo $(cat node_user_password.txt) | sudo -S screen -d -m timeout 30 sudo ./experiment_scripts/start_local.sh 0 centralized node_0 \"$state_0\""; echo ""
  sshpass -f node_user_password.txt ssh ubuntu@$node_2 "cd orchestrating-migrating-computations-poc; echo $(cat node_user_password.txt) | sudo -S screen -d -m timeout 30 sudo ./experiment_scripts/start_local.sh 1 centralized node_1 \"$state_1\""; echo ""
  sshpass -f node_user_password.txt ssh ubuntu@$node_3 "cd orchestrating-migrating-computations-poc; echo $(cat node_user_password.txt) | sudo -S screen -d -m timeout 30 sudo ./experiment_scripts/start_local.sh 0 centralized node_2 \"$state_2\""; echo ""

  sleep 5s

  mosquitto_pub -h 192.168.1.210 -t "federation/node_available" -m "node_available"

  sleep 30s
done
COMMENT