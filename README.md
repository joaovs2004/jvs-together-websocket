# Jvs Together WebSocket

This project implements a WebSocket server in **Rust** using the **tokio** and **tokio-tungstenite** crates. The server allows real-time, bidirectional communication over WebSocket connections, which will be used by [Jvs Together Client](https://github.com/joaovs2004/jvs-together-client), a project that allows users to watch YouTube videos simultaneously with others.

## Features

- **Real-time Communication:** Establish WebSocket connections that allow bidirectional communication between the server and clients.
- **Error Handling:** Built-in robust error handling.

## Tech Stack

- **Rust**: The language used to build the server.
- **tokio**: An asynchronous runtime for Rust, providing efficient concurrency.
- **tokio-tungstenite**: WebSocket implementation built on top of tokio for asynchronous WebSocket connections.
- **serde**: For serializing and deserializing messages.
- **Youtube Data API**: The API used to get information about the youtube videos.

## Getting Started

### Prerequisites

Before you begin, ensure you have the following:

- **Rust**: Install Rust from [https://www.rust-lang.org/tools/install](https://www.rust-lang.org/tools/install)
- **Cargo**: Cargo comes bundled with Rust; it's used to manage dependencies and build the project
- **Youtube Data API**: Get your Youtube Data Api at https://console.cloud.google.com

### Running the project

1. Clone the repository:
   ```bash
   git clone https://github.com/joaovs2004/jvs-together-websocket
   cd jvs-together-websocket
   ```
2. Create a .env with your Youtube Data API key:
    ```bash
   echo 'YOUTUBE_API_KEY="your_api_key"' > .env
   ```
3. Run the project with:
    ```bash
   cargo run
   ```

The server will be available at ws://localhost:9001