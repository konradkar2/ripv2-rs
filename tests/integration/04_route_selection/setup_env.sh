#!/usr/bin/env bash
set -euo pipefail

#   r2
#    |
#   r1 --- r3 --- r4 --- r5
#
#   r1-r2: 10.0.12.0/24
#   r1-r3: 10.0.13.0/24
#   r3-r4: 10.0.34.0/24
#   r4-r5: 10.0.45.0/24
#
#   Both r2 and r5 advertise 10.99.0.0/24. From r1, r2 is closer.

ip netns add r1
ip netns add r2
ip netns add r3
ip netns add r4
ip netns add r5

ip link add r1-r2 type veth peer name r2-r1
ip link add r1-r3 type veth peer name r3-r1
ip link add r3-r4 type veth peer name r4-r3
ip link add r4-r5 type veth peer name r5-r4

ip link set r1-r2 netns r1
ip link set r2-r1 netns r2
ip link set r1-r3 netns r1
ip link set r3-r1 netns r3
ip link set r3-r4 netns r3
ip link set r4-r3 netns r4
ip link set r4-r5 netns r4
ip link set r5-r4 netns r5

for namespace in r1 r2 r3 r4 r5; do
    ip netns exec "$namespace" ip link set lo up
done

ip netns exec r1 ip addr add 10.0.12.1/24 dev r1-r2
ip netns exec r2 ip addr add 10.0.12.2/24 dev r2-r1
ip netns exec r1 ip addr add 10.0.13.1/24 dev r1-r3
ip netns exec r3 ip addr add 10.0.13.3/24 dev r3-r1
ip netns exec r3 ip addr add 10.0.34.3/24 dev r3-r4
ip netns exec r4 ip addr add 10.0.34.4/24 dev r4-r3
ip netns exec r4 ip addr add 10.0.45.4/24 dev r4-r5
ip netns exec r5 ip addr add 10.0.45.5/24 dev r5-r4

ip netns exec r1 ip link set r1-r2 up
ip netns exec r2 ip link set r2-r1 up
ip netns exec r1 ip link set r1-r3 up
ip netns exec r3 ip link set r3-r1 up
ip netns exec r3 ip link set r3-r4 up
ip netns exec r4 ip link set r4-r3 up
ip netns exec r4 ip link set r4-r5 up
ip netns exec r5 ip link set r5-r4 up

ip netns exec r2 ip link add target2 type dummy
ip netns exec r2 ip addr add 10.99.0.2/24 dev target2
ip netns exec r2 ip link set target2 up

ip netns exec r5 ip link add target5 type dummy
ip netns exec r5 ip addr add 10.99.0.5/24 dev target5
ip netns exec r5 ip link set target5 up

for namespace in r1 r2 r3 r4 r5; do
    ip netns exec "$namespace" sysctl -w net.ipv4.ip_forward=1
done
