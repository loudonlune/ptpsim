#!/usr/bin/env python3

import argparse
import matplotlib.pyplot as plt
import re

def adjust_timestamps(timestamps):
    """
    Adjusts timestamps to start from zero.

    Parameters:
    - timestamps: List of timestamp values.

    Returns:
    - adjusted_timestamps: List of adjusted timestamp values.
    """
    if not timestamps:
        return []

    start_time = timestamps[0]
    adjusted_timestamps = [t - start_time for t in timestamps]
    return adjusted_timestamps

def convert_to_microseconds(value: list[float], exponent: int) -> list[float]:
    """
    Converts a list of time values from any division of a second to microseconds.

    Parameters:
    - value: List of time values.
    - exponent: Exponent indicating the exponent of the input unit unit (e.g., 0 for seconds to microseconds).

    Returns:
    - List of time values in microseconds.
    """
    return [v * (10 ** (6 + exponent)) for v in value]

def plot_ptp4l_data(timestamps: list[float], offsets: list[float], delays: list[float], output_file: str, device_name: str):
    """
    Plots PTP4L data including timestamps, offsets, and delays.

    Parameters:
    - timestamps: List of timestamp values.
    - offsets: List of offset values corresponding to the timestamps.
    - delays: List of delay values corresponding to the timestamps.
    - output_file: Filename to save the plot image.
    """
    plt.figure(figsize=(12, 6))

    # From nanoseconds to microseconds
    offsets = convert_to_microseconds(offsets, -9)
    delays = convert_to_microseconds(delays, -9)

    # Plot offsets
    plt.subplot(2, 1, 1)
    plt.plot(timestamps, offsets, label='Offset', color='blue')
    plt.title(f'{device_name} Offset Over Time')
    plt.xlabel('Time (seconds)')
    plt.ylabel('Offset (microseconds)')
    #plt.ylim(-1000, 1000)
    plt.grid(True)
    plt.legend()

    # Plot delays
    plt.subplot(2, 1, 2)
    plt.plot(timestamps, delays, label='Delay', color='orange')
    plt.title(f'{device_name} Link Delay Over Time')
    plt.xlabel('Time (seconds)')
    plt.ylabel('Delay (microseconds)')
    #plt.ylim(-100000, 100000)
    plt.grid(True)
    plt.legend()

    plt.tight_layout()
    plt.savefig(output_file)
    plt.close()

def read_ptp4l_log(file_path):
    """
    Reads a PTP4L log file and extracts timestamps, offsets, and delays.

    Parameters:
    - file_path: Path to the PTP4L log file.

    Returns:
    - timestamps: List of timestamp values.
    - offsets: List of offset values.
    - delays: List of delay values.
    """
    timestamps = []
    offsets = []
    delays = []

    with open(file_path, 'r') as file:
        for line in file:
            if 'offset' in line and 'delay' in line:
                parts = line.split()

                timestamp_part = parts[0]
                timestamp_str = re.search(r'ptp4l\[(\d+\.\d+)\]', timestamp_part).group(1)

                timestamp = float(timestamp_str)  # Assuming the first part is the timestamp
                offset = float(parts[parts.index('offset') + 1])
                delay = float(parts[parts.index('delay') + 1])

                timestamps.append(timestamp)
                offsets.append(offset)
                delays.append(delay)

    timestamps = adjust_timestamps(timestamps)
    return timestamps, offsets, delays

def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description='Plot PTP4L log data.')
    parser.add_argument('logfile', type=str, help='Path to the PTP4L log file.')
    parser.add_argument('--output', type=str, default='ptp4l_graph.png', help='Output image file name.')
    parser.add_argument('--device', type=str, default='Device', help='Device name for labeling the plots.')
    return parser.parse_args()

def main():
    args = parse_args()
    timestamps, offsets, delays = read_ptp4l_log(args.logfile)
    plot_ptp4l_data(timestamps, offsets, delays, args.output, args.device)

if __name__ == "__main__":
    main()
