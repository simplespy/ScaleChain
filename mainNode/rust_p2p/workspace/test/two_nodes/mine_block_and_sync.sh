#!/bin/bash

curl 'localhost:41000/blockchain/get-curr-state'
curl 'localhost:41000/contract/get-curr-state'
curl 'localhost:41000/mempool/change-size?size=1'
curl 'localhost:41001/mempool/change-size?size=1000000'

curl "localhost:41000/transaction-generator/step?step=1"


