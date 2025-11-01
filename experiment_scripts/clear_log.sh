#!/bin/bash

cd ../experiment_data

date=$(date +%Y_%m_%d_%H_%M)

mv convergence_c.txt previous/convergence_c_$date.txt
mv convergence_d.txt previous/convergence_d_$date.txt
mv iterations_c.txt previous/iterations_c_$date.txt
mv iterations_d.txt previous/iterations_d_$date.txt
mv send.txt previous/send_$date.txt
mv receive.txt previous/receive_$date.txt
mv migration.txt previous/migration_$date.txt

touch convergence_c.txt
touch convergence_d.txt
touch iterations_c.txt
touch iterations_d.txt
touch send.txt
touch receive.txt
