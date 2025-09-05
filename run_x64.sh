# Start the orchestrator.
# $1 : number of nodes.
# $2 : index of the current node.
# $3 : node state => "[(1.2,3.4);0.5]"
# $4 : affinity
$IP = hostname -I
./out/app_lev_orc_x64 $1 "$IP:8080" "192.168.1.210" $2 0 $3 $4