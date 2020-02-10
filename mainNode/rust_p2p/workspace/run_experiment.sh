#!/bin/bash
trap kill_test INT

function kill_test() {
	for pid in $pids; do 
		echo "Kill $pid"
		kill $pid
	done	
}

pids="" 

../target/debug/system_rust --ip 127.0.0.1 --port 40000 --neighbor neighbor0 --api_port 41000 --account account0 & 
pid="$!"
echo $pid
pids="$pids $pid"

../target/debug/system_rust --ip 127.0.0.1 --port 40001 --neighbor neighbor1 --api_port 41001 --account account1 &
pid="$!"
echo $pid
pids="$pids $pid"

for pid in $pids; do 
	wait $pid
done

