# Start the orchestrator (from the nodes in my cluster).
# $1 : number of nodes.
# $2 : index of the current node.
# $3 : node state => "[(1.2,3.4);0.5]"
# $4 : affinity
# $5 : is controller? (ony for centralized)
$IP = hostname -I
./out/app_lev_orc_aarch64 $1 "$IP:8080" "192.168.1.210" $2 0 $3 $4 $5