import subprocess
import json
import sys
import time

def run_test(child_cmd):
    # start the child process
    proc = subprocess.Popen(
        child_cmd,
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        bufsize=1  # line-buffered
    )

    def send(line: str):
        print(f">>> {line.strip()}")
        proc.stdin.write(line + "\n")
        proc.stdin.flush()

    def read_json():
        line = proc.stdout.readline()
        if not line:
            return None
        line = line.strip()
        print(f"<<< {line}")
        try:
            return json.loads(line)
        except json.JSONDecodeError:
            return {"Error": f"Invalid JSON: {line}"}

    # send "1" to create a watch with a 1 minute duration
    send("1")
    msg = read_json()
    if msg and "Error" in msg:
        print("Child reported error:", msg["Error"])
        proc.terminate()
        return

    # print back the status message to the user
    print("Status:", msg)

    # wait for {"Idle": true}
    while True:
        msg = read_json()
        if msg is None:
            break
        if "Error" in msg:
            print("Child reported error:", msg["Error"])
            break
        if msg.get("Idle") is True:
            print("Got Idle:true")
            break

    # wait for {"Idle": false}
    while True:
        msg = read_json()
        if msg is None:
            break
        if "Error" in msg:
            print("Child reported error:", msg["Error"])
            break
        if msg.get("Idle") is False:
            print("Got Idle:false")
            break

    # stop the watch
    send("stop")
    msg = read_json()
    print("Status:", msg)

    # quit the child process and exit
    send("quit")
    proc.wait(timeout=5)
    print("Child process exited.")

if __name__ == "__main__":
    run_test("target/debug/wayafknext-monitor")
