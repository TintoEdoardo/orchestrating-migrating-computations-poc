#!/bin/bash

# This script should be started from the
#  root directory (".." from here).

cd experiment_folder
rm app_lev_orc &> /dev/null
cp app_lev_orc_d app_lev_orc
