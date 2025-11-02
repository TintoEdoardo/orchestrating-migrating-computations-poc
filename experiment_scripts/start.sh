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
echo "SAVE AND CLEAR THE LOG FILES"
sshpass -f node_user_password.txt ssh ubuntu@$node_1 "cd orchestrating-migrating-computations-poc; cd experiment_scripts; chmod u+x clear_log.sh; ./clear_log.sh"; echo ""
sshpass -f node_user_password.txt ssh ubuntu@$node_2 "cd orchestrating-migrating-computations-poc; cd experiment_scripts; chmod u+x clear_log.sh; ./clear_log.sh"; echo ""
sshpass -f node_user_password.txt ssh ubuntu@$node_3 "cd orchestrating-migrating-computations-poc; cd experiment_scripts; chmod u+x clear_log.sh; ./clear_log.sh"; echo ""
echo "LOG FILES SAVED AND CLEARED"

: <<'COMMENT'

# Experiment 1.
iteration=0
while read -u 9 line; do

  read state_0 state_1 state_2 request <<< $line

  echo "> CONFIGURATION is $state_0 $state_1 $state_2 $request"

  echo "  distributed"
  sshpass -f node_user_password.txt ssh ubuntu@$node_3 "cd orchestrating-migrating-computations-poc; echo $(cat node_user_password.txt) | sudo -S screen -d -m ./experiment_scripts/start_local.sh 0 distributed node_2 \"$state_2\" $iteration"; echo ""
  sshpass -f node_user_password.txt ssh ubuntu@$node_2 "cd orchestrating-migrating-computations-poc; echo $(cat node_user_password.txt) | sudo -S screen -d -m ./experiment_scripts/start_local.sh 1 distributed node_1 \"$state_1\" $iteration"; echo ""
  sshpass -f node_user_password.txt ssh ubuntu@$node_1 "cd orchestrating-migrating-computations-poc; echo $(cat node_user_password.txt) | sudo -S screen -d -m ./experiment_scripts/start_local.sh 0 distributed node_0 \"$state_0\" $iteration"; echo ""

  sleep 10s

  mosquitto_pub -h 192.168.1.210 -t federation/migration -m "$request"

  sleep 60s

  echo "  centralized"
  sshpass -f node_user_password.txt ssh ubuntu@$node_3 "cd orchestrating-migrating-computations-poc; echo $(cat node_user_password.txt) | sudo -S screen -d -m ./experiment_scripts/start_local.sh 0 centralized node_2 \"$state_2\" $iteration"; echo ""
  sshpass -f node_user_password.txt ssh ubuntu@$node_2 "cd orchestrating-migrating-computations-poc; echo $(cat node_user_password.txt) | sudo -S screen -d -m ./experiment_scripts/start_local.sh 1 centralized node_1 \"$state_1\" $iteration"; echo ""
  sshpass -f node_user_password.txt ssh ubuntu@$node_1 "cd orchestrating-migrating-computations-poc; echo $(cat node_user_password.txt) | sudo -S screen -d -m ./experiment_scripts/start_local.sh 0 centralized node_0 \"$state_0\" $iteration"; echo ""

  sleep 10s

  mosquitto_pub -h 192.168.1.210 -t federation/migration -m "$request"

  sleep 60s

  ((iteration++))
done 9< experiment_data/distances.txt
COMMENT

# Experiment 2.
for (( i=1; i<=50; i++))
do

  echo "RUN $i"

  state_0="[(4,2.2);1]"
  state_1="[(91,87.4);0.2]"
  state_2="[(21,46);0.1]"

  sshpass -f node_user_password.txt ssh ubuntu@$node_1 "cd orchestrating-migrating-computations-poc; echo $(cat node_user_password.txt) | sudo -S screen -d -m ./experiment_scripts/start_local_2.sh 0 centralized node_0 \"$state_0\" $i"; echo ""
  sshpass -f node_user_password.txt ssh ubuntu@$node_2 "cd orchestrating-migrating-computations-poc; echo $(cat node_user_password.txt) | sudo -S screen -d -m ./experiment_scripts/start_local_2.sh 1 centralized node_1 \"$state_1\" $i"; echo ""
  sshpass -f node_user_password.txt ssh ubuntu@$node_3 "cd orchestrating-migrating-computations-poc; echo $(cat node_user_password.txt) | sudo -S screen -d -m ./experiment_scripts/start_local_2.sh 0 centralized node_2 \"$state_2\" $i"; echo ""

  sleep 110s

  mosquitto_pub -h 192.168.1.210 -t "federation/node_available" -m "node_available"

  sleep 320s
done
