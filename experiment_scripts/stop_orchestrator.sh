#!/bin/bash

# Retrieve the pid of the orchestrator task.
pid="$(cat experiments/pid.txt)"

# Then terminate it.
kill $pid