#!/bin/bash

# roughly a block size
num_tx=16

cmd_n4_tx="localhost:41004/transaction-generator/step?step=${num_tx}"
cmd_n5_tx="localhost:41005/transaction-generator/step?step=${num_tx}"
cmd_get_state="localhost:41001/contract/get-curr-state"




while true; do
	# node 4 send transactions
	curl $cmd_n4_tx
	curl $cmd_get_state

	# node 5 send transactions
	sleep 10
	curl $cmd_n5_tx
	curl $cmd_get_state
done
