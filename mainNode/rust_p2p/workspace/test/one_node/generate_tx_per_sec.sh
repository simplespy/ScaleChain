#!/bin/bash

num_tx=$1
curl 'localhost:41000/mempool/change-size?size=1000000'

while true; do
	cmd="localhost:41000/transaction-generator/step?step=${num_tx}"
	echo $cmd
	curl $cmd
	sleep 1
done

