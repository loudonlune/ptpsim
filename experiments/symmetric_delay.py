
from pydantic import BaseModel

from experiments.lib.utils import Timespec
from experiments.lib import Experiment, register_experiment
from experiments.lib.ptpsim import *

import time

class LinearRampUpParameters(BaseModel):
    # Number of discrete delay increase steps to go through
    steps: int
    # Total duration of the process in seconds
    duration: float        

class SymmetricDelayParameters(BaseModel):
    base_delay: Timespec
    delay_increase: Timespec
    warmup_time: float
    ramp_up: LinearRampUpParameters
    settle_time: float

class SymmetricDelayExperiment(Experiment):
    topology_file: str = "topologies/two_nodes.toml"
    parameters: SymmetricDelayParameters
    output_dir: str

    def set_up(self, params: dict, output_dir: str):
        # Base delay default is 100 microseconds
        self.parameters = SymmetricDelayParameters(**params)
        self.output_dir = output_dir

    def run(self):
        start_ptpsim(self.topology_file, self.output_dir)
        set_delay("node1", 0, self.parameters.base_delay.sec, self.parameters.base_delay.nsec)
        set_delay("node2", 0, self.parameters.base_delay.sec, self.parameters.base_delay.nsec)
        
        # Wait for warm-up time
        time.sleep(self.parameters.warmup_time)        

        current_delay = self.parameters.base_delay
        increment = self.parameters.delay_increase.divide(self.parameters.ramp_up.steps)
        step_wait_time = self.parameters.ramp_up.duration / self.parameters.ramp_up.steps

        # Begin delay ramp-up
        for step in range(1, self.parameters.ramp_up.steps + 1):
            current_delay = current_delay.add(increment)
            set_delay("node1", 0, current_delay.sec, current_delay.nsec)
            set_delay("node2", 0, current_delay.sec, current_delay.nsec)
            time.sleep(step_wait_time)

        # Wait for settle time
        time.sleep(self.parameters.settle_time)
        stop_ptpsim()

register_experiment("symmetric_delay", SymmetricDelayExperiment())
