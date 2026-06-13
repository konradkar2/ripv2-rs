#   r1 --- r2 --- r3
#
#   r1-r2: 10.0.12.0/24
#   r2-r3: 10.0.23.0/24
#   LAN r1: 10.1.1.0/24
#   LAN r3: 10.3.3.0/24

#create network namespaces
sudo ip netns add r1
sudo ip netns add r2
sudo ip netns add r3

#create interfaces
sudo ip link add r1-r2 type veth peer name r2-r1
sudo ip link add r2-r3 type veth peer name r3-r2

#assign interfaces to namespaces
sudo ip link set r1-r2 netns r1
sudo ip link set r2-r1 netns r2
sudo ip link set r2-r3 netns r2
sudo ip link set r3-r2 netns r3

#enable loopback interfaces
sudo ip netns exec r1 ip link set lo up
sudo ip netns exec r2 ip link set lo up
sudo ip netns exec r3 ip link set lo up

#configure addreses
sudo ip netns exec r1 ip addr add 10.0.12.1/24 dev r1-r2
sudo ip netns exec r2 ip addr add 10.0.12.2/24 dev r2-r1
sudo ip netns exec r2 ip addr add 10.0.23.2/24 dev r2-r3
sudo ip netns exec r3 ip addr add 10.0.23.3/24 dev r3-r2

#enable interfaces
sudo ip netns exec r1 ip link set r1-r2 up
sudo ip netns exec r2 ip link set r2-r1 up
sudo ip netns exec r2 ip link set r2-r3 up
sudo ip netns exec r3 ip link set r3-r2 up

# add dummy addresses to be broadcasted by RIP

#R1
sudo ip netns exec r1 ip link add lan1 type dummy
sudo ip netns exec r1 ip addr add 10.1.1.0/24 dev lan1
sudo ip netns exec r1 ip link set lan1 up

#R3
sudo ip netns exec r3 ip link add lan3 type dummy
sudo ip netns exec r3 ip addr add 10.3.3.0/24/24 dev lan3
sudo ip netns exec r3 ip link set lan3 up

#enable forwarding ip packets
sudo ip netns exec r1 sysctl -w net.ipv4.ip_forward=1
sudo ip netns exec r2 sysctl -w net.ipv4.ip_forward=1
sudo ip netns exec r3 sysctl -w net.ipv4.ip_forward=1