#!/usr/bin/env python
import os
import sys
from multiprocessing import Pool

if len(sys.argv) < 2:
    print("need num node")
    sys.exit()

num_node = int(sys.argv[1])


socket_prefix = "127.0.0.1"
p2p_port = 8000

sockets = []
neighbors = []
for i in range(num_node):
    socket = socket_prefix + ":" + str(p2p_port+i)
    neighbor = "../workspace/neighbor" + str(i)
    sockets.append(socket)
    neighbors.append(neighbor)

print(sockets)
print(neighbors)

for i in range(num_node):
    cmd = ["../target/debug/system_rust", "--ip", sockets[i], "--neighbor", neighbors[i]]
    



