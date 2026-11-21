# Jvs Together WebSocket

WebSocket server for [Jvs Together Client](https://github.com/joaovs2004/jvs-together-client), which lets people watch YouTube videos in sync. The server is implemented in Java 17 and listens on port `9001` by default.

## Run

Set the YouTube Data API key, then start the server with Maven:

```bash
export YOUTUBE_API_KEY="your_api_key"
mvn package
java -jar target/jvs-together-websocket-1.0.0.jar
```

Set `PORT` to change the listening port. The server is also available through the included Dockerfile; pass `YOUTUBE_API_KEY` to the container environment.

## Synchronization

The server keeps the current video position, play/pause status, and playback rate for each room. While a video is playing, its position advances using elapsed server time. Every two seconds the server broadcasts `setPlaying`, `seeked`, and `setPlaybackRate` using the existing client message format. New clients receive the current video and playback state on joining, so a missed event is corrected at the next sync update.

Video metadata and age restriction validation use the YouTube Data API. Set `YOUTUBE_API_KEY` for `setVideo` requests to work.
