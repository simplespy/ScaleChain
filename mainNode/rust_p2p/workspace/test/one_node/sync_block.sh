#!/bin/bash

curl 'localhost:41000/contract/get-curr-state'
curl 'localhost:41000/blockchain/get-curr-state'
curl 'localhost:41000/contract/sync-chain'
curl 'localhost:41000/contract/get-curr-state'
curl 'localhost:41000/blockchain/get-curr-state'
