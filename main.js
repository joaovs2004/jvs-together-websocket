import WebSocket, { WebSocketServer } from 'ws';
import { v4 as uuidv4 } from 'uuid';
import got from 'got';

const wss = new WebSocketServer({ port: 8080 });

const rooms = {};

wss.on('connection', function connection(ws) {
    const userId = uuidv4();
    ws.isAlive = true;

    const interval = setInterval(function ping() {
        if (ws.isAlive === false) return ws.terminate();

        ws.isAlive = false;
        ws.send(JSON.stringify({ type: "ping" }));
    }, 30000);

    ws.on('message', async function message(data, isBinary) {
        const parsedData = JSON.parse(data);

        switch(parsedData.type) {
            case "setName":
                rooms[parsedData.roomId].clients[parsedData.id].name = parsedData.name;

                for(const client of Object.values(rooms[parsedData.roomId].clients)) {
                    client.ws.send(JSON.stringify({ type: "connectedClients", clients: getClientsNames(rooms[parsedData.roomId].clients) }), { binary: isBinary });
                }

                break;
            case "setReady":
                rooms[parsedData.roomId].isReadyCount++;

                if(rooms[parsedData.roomId].isReadyCount == Object.keys(rooms[parsedData.roomId].clients).length) {
                    for(const client of Object.values(rooms[parsedData.roomId].clients)) {
                        client.ws.send(JSON.stringify({ type: "setPlaying", status: true }), { binary: isBinary });
                    }

                    rooms[parsedData.roomId].isReadyCount = 0;
                }

                break;
            case "sendToRoom":
                const clientsInRoom = rooms[parsedData.roomId];

                if(clientsInRoom == undefined) {
                    rooms[parsedData.roomId] = {
                        currentVideo: "",
                        clients: {},
                        isReadyCount: 0,
                        history: []
                    };
                }

                rooms[parsedData.roomId].clients[parsedData.clientId] = {
                    id: userId,
                    name: userId,
                    ws,
                };

                ws.send(JSON.stringify({ type: "connectedClients", clients: getClientsNames(rooms[parsedData.roomId].clients) }));
                ws.send(JSON.stringify({type: "updateHistory", history: rooms[parsedData.roomId].history}));

                if(rooms[parsedData.roomId].currentVideo != "") {
                    ws.send(JSON.stringify({ type: "setVideo", videoId: rooms[parsedData.roomId].currentVideo, broadcast: true, roomId: parsedData.roomId }));
                }

                break;
            case "setVideo":
                if(rooms[parsedData.roomId].currentVideo == parsedData.videoId) {
                    break;
                }

                rooms[parsedData.roomId].currentVideo = parsedData.videoId;
                rooms[parsedData.roomId].isReadyCount = 0;

                for(const client of Object.values(rooms[parsedData.roomId].clients)) {
                    if(client.ws.readyState === WebSocket.OPEN) {
                        client.ws.send(data, { binary: isBinary });
                    }
                }

                const info = await got.get(`https://search.w2g.tv/w2g_search/lookup?url=//www.youtube.com/watch?v=${parsedData.videoId}`).json();

                rooms[parsedData.roomId].history.push({videoId: parsedData.videoId, title: info.title});

                for(const client of Object.values(rooms[parsedData.roomId].clients)) {
                    if(client.ws.readyState === WebSocket.OPEN) {
                        client.ws.send(JSON.stringify({type: "updateHistory", history: rooms[parsedData.roomId].history}), { binary: isBinary });
                    }
                }

                break;
            case "pong":
                ws.isAlive = true;

                break;
            default:
                for(const client of Object.values(rooms[parsedData.roomId].clients)) {
                    if ((client.ws !== ws || parsedData.broadcast) && client.ws.readyState === WebSocket.OPEN) {
                        client.ws.send(data, { binary: isBinary });
                    }
                }

                break;
        }
    });

    ws.on("close", function close() {
        let roomIdtoSend;

        clearInterval(interval);

        for(const room of Object.entries(rooms)) {
            for(const client of Object.values(room[1].clients)) {
                if(ws == client.ws) {
                    roomIdtoSend = room[0];
                    delete rooms[roomIdtoSend].clients[client.id];
                    break;
                }
            }
        }

        if(!roomIdtoSend) return;

        if(Object.values(rooms[roomIdtoSend].clients).length == 0) {
            delete rooms[roomIdtoSend];
            return;
        }

        for(const client of Object.values(rooms[roomIdtoSend].clients)) {
            client.ws.send(JSON.stringify({ type: "connectedClients", clients: getClientsNames(rooms[roomIdtoSend].clients) }), { binary: false });
        }
    });

    ws.send(JSON.stringify({ type: "clientConnected", id: userId }));

});

function getClientsNames(clients) {
    const clientsNames = Object.values(clients).map((client) => { return client.name });

    return  clientsNames;
}