## How to use after clonging the repository

#### 1. Open the terminal and navigate to the directory where the repository is cloned.

> [!IMPORTANT]
> Before the following points make sure, that 127.0.0.1:9999 and 127.0.0.1:3030 is not in use.

#### 2. Run the following command to start a server:

```plaintext
cargo run -- -s
```

#### 3. Run the following command to start a proxy server:

```plaintext
cargo run -- -p
```

#### 4. Run the following command to start a client:

```plaintext
cargo run -- -c
```

#### 4. As the client, you can send messages to the server by typing them in the terminal.

#### Examples:

```plaintext
{ "jsonrpc": "2.0", "method": "say_hello", "params": "null", "id":1 }
```

```plaintext
{ "jsonrpc": "2.0", "method": "add", "params": [1, 2], "id":2 }
```

#### 5. You can exit by sending the following message as the client:

```plaintext
{ "jsonrpc": "2.0", "method": "exit", "params": null, "id": 3 }
```

### Github link for jsonrpsee integration
