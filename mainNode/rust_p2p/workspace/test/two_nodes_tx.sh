#!/bin/bash

curl 'localhost:41000/blockchain/get-curr-state'
curl 'localhost:41001/blockchain/get-curr-state'

curl 'localhost:41000/transaction-generator/step?step=1'
curl 'localhost:41000/contract/get-curr-state'
