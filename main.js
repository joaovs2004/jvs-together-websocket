import WebSocket, { WebSocketServer } from 'ws';
import { v4 as uuidv4 } from 'uuid';
import got from 'got';
import "dotenv/config";

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
                        currentVideoPayload: "",
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

                if(rooms[parsedData.roomId].currentVideoPayload != "") {
                    ws.send(rooms[parsedData.roomId].currentVideoPayload);
                }

                break;
            case "setVideo":
                let videoUrl;

                try {
                    videoUrl = new URL(parsedData.url);
                } catch {
                    ws.send(JSON.stringify({ type: "unlockSetVideo" }));
                    break;
                }


                let videoId;

                if (!["www.youtube.com", "youtube.com", "youtu.be"].includes(videoUrl.hostname)) {
                    ws.send(JSON.stringify({ type: "unlockSetVideo" }));
                    break;
                }

                // Handle youtu.be
                if (videoUrl.pathname == "/watch") {
                    videoId = videoUrl.searchParams.get("v");
                } else if(videoUrl.hostname == "youtu.be") {
                    videoId = videoUrl.pathname.substring(1);
                } else {
                    videoId = videoUrl.pathname.substring(8);
                }

                // Don't change video if it's the same as the current video
                if (rooms[parsedData.roomId].currentVideo == videoId) {
                    ws.send(JSON.stringify({ type: "unlockSetVideo" }));
                    break;
                }

                let basicInfo;

                try {
                    // Gather basic video information (title & restrictions)
                    basicInfo = await got.get(`${process.env.INVIDIOUS_INSTANCE_URL}/api/v1/videos/${videoId}?fields=title,isFamilyFriendly`).json();

                } catch (error) {
                    console.log(error);
                    ws.send(JSON.stringify({ type: "unlockSetVideo" }));

                    break;
                }

                let payload = "";

                rooms[parsedData.roomId].currentVideo = videoId;
                rooms[parsedData.roomId].isReadyCount = 0;
                rooms[parsedData.roomId].history.push({ url: parsedData.url, title: basicInfo.title, videoId: videoId });

                if (basicInfo.isFamilyFriendly) {
                    payload = JSON.stringify({ type: "setVideo", videoId: videoId, isRestrictedVideo: false });

                    for(const client of Object.values(rooms[parsedData.roomId].clients)) {
                        if(client.ws.readyState === WebSocket.OPEN) {
                            client.ws.send(payload);
                            client.ws.send(JSON.stringify({ type: "updateHistory", history: rooms[parsedData.roomId].history }), { binary: isBinary });
                        }
                    }
                } else {
                    const info = await got.get(`${process.env.PIPED_API_URL}/streams/${videoId}`).json();
                    const tracks = [...info.audioStreams, ...info.videoStreams];

                    payload = JSON.stringify({ type: "setVideo", videoId: videoId, tracks: tracks, duration: info.duration, thumbnail: info.thumbnailUrl, isRestrictedVideo: true });

                    for (const client of Object.values(rooms[parsedData.roomId].clients)) {
                        if (client.ws.readyState === WebSocket.OPEN) {
                            client.ws.send(payload);
                            client.ws.send(JSON.stringify({ type: "updateHistory", history: rooms[parsedData.roomId].history }), { binary: isBinary });
                        }
                    }
                }

                ws.send(JSON.stringify({ type: "unlockSetVideo" }));

                rooms[parsedData.roomId].currentVideoPayload = payload;

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