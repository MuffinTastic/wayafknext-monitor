import socket
import json
import time
import threading

SOCKET_PATH = "target/debug/wayafknext.sock"

# ClientInput (client-to-server)
#  {"Quit":null}\n
#  {"StartWatch":[2,3]}\n
#  {"StopWatch":null}\n
# Broadcast (server-to-client)
#  {"WatchEvent":{"StatusIdle":true}}\n
#  {"WatchEvent":{"StatusIdle":false}}\n
#  {"WatchEvent":{"NotifsIdle":true}}\n
#  {"WatchEvent":{"NotifsIdle":false}}\n
#  {"WatchStarted":[2,3]}\n
#  {"WatchStopped":null}\n

def recv_loop(sock):
    buffer = ""
    while True:
        try:
            data = sock.recv(4096)
            if not data:
                print("Server closed connection.")
                break
            buffer += data.decode("utf-8")
            while "\n" in buffer:
                line, buffer = buffer.split("\n", 1)
                if line.strip():
                    try:
                        message = json.loads(line)
                        print("Broadcast:", message)
                    except json.JSONDecodeError:
                        print("Invalid JSON from server:", line)
        except Exception as e:
            print("Receive error:", e)
            break


def send_json(sock, payload):
    msg = json.dumps(payload) + "\n"
    sock.sendall(msg.encode("utf-8"))
    print("Sent:", payload)


def main():
    sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    sock.connect(SOCKET_PATH)
    print(f"Connected to {SOCKET_PATH}")

    threading.Thread(target=recv_loop, args=(sock,), daemon=True).start()

    try:
        send_json(sock, {"StartWatch":  { "status_mins": 2, "notifs_mins": 3 }})
        time.sleep(30)
        send_json(sock, {"StopWatch": None})
        time.sleep(2)
        send_json(sock, {"StartWatch":  { "status_mins": 5, "notifs_mins": 2 }})
        time.sleep(30)
        send_json(sock, {"StopWatch": None})
        time.sleep(1)
        send_json(sock, {"Quit": None})
        time.sleep(1)
    finally:
        sock.close()
        print("Connection closed.")


if __name__ == "__main__":
    main()
