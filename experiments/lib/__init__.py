
from abc import ABC, abstractmethod

class Experiment(ABC):
    def set_up(self, params: dict, output_dir: str): ...
    @abstractmethod
    def run(self): ...

REGISTRY: dict[str, Experiment] = {}

def register_experiment(name: str, experiment: Experiment):
    if name in REGISTRY:
        raise Exception(f"Experiment with name '{name}' is already registered")
    REGISTRY[name] = experiment

def run_experiment(name: str, params: dict, output_dir: str):
    if name not in REGISTRY:
        raise Exception(f"Experiment with name '{name}' is not registered")
    experiment = REGISTRY[name]
    experiment.set_up(params, output_dir)
    experiment.run()
