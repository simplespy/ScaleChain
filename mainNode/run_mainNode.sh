#!/bin/sh
for i in {1..10}
do
	echo "---Sending Block $i---"
	node mainNode.js -s --config $1
	sleep 5
	node mainNode.js -i --config $1
done
