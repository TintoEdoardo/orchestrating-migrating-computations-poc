# Start the orchestrator.
# $1 : number of nodes.
# $2 : index of the current node.
# $3 : node state => "[(1.2,3.4);0.5]"
# $4 : affinity
# $5 : is controller? (only for centralized)
cd out
./app_lev_orc_x86 $1 "$(hostname -I):8080" "192.168.1.210" $2 0 $3 $4 $5