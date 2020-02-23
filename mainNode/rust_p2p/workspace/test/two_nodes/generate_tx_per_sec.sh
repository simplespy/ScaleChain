#!/bin/bash

num_tx=$1
curl 'localhost:41000/mempool/change-size?size=1000000'
curl 'localhost:41001/mempool/change-size?size=1000000'

while true; do
	cmd0="localhost:41000/transaction-generator/step?step=${num_tx}"
	cmd1="localhost:41001/transaction-generator/step?step=${num_tx}"
	echo $cmd0
	echo $cmd1
	curl $cmd0
	curl $cmd1
	sleep 1
done

