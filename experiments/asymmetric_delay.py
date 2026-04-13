
from pydantic import BaseModel

from experiments.lib.utils import Timespec
from experiments.lib import Experiment, register_experiment
from experiments.lib.ptpsim import *
from experiments.symmetric_delay import LinearRampUpParameters

class AsymmetricDelayParameters(BaseModel):
    base_delay: Timespec
    delay_increase: Timespec
    asymmetry: float
    warmup_time: float
    ramp_up: LinearRampUpParameters
    settle_time: float

class AsymmetricDelayExperiment(Experiment):

    def set_up(self, params, output_dir):
        self.parameters = AsymmetricDelayParameters(**params)
        self.output_dir = output_dir

    def run(self):
        start_ptpsim("topologies/two_nodes.toml", self.output_dir)
        set_delay("node1", 0, self.parameters.base_delay.sec, self.parameters.base_delay.nsec)
        set_delay("node2", 0, self.parameters.base_delay.sec, self.parameters.base_delay.nsec)
        
        # Wait for warm-up time
        time.sleep(self.parameters.warmup_time)

        current_delay = self.parameters.base_delay
        other_current_delay = self.parameters.base_delay
        increment = self.parameters.delay_increase.divide(self.parameters.ramp_up.steps)
        otherIncrement = increment.multiply(self.parameters.asymmetry)
        step_wait_time = self.parameters.ramp_up.duration / self.parameters.ramp_up.steps

        # Begin delay ramp-up on node1 only
        for step in range(1, self.parameters.ramp_up.steps + 1):
            current_delay = current_delay.add(increment)
            other_current_delay = other_current_delay.add(otherIncrement)
            set_delay("node1", 0, current_delay.sec, current_delay.nsec)
            set_delay("node2", 0, other_current_delay.sec, other_current_delay.nsec)
            time.sleep(step_wait_time)

        # Wait for settle time
        time.sleep(self.parameters.settle_time)
        stop_ptpsim()
        
register_experiment("asymmetric_delay", AsymmetricDelayExperiment())
