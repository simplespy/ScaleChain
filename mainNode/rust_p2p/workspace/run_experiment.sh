#!/bin/bash


trap "trap - SIGTERM && kill -- -$$" SIGINT SIGTERM EXIT

../target/debug/system_rust --ip 127.0.0.1:40000 --neighbor neighbor0 &
sleep 1

../target/debug/system_rust --ip 127.0.0.1:40001 --neighbor neighbor1 &

