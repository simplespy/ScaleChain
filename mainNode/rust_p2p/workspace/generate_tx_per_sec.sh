#!/bin/bash

num_tx=$1
cmd0="localhost:41004/transaction-generator/step?step=${num_tx}"
cmd1="localhost:41005/transaction-generator/step?step=${num_tx}"
curl $cmd0
curl $cmd1

while true; do
	cmd2="localhost:41000/contract/get-curr-state"
	curl $cmd2
	sleep 10
done
