#!/usr/bin/env python3

from pydantic import BaseModel
import yaml
import argparse
import os
import datetime

from experiments.lib import run_experiment
from experiments import *

class Plan(BaseModel):
    experiment: str
    parameters: list[dict]

def get_base_directory():
    timestamp: str = datetime.datetime.now().strftime("%Y%m%d_%H%M%S")
    return f"experiment_logs_{timestamp}"

def init_experiment_run_directory(base_dir: str, exp: str, n: int):
    output_dir = f"{base_dir}/{exp}_{n}"
    os.makedirs(output_dir, exist_ok=True)
    return output_dir


def main():
    parser = argparse.ArgumentParser(description="Run an experiment plan")
    parser.add_argument("plan_files", type=str, nargs="+", help="Path to the experiment plan YAML file")
    args = parser.parse_args()
    base_dir = get_base_directory()
    os.makedirs(base_dir, exist_ok=True)

    for plan_file in args.plan_files:
        plan_data: dict

        with open(plan_file, "r") as f:
            plan_data = yaml.safe_load(f)

        plan = Plan(**plan_data)

        for n, params in enumerate(plan.parameters):
            output_dir = init_experiment_run_directory(base_dir, plan.experiment, n)
            print(f"Running experiment '{plan.experiment}' with parameters: {params}")
            run_experiment(plan.experiment, params, output_dir)


if __name__ == "__main__":
    main()
