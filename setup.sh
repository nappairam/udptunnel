#!/usr/bin/env bash

# Create two network namespaces
ip netns add 'peer1'
ip netns add 'peer2'

# Create a veth virtual-interface pair
ip link add 'peer1-eth0' type veth peer name 'peer2-eth0'

# Assign the interfaces to the namespaces
ip link set 'peer1-eth0' netns 'peer1'
ip link set 'peer2-eth0' netns 'peer2'

# Change the names of the interfaces (I prefer to use standard interface names)
ip netns exec 'peer1' ip link set 'peer1-eth0' name 'eth0'
ip netns exec 'peer2' ip link set 'peer2-eth0' name 'eth0'

# Assign an address to each interface
ip netns exec 'peer1' ip addr add 192.168.1.1/24 dev eth0
ip netns exec 'peer2' ip addr add 192.168.1.2/24 dev eth0

# Bring up the interfaces (the veth interfaces the loopback interfaces)
ip netns exec 'peer1' ip link set 'lo' up
ip netns exec 'peer1' ip link set 'eth0' up
ip netns exec 'peer2' ip link set 'lo' up
ip netns exec 'peer2' ip link set 'eth0' up

# Configure routes
#ip netns exec 'peer1' ip route add default via 192.168.1.1 dev eth0
#ip netns exec 'peer2' ip route add default via 192.168.2.1 dev eth0

# Test the connection (in both directions)
ip netns exec 'peer1' ping -c 1 192.168.1.2
ip netns exec 'peer2' ping -c 1 192.168.1.1
