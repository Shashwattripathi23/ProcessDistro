#!/usr/bin/env python3
"""
Node Simulator for ProcessDistro Testing

Simulates multiple edge nodes for testing the controller
without requiring actual distributed hardware.
"""

import asyncio
import json
import random
import time
from typing import List, Dict, Optional


class SimulatedNode:
    def __init__(self, node_id: str, capabilities: Dict):
        self.node_id = node_id
        self.capabilities = capabilities
        self.is_connected = False
        self.current_tasks = []

    async def connect_to_controller(self, controller_addr: str):
        """Simulate connection to controller"""
        print(f"Node {self.node_id} connecting to {controller_addr}")
        await asyncio.sleep(random.uniform(0.1, 0.5))
        self.is_connected = True
        print(f"Node {self.node_id} connected")

    async def execute_task(self, task: Dict):
        """Simulate task execution"""
        task_type = task.get('task_type', 'unknown')
        execution_time = random.uniform(1.0, 10.0)  # 1-10 seconds

        print(
            f"Node {self.node_id} executing {task_type} (ETA: {execution_time:.1f}s)")
        await asyncio.sleep(execution_time)

        # Simulate occasional failures
        if random.random() < 0.05:  # 5% failure rate
            print(f"Node {self.node_id} task failed")
            return {"success": False, "error": "Simulated failure"}

        print(f"Node {self.node_id} task completed")
        return {
            "success": True,
            "result": f"Task {task['id']} completed by {self.node_id}",
            "execution_time": execution_time
        }


class NodeSimulator:
    def __init__(self, num_nodes: int = 5):
        self.nodes = []
        for i in range(num_nodes):
            capabilities = {
                "cpu_cores": random.choice([2, 4, 8, 16]),
                "memory_gb": random.choice([4, 8, 16, 32]),
                "cpu_frequency": random.uniform(2.0, 4.0),
                "wasm_runtime": "wasmtime-25.0"
            }
            node = SimulatedNode(f"sim_node_{i:03d}", capabilities)
            self.nodes.append(node)

    async def start_simulation(self, controller_addr: str = "localhost:30000"):
        """Start the node simulation"""
        print(f"Starting simulation with {len(self.nodes)} nodes")

        # Connect all nodes
        connect_tasks = []
        for node in self.nodes:
            connect_tasks.append(node.connect_to_controller(controller_addr))

        await asyncio.gather(*connect_tasks)

        # Keep nodes running
        while True:
            await asyncio.sleep(1)
            # TODO: Implement heartbeat and task request loops


if __name__ == "__main__":
    simulator = NodeSimulator(num_nodes=5)
    asyncio.run(simulator.start_simulation())
