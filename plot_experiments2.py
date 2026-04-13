#!/usr/bin/env python3

import matplotlib.pyplot as plt
import numpy as np
import os
import argparse
import yaml
from pydantic import BaseModel

class PhcPoll(BaseModel):
    phc_time: float

class PTP4LLog(BaseModel):
    path_delay: float
    offset_from_master: float
    frequency: float
    state: str

class RealTimeEvent(BaseModel):
    delay_sec: int
    delay_nsec: int

class Data(BaseModel):
    PhcPollingResult: 'PhcPoll | None' = None
    Ptp4lLog: 'PTP4LLog | None' = None
    RealTime: 'RealTimeEvent | None' = None

class Event(BaseModel):
    message_type: str
    node: str
    relative_timestamp: float
    data: Data

def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Plot experiment results")
    parser.add_argument("--results_dir", type=str, required=True, help="Directory containing experiment results")
    parser.add_argument("--output_file", type=str, default="experiment_results.png", help="Output file for the plot")
    return parser.parse_args()

def load_events(results_dir: str) -> list[Event]:
    """
    Return a list of loaded Events, for plotting.
    """
    # Read the events.yaml, which contains a list of event objects
    events_file = os.path.join(results_dir, "events.yaml")
    with open(events_file, "r") as f:
        events = [ Event(**event_dict) for event_dict in yaml.safe_load_all(f) if event_dict is not None ]
    return events

def main():
    args = parse_args()
    events = load_events(args.results_dir)

    ptp4lLogEvents = [ event for event in events if event.data.Ptp4lLog is not None ]
    phcPollEvents = [ event for event in events if event.data.PhcPollingResult is not None ]
    realTimeEvents = [ event for event in events if event.data.RealTime is not None ]

    # Create multiple subplots in one figure
    # Figure 1: Plot path delay with respect to time
    # Also plot the frequency as a red line on the same plot, with a secondary y-axis
    path_delays = [ (event.relative_timestamp, event.data.Ptp4lLog.path_delay) for event in ptp4lLogEvents ]
    frequency_offset = [ (event.relative_timestamp, event.data.Ptp4lLog.frequency) for event in ptp4lLogEvents ]
    delay_adjustments = [ (event.relative_timestamp, event.data.RealTime.delay_sec + event.data.RealTime.delay_nsec / 1e9) for event in realTimeEvents ]

    plt.figure(figsize=(10, 14))
    plt.subplot(4, 1, 1)
    plt.plot([t for t, _ in delay_adjustments], [d for _, d in delay_adjustments], label="Delay Adjustments")
    plt.xlabel("Time (s)")
    plt.ylabel("Delay Adjustment (s)")
    plt.title("Delay Adjustments over Time")
    plt.legend()
    plt.grid()

    plt.subplot(4, 1, 2)
    plt.plot([t for t, _ in path_delays], [d for _, d in path_delays], label="Path Delay")
    plt.xlabel("Time (s)")
    plt.ylabel("Path Delay (s)")

    # Plot frequency on the same plot with a secondary y-axis
    ax2 = plt.gca().twinx()
    ax2.plot([t for t, _ in frequency_offset], [f for _, f in frequency_offset], label="Frequency Offset", color="red")
    ax2.set_ylabel("Frequency Offset (ppm)", color="red")
    ax2.tick_params(axis='y', labelcolor="red")
    
    plt.title("Path Delay over Time")
    plt.legend()
    plt.grid()

    # Figure 2: Get offset from master with respect to time
    offsets = [ (event.relative_timestamp, event.data.Ptp4lLog.offset_from_master) for event in ptp4lLogEvents ]

    plt.subplot(4, 1, 3)
    plt.plot([t for t, _ in offsets], [o for _, o in offsets], label="Offset from Master")
    plt.xlabel("Time (s)")
    plt.ylabel("Offset (s)")
    plt.title("Offset from Master over Time")
    plt.legend()
    plt.grid()

    # Figure 3: Delta between node2 PHC relative to node1 PHC, with respect to time
    # Get the PHC polling results for node1 and node2
    node1_phc_poll_events = [ event for event in phcPollEvents if event.node == "node1" ]
    node2_phc_poll_events = [ event for event in phcPollEvents if event.node == "node2" ]

    # The relative timestamps are slightly offset, so we need to interpolate the PHC polling results to get the delta between node2 and node1 at the same timestamps
    node1_times = [ event.relative_timestamp for event in node1_phc_poll_events ]
    node1_phc_times = [ event.data.PhcPollingResult.phc_time for event in node1_phc_poll_events ]
    node2_times = [ event.relative_timestamp for event in node2_phc_poll_events ]
    node2_phc_times = [ event.data.PhcPollingResult.phc_time for event in node2_phc_poll_events ]

    # Interpolate node1 PHC times at node2 timestamps
    node1_phc_times_interp = np.interp(node2_times, node1_times, node1_phc_times)
    phc_deltas = [ node2_phc - node1_phc for node2_phc, node1_phc in zip(node2_phc_times, node1_phc_times_interp) ]

    plt.subplot(4, 1, 4)
    plt.plot(node2_times, phc_deltas, label="PHC Delta (Node2 - Node1)")
    plt.xlabel("Time (s)")
    plt.ylabel("PHC Delta (s)")
    plt.title("Delta between Node2 PHC and Node1 PHC over Time")
    plt.legend()
    plt.grid()
    plt.tight_layout()
    plt.savefig(args.output_file)

if __name__ == "__main__":
    main()