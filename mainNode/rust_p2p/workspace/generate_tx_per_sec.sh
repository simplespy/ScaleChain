#!/bin/bash

num_tx=$1
curl 'localhost:41004/mempool/change-size?size=1000000'
curl 'localhost:41005/mempool/change-size?size=1000000'

while true; do
	cmd0="localhost:41004/transaction-generator/step?step=${num_tx}"
	cmd1="localhost:41005/transaction-generator/step?step=${num_tx}"
	curl $cmd0
	curl $cmd1
	sleep 1
done

