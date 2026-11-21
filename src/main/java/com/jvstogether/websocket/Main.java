package com.jvstogether.websocket;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import org.java_websocket.WebSocket;
import org.java_websocket.handshake.ClientHandshake;
import org.java_websocket.server.WebSocketServer;

import java.net.InetSocketAddress;
import java.net.URI;
import java.net.http.HttpClient;
import java.net.http.HttpRequest;
import java.net.http.HttpResponse;
import java.util.*;
import java.util.concurrent.Executors;
import java.util.concurrent.ScheduledExecutorService;
import java.util.concurrent.TimeUnit;

public final class Main extends WebSocketServer {
    private static final ObjectMapper JSON = new ObjectMapper();
    private static final long SYNC_INTERVAL_SECONDS = 2;
    private final Map<WebSocket, Client> clients = new HashMap<>();
    private final Map<String, Room> rooms = new HashMap<>();
    private final ScheduledExecutorService syncScheduler = Executors.newSingleThreadScheduledExecutor();
    private final HttpClient http = HttpClient.newHttpClient();

    private Main(int port) {
        super(new InetSocketAddress("0.0.0.0", port));
    }

    public static void main(String[] args) {
        int port = Integer.parseInt(System.getenv().getOrDefault("PORT", "9001"));
        Main server = new Main(port);
        server.start();
        server.syncScheduler.scheduleAtFixedRate(server::synchronizeRooms,
                SYNC_INTERVAL_SECONDS, SYNC_INTERVAL_SECONDS, TimeUnit.SECONDS);
        System.out.println("WebSocket listening on port " + port);
    }

    @Override public synchronized void onOpen(WebSocket conn, ClientHandshake handshake) {
        clients.put(conn, new Client(UUID.randomUUID().toString()));
    }

    @Override public void onMessage(WebSocket conn, String raw) {
        try {
            JsonNode msg = JSON.readTree(raw);
            String type = msg.path("type").asText();
            synchronized (this) { handle(conn, type, msg); }
        } catch (Exception e) {
            System.err.println("Ignoring invalid client message: " + e.getMessage());
        } finally {
            send(conn, message("unlockSetVideo"));
        }
    }

    private void handle(WebSocket conn, String type, JsonNode msg) throws Exception {
        Client client = clients.get(conn);
        if (client == null) return;
        String roomId = msg.path("roomId").asText("");
        switch (type) {
            case "setName" -> {
                Room room = rooms.get(roomId);
                if (room != null && room.users.containsKey(conn)) {
                    room.users.put(conn, msg.path("name").asText(client.id));
                    client.name = msg.path("name").asText(client.id);
                    broadcastClients(roomId, room);
                }
            }
            case "sendToRoom" -> {
                Room room = rooms.computeIfAbsent(roomId, ignored -> new Room());
                client.roomId = roomId;
                room.users.put(conn, client.name);
                broadcast(roomId, message("updateHistory", "history", room.history));
                broadcastClients(roomId, room);
                if (!room.videoId.isEmpty()) sendVideoAndState(conn, room);
            }
            case "setReady" -> {
                Room room = rooms.get(roomId);
                if (room != null && ++room.readyCount >= room.users.size()) {
                    room.readyCount = 0;
                    room.playing = true;
                    room.updatedAt = System.nanoTime();
                    broadcast(roomId, message("setPlaying", "status", true));
                }
            }
            case "setVideo" -> setVideo(roomId, msg.path("url").asText(""));
            case "setPlaying" -> {
                Room room = rooms.get(roomId);
                if (room != null) {
                    room.position = currentPosition(room);
                    room.playing = msg.path("status").asBoolean();
                    room.updatedAt = System.nanoTime();
                    broadcast(roomId, message("setPlaying", "status", room.playing));
                }
            }
            case "seeked" -> {
                Room room = rooms.get(roomId);
                if (room != null) {
                    room.position = Math.max(0, msg.path("time").asDouble());
                    room.updatedAt = System.nanoTime();
                    broadcast(roomId, message("seeked", "time", room.position));
                }
            }
            case "setPlaybackRate" -> {
                Room room = rooms.get(roomId);
                if (room != null) {
                    room.rate = Math.max(0.25, Math.min(2.0, msg.path("rate").asDouble(1.0)));
                    broadcast(roomId, message("setPlaybackRate", "rate", room.rate));
                }
            }
            case "rewind" -> {
                Room room = rooms.get(roomId);
                if (room != null) {
                    boolean announce = !room.rewindAnnounced;
                    room.rewindAnnounced = true;
                    broadcast(roomId, message("rewind", "seconds", msg.path("seconds").asInt(), "shouldAnnounce", announce));
                }
            }
            case "pong" -> { }
            default -> { }
        }
    }

    private void setVideo(String roomId, String rawUrl) throws Exception {
        String videoId = youtubeVideoId(rawUrl);
        if (videoId.isEmpty()) return;
        Room room = rooms.computeIfAbsent(roomId, ignored -> new Room());
        if (room.videoId.equals(videoId)) return;
        VideoInfo info = fetchVideo(videoId);
        if (info == null) return;
        room.videoId = videoId;
        room.position = 0;
        room.playing = false;
        room.rate = 1.0;
        room.updatedAt = System.nanoTime();
        room.readyCount = 0;
        room.rewindAnnounced = false;
        room.history.add(Map.of("url", rawUrl, "videoId", videoId, "title", info.title));
        if (!info.restricted) {
            broadcast(roomId, message("setVideo", "videoId", videoId, "isRestrictedVideo", false));
            broadcast(roomId, message("updateHistory", "history", room.history));
        }
    }

    private VideoInfo fetchVideo(String videoId) throws Exception {
        String key = System.getenv("YOUTUBE_API_KEY");
        if (key == null || key.isBlank()) key = readDotEnv("YOUTUBE_API_KEY");
        if (key == null || key.isBlank()) {
            System.err.println("YOUTUBE_API_KEY is required to validate videos");
            return null;
        }
        URI uri = URI.create("https://www.googleapis.com/youtube/v3/videos?part=contentDetails,snippet&id=" + videoId + "&key=" + key);
        HttpRequest request = HttpRequest.newBuilder(uri).GET().build();
        JsonNode response = JSON.readTree(http.send(request, HttpResponse.BodyHandlers.ofString()).body());
        JsonNode item = response.path("items").path(0);
        if (item.isMissingNode()) return null;
        return new VideoInfo(item.path("snippet").path("title").asText(""),
                !item.path("contentDetails").path("contentRating").path("ytRating").isMissingNode());
    }

    private static String readDotEnv(String name) {
        try (var lines = java.nio.file.Files.lines(java.nio.file.Path.of(".env"))) {
            return lines.map(String::trim).filter(line -> line.startsWith(name + "="))
                    .map(line -> line.substring(name.length() + 1).trim())
                    .map(value -> value.replaceAll("^[\\\"']|[\\\"']$", ""))
                    .findFirst().orElse(null);
        } catch (Exception ignored) { return null; }
    }

    private synchronized void synchronizeRooms() {
        long now = System.nanoTime();
        for (Map.Entry<WebSocket, Client> entry : clients.entrySet()) {
            if (now - entry.getValue().lastPing >= TimeUnit.SECONDS.toNanos(20)) {
                send(entry.getKey(), message("ping"));
                entry.getValue().lastPing = now;
            }
        }
        for (Map.Entry<String, Room> entry : rooms.entrySet()) {
            Room room = entry.getValue();
            if (room.videoId.isEmpty()) continue;
            room.position = currentPosition(room);
            room.updatedAt = System.nanoTime();
            broadcast(entry.getKey(), message("setPlaying", "status", room.playing));
            broadcast(entry.getKey(), message("seeked", "time", room.position));
            broadcast(entry.getKey(), message("setPlaybackRate", "rate", room.rate));
        }
    }

    private double currentPosition(Room room) {
        if (!room.playing) return room.position;
        return room.position + (System.nanoTime() - room.updatedAt) / 1_000_000_000.0 * room.rate;
    }

    private void sendVideoAndState(WebSocket conn, Room room) {
        send(conn, message("setVideo", "videoId", room.videoId, "isRestrictedVideo", false));
        send(conn, message("setPlaying", "status", room.playing));
        send(conn, message("seeked", "time", currentPosition(room)));
        send(conn, message("setPlaybackRate", "rate", room.rate));
    }

    private void broadcastClients(String roomId, Room room) {
        List<String> names = new ArrayList<>(room.users.values());
        broadcast(roomId, message("connectedClients", "clients", names));
    }

    private void broadcast(String roomId, Map<String, Object> message) {
        Room room = rooms.get(roomId);
        if (room == null) return;
        for (WebSocket socket : room.users.keySet()) send(socket, message);
    }

    private void send(WebSocket socket, Map<String, Object> message) {
        if (socket == null || !socket.isOpen()) return;
        try { socket.send(JSON.writeValueAsString(message)); }
        catch (Exception e) { System.err.println("WebSocket send failed: " + e.getMessage()); }
    }

    private static Map<String, Object> message(String type, Object... fields) {
        Map<String, Object> result = new LinkedHashMap<>();
        result.put("type", type);
        for (int i = 0; i < fields.length; i += 2) result.put((String) fields[i], fields[i + 1]);
        return result;
    }

    private static String youtubeVideoId(String value) {
        try {
            URI uri = URI.create(value);
            String host = uri.getHost();
            if (host == null) return "";
            if (host.equals("youtu.be")) return uri.getPath().replaceFirst("^/", "");
            if (!host.equals("youtube.com") && !host.equals("www.youtube.com")) return "";
            if (uri.getPath().startsWith("/shorts/")) return uri.getPath().substring(8).split("/")[0];
            String query = uri.getRawQuery();
            if (query != null) for (String part : query.split("&")) {
                String[] pair = part.split("=", 2);
                if (pair[0].equals("v")) return java.net.URLDecoder.decode(pair.length > 1 ? pair[1] : "", java.nio.charset.StandardCharsets.UTF_8);
            }
        } catch (Exception ignored) { }
        return "";
    }

    @Override public synchronized void onClose(WebSocket conn, int code, String reason, boolean remote) {
        Client client = clients.remove(conn);
        if (client == null || client.roomId == null) return;
        Room room = rooms.get(client.roomId);
        if (room == null) return;
        room.users.remove(conn);
        room.readyCount = 0;
        if (room.users.isEmpty()) rooms.remove(client.roomId);
        else broadcastClients(client.roomId, room);
    }

    @Override public void onError(WebSocket conn, Exception ex) {
        System.err.println("WebSocket error: " + ex.getMessage());
    }

    @Override public void onStart() { }

    private static final class Client {
        final String id;
        String name;
        String roomId;
        long lastPing = System.nanoTime();
        Client(String id) { this.id = id; this.name = id; }
    }

    private static final class Room {
        final Map<WebSocket, String> users = new LinkedHashMap<>();
        final List<Map<String, String>> history = new ArrayList<>();
        int readyCount;
        String videoId = "";
        double position;
        double rate = 1.0;
        long updatedAt = System.nanoTime();
        boolean playing;
        boolean rewindAnnounced;
    }

    private record VideoInfo(String title, boolean restricted) { }
}
