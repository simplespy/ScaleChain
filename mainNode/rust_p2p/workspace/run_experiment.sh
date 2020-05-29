#!/bin/bash
trap kill_test INT

function kill_test() {
	for pid in $pids; do 
		echo "Kill $pid"
		kill $pid
	done	
}

pids="" 

RUST_LOG=info ../target/debug/system_rust --ip 127.0.0.1 --port 40000 --neighbor neighbor0 --api_port 41000 --account account0 --has_token 1 & 
pid="$!"
echo $pid
pids="$pids $pid"

RUST_LOG=info ../target/debug/system_rust --ip 127.0.0.1 --port 40001 --neighbor neighbor1 --api_port 41001 --account account1 --has_token 0 &
pid="$!"
echo $pid
pids="$pids $pid"

RUST_LOG=info ../target/debug/system_rust --ip 127.0.0.1 --port 40002 --neighbor neighbor1 --api_port 41002 --account account1 --has_token 0 &
pid="$!"
echo $pid
pids="$pids $pid"

RUST_LOG=info ../target/debug/system_rust --ip 127.0.0.1 --port 40003 --neighbor neighbor1 --api_port 41003 --account account1 --has_token 0 &
pid="$!"
echo $pid
pids="$pids $pid"

for pid in $pids; do 
	wait $pid
done

