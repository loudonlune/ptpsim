
import subprocess
import time
import requests

ptpsim_process = None

def start_ptpsim(topology_path: str, output_dir: str) -> subprocess.Popen:
    """
    Start the ptpsim simulator with the given topology file.

    Args:
        topology_path (str): The path to the topology file.
        output_dir (str): The directory where output files will be stored for this experiment.
    Returns:
        subprocess.Popen: The process object for the running simulator.
    """
    global ptpsim_process
    command = ["sudo", "ptpsim", "--topology", topology_path, "--output-dir", output_dir]

    if ptpsim_process is not None:
        raise Exception("ptpsim is already running")

    # Start the simulator as a subprocess
    process = subprocess.Popen(command, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    ptpsim_process = process

    time.sleep(1)  # Wait a moment for the simulator to start
    print('Process: ', str(type(ptpsim_process)))

    return process

def check_ptpsim_running() -> bool:
    """
    Check if the ptpsim simulator is currently running.

    Returns:
        bool: True if the simulator is running, False otherwise.
    """
    global ptpsim_process

    if ptpsim_process is not None:
        poll_result = ptpsim_process.poll()
        if poll_result is not None and poll_result != 0:
            print('WARNING: ptpsim has crashed unexpectedly')
            print('LOG: ', ptpsim_process.stdout.read().decode())
            print('ERR: ', ptpsim_process.stderr.read().decode())
            ptpsim_process = None

    print('Process: ', str(type(ptpsim_process)))


    return ptpsim_process is not None

def set_link(node_id: str, other_node_id: str, port: int, other_port: int, present: bool):
    """
    Set the link between two nodes in the ptpsim simulator.

    Args:
        node_id (str): The ID of the first node.
        other_node_id (str): The ID of the second node.
        port (int): The port number on the first node.
        other_port (int): The port number on the second node.
        present (bool): Whether the link is present or not.
    """
    if not check_ptpsim_running():
        raise Exception("ptpsim is not running")

    url = f"http://localhost:8300/link/{node_id}/{port}/{other_node_id}/{other_port}"
    
    if present:
        response = requests.post(url)
    else:
        response = requests.delete(url)

    if response.status_code != 200:
        raise Exception(f"Failed to set link: {response.text}")

def set_delay(node_id: str, port: int, delay_sec: int, delay_nsec: int):
    """
    Set the delay for a specific node and port in the ptpsim simulator.

    Args:
        node_id (str): The ID of the node to set the delay for.
        port (int): The port number to set the delay for.
        delay_sec (int): The delay in seconds.
        delay_nsec (int): The delay in nanoseconds.
    """
    if not check_ptpsim_running():
        raise Exception("ptpsim is not running")

    url = f"http://localhost:8300/delay/{node_id}/{port}"
    data = {
        "sec": delay_sec,
        "nsec": delay_nsec
    }
    response = requests.put(url, json=data)
    if response.status_code != 200:
        raise Exception(f"Failed to set delay: {response.text}")


def stop_ptpsim():
    # Stop ptpsim by sending Ctrl+C signal
    global ptpsim_process

    if not check_ptpsim_running():
        raise Exception("ptpsim is not running")

    ptpsim_process.send_signal(subprocess.signal.SIGINT)
    ptpsim_process.wait()

    ptpsim_process = None
