#!/bin/bash

# Retrieve the pid of the orchestrator task.
pid="$(cat experiment_folder/pid.txt)"

# Then terminate it.
kill $pid