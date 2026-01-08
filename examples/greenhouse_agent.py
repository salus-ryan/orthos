#!/usr/bin/env python3
"""
ORTHOS Greenhouse Agent - The First Agent

This demonstrates Constraint-Based Control: the agent doesn't "decide" to turn on
the heater. It insists on survival, and the heater turning on becomes a
*necessary consequence* of that insistence.

The Paradigm Shift:
    Imperative:  if temp < 20: turn_on_heater()
    Orthos:      constrain(Temp >= 20) -> query(Heater) -> act on derived truth

Usage:
    python greenhouse_agent.py
"""

import json
import subprocess
import sys
import time
import random
from pathlib import Path

# ANSI colors for terminal output
class Colors:
    HEADER = '\033[95m'
    BLUE = '\033[94m'
    CYAN = '\033[96m'
    GREEN = '\033[92m'
    YELLOW = '\033[93m'
    RED = '\033[91m'
    ENDC = '\033[0m'
    BOLD = '\033[1m'

class OrthosDaemon:
    """Client for the Orthos Daemon JSON-RPC protocol."""
    
    def __init__(self, daemon_path: str):
        self.daemon_path = daemon_path
        self.process = None
        self.request_id = 0
    
    def start(self):
        """Spawn the daemon process."""
        self.process = subprocess.Popen(
            [self.daemon_path],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            bufsize=1
        )
        print(f"{Colors.CYAN}[DAEMON] Started orthos-daemon{Colors.ENDC}")
    
    def stop(self):
        """Shutdown the daemon."""
        if self.process:
            self._send("shutdown")
            self.process.terminate()
            self.process.wait()
            print(f"{Colors.CYAN}[DAEMON] Stopped{Colors.ENDC}")
    
    def _send(self, method: str, params: dict = None) -> dict:
        """Send a JSON-RPC request and return the response."""
        self.request_id += 1
        request = {
            "method": method,
            "id": self.request_id
        }
        if params:
            request["params"] = params
        
        request_json = json.dumps(request)
        self.process.stdin.write(request_json + "\n")
        self.process.stdin.flush()
        
        response_line = self.process.stdout.readline()
        if not response_line:
            raise RuntimeError("Daemon closed unexpectedly")
        
        return json.loads(response_line)
    
    def initialize(self, source: str) -> dict:
        """Initialize the daemon with Orthos source code."""
        return self._send("initialize", {"source": source})
    
    def checkpoint(self) -> dict:
        """Create a checkpoint for hypothetical reasoning."""
        return self._send("checkpoint")
    
    def restore(self) -> dict:
        """Restore to the last checkpoint."""
        return self._send("restore")
    
    def constrain(self, laws: list) -> dict:
        """Add dynamic constraints. Auto-rollback on UNSAT."""
        return self._send("constrain", {"laws": laws})
    
    def query(self, flux: list) -> dict:
        """Query the current values of flux variables."""
        return self._send("query", {"flux": flux})


class GreenhouseEnvironment:
    """Simulates the physical greenhouse with entropy (decay)."""
    
    def __init__(self):
        # Start in a comfortable state
        self.temp = 23  # 23°C
        self.moisture = 50  # 50%
    
    def tick(self, heater_on: bool, cooler_on: bool, sprinkler_on: bool):
        """Advance time by one step, applying entropy and actuator effects."""
        # === ENTROPY (Environmental Decay) ===
        # Temperature drifts toward ambient (15°C) with noise
        ambient = 15
        self.temp += (ambient - self.temp) * 0.1 + random.uniform(-1, 1)
        
        # Moisture evaporates
        self.moisture -= random.uniform(2, 5)
        
        # === ACTUATOR EFFECTS ===
        if heater_on:
            self.temp += 3  # Heater adds 3°C
        if cooler_on:
            self.temp -= 3  # Cooler removes 3°C
        if sprinkler_on:
            self.moisture += 15  # Sprinkler adds 15% moisture
        
        # Clamp to physical limits
        self.temp = max(0, min(50, self.temp))
        self.moisture = max(0, min(100, self.moisture))
        
        return int(self.temp), int(self.moisture)


def status_bar(temp: int, moisture: int, heater: bool, cooler: bool, sprinkler: bool):
    """Print a visual status bar."""
    # Temperature indicator
    if temp < 18:
        temp_color = Colors.BLUE
        temp_status = "COLD"
    elif temp > 28:
        temp_color = Colors.RED
        temp_status = "HOT"
    else:
        temp_color = Colors.GREEN
        temp_status = "OK"
    
    # Moisture indicator
    if moisture < 30:
        moist_color = Colors.YELLOW
        moist_status = "DRY"
    elif moisture > 70:
        moist_color = Colors.BLUE
        moist_status = "WET"
    else:
        moist_color = Colors.GREEN
        moist_status = "OK"
    
    # Actuator states
    heater_str = f"{Colors.RED}ON{Colors.ENDC}" if heater else "off"
    cooler_str = f"{Colors.CYAN}ON{Colors.ENDC}" if cooler else "off"
    sprinkler_str = f"{Colors.BLUE}ON{Colors.ENDC}" if sprinkler else "off"
    
    print(f"  {Colors.BOLD}│{Colors.ENDC} Temp: {temp_color}{temp:3d}°C [{temp_status:4s}]{Colors.ENDC} "
          f"│ Moisture: {moist_color}{moisture:3d}% [{moist_status:3s}]{Colors.ENDC} "
          f"│ Heater: {heater_str} │ Cooler: {cooler_str} │ Sprinkler: {sprinkler_str} │")


def main():
    print(f"\n{Colors.BOLD}{Colors.HEADER}╔══════════════════════════════════════════════════════════════╗{Colors.ENDC}")
    print(f"{Colors.BOLD}{Colors.HEADER}║         ORTHOS GREENHOUSE AGENT - The First Agent            ║{Colors.ENDC}")
    print(f"{Colors.BOLD}{Colors.HEADER}║     Constraint-Based Control: Survival Through Logic         ║{Colors.ENDC}")
    print(f"{Colors.BOLD}{Colors.HEADER}╚══════════════════════════════════════════════════════════════╝{Colors.ENDC}\n")
    
    # Find the daemon binary
    script_dir = Path(__file__).parent.parent
    daemon_path = script_dir / "target" / "release" / "orthos-daemon"
    if not daemon_path.exists():
        daemon_path = script_dir / "target" / "debug" / "orthos-daemon"
    if not daemon_path.exists():
        print(f"{Colors.RED}[ERROR] orthos-daemon not found. Build it first:{Colors.ENDC}")
        print(f"  cd orthos-daemon && cargo build --release")
        sys.exit(1)
    
    # Load the greenhouse ontology
    orth_path = Path(__file__).parent / "greenhouse.orth"
    with open(orth_path) as f:
        source = f.read()
    
    # Initialize
    daemon = OrthosDaemon(str(daemon_path))
    env = GreenhouseEnvironment()
    
    try:
        daemon.start()
        
        # Initialize the ontology
        print(f"{Colors.CYAN}[INIT] Loading greenhouse.orth...{Colors.ENDC}")
        result = daemon.initialize(source)
        if "error" in result and result["error"]:
            print(f"{Colors.RED}[ERROR] Failed to initialize: {result['error']}{Colors.ENDC}")
            sys.exit(1)
        
        flux_list = result.get("result", {}).get("flux_list", [])
        print(f"{Colors.GREEN}[INIT] Universe is SAT. Flux: {flux_list}{Colors.ENDC}")
        print()
        
        print(f"{Colors.BOLD}Starting homeostasis loop (Ctrl+C to stop)...{Colors.ENDC}")
        print(f"{Colors.BOLD}{'─' * 80}{Colors.ENDC}")
        
        cycle = 0
        while True:
            cycle += 1
            print(f"\n{Colors.BOLD}[Cycle {cycle}]{Colors.ENDC}")
            
            # === STEP 1: OBSERVE (Read sensors) ===
            temp, moisture = int(env.temp), int(env.moisture)
            print(f"  {Colors.YELLOW}[OBSERVE]{Colors.ENDC} Sensors: Temp={temp}°C, Moisture={moisture}%")
            
            # === STEP 2: CHECKPOINT (Enter hypothetical reasoning) ===
            # This creates an isolated sandbox for "what if" reasoning
            daemon.checkpoint()
            
            # === STEP 3: CONSTRAIN (Assert current sensor readings) ===
            # We tell the daemon: "Given this is the current reality..."
            # The solver will derive what actuators are NECESSARY for survival
            constraints = [
                f"Greenhouse.TempNow == {temp}",
                f"Greenhouse.MoistureNow == {moisture}"
            ]
            print(f"  {Colors.BLUE}[CONSTRAIN]{Colors.ENDC} Asserting: {constraints}")
            
            result = daemon.constrain(constraints)
            status = result.get("result", {}).get("status", "ERROR")
            
            if status == "UNSAT":
                # Current reality violates survival laws - the plant is dying!
                print(f"  {Colors.RED}[UNSAT]{Colors.ENDC} Current state violates survival laws!")
                print(f"  {Colors.RED}        THE PLANT IS DYING - no actuator combination can save it!{Colors.ENDC}")
                daemon.restore()
                
                # Emergency: try to recover anyway
                heater = temp < 18
                cooler = temp > 28
                sprinkler = moisture < 30
                status_bar(temp, moisture, heater, cooler, sprinkler)
                env.tick(heater, cooler, sprinkler)
                time.sleep(1)
                continue
            
            # === STEP 4: QUERY (Derive necessary actuator states) ===
            # The solver finds actuator values that satisfy ALL laws
            # including survival constraints. The agent doesn't "decide" -
            # it asks "what must be true for survival?"
            print(f"  {Colors.GREEN}[SAT]{Colors.ENDC} Reality is consistent with survival.")
            
            query_result = daemon.query([
                "Greenhouse.Heater",
                "Greenhouse.Cooler", 
                "Greenhouse.Sprinkler"
            ])
            values = query_result.get("result", {}).get("values", {})
            
            heater = values.get("Greenhouse.Heater", False)
            cooler = values.get("Greenhouse.Cooler", False)
            sprinkler = values.get("Greenhouse.Sprinkler", False)
            
            # Handle None values (solver didn't constrain them)
            if heater is None:
                heater = False
            if cooler is None:
                cooler = False
            if sprinkler is None:
                sprinkler = False
            
            print(f"  {Colors.CYAN}[DERIVE]{Colors.ENDC} Necessary actions: Heater={heater}, Cooler={cooler}, Sprinkler={sprinkler}")
            
            # === STEP 5: RESTORE (Exit hypothetical, discard this cycle's constraints) ===
            # Critical: this ensures each cycle starts fresh
            daemon.restore()
            
            # === STEP 6: ACT (Apply derived actions to environment) ===
            status_bar(temp, moisture, heater, cooler, sprinkler)
            
            # === STEP 7: TICK (Advance time, apply entropy) ===
            env.tick(heater, cooler, sprinkler)
            
            time.sleep(1)
    
    except KeyboardInterrupt:
        print(f"\n\n{Colors.YELLOW}[SHUTDOWN] Agent terminated by user.{Colors.ENDC}")
    finally:
        daemon.stop()
    
    print(f"\n{Colors.GREEN}The ghost has left the machine.{Colors.ENDC}\n")


if __name__ == "__main__":
    main()
