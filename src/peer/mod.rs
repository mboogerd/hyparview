pub struct Connection<T> {
    peer: Peer
}

pub trait Peer<T> {
    fn send(payload: T)
}

pub trait Environment {

}

pub trait Room<T> {

}

struct Key

pub trait KeyMap<T> {
    fn key_to_var(): HashSet<(Key, &mut )>
}



/*
- Peer/Connection must be reusable across modules
- MUST allow sending messages
- Should allow keeping track of connection metrics
- Should allow linking details of the other party (topics of interest)
- Could allow linking up higher level protocols (Encapsulation would work here probably)

Current conclusion:
- Each module defines its own traits, for its needs
- Struct is defined from the outside, implements each of the traits in modules where it is used.

But, modules are concurrent, we don't want to have threads synchronize over peer updates.
- Each thread could own part of the interface
- Common part of the interface (id / address) could just be a copy as it is fixed. Allows dispatching data
- Shared parts (metrics?) could be copies if eventual consistency is sufficient.

Current conclusions:
- Have a copy per thread of details that are fixed (address)
- Have a thread owner for each other part of the interface
- Have thread subscribers when interested but not owner. Subscribers auto update with changes of the thread owner

Question: How to synchronize across actors using the main messaging flow with minimal overhead?
- DistributedPubSubBroker can receive / replicate all messages related to variable updates.
- ActorWithBroker could be created to have a single side-channel for an actor distributing messages between it and a central broker.

Concepts:

Environment - The whole. Performs a role similar to a broker in messaging. Allows any kind of partitioning into independent Rooms.
Partitioning is first done through namespaces, and second through types. Environments can be forked. A forked environment starts empty.
A spawned room is automatically shared between all forked environments, assuming its types are compatible. Incompatible types implies having concurrent rooms.

- owner: Agent
- rooms: Map[Namespace, Map[Type, Room]]
- occupations: Set[(Agent, Room)] // bidirectionally indexed
- spawnRoom[T](namespace): Room[T]
- connect(environmentRef)
- disconnect(environmentRef)


Room - The parts. Occupants of a room can examine the current state of the room and modify the state of the room. Contrary to ordinary rooms, Rooms are concurrent.
This means they can be taken offline and modified in isolation. Isolated changes can be integrated at any point in time.

Room[T]
- occupants: Set[Agent]
- examine: T
- modify(f: T => T)
- enter(agent)
- leave(agent)

Messages:
- Connect(EnvRef)
- Disconnect(EnvRef)
- Entered(Set[(Agent, Room)])
- Left(Set[(Agent, Room)])
- Modified(Room, delta: T)
*/
