# teeobserver

This tool continuously fetches the DDNet master server and observes the changes, broadcasting the events to connected websockets.

By default it listens to `127.0.0.1:3000` and the websocket connection is at `ws://localhost:3000/ws`

Port can be changed with the `PORT` environment variable.

```
> RUST_LOG="teeobserver=debug"
> teeobserver
2023-10-02T14:18:54.933535Z DEBUG teeobserver::util: making request to master
2023-10-02T14:18:55.419999Z DEBUG teeobserver::util: got 1139 servers
2023-10-02T14:18:55.421385Z  INFO teeobserver: listening on http://127.0.0.1:3000
2023-10-02T14:18:55.422382Z DEBUG teeobserver::util: making request to master
2023-10-02T14:18:55.525406Z DEBUG teeobserver::util: got 1139 servers
2023-10-02T14:18:55.530730Z  INFO teeobserver: sent 0 events to 0 receivers
2023-10-02T14:18:57.627921Z  INFO teeobserver::routes: `websocket: Mozilla/5.0 (X11; Linux x86_64; rv:109.0) Gecko/20100101 Firefox/118.0` at 127.0.0.1:44564 connected.
2023-10-02T14:19:05.422137Z DEBUG teeobserver::util: making request to master
2023-10-02T14:19:05.554810Z DEBUG teeobserver::util: got 1139 servers
2023-10-02T14:19:05.559945Z  INFO teeobserver: sent 68 events to 1 receivers
2023-10-02T14:19:15.421549Z DEBUG teeobserver::util: making request to master
2023-10-02T14:19:15.540133Z DEBUG teeobserver::util: got 1139 servers
2023-10-02T14:19:15.545310Z  INFO teeobserver: sent 75 events to 1 receivers
```

To connect with a simple webpage:

```html
<html>
<body>
</body>
<script>
// Create WebSocket connection.
const socket = new WebSocket("ws://localhost:3000/ws");

// Listen for messages
socket.addEventListener("message", (event) => {
  console.log("Message from server ", event.data);
});

</script>
</html>
```


Useful typescript interfaces:

```typescript

// Event structure send to the websocket

export interface MasterEvent {
    observers: number
    event: Event
    time: string
}

export interface Event {
    ClientJoined?: ClientJoined
    ClientLeft?: ClientLeft
    ServerWentOffline?: ServerWentOffline
    ServerWentOnline?: ServerWentOnline
}

export interface ClientLeft {
    client: Client
    server: Server
}

export interface ClientJoined {
    client: Client
    server: Server
}

export interface ServerWentOffline {
    addresses: string[]
    info: Info
    location: string
}

export interface ServerWentOnline {
    addresses: string[]
    info: Info
    location: string
}

export interface Client {
    afk?: boolean
    clan: string
    country: number
    is_player: boolean
    name: string
    score: number
    skin: Skin
    team?: number
}

export interface Skin {
    color_body?: number
    color_feet?: number
    name: string
}

export interface Server {
    addresses: string[]
    info: Info
    location: string
}

export interface Info {
    client_score_kind?: string
    clients: Client[]
    game_type: string
    map: Map
    max_clients: number
    max_players: number
    name: string
    passworded: boolean
    version: string
}

export interface Map {
    name: string
    sha256?: string
    size: number
}

```
