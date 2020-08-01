#!/bin/bash
trap kill_test INT

if [ "$#" -lt 1 ]; then
	echo "Usgae ./run_experiment.sh help"
	exit 0
fi



function kill_test() {
	for pid in $pids; do 
		echo "Kill $pid"
		kill $pid
	done	
}

function start_scale_node() {
	port=$(($1+40000))
	api_port=$((41000+$1))
	keyfile='keyfile/node'$1
	account='accounts/account'$1
	RUST_LOG=info ../target/debug/system_rust --ip 127.0.0.1 --port "$port" --neighbor neighbors --side_node side_node --api_port "$api_port" --account "$account" --key "$keyfile" --scale_id $1&
	pid="$!"
	echo $pid
	pids="$pids $pid"
}

function start_side_node() {
	port=$((40000+$1))
	api_port=$((41000+$1))
	keyfile='keyfile/node'$1
	account='accounts/account'$1

	# only node 1 has token to start with
	if [ $1 == $2 ]; then
		RUST_LOG=info ../target/debug/system_rust --ip 127.0.0.1 --port "$port" --neighbor neighbors --side_node side_node --api_port "$api_port" --account "$account" --key "$keyfile" --has_token --scale_id 0&
	else
		RUST_LOG=info ../target/debug/system_rust --ip 127.0.0.1 --port $port --neighbor neighbors --side_node side_node --api_port $api_port --account $account --key $keyfile --scale_id 0&
	fi
	pid="$!"
	echo $pid
	return $pid
}

function start_trans {
	sh ./scripts/start.sh
}

function config() {
	start_file="scripts/start.sh"
	neighbor_file="neighbors"
	side_file="side_node"
	tel_file="telematics/nodes.txt"
	total=$(expr $1 + $2)

	# neighbors
	rm  $neighbor_file
	for (( i = 1; i<=$total ; i++ )); do
		node=$(expr 40000 + $i)
		echo "127.0.0.1:${node}">> $neighbor_file
	done

	# sidenodes + start
	rm $side_file
	rm $tel_file
	rm $start_file
	echo "#!/bin/bash" >> $start_file
	chmod +x $start_file
	for (( i = $1+1 ; i<=$total ; i++)); do
		node=$(expr 40000 + $i)
		echo "127.0.0.1:${node}" >> $side_file
		api=$(expr 41000 + $i)
		echo "curl "localhost:${api}/transaction-generator/start?interval=1000"" >> $start_file
		echo "$i,127.0.0.1,$api" >> $tel_file
	done	
}

function start_local() {
	if [ "$#" -ne 2 ]; then
		echo "Usgae ./run_experiment.sh start_local <NUM SCALE NODE> <NUM SIDE NODE>"
		exit 0
	fi


	num_scale_node=$1
	num_side_node=$2

	# config files
	config $1 $2

	pids="" 

	sn=`seq 1 $num_scale_node`
	for i in $sn
	do 
		start_scale_node $i 
		pids="$pids $pid"
	done

	s=$(expr $num_scale_node + 1)
	e=$(expr $num_scale_node + $num_side_node)

	tn=`seq $s $e`
	for i in $tn 
	do 
		start_side_node $i $s
		pids="$pids $pid"
	done

	for pid in $pids; do 
		wait $pid
	done
}


case $1 in 
	help) 
		cat <<- EOF
		Helper funciton 

			Run local experiment 
				start i i
				gen		
		EOF
		;;	
	start)
		start_local $2 $3 ;;
	gen)
		start_trans ;;
esac
		
